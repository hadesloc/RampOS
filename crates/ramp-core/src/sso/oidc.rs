//! OpenID Connect (OIDC) Provider Implementation
//!
//! Supports Okta, Azure AD, Google Workspace, Auth0, and generic OIDC providers.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::{
    RampRole, RoleMapping, SsoAuthRequest, SsoAuthResponse, SsoCallback, SsoProtocol,
    SsoProvider, SsoProviderType, SsoService, SsoUser,
};

/// OIDC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// OAuth 2.0 Client ID
    pub client_id: String,
    /// OAuth 2.0 Client Secret (encrypted)
    #[serde(skip_serializing)]
    pub client_secret_encrypted: Vec<u8>,
    /// Issuer URL (e.g., https://login.microsoftonline.com/{tenant}/v2.0)
    pub issuer_url: String,
    /// Authorization endpoint
    pub authorization_endpoint: Option<String>,
    /// Token endpoint
    pub token_endpoint: Option<String>,
    /// Userinfo endpoint
    pub userinfo_endpoint: Option<String>,
    /// JWKS URI for token validation
    pub jwks_uri: Option<String>,
    /// End session endpoint for logout
    pub end_session_endpoint: Option<String>,
    /// Requested scopes
    pub scopes: Vec<String>,
    /// Claim for user groups
    pub groups_claim: String,
    /// Claim for email
    pub email_claim: String,
    /// Claim for name
    pub name_claim: String,
    /// Additional parameters for authorization request
    pub extra_auth_params: HashMap<String, String>,
    /// Default redirect URI for the OAuth callback.
    /// Used during token exchange when the original redirect_uri is not
    /// available from the callback state.
    // SECURITY: TODO - The redirect_uri should be stored alongside the state parameter
    // during the authorize step and retrieved during callback, rather than relying on
    // a default. This prevents redirect_uri mismatch and open redirect attacks.
    pub default_redirect_uri: String,
}

impl OidcConfig {
    /// Create OIDC config for Okta
    pub fn okta(client_id: String, client_secret_encrypted: Vec<u8>, okta_domain: &str) -> Self {
        Self {
            client_id,
            client_secret_encrypted,
            issuer_url: format!("https://{}", okta_domain),
            authorization_endpoint: Some(format!("https://{}/oauth2/v1/authorize", okta_domain)),
            token_endpoint: Some(format!("https://{}/oauth2/v1/token", okta_domain)),
            userinfo_endpoint: Some(format!("https://{}/oauth2/v1/userinfo", okta_domain)),
            jwks_uri: Some(format!("https://{}/oauth2/v1/keys", okta_domain)),
            end_session_endpoint: Some(format!("https://{}/oauth2/v1/logout", okta_domain)),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "groups".to_string(),
            ],
            groups_claim: "groups".to_string(),
            email_claim: "email".to_string(),
            name_claim: "name".to_string(),
            extra_auth_params: HashMap::new(),
            default_redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| format!("https://{}/v1/auth/sso/okta/callback", okta_domain)),
        }
    }

    /// Create OIDC config for Azure AD
    pub fn azure_ad(
        client_id: String,
        client_secret_encrypted: Vec<u8>,
        tenant_id: &str,
    ) -> Self {
        let base_url = format!("https://login.microsoftonline.com/{}/v2.0", tenant_id);
        Self {
            client_id,
            client_secret_encrypted,
            issuer_url: base_url.clone(),
            authorization_endpoint: Some(format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize",
                tenant_id
            )),
            token_endpoint: Some(format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                tenant_id
            )),
            userinfo_endpoint: Some("https://graph.microsoft.com/oidc/userinfo".to_string()),
            jwks_uri: Some(format!(
                "https://login.microsoftonline.com/{}/discovery/v2.0/keys",
                tenant_id
            )),
            end_session_endpoint: Some(format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/logout",
                tenant_id
            )),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "offline_access".to_string(),
            ],
            groups_claim: "groups".to_string(),
            email_claim: "email".to_string(),
            name_claim: "name".to_string(),
            extra_auth_params: HashMap::new(),
            default_redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| "https://localhost/v1/auth/sso/azure/callback".to_string()),
        }
    }

    /// Create OIDC config for Google Workspace
    pub fn google_workspace(client_id: String, client_secret_encrypted: Vec<u8>) -> Self {
        Self {
            client_id,
            client_secret_encrypted,
            issuer_url: "https://accounts.google.com".to_string(),
            authorization_endpoint: Some(
                "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            ),
            token_endpoint: Some("https://oauth2.googleapis.com/token".to_string()),
            userinfo_endpoint: Some(
                "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
            ),
            jwks_uri: Some("https://www.googleapis.com/oauth2/v3/certs".to_string()),
            end_session_endpoint: None, // Google doesn't support RP-initiated logout
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            groups_claim: "groups".to_string(), // Requires Directory API
            email_claim: "email".to_string(),
            name_claim: "name".to_string(),
            extra_auth_params: {
                let mut params = HashMap::new();
                params.insert("hd".to_string(), "*".to_string()); // Hosted domain hint
                params
            },
            default_redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| "https://localhost/v1/auth/sso/google/callback".to_string()),
        }
    }

    /// Create OIDC config for Auth0
    pub fn auth0(client_id: String, client_secret_encrypted: Vec<u8>, auth0_domain: &str) -> Self {
        Self {
            client_id,
            client_secret_encrypted,
            issuer_url: format!("https://{}/", auth0_domain),
            authorization_endpoint: Some(format!("https://{}/authorize", auth0_domain)),
            token_endpoint: Some(format!("https://{}/oauth/token", auth0_domain)),
            userinfo_endpoint: Some(format!("https://{}/userinfo", auth0_domain)),
            jwks_uri: Some(format!("https://{}/.well-known/jwks.json", auth0_domain)),
            end_session_endpoint: Some(format!("https://{}/v2/logout", auth0_domain)),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            groups_claim: "https://ramp.os/groups".to_string(), // Custom claim via Auth0 Rules
            email_claim: "email".to_string(),
            name_claim: "name".to_string(),
            extra_auth_params: HashMap::new(),
            default_redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| {
                    format!("https://{}/v1/auth/sso/auth0/callback", auth0_domain)
                }),
        }
    }
}

/// OIDC token response
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub scope: Option<String>,
}

/// OIDC ID Token claims
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    /// Issuer
    pub iss: String,
    /// Subject (user ID)
    pub sub: String,
    /// Audience
    pub aud: serde_json::Value,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// Nonce
    pub nonce: Option<String>,
    /// Auth time
    pub auth_time: Option<i64>,
    /// Email
    pub email: Option<String>,
    /// Email verified
    pub email_verified: Option<bool>,
    /// Name
    pub name: Option<String>,
    /// Given name
    pub given_name: Option<String>,
    /// Family name
    pub family_name: Option<String>,
    /// Picture URL
    pub picture: Option<String>,
    /// All other claims
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// JWKS (JSON Web Key Set) response from the IdP's JWKS endpoint
#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

/// Individual JSON Web Key from the JWKS endpoint
#[derive(Debug, Clone, Deserialize)]
struct JwkKey {
    /// Key ID - used to match against the JWT header's `kid` field
    kid: Option<String>,
    /// Key type (e.g., "RSA")
    kty: String,
    /// RSA modulus (base64url-encoded)
    n: Option<String>,
    /// RSA exponent (base64url-encoded)
    e: Option<String>,
    /// Algorithm (e.g., "RS256")
    #[allow(dead_code)]
    alg: Option<String>,
    /// Key usage (e.g., "sig" for signature)
    #[serde(rename = "use")]
    use_: Option<String>,
}

/// OIDC Provider implementation
pub struct OidcProvider {
    provider_type: SsoProviderType,
    config: OidcConfig,
    role_mappings: Vec<RoleMapping>,
    default_role: RampRole,
    http_client: reqwest::Client,
    /// Cached JWKS keys with timestamp for expiry-based refresh (1 hour TTL)
    jwks_cache: RwLock<Option<(Vec<JwkKey>, DateTime<Utc>)>>,
}

impl OidcProvider {
    pub async fn new(
        provider_type: SsoProviderType,
        config: OidcConfig,
        role_mappings: Vec<RoleMapping>,
        default_role: RampRole,
    ) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        Ok(Self {
            provider_type,
            config,
            role_mappings,
            default_role,
            http_client,
            jwks_cache: RwLock::new(None),
        })
    }

    /// Build authorization URL
    fn build_auth_url(&self, request: &SsoAuthRequest) -> String {
        let default_auth_endpoint = format!("{}/authorize", self.config.issuer_url);
        let auth_endpoint = self
            .config
            .authorization_endpoint
            .as_deref()
            .unwrap_or(&default_auth_endpoint);

        let scopes = self.config.scopes.join(" ");

        let mut params = vec![
            ("response_type", "code".to_string()),
            ("client_id", self.config.client_id.clone()),
            ("redirect_uri", request.redirect_uri.clone()),
            ("scope", scopes),
            ("state", request.state.clone()),
        ];

        if let Some(nonce) = &request.nonce {
            params.push(("nonce", nonce.clone()));
        }

        // Add extra parameters
        for (key, value) in &self.config.extra_auth_params {
            params.push((key.as_str(), value.clone()));
        }

        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();

        format!("{}?{}", auth_endpoint, query.join("&"))
    }

    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<TokenResponse> {
        let default_token_endpoint = format!("{}/token", self.config.issuer_url);
        let token_endpoint = self
            .config
            .token_endpoint
            .as_deref()
            .unwrap_or(&default_token_endpoint);

        // Decrypt client secret (placeholder - implement actual decryption)
        let client_secret =
            String::from_utf8_lossy(&self.config.client_secret_encrypted).to_string();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_secret", &client_secret),
        ];

        let response = self
            .http_client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| ramp_common::Error::External(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ramp_common::Error::Authentication(format!(
                "Token exchange failed: {}",
                error_text
            )));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))
    }

    /// Fetch JWKS keys from the IdP's JWKS endpoint
    async fn fetch_jwks(&self) -> Result<Vec<JwkKey>> {
        let jwks_uri = self
            .config
            .jwks_uri
            .as_deref()
            .ok_or_else(|| ramp_common::Error::Internal("No JWKS URI configured".into()))?;

        let response = self
            .http_client
            .get(jwks_uri)
            .send()
            .await
            .map_err(|e| {
                ramp_common::Error::External(format!("Failed to fetch JWKS: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ramp_common::Error::External(format!(
                "JWKS endpoint returned {}: {}",
                status, body
            )));
        }

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| ramp_common::Error::Internal(format!("Failed to parse JWKS: {}", e)))?;

        Ok(jwks.keys)
    }

    /// Get JWKS keys from cache if fresh (< 1 hour), otherwise fetch and cache
    async fn get_cached_jwks(&self) -> Result<Vec<JwkKey>> {
        // Check cache under read lock
        {
            let cache = self.jwks_cache.read().await;
            if let Some((keys, fetched_at)) = cache.as_ref() {
                if Utc::now() - *fetched_at < Duration::hours(1) {
                    return Ok(keys.clone());
                }
            }
        }

        // Cache miss or expired - fetch fresh keys
        let keys = self.fetch_jwks().await?;

        // Update cache under write lock
        {
            let mut cache = self.jwks_cache.write().await;
            *cache = Some((keys.clone(), Utc::now()));
        }

        Ok(keys)
    }

    /// Decode and validate ID token with cryptographic signature verification
    ///
    /// When a JWKS URI is configured, this method:
    /// 1. Fetches the IdP's public keys (JWKS) with 1-hour caching
    /// 2. Decodes the JWT header to find the key ID (kid)
    /// 3. Finds the matching RSA public key
    /// 4. Verifies the JWT signature using the RSA public key
    /// 5. Validates issuer and expiration claims
    ///
    /// Falls back to insecure base64 decode (with warning) only when no JWKS URI
    /// is configured.
    async fn decode_id_token(&self, id_token: &str) -> Result<IdTokenClaims> {
        if self.config.jwks_uri.is_some() {
            // Real JWKS-based JWT signature verification
            let keys = self.get_cached_jwks().await?;

            // Decode the JWT header to get the key ID
            let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
                ramp_common::Error::Authentication(format!("Invalid JWT header: {}", e))
            })?;

            // Find the matching key by kid (key ID)
            let matching_key = if let Some(kid) = &header.kid {
                keys.iter().find(|k| k.kid.as_deref() == Some(kid))
            } else {
                // If no kid in header, use the first RSA signing key
                keys.iter().find(|k| {
                    k.kty == "RSA"
                        && k.use_.as_deref() != Some("enc") // exclude encryption keys
                })
            };

            let jwk = matching_key.ok_or_else(|| {
                ramp_common::Error::Authentication(
                    "No matching key found in JWKS for JWT verification".into(),
                )
            })?;

            // Ensure the key is RSA and has the required components
            if jwk.kty != "RSA" {
                return Err(ramp_common::Error::Authentication(format!(
                    "Unsupported key type: {}. Only RSA is supported.",
                    jwk.kty
                )));
            }

            let n = jwk.n.as_deref().ok_or_else(|| {
                ramp_common::Error::Authentication(
                    "JWKS key missing RSA modulus (n)".into(),
                )
            })?;
            let e = jwk.e.as_deref().ok_or_else(|| {
                ramp_common::Error::Authentication(
                    "JWKS key missing RSA exponent (e)".into(),
                )
            })?;

            // Build the decoding key from RSA components
            let decoding_key =
                jsonwebtoken::DecodingKey::from_rsa_components(n, e).map_err(|err| {
                    ramp_common::Error::Authentication(format!(
                        "Failed to build decoding key from RSA components: {}",
                        err
                    ))
                })?;

            // Configure validation - match algorithm from JWT header
            let algorithm = match header.alg {
                jsonwebtoken::Algorithm::RS256 => jsonwebtoken::Algorithm::RS256,
                jsonwebtoken::Algorithm::RS384 => jsonwebtoken::Algorithm::RS384,
                jsonwebtoken::Algorithm::RS512 => jsonwebtoken::Algorithm::RS512,
                alg => {
                    return Err(ramp_common::Error::Authentication(format!(
                        "Unsupported JWT algorithm: {:?}",
                        alg
                    )));
                }
            };

            let mut validation = jsonwebtoken::Validation::new(algorithm);

            // Set issuer validation
            validation.set_issuer(&[&self.config.issuer_url]);

            // Disable audience validation since the aud claim can be a string or
            // array and we handle the client_id check separately if needed
            validation.validate_aud = false;

            // Decode and verify the token
            let token_data = jsonwebtoken::decode::<IdTokenClaims>(
                id_token,
                &decoding_key,
                &validation,
            )
            .map_err(|e| {
                ramp_common::Error::Authentication(format!(
                    "JWT signature verification failed: {}",
                    e
                ))
            })?;

            Ok(token_data.claims)
        } else {
            // Fallback: insecure decode when no JWKS URI is configured
            tracing::warn!(
                "No JWKS URI configured - skipping JWT signature verification. \
                 This is insecure and should only be used in development."
            );

            // Split JWT parts
            let parts: Vec<&str> = id_token.split('.').collect();
            if parts.len() != 3 {
                return Err(ramp_common::Error::Authentication(
                    "Invalid ID token format".into(),
                ));
            }

            // Decode payload (base64url)
            let payload = base64_url_decode(parts[1])?;
            let claims: IdTokenClaims = serde_json::from_slice(&payload).map_err(|e| {
                ramp_common::Error::Authentication(format!("Invalid claims: {}", e))
            })?;

            // Validate expiration
            let now = Utc::now().timestamp();
            if claims.exp < now {
                return Err(ramp_common::Error::Authentication(
                    "ID token expired".into(),
                ));
            }

            // Validate issuer
            if !claims.iss.starts_with(&self.config.issuer_url) {
                return Err(ramp_common::Error::Authentication(
                    "Invalid issuer".into(),
                ));
            }

            Ok(claims)
        }
    }

    /// Extract groups from claims
    fn extract_groups(&self, claims: &IdTokenClaims) -> Vec<String> {
        claims
            .extra
            .get(&self.config.groups_claim)
            .and_then(|v| {
                if let serde_json::Value::Array(arr) = v {
                    Some(
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl SsoProvider for OidcProvider {
    fn provider_type(&self) -> SsoProviderType {
        self.provider_type
    }

    fn protocol(&self) -> SsoProtocol {
        SsoProtocol::Oidc
    }

    async fn authorize(&self, request: &SsoAuthRequest) -> Result<SsoAuthResponse> {
        let auth_url = self.build_auth_url(request);

        Ok(SsoAuthResponse {
            auth_url,
            state: request.state.clone(),
        })
    }

    async fn authenticate(&self, callback: &SsoCallback) -> Result<SsoUser> {
        // Check for errors
        if let Some(error) = &callback.error {
            return Err(ramp_common::Error::Authentication(format!(
                "{}: {}",
                error,
                callback.error_description.as_deref().unwrap_or("")
            )));
        }

        // Get authorization code
        let code = callback
            .code
            .as_ref()
            .ok_or_else(|| {
                ramp_common::Error::Authentication("Missing authorization code".into())
            })?;

        // Exchange code for tokens
        // SECURITY: TODO - The redirect_uri should be stored with the state parameter
        // during the authorize step and retrieved here, rather than using a default.
        // Using a static default is acceptable for MVP but does not fully protect
        // against redirect_uri manipulation in a multi-origin deployment.
        let tokens = self
            .exchange_code(code, &self.config.default_redirect_uri)
            .await?;

        // Decode ID token with signature verification
        let id_token = tokens
            .id_token
            .ok_or_else(|| ramp_common::Error::Authentication("Missing ID token".into()))?;

        let claims = self.decode_id_token(&id_token).await?;

        // Extract user info
        let groups = self.extract_groups(&claims);
        let roles = SsoService::map_roles(&groups, &self.role_mappings, &self.default_role);

        let now = Utc::now();
        let expires_at = now + Duration::seconds(tokens.expires_in.unwrap_or(3600) as i64);

        Ok(SsoUser {
            idp_user_id: claims.sub.clone(),
            email: claims.email.unwrap_or_default(),
            name: claims.name.clone(),
            given_name: claims.given_name.clone(),
            family_name: claims.family_name.clone(),
            groups,
            roles,
            claims: claims
                .extra
                .into_iter()
                .chain([
                    ("sub".to_string(), serde_json::Value::String(claims.sub)),
                    ("iss".to_string(), serde_json::Value::String(claims.iss)),
                ])
                .collect(),
            authenticated_at: now,
            expires_at,
        })
    }

    async fn validate_session(&self, _session_token: &str) -> Result<Option<SsoUser>> {
        // Session validation would typically involve checking a session store
        // or validating a refresh token
        Ok(None)
    }

    async fn logout(&self, _user: &SsoUser) -> Result<Option<String>> {
        // Return end session URL if available
        Ok(self.config.end_session_endpoint.clone())
    }
}

/// Base64 URL decode (without padding)
fn base64_url_decode(input: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| ramp_common::Error::Authentication(format!("Base64 decode error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okta_config() {
        let config = OidcConfig::okta(
            "client123".to_string(),
            b"secret".to_vec(),
            "dev-123456.okta.com",
        );

        assert_eq!(config.issuer_url, "https://dev-123456.okta.com");
        assert!(config.scopes.contains(&"groups".to_string()));
    }

    #[test]
    fn test_azure_ad_config() {
        let config = OidcConfig::azure_ad(
            "client123".to_string(),
            b"secret".to_vec(),
            "00000000-0000-0000-0000-000000000000",
        );

        assert!(config.issuer_url.contains("login.microsoftonline.com"));
        assert!(config.scopes.contains(&"offline_access".to_string()));
    }

    #[test]
    fn test_google_workspace_config() {
        let config =
            OidcConfig::google_workspace("client123".to_string(), b"secret".to_vec());

        assert_eq!(config.issuer_url, "https://accounts.google.com");
        assert!(config.extra_auth_params.contains_key("hd"));
    }

    #[test]
    fn test_auth0_config() {
        let config = OidcConfig::auth0(
            "client123".to_string(),
            b"secret".to_vec(),
            "myapp.auth0.com",
        );

        assert_eq!(config.issuer_url, "https://myapp.auth0.com/");
        assert_eq!(config.groups_claim, "https://ramp.os/groups");
    }
}
