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
            validation.validate_aud = true;
            validation.set_audience(&[&self.config.client_id]);

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
            Err(ramp_common::Error::Authentication(
                "JWKS URI is not configured - cannot verify JWT signature".into(),
            ))
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
#[cfg(test)]
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

    // ---- OidcConfig endpoint validation ----

    #[test]
    fn test_okta_config_endpoints() {
        let config = OidcConfig::okta(
            "client_id".to_string(),
            b"secret".to_vec(),
            "dev-test.okta.com",
        );
        assert_eq!(
            config.authorization_endpoint.as_deref(),
            Some("https://dev-test.okta.com/oauth2/v1/authorize")
        );
        assert_eq!(
            config.token_endpoint.as_deref(),
            Some("https://dev-test.okta.com/oauth2/v1/token")
        );
        assert_eq!(
            config.jwks_uri.as_deref(),
            Some("https://dev-test.okta.com/oauth2/v1/keys")
        );
        assert_eq!(
            config.end_session_endpoint.as_deref(),
            Some("https://dev-test.okta.com/oauth2/v1/logout")
        );
        assert_eq!(config.email_claim, "email");
        assert_eq!(config.name_claim, "name");
    }

    #[test]
    fn test_azure_ad_config_endpoints() {
        let tenant = "aaaabbbb-cccc-dddd-eeee-ffffgggghhhh";
        let config = OidcConfig::azure_ad(
            "az_client".to_string(),
            b"az_secret".to_vec(),
            tenant,
        );
        assert!(config.authorization_endpoint.as_ref().unwrap().contains(tenant));
        assert!(config.token_endpoint.as_ref().unwrap().contains(tenant));
        assert!(config.jwks_uri.as_ref().unwrap().contains(tenant));
        assert!(config.scopes.contains(&"offline_access".to_string()));
        assert!(!config.scopes.contains(&"groups".to_string())); // Azure uses different scope
    }

    #[test]
    fn test_google_workspace_no_end_session() {
        let config = OidcConfig::google_workspace("g_client".to_string(), b"g_secret".to_vec());
        assert!(config.end_session_endpoint.is_none()); // Google doesn't support RP-initiated logout
        assert_eq!(config.extra_auth_params.get("hd"), Some(&"*".to_string()));
    }

    #[test]
    fn test_auth0_config_endpoints() {
        let config = OidcConfig::auth0(
            "a0_client".to_string(),
            b"a0_secret".to_vec(),
            "my-tenant.us.auth0.com",
        );
        assert_eq!(
            config.authorization_endpoint.as_deref(),
            Some("https://my-tenant.us.auth0.com/authorize")
        );
        assert_eq!(
            config.token_endpoint.as_deref(),
            Some("https://my-tenant.us.auth0.com/oauth/token")
        );
        assert_eq!(
            config.jwks_uri.as_deref(),
            Some("https://my-tenant.us.auth0.com/.well-known/jwks.json")
        );
        assert_eq!(
            config.end_session_endpoint.as_deref(),
            Some("https://my-tenant.us.auth0.com/v2/logout")
        );
    }

    // ---- OidcConfig scopes ----

    #[test]
    fn test_all_configs_have_openid_scope() {
        let okta = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let azure = OidcConfig::azure_ad("c".into(), vec![], "tid");
        let google = OidcConfig::google_workspace("c".into(), vec![]);
        let auth0 = OidcConfig::auth0("c".into(), vec![], "test.auth0.com");

        for (name, config) in [("okta", okta), ("azure", azure), ("google", google), ("auth0", auth0)] {
            assert!(
                config.scopes.contains(&"openid".to_string()),
                "{} config missing openid scope", name
            );
            assert!(
                config.scopes.contains(&"email".to_string()),
                "{} config missing email scope", name
            );
        }
    }

    // ---- build_auth_url ----

    #[tokio::test]
    async fn test_build_auth_url() {
        let config = OidcConfig::okta("my_client".to_string(), b"sec".to_vec(), "dev.okta.com");
        let provider = OidcProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).await.unwrap();

        let request = SsoAuthRequest {
            tenant_id: ramp_common::types::TenantId::new("t1"),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "random_state_123".to_string(),
            nonce: Some("nonce_456".to_string()),
        };

        let url = provider.build_auth_url(&request);
        assert!(url.starts_with("https://dev.okta.com/oauth2/v1/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my_client"));
        assert!(url.contains("state=random_state_123"));
        assert!(url.contains("nonce=nonce_456"));
        assert!(url.contains("scope="));
    }

    #[tokio::test]
    async fn test_build_auth_url_no_nonce() {
        let config = OidcConfig::okta("client1".to_string(), b"sec".to_vec(), "test.okta.com");
        let provider = OidcProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).await.unwrap();

        let request = SsoAuthRequest {
            tenant_id: ramp_common::types::TenantId::new("t1"),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "state_abc".to_string(),
            nonce: None,
        };

        let url = provider.build_auth_url(&request);
        assert!(!url.contains("nonce="));
    }

    #[tokio::test]
    async fn test_build_auth_url_with_extra_params() {
        let config = OidcConfig::google_workspace("g_client".to_string(), b"sec".to_vec());
        let provider = OidcProvider::new(
            SsoProviderType::GoogleWorkspace,
            config,
            vec![],
            RampRole::Viewer,
        ).await.unwrap();

        let request = SsoAuthRequest {
            tenant_id: ramp_common::types::TenantId::new("t1"),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "state_xyz".to_string(),
            nonce: None,
        };

        let url = provider.build_auth_url(&request);
        // Google config has extra_auth_params with hd=*
        assert!(url.contains("hd="));
    }

    // ---- extract_groups ----

    #[tokio::test]
    async fn test_extract_groups_with_groups_claim() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).await.unwrap();

        let mut extra = HashMap::new();
        extra.insert("groups".to_string(), serde_json::json!(["Admins", "Engineers"]));

        let claims = IdTokenClaims {
            iss: "https://test.okta.com".to_string(),
            sub: "user123".to_string(),
            aud: serde_json::json!("client_id"),
            exp: Utc::now().timestamp() + 3600,
            iat: Utc::now().timestamp(),
            nonce: None,
            auth_time: None,
            email: Some("user@test.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            picture: None,
            extra,
        };

        let groups = provider.extract_groups(&claims);
        assert_eq!(groups, vec!["Admins".to_string(), "Engineers".to_string()]);
    }

    #[tokio::test]
    async fn test_extract_groups_missing_claim() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).await.unwrap();

        let claims = IdTokenClaims {
            iss: "https://test.okta.com".to_string(),
            sub: "user123".to_string(),
            aud: serde_json::json!("client_id"),
            exp: Utc::now().timestamp() + 3600,
            iat: Utc::now().timestamp(),
            nonce: None,
            auth_time: None,
            email: None,
            email_verified: None,
            name: None,
            given_name: None,
            family_name: None,
            picture: None,
            extra: HashMap::new(), // no groups claim
        };

        let groups = provider.extract_groups(&claims);
        assert!(groups.is_empty());
    }

    #[tokio::test]
    async fn test_extract_groups_non_array_claim() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(
            SsoProviderType::Okta,
            config,
            vec![],
            RampRole::Viewer,
        ).await.unwrap();

        let mut extra = HashMap::new();
        extra.insert("groups".to_string(), serde_json::json!("not_an_array"));

        let claims = IdTokenClaims {
            iss: "https://test.okta.com".to_string(),
            sub: "user123".to_string(),
            aud: serde_json::json!("client_id"),
            exp: 0,
            iat: 0,
            nonce: None,
            auth_time: None,
            email: None,
            email_verified: None,
            name: None,
            given_name: None,
            family_name: None,
            picture: None,
            extra,
        };

        let groups = provider.extract_groups(&claims);
        assert!(groups.is_empty());
    }

    // ---- base64_url_decode ----

    #[test]
    fn test_base64_url_decode_valid() {
        // "hello" in base64url
        let encoded = "aGVsbG8";
        let decoded = base64_url_decode(encoded).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_url_decode_invalid() {
        let result = base64_url_decode("!!!invalid!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_url_decode_empty() {
        let decoded = base64_url_decode("").unwrap();
        assert!(decoded.is_empty());
    }

    // ---- OidcProvider SsoProvider trait ----

    #[tokio::test]
    async fn test_oidc_provider_type() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer)
            .await.unwrap();
        assert_eq!(provider.provider_type(), SsoProviderType::Okta);
        assert_eq!(provider.protocol(), SsoProtocol::Oidc);
    }

    #[tokio::test]
    async fn test_oidc_authorize() {
        let config = OidcConfig::okta("client_x".into(), vec![], "dev.okta.com");
        let provider = OidcProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer)
            .await.unwrap();

        let request = SsoAuthRequest {
            tenant_id: ramp_common::types::TenantId::new("t1"),
            redirect_uri: "https://localhost/callback".to_string(),
            state: "abc123".to_string(),
            nonce: None,
        };

        let response = provider.authorize(&request).await.unwrap();
        assert_eq!(response.state, "abc123");
        assert!(response.auth_url.contains("client_id=client_x"));
    }

    #[tokio::test]
    async fn test_oidc_authenticate_error_callback() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer)
            .await.unwrap();

        let callback = SsoCallback {
            code: None,
            state: "state".to_string(),
            error: Some("access_denied".to_string()),
            error_description: Some("User denied access".to_string()),
            saml_response: None,
        };

        let result = provider.authenticate(&callback).await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("access_denied"));
    }

    #[tokio::test]
    async fn test_oidc_authenticate_missing_code() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer)
            .await.unwrap();

        let callback = SsoCallback {
            code: None,
            state: "state".to_string(),
            error: None,
            error_description: None,
            saml_response: None,
        };

        let result = provider.authenticate(&callback).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_oidc_validate_session_returns_none() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer)
            .await.unwrap();

        let result = provider.validate_session("some_token").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_oidc_logout_returns_endpoint() {
        let config = OidcConfig::okta("c".into(), vec![], "test.okta.com");
        let provider = OidcProvider::new(SsoProviderType::Okta, config, vec![], RampRole::Viewer)
            .await.unwrap();

        let user = SsoUser {
            idp_user_id: "user1".to_string(),
            email: "user@test.com".to_string(),
            name: None,
            given_name: None,
            family_name: None,
            groups: vec![],
            roles: vec![],
            claims: HashMap::new(),
            authenticated_at: Utc::now(),
            expires_at: Utc::now(),
        };

        let result = provider.logout(&user).await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("logout"));
    }

    // ---- IdTokenClaims ----

    #[test]
    fn test_id_token_claims_deserialization() {
        let json = serde_json::json!({
            "iss": "https://issuer.example.com",
            "sub": "user_456",
            "aud": "client_id",
            "exp": 1700000000i64,
            "iat": 1699996400i64,
            "email": "user@example.com",
            "name": "John Doe",
            "custom_claim": "custom_value"
        });
        let claims: IdTokenClaims = serde_json::from_value(json).unwrap();
        assert_eq!(claims.iss, "https://issuer.example.com");
        assert_eq!(claims.sub, "user_456");
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.name, Some("John Doe".to_string()));
        // extra fields should capture custom_claim
        assert_eq!(
            claims.extra.get("custom_claim"),
            Some(&serde_json::json!("custom_value"))
        );
    }
}
