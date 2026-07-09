mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::setup_portal_app;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn refresh_rotation_preserves_family_and_replay_revokes_it() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };
    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");
    let app = setup_portal_app(pool.clone(), "portal-session-rotation-test-secret");
    let email = format!("rotation-{}@example.com", Uuid::new_v4());

    let register_response = app
        .clone()
        .oneshot(request_json(
            "/v1/auth/register",
            json!({
                "email": email,
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(register_response.status(), StatusCode::OK);
    let original_refresh = cookie_value(&register_response, "refresh_token");
    let original_hash = Sha256::digest(original_refresh.as_bytes()).to_vec();
    let (family_id, portal_user_id): (Uuid, String) =
        sqlx::query_as("SELECT family_id, user_id FROM refresh_tokens WHERE token_hash = $1")
            .bind(&original_hash)
            .fetch_one(&pool)
            .await
            .expect("original refresh token should be stored");

    let refresh_response = app
        .clone()
        .oneshot(request_with_cookie(
            "/v1/auth/refresh",
            &format!("refresh_token={original_refresh}"),
        ))
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let replacement_refresh = cookie_value(&refresh_response, "refresh_token");
    assert_ne!(replacement_refresh, original_refresh);

    let replacement_hash = Sha256::digest(replacement_refresh.as_bytes()).to_vec();
    let replacement_family: Uuid =
        sqlx::query_scalar("SELECT family_id FROM refresh_tokens WHERE token_hash = $1")
            .bind(&replacement_hash)
            .fetch_one(&pool)
            .await
            .expect("replacement refresh token should be stored");
    assert_eq!(replacement_family, family_id);

    let replay_response = app
        .clone()
        .oneshot(request_with_cookie(
            "/v1/auth/refresh",
            &format!("refresh_token={original_refresh}"),
        ))
        .await
        .unwrap();
    assert_eq!(replay_response.status(), StatusCode::UNAUTHORIZED);

    let active_family_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM refresh_tokens WHERE family_id = $1 AND revoked = FALSE",
    )
    .bind(family_id)
    .fetch_one(&pool)
    .await
    .expect("active family count should load");
    assert_eq!(
        active_family_rows, 0,
        "replay must revoke the complete token family"
    );

    cleanup_identity(&pool, &portal_user_id).await;
}

#[tokio::test]
async fn password_change_and_logout_all_revoke_active_sessions() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };
    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");
    let app = setup_portal_app(pool.clone(), "portal-session-revocation-test-secret");
    let email = format!("revoke-{}@example.com", Uuid::new_v4());
    let original_password = "correct horse battery staple";
    let new_password = "another correct horse staple";

    let register_response = app
        .clone()
        .oneshot(request_json(
            "/v1/auth/register",
            json!({
                "email": email,
                "password": original_password
            }),
        ))
        .await
        .unwrap();
    assert_eq!(register_response.status(), StatusCode::OK);
    let auth_token = cookie_value(&register_response, "auth_token");
    let refresh_token = cookie_value(&register_response, "refresh_token");
    let portal_user_id: String =
        sqlx::query_scalar("SELECT user_id FROM refresh_tokens WHERE token_hash = $1")
            .bind(Sha256::digest(refresh_token.as_bytes()).to_vec())
            .fetch_one(&pool)
            .await
            .expect("registered session should exist");

    let change_password = request_json_method(
        "/v1/portal/settings/security",
        "PUT",
        json!({
            "currentPassword": original_password,
            "newPassword": new_password
        }),
        Some(&format!(
            "auth_token={auth_token}; refresh_token={refresh_token}"
        )),
    );
    let change_response = app.clone().oneshot(change_password).await.unwrap();
    assert_eq!(change_response.status(), StatusCode::OK);
    assert_eq!(active_session_count(&pool, &portal_user_id).await, 0);

    let login_response = app
        .clone()
        .oneshot(request_json(
            "/v1/auth/login",
            json!({
                "email": email,
                "password": new_password
            }),
        ))
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let new_auth = cookie_value(&login_response, "auth_token");
    let new_refresh = cookie_value(&login_response, "refresh_token");
    assert_eq!(active_session_count(&pool, &portal_user_id).await, 1);

    let logout_all = request_json_method(
        "/v1/portal/auth/logout-all",
        "POST",
        json!({}),
        Some(&format!(
            "auth_token={new_auth}; refresh_token={new_refresh}"
        )),
    );
    let logout_response = app.clone().oneshot(logout_all).await.unwrap();
    assert_eq!(logout_response.status(), StatusCode::OK);
    assert_eq!(active_session_count(&pool, &portal_user_id).await, 0);

    cleanup_identity(&pool, &portal_user_id).await;
}

fn request_json(path: &str, body: serde_json::Value) -> Request<Body> {
    request_json_method(path, "POST", body, None)
}

fn request_json_method(
    path: &str,
    method: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .uri(path)
        .method(method)
        .header("Content-Type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("Cookie", cookie);
    }
    builder
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn request_with_cookie(path: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .method("POST")
        .header("Cookie", cookie)
        .body(Body::empty())
        .unwrap()
}

fn cookie_value(response: &axum::response::Response, name: &str) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie| {
            cookie
                .strip_prefix(&format!("{name}="))
                .and_then(|value| value.split(';').next())
                .map(str::to_string)
        })
        .unwrap_or_else(|| panic!("missing {name} cookie"))
}

async fn active_session_count(pool: &PgPool, portal_user_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 AND revoked = FALSE")
        .bind(portal_user_id)
        .fetch_one(pool)
        .await
        .expect("active session count should load")
}

async fn cleanup_identity(pool: &PgPool, portal_user_id: &str) {
    let (tenant_id, financial_user_id): (String, String) =
        sqlx::query_as("SELECT tenant_id, financial_user_id FROM portal_users WHERE id = $1")
            .bind(portal_user_id)
            .fetch_one(pool)
            .await
            .expect("portal identity links should load");
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(portal_user_id)
        .execute(pool)
        .await
        .expect("refresh tokens should clean up");
    sqlx::query("DELETE FROM portal_users WHERE id = $1")
        .bind(portal_user_id)
        .execute(pool)
        .await
        .expect("portal user should clean up");
    sqlx::query("DELETE FROM users WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id)
        .bind(financial_user_id)
        .execute(pool)
        .await
        .expect("financial user should clean up");
}
