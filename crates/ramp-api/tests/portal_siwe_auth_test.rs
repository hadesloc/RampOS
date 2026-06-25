mod common;

use alloy_primitives::{keccak256, Address};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use common::setup_portal_app;
use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn siwe_login_creates_unified_identity_and_rejects_replay() {
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

    let private_key = [0x42; 32];
    let signing_key = SigningKey::from_bytes((&private_key).into()).unwrap();
    let address = address_from_key(&signing_key);
    cleanup_wallet(&pool, &address).await;
    let app = setup_portal_app(pool.clone(), "portal-siwe-auth-test-secret");

    let nonce_response = app
        .clone()
        .oneshot(request_json(
            "/v1/auth/wallet/nonce",
            json!({ "address": address }),
        ))
        .await
        .unwrap();
    assert_eq!(nonce_response.status(), StatusCode::OK);
    let nonce_body = response_json(nonce_response).await;
    let message = nonce_body["message"].as_str().unwrap().to_string();
    let signature = sign_personal_message(message.as_bytes(), &signing_key);

    let verify_body = json!({
        "message": message,
        "signature": format!("0x{}", hex::encode(signature))
    });
    let verify_response = app
        .clone()
        .oneshot(request_json("/v1/auth/wallet/verify", verify_body.clone()))
        .await
        .unwrap();
    assert_eq!(verify_response.status(), StatusCode::OK);
    let body = response_json(verify_response).await;
    assert_eq!(body["user"]["walletAddress"], address);

    let portal_id = body["user"]["id"].as_str().unwrap();
    let linked: (String, String, String) = sqlx::query_as(
        r#"
        SELECT portal.id, portal.financial_user_id, portal.wallet_address
        FROM portal_users portal
        WHERE portal.id = $1
        "#,
    )
    .bind(portal_id)
    .fetch_one(&pool)
    .await
    .expect("portal identity should exist");
    assert_eq!(linked.0, linked.1);
    assert_eq!(linked.2, address);

    let replay_response = app
        .clone()
        .oneshot(request_json("/v1/auth/wallet/verify", verify_body))
        .await
        .unwrap();
    assert_eq!(replay_response.status(), StatusCode::UNAUTHORIZED);

    cleanup_wallet(&pool, &address).await;
}

#[tokio::test]
async fn authenticated_password_identity_can_link_a_wallet() {
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

    let signing_key = SigningKey::from_bytes((&[0x43; 32]).into()).unwrap();
    let address = address_from_key(&signing_key);
    cleanup_wallet(&pool, &address).await;
    let app = setup_portal_app(pool.clone(), "portal-siwe-link-test-secret");
    let email = format!("link-{}@example.com", Uuid::new_v4());

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
    let session_cookie = cookie_header(&register_response);

    let nonce_response = app
        .clone()
        .oneshot(request_json_with_cookie(
            "/v1/portal/auth/wallet/link/nonce",
            json!({ "address": address }),
            &session_cookie,
        ))
        .await
        .unwrap();
    assert_eq!(nonce_response.status(), StatusCode::OK);
    let nonce_body = response_json(nonce_response).await;
    let message = nonce_body["message"].as_str().unwrap().to_string();
    let signature = sign_personal_message(message.as_bytes(), &signing_key);

    let verify_response = app
        .clone()
        .oneshot(request_json_with_cookie(
            "/v1/portal/auth/wallet/link/verify",
            json!({
                "message": message,
                "signature": format!("0x{}", hex::encode(signature))
            }),
            &session_cookie,
        ))
        .await
        .unwrap();
    assert_eq!(verify_response.status(), StatusCode::OK);
    let linked = response_json(verify_response).await;
    assert_eq!(linked["walletAddress"], address);

    let methods: Vec<String> =
        sqlx::query_scalar("SELECT auth_methods FROM portal_users WHERE email_normalized = $1")
            .bind(&email)
            .fetch_one(&pool)
            .await
            .expect("linked auth methods should load");
    assert_eq!(methods, vec!["password", "wallet"]);

    let second_email = format!("link-conflict-{}@example.com", Uuid::new_v4());
    let second_register = app
        .clone()
        .oneshot(request_json(
            "/v1/auth/register",
            json!({
                "email": second_email,
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(second_register.status(), StatusCode::OK);
    let second_cookie = cookie_header(&second_register);
    let conflict_nonce = app
        .clone()
        .oneshot(request_json_with_cookie(
            "/v1/portal/auth/wallet/link/nonce",
            json!({ "address": address }),
            &second_cookie,
        ))
        .await
        .unwrap();
    assert_eq!(conflict_nonce.status(), StatusCode::OK);
    let conflict_message = response_json(conflict_nonce).await["message"]
        .as_str()
        .unwrap()
        .to_string();
    let conflict_signature = sign_personal_message(conflict_message.as_bytes(), &signing_key);
    let conflict_verify = app
        .clone()
        .oneshot(request_json_with_cookie(
            "/v1/portal/auth/wallet/link/verify",
            json!({
                "message": conflict_message,
                "signature": format!("0x{}", hex::encode(conflict_signature))
            }),
            &second_cookie,
        ))
        .await
        .unwrap();
    assert_eq!(conflict_verify.status(), StatusCode::CONFLICT);

    cleanup_email(&pool, &second_email).await;
    cleanup_wallet(&pool, &address).await;
}

fn address_from_key(signing_key: &SigningKey) -> String {
    let public_key = signing_key.verifying_key().to_encoded_point(false);
    let hash = keccak256(&public_key.as_bytes()[1..]);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    format!("0x{}", hex::encode(Address::from(address).as_slice()))
}

fn sign_personal_message(message: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut payload = Vec::with_capacity(prefix.len() + message.len());
    payload.extend_from_slice(prefix.as_bytes());
    payload.extend_from_slice(message);
    let hash = keccak256(payload);
    let (signature, recovery_id): (k256::ecdsa::Signature, _) =
        signing_key.sign_prehash(hash.as_slice()).unwrap();

    let mut output = vec![0u8; 65];
    output[..64].copy_from_slice(&signature.to_bytes());
    output[64] = recovery_id.to_byte() + 27;
    output
}

fn request_json(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .uri(path)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn request_json_with_cookie(path: &str, body: Value, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Cookie", cookie)
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn cookie_header(response: &axum::response::Response) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn cleanup_wallet(pool: &PgPool, wallet: &str) {
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM portal_users WHERE lower(wallet_address) = lower($1)")
            .bind(wallet)
            .fetch_all(pool)
            .await
            .expect("wallet identity lookup should succeed");
    for id in ids {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
            .bind(&id)
            .execute(pool)
            .await
            .expect("refresh tokens should clean up");
        sqlx::query("DELETE FROM portal_users WHERE id = $1")
            .bind(&id)
            .execute(pool)
            .await
            .expect("portal identity should clean up");
        sqlx::query(
            "DELETE FROM users WHERE tenant_id = '11111111-1111-1111-1111-111111111111' AND id = $1",
        )
        .bind(&id)
        .execute(pool)
        .await
        .expect("financial identity should clean up");
    }
    sqlx::query("DELETE FROM portal_auth_nonces WHERE lower(address) = lower($1)")
        .bind(wallet)
        .execute(pool)
        .await
        .expect("wallet nonces should clean up");
}

async fn cleanup_email(pool: &PgPool, email: &str) {
    let id: Option<String> =
        sqlx::query_scalar("SELECT id FROM portal_users WHERE email_normalized = lower($1)")
            .bind(email)
            .fetch_optional(pool)
            .await
            .expect("email identity lookup should succeed");
    if let Some(id) = id {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
            .bind(&id)
            .execute(pool)
            .await
            .expect("refresh tokens should clean up");
        sqlx::query("DELETE FROM portal_users WHERE id = $1")
            .bind(&id)
            .execute(pool)
            .await
            .expect("portal identity should clean up");
        sqlx::query(
            "DELETE FROM users WHERE tenant_id = '11111111-1111-1111-1111-111111111111' AND id = $1",
        )
        .bind(&id)
        .execute(pool)
        .await
        .expect("financial identity should clean up");
    }
}
