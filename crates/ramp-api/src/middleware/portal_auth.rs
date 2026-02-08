//! Portal Authentication Middleware
//!
//! JWT-based authentication for portal users (end-users of the application).
//! This is different from tenant API key authentication which is used for
//! B2B integrations.

use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

/// JWT Claims for portal users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Tenant ID
    pub tenant_id: Option<String>,
    /// User email
    pub email: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Token type (access or refresh)
    #[serde(default = "default_token_type")]
    pub token_type: String,
}

fn default_token_type() -> String {
    "access".to_string()
}

/// Portal user context extracted from JWT token.
/// This can be used as an extractor in handlers.
#[derive(Debug, Clone)]
pub struct PortalUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
}

/// Configuration for portal authentication
#[derive(Clone)]
pub struct PortalAuthConfig {
    /// JWT secret for verifying tokens
    pub jwt_secret: String,
    /// Expected issuer (optional)
    pub issuer: Option<String>,
    /// Expected audience (optional)
    pub audience: Option<String>,
    /// Allow JWTs without tenant_id (legacy single-tenant mode)
    pub allow_missing_tenant: bool,
}

impl Default for PortalAuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET environment variable must be set"),
            issuer: std::env::var("JWT_ISSUER").ok(),
            audience: std::env::var("JWT_AUDIENCE").ok(),
            allow_missing_tenant: false,
        }
    }
}

/// Portal authentication middleware
///
/// Extracts and verifies JWT token from Authorization header,
/// then injects PortalUser into request extensions.
pub async fn portal_auth_middleware(
    State(config): State<Arc<PortalAuthConfig>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            warn!("Portal auth: Missing or invalid Authorization header");
            return Err(unauthorized_response(
                "Missing or invalid Authorization header",
            ));
        }
    };

    // Verify and decode the JWT token
    let portal_user = match verify_jwt_token(token, &config) {
        Ok(user) => user,
        Err(e) => {
            warn!(error = %e, "Portal auth: JWT verification failed");
            return Err(unauthorized_response(&e));
        }
    };

    debug!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        email = %portal_user.email,
        "Portal auth: User authenticated"
    );

    // Inject PortalUser into request extensions
    req.extensions_mut().insert(portal_user);

    Ok(next.run(req).await)
}

/// Verify JWT token and extract PortalUser
fn verify_jwt_token(token: &str, config: &PortalAuthConfig) -> Result<PortalUser, String> {
    // Configure validation
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    if let Some(ref issuer) = config.issuer {
        validation.set_issuer(&[issuer.as_str()]);
    }

    if let Some(ref audience) = config.audience {
        validation.set_audience(&[audience.as_str()]);
    }

    // Decode the token
    let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
    let token_data = decode::<PortalClaims>(token, &decoding_key, &validation)
        .map_err(|e| format!("Invalid token: {}", e))?;

    let claims = token_data.claims;

    if claims.tenant_id.is_none() && !config.allow_missing_tenant {
        return Err("Tenant ID required".to_string());
    }

    // Verify token type is access token
    if claims.token_type != "access" {
        return Err("Invalid token type".to_string());
    }

    // Parse user_id from sub claim
    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| "Invalid user ID in token".to_string())?;

    // Parse tenant_id (use default if not provided and allowed)
    let tenant_id = match &claims.tenant_id {
        Some(tid) => Uuid::parse_str(tid).map_err(|_| "Invalid tenant ID in token".to_string())?,
        None => Uuid::nil(),
    };

    Ok(PortalUser {
        user_id,
        tenant_id,
        email: claims.email,
    })
}

/// Create an unauthorized response
fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": {
                "code": "UNAUTHORIZED",
                "message": message
            }
        })),
    )
        .into_response()
}

/// Extract PortalUser from request extensions
pub fn extract_portal_user(req: &Request) -> Option<&PortalUser> {
    req.extensions().get::<PortalUser>()
}

/// FromRequestParts implementation for PortalUser extractor
/// This allows handlers to use `PortalUser` directly as a parameter
#[async_trait]
impl<S> FromRequestParts<S> for PortalUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<PortalUser>()
            .cloned()
            .ok_or_else(|| unauthorized_response("Authentication required"))
    }
}

/// Optional PortalUser extractor - returns Option<PortalUser>
/// Useful for routes that work with or without authentication
#[derive(Debug, Clone)]
pub struct OptionalPortalUser(pub Option<PortalUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalPortalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalPortalUser(
            parts.extensions.get::<PortalUser>().cloned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn create_test_config() -> PortalAuthConfig {
        PortalAuthConfig {
            jwt_secret: "test-secret-key-for-testing".to_string(),
            issuer: None,
            audience: None,
            allow_missing_tenant: false,
        }
    }

    fn create_test_token(claims: &PortalClaims, secret: &str) -> String {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        encode(&Header::default(), claims, &encoding_key).expect("token creation should succeed")
    }

    #[test]
    fn test_verify_valid_token() {
        let config = create_test_config();
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
            email: "test@example.com".to_string(),
            iat: now,
            exp: now + 3600,
            token_type: "access".to_string(),
        };

        let token = create_test_token(&claims, &config.jwt_secret);
        let result = verify_jwt_token(&token, &config);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(
            user.user_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(
            user.tenant_id.to_string(),
            "660e8400-e29b-41d4-a716-446655440001"
        );
    }

    #[test]
    fn test_verify_expired_token() {
        let config = create_test_config();
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
            email: "test@example.com".to_string(),
            iat: now - 7200,
            exp: now - 3600, // Expired 1 hour ago
            token_type: "access".to_string(),
        };

        let token = create_test_token(&claims, &config.jwt_secret);
        let result = verify_jwt_token(&token, &config);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid token"));
    }

    #[test]
    fn test_verify_wrong_token_type() {
        let config = create_test_config();
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
            email: "test@example.com".to_string(),
            iat: now,
            exp: now + 3600,
            token_type: "refresh".to_string(), // Wrong type
        };

        let token = create_test_token(&claims, &config.jwt_secret);
        let result = verify_jwt_token(&token, &config);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid token type");
    }

    #[test]
    fn test_verify_invalid_secret() {
        let config = create_test_config();
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
            email: "test@example.com".to_string(),
            iat: now,
            exp: now + 3600,
            token_type: "access".to_string(),
        };

        let token = create_test_token(&claims, "wrong-secret");
        let result = verify_jwt_token(&token, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_invalid_user_id() {
        let config = create_test_config();
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: "not-a-uuid".to_string(),
            tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
            email: "test@example.com".to_string(),
            iat: now,
            exp: now + 3600,
            token_type: "access".to_string(),
        };

        let token = create_test_token(&claims, &config.jwt_secret);
        let result = verify_jwt_token(&token, &config);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid user ID in token");
    }

    #[test]
    fn test_rejects_missing_tenant_id_when_not_allowed() {
        let config = create_test_config();
        let now = Utc::now().timestamp();
        let claims = PortalClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tenant_id: None, // No tenant ID
            email: "test@example.com".to_string(),
            iat: now,
            exp: now + 3600,
            token_type: "access".to_string(),
        };

        let token = create_test_token(&claims, &config.jwt_secret);
        let result = verify_jwt_token(&token, &config);

        assert!(result.is_err());
    }
}
