//! Enterprise SSO Integration Module
//!
//! Provides SAML 2.0 and OpenID Connect (OIDC) integration for enterprise customers.
//! Supports multiple identity providers: Okta, Azure AD, Google Workspace, Auth0.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub mod oidc;
pub mod saml;

pub use oidc::{OidcConfig, OidcProvider};
pub use saml::{SamlConfig, SamlProvider};

/// Supported SSO provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProviderType {
    /// Okta Identity Provider
    Okta,
    /// Microsoft Azure Active Directory
    AzureAd,
    /// Google Workspace (formerly G Suite)
    GoogleWorkspace,
    /// Auth0 Identity Platform
    Auth0,
    /// Generic OIDC provider
    GenericOidc,
    /// Generic SAML 2.0 provider
    GenericSaml,
}

impl SsoProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Okta => "okta",
            Self::AzureAd => "azure_ad",
            Self::GoogleWorkspace => "google_workspace",
            Self::Auth0 => "auth0",
            Self::GenericOidc => "generic_oidc",
            Self::GenericSaml => "generic_saml",
        }
    }

    pub fn supports_oidc(&self) -> bool {
        matches!(
            self,
            Self::Okta | Self::AzureAd | Self::GoogleWorkspace | Self::Auth0 | Self::GenericOidc
        )
    }

    pub fn supports_saml(&self) -> bool {
        matches!(
            self,
            Self::Okta | Self::AzureAd | Self::GoogleWorkspace | Self::GenericSaml
        )
    }
}

/// SSO authentication protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProtocol {
    /// OpenID Connect 1.0
    Oidc,
    /// SAML 2.0
    Saml,
}

/// RampOS role for authorization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RampRole {
    /// Super admin - full access
    SuperAdmin,
    /// Tenant admin - manage tenant settings
    TenantAdmin,
    /// Compliance officer - view compliance data
    ComplianceOfficer,
    /// Finance manager - manage transactions
    FinanceManager,
    /// Support agent - customer support
    SupportAgent,
    /// Viewer - read-only access
    Viewer,
    /// Custom role with permissions
    Custom(String),
}

impl RampRole {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "super_admin" | "superadmin" => Self::SuperAdmin,
            "tenant_admin" | "tenantadmin" | "admin" => Self::TenantAdmin,
            "compliance_officer" | "compliance" => Self::ComplianceOfficer,
            "finance_manager" | "finance" => Self::FinanceManager,
            "support_agent" | "support" => Self::SupportAgent,
            "viewer" | "readonly" => Self::Viewer,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::SuperAdmin => "super_admin",
            Self::TenantAdmin => "tenant_admin",
            Self::ComplianceOfficer => "compliance_officer",
            Self::FinanceManager => "finance_manager",
            Self::SupportAgent => "support_agent",
            Self::Viewer => "viewer",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Mapping rule from IdP group to RampOS role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMapping {
    /// IdP group name or ID
    pub idp_group: String,
    /// RampOS role to assign
    pub ramp_role: RampRole,
    /// Priority (lower = higher priority, for conflict resolution)
    pub priority: u32,
}

/// SSO configuration for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Provider type
    pub provider_type: SsoProviderType,
    /// Authentication protocol
    pub protocol: SsoProtocol,
    /// OIDC configuration (if protocol is OIDC)
    pub oidc_config: Option<OidcConfig>,
    /// SAML configuration (if protocol is SAML)
    pub saml_config: Option<SamlConfig>,
    /// Role mappings from IdP groups to RampOS roles
    pub role_mappings: Vec<RoleMapping>,
    /// Default role if no mapping matches
    pub default_role: RampRole,
    /// Whether SSO is enabled
    pub enabled: bool,
    /// Allow bypass for emergency access
    pub allow_password_bypass: bool,
    /// JIT (Just-In-Time) provisioning enabled
    pub jit_provisioning: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl SsoConfig {
    pub fn new_oidc(
        tenant_id: TenantId,
        provider_type: SsoProviderType,
        oidc_config: OidcConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            tenant_id,
            provider_type,
            protocol: SsoProtocol::Oidc,
            oidc_config: Some(oidc_config),
            saml_config: None,
            role_mappings: Vec::new(),
            default_role: RampRole::Viewer,
            enabled: true,
            allow_password_bypass: false,
            jit_provisioning: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_saml(
        tenant_id: TenantId,
        provider_type: SsoProviderType,
        saml_config: SamlConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            tenant_id,
            provider_type,
            protocol: SsoProtocol::Saml,
            oidc_config: None,
            saml_config: Some(saml_config),
            role_mappings: Vec::new(),
            default_role: RampRole::Viewer,
            enabled: true,
            allow_password_bypass: false,
            jit_provisioning: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_role_mappings(mut self, mappings: Vec<RoleMapping>) -> Self {
        self.role_mappings = mappings;
        self
    }

    pub fn with_default_role(mut self, role: RampRole) -> Self {
        self.default_role = role;
        self
    }
}

/// Authenticated user from SSO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUser {
    /// Unique identifier from IdP
    pub idp_user_id: String,
    /// Email address
    pub email: String,
    /// Display name
    pub name: Option<String>,
    /// First name
    pub given_name: Option<String>,
    /// Last name
    pub family_name: Option<String>,
    /// IdP groups the user belongs to
    pub groups: Vec<String>,
    /// Mapped RampOS roles
    pub roles: Vec<RampRole>,
    /// Raw claims from IdP
    pub claims: HashMap<String, serde_json::Value>,
    /// Authentication timestamp
    pub authenticated_at: DateTime<Utc>,
    /// Session expiry
    pub expires_at: DateTime<Utc>,
}

/// Summary of configured SSO provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProviderSummary {
    /// Provider ID (tenant key)
    pub provider_id: String,
    /// Provider type (e.g. okta, azure_ad)
    pub provider_type: String,
    /// Protocol (oidc or saml)
    pub protocol: String,
    /// Whether provider is enabled
    pub enabled: bool,
}

/// SSO authentication request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoAuthRequest {
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Redirect URI after authentication
    pub redirect_uri: String,
    /// State parameter for CSRF protection
    pub state: String,
    /// Nonce for replay protection (OIDC)
    pub nonce: Option<String>,
}

/// SSO authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoAuthResponse {
    /// Authorization URL to redirect user
    pub auth_url: String,
    /// State for verification
    pub state: String,
}

/// SSO callback data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoCallback {
    /// Authorization code (OIDC) or SAMLResponse
    pub code: Option<String>,
    /// SAML response (base64 encoded)
    pub saml_response: Option<String>,
    /// State parameter
    pub state: String,
    /// Error (if any)
    pub error: Option<String>,
    /// Error description
    pub error_description: Option<String>,
}

/// SSO provider trait
#[async_trait]
pub trait SsoProvider: Send + Sync {
    /// Get provider type
    fn provider_type(&self) -> SsoProviderType;

    /// Get protocol
    fn protocol(&self) -> SsoProtocol;

    /// Generate authorization URL
    async fn authorize(&self, request: &SsoAuthRequest) -> Result<SsoAuthResponse>;

    /// Handle callback and authenticate user
    async fn authenticate(&self, callback: &SsoCallback) -> Result<SsoUser>;

    /// Validate existing session
    async fn validate_session(&self, session_token: &str) -> Result<Option<SsoUser>>;

    /// Logout user
    async fn logout(&self, user: &SsoUser) -> Result<Option<String>>;
}

/// SSO service for managing enterprise SSO
pub struct SsoService {
    /// SSO configurations by tenant
    configs: HashMap<String, SsoConfig>,
    /// OIDC providers by tenant
    oidc_providers: HashMap<String, Arc<OidcProvider>>,
    /// SAML providers by tenant
    saml_providers: HashMap<String, Arc<SamlProvider>>,
}

impl SsoService {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            oidc_providers: HashMap::new(),
            saml_providers: HashMap::new(),
        }
    }

    /// Register SSO configuration for a tenant
    pub async fn register(&mut self, config: SsoConfig) -> Result<()> {
        let tenant_key = config.tenant_id.0.clone();

        match config.protocol {
            SsoProtocol::Oidc => {
                if let Some(oidc_config) = &config.oidc_config {
                    let provider = OidcProvider::new(
                        config.provider_type,
                        oidc_config.clone(),
                        config.role_mappings.clone(),
                        config.default_role.clone(),
                    )
                    .await?;
                    self.oidc_providers.insert(tenant_key.clone(), Arc::new(provider));
                }
            }
            SsoProtocol::Saml => {
                if let Some(saml_config) = &config.saml_config {
                    let provider = SamlProvider::new(
                        config.provider_type,
                        saml_config.clone(),
                        config.role_mappings.clone(),
                        config.default_role.clone(),
                    )?;
                    self.saml_providers.insert(tenant_key.clone(), Arc::new(provider));
                }
            }
        }

        self.configs.insert(tenant_key, config);
        Ok(())
    }

    /// Get SSO configuration for tenant
    pub fn get_config(&self, tenant_id: &TenantId) -> Option<&SsoConfig> {
        self.configs.get(&tenant_id.0)
    }

    /// Get SSO provider for tenant
    pub fn get_provider(&self, tenant_id: &TenantId) -> Option<Arc<dyn SsoProvider>> {
        let config = self.configs.get(&tenant_id.0)?;

        match config.protocol {
            SsoProtocol::Oidc => self
                .oidc_providers
                .get(&tenant_id.0)
                .map(|p| Arc::clone(p) as Arc<dyn SsoProvider>),
            SsoProtocol::Saml => self
                .saml_providers
                .get(&tenant_id.0)
                .map(|p| Arc::clone(p) as Arc<dyn SsoProvider>),
        }
    }

    /// Initiate SSO authentication
    pub async fn initiate_auth(&self, request: &SsoAuthRequest) -> Result<SsoAuthResponse> {
        let provider = self
            .get_provider(&request.tenant_id)
            .ok_or_else(|| ramp_common::Error::NotFound("SSO not configured".into()))?;

        provider.authorize(request).await
    }

    /// Handle SSO callback
    pub async fn handle_callback(
        &self,
        tenant_id: &TenantId,
        callback: &SsoCallback,
    ) -> Result<SsoUser> {
        let provider = self
            .get_provider(tenant_id)
            .ok_or_else(|| ramp_common::Error::NotFound("SSO not configured".into()))?;

        provider.authenticate(callback).await
    }

    /// Map IdP groups to RampOS roles
    pub fn map_roles(groups: &[String], mappings: &[RoleMapping], default: &RampRole) -> Vec<RampRole> {
        let mut roles: Vec<RampRole> = Vec::new();
        let mut matched_groups: HashMap<String, u32> = HashMap::new();

        // Sort mappings by priority (lower number = higher priority)
        let mut sorted_mappings = mappings.to_vec();
        sorted_mappings.sort_by_key(|m| m.priority);

        for group in groups {
            for mapping in &sorted_mappings {
                if group == &mapping.idp_group || group.to_lowercase() == mapping.idp_group.to_lowercase() {
                    let group_key = group.to_lowercase();

                    // Only add the highest priority mapping per group
                    if !matched_groups.contains_key(&group_key) {
                        roles.push(mapping.ramp_role.clone());
                        matched_groups.insert(group_key, mapping.priority);
                    }
                }
            }
        }

        // Add default role if no mappings matched
        if roles.is_empty() {
            roles.push(default.clone());
        }

        roles
    }

    /// List configured SSO providers as runtime-safe summaries
    pub fn list_provider_summaries(&self) -> Vec<SsoProviderSummary> {
        let mut providers: Vec<SsoProviderSummary> = self
            .configs
            .iter()
            .map(|(provider_id, config)| SsoProviderSummary {
                provider_id: provider_id.clone(),
                provider_type: config.provider_type.as_str().to_string(),
                protocol: match config.protocol {
                    SsoProtocol::Oidc => "oidc".to_string(),
                    SsoProtocol::Saml => "saml".to_string(),
                },
                enabled: config.enabled,
            })
            .collect();

        providers.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        providers
    }

    /// Check if tenant has SSO enabled
    pub fn is_enabled(&self, tenant_id: &TenantId) -> bool {
        self.configs
            .get(&tenant_id.0)
            .map(|c| c.enabled)
            .unwrap_or(false)
    }

    /// Disable SSO for tenant
    pub fn disable(&mut self, tenant_id: &TenantId) {
        if let Some(config) = self.configs.get_mut(&tenant_id.0) {
            config.enabled = false;
            config.updated_at = Utc::now();
        }
    }
}

impl Default for SsoService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_oidc_support() {
        assert!(SsoProviderType::Okta.supports_oidc());
        assert!(SsoProviderType::AzureAd.supports_oidc());
        assert!(SsoProviderType::GoogleWorkspace.supports_oidc());
        assert!(SsoProviderType::Auth0.supports_oidc());
        assert!(!SsoProviderType::GenericSaml.supports_oidc());
    }

    #[test]
    fn test_provider_type_saml_support() {
        assert!(SsoProviderType::Okta.supports_saml());
        assert!(SsoProviderType::AzureAd.supports_saml());
        assert!(!SsoProviderType::Auth0.supports_saml());
        assert!(SsoProviderType::GenericSaml.supports_saml());
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!(RampRole::from_str("super_admin"), RampRole::SuperAdmin);
        assert_eq!(RampRole::from_str("admin"), RampRole::TenantAdmin);
        assert_eq!(RampRole::from_str("viewer"), RampRole::Viewer);
        assert_eq!(
            RampRole::from_str("custom_role"),
            RampRole::Custom("custom_role".to_string())
        );
    }

    #[test]
    fn test_role_mapping() {
        let mappings = vec![
            RoleMapping {
                idp_group: "Admins".to_string(),
                ramp_role: RampRole::TenantAdmin,
                priority: 1,
            },
            RoleMapping {
                idp_group: "Finance".to_string(),
                ramp_role: RampRole::FinanceManager,
                priority: 2,
            },
        ];

        let groups = vec!["Admins".to_string(), "Finance".to_string()];
        let roles = SsoService::map_roles(&groups, &mappings, &RampRole::Viewer);

        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&RampRole::TenantAdmin));
        assert!(roles.contains(&RampRole::FinanceManager));
    }

    #[test]
    fn test_role_mapping_default() {
        let mappings = vec![RoleMapping {
            idp_group: "Admins".to_string(),
            ramp_role: RampRole::TenantAdmin,
            priority: 1,
        }];

        let groups = vec!["Users".to_string()];
        let roles = SsoService::map_roles(&groups, &mappings, &RampRole::Viewer);

        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], RampRole::Viewer);
    }

    #[test]
    fn test_role_mapping_priority() {
        let mappings = vec![
            RoleMapping {
                idp_group: "Admins".to_string(),
                ramp_role: RampRole::TenantAdmin,
                priority: 2,
            },
            RoleMapping {
                idp_group: "Admins".to_string(),
                ramp_role: RampRole::SuperAdmin,
                priority: 1,
            },
        ];

        let groups = vec!["Admins".to_string()];
        let roles = SsoService::map_roles(&groups, &mappings, &RampRole::Viewer);

        // Higher priority (lower number) should be matched first
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], RampRole::SuperAdmin);
    }

    #[test]
    fn test_role_mapping_case_insensitive() {
        let mappings = vec![
            RoleMapping {
                idp_group: "admins".to_string(),
                ramp_role: RampRole::TenantAdmin,
                priority: 1,
            },
        ];

        let groups = vec!["Admins".to_string()];
        let roles = SsoService::map_roles(&groups, &mappings, &RampRole::Viewer);

        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], RampRole::TenantAdmin);
    }

    #[test]
    fn test_role_from_str_all_variants() {
        assert_eq!(RampRole::from_str("superadmin"), RampRole::SuperAdmin);
        assert_eq!(RampRole::from_str("tenantadmin"), RampRole::TenantAdmin);
        assert_eq!(RampRole::from_str("compliance"), RampRole::ComplianceOfficer);
        assert_eq!(RampRole::from_str("finance"), RampRole::FinanceManager);
        assert_eq!(RampRole::from_str("support"), RampRole::SupportAgent);
        assert_eq!(RampRole::from_str("readonly"), RampRole::Viewer);
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(RampRole::SuperAdmin.as_str(), "super_admin");
        assert_eq!(RampRole::TenantAdmin.as_str(), "tenant_admin");
        assert_eq!(RampRole::ComplianceOfficer.as_str(), "compliance_officer");
        assert_eq!(RampRole::FinanceManager.as_str(), "finance_manager");
        assert_eq!(RampRole::SupportAgent.as_str(), "support_agent");
        assert_eq!(RampRole::Viewer.as_str(), "viewer");
        assert_eq!(RampRole::Custom("custom_role".to_string()).as_str(), "custom_role");
    }

    #[test]
    fn test_sso_provider_type_as_str() {
        assert_eq!(SsoProviderType::Okta.as_str(), "okta");
        assert_eq!(SsoProviderType::AzureAd.as_str(), "azure_ad");
        assert_eq!(SsoProviderType::GoogleWorkspace.as_str(), "google_workspace");
        assert_eq!(SsoProviderType::Auth0.as_str(), "auth0");
        assert_eq!(SsoProviderType::GenericOidc.as_str(), "generic_oidc");
        assert_eq!(SsoProviderType::GenericSaml.as_str(), "generic_saml");
    }

    #[test]
    fn test_sso_service_new_default() {
        let service = SsoService::new();
        let tenant = TenantId::new("test");
        assert!(!service.is_enabled(&tenant));
        assert!(service.get_config(&tenant).is_none());
        assert!(service.get_provider(&tenant).is_none());
    }

    #[test]
    fn test_sso_service_default_trait() {
        let _service = SsoService::default();
    }

    #[test]
    fn test_sso_config_new_oidc() {
        let config = SsoConfig::new_oidc(
            TenantId::new("t1"),
            SsoProviderType::Okta,
            OidcConfig::okta("client_id".into(), b"secret".to_vec(), "dev.okta.com"),
        );
        assert_eq!(config.protocol, SsoProtocol::Oidc);
        assert!(config.oidc_config.is_some());
        assert!(config.saml_config.is_none());
        assert!(config.enabled);
        assert!(config.jit_provisioning);
        assert!(!config.allow_password_bypass);
        assert_eq!(config.default_role, RampRole::Viewer);
    }

    #[test]
    fn test_sso_config_new_saml() {
        let config = SsoConfig::new_saml(
            TenantId::new("t1"),
            SsoProviderType::Okta,
            SamlConfig::okta("sp_entity".into(), "https://idp/metadata", "cert".into()),
        );
        assert_eq!(config.protocol, SsoProtocol::Saml);
        assert!(config.saml_config.is_some());
        assert!(config.oidc_config.is_none());
    }

    #[test]
    fn test_sso_config_with_role_mappings() {
        let config = SsoConfig::new_oidc(
            TenantId::new("t1"),
            SsoProviderType::Okta,
            OidcConfig::okta("cid".into(), b"sec".to_vec(), "dev.okta.com"),
        )
        .with_role_mappings(vec![
            RoleMapping {
                idp_group: "Admins".to_string(),
                ramp_role: RampRole::TenantAdmin,
                priority: 1,
            },
        ])
        .with_default_role(RampRole::SupportAgent);

        assert_eq!(config.role_mappings.len(), 1);
        assert_eq!(config.default_role, RampRole::SupportAgent);
    }

    #[test]
    fn test_sso_service_disable() {
        let mut service = SsoService::new();
        let tenant_id = TenantId::new("t1");

        // Manually insert a config (bypassing register which needs async OIDC provider init)
        let now = Utc::now();
        service.configs.insert("t1".to_string(), SsoConfig {
            tenant_id: tenant_id.clone(),
            provider_type: SsoProviderType::GenericOidc,
            protocol: SsoProtocol::Oidc,
            oidc_config: None,
            saml_config: None,
            role_mappings: vec![],
            default_role: RampRole::Viewer,
            enabled: true,
            allow_password_bypass: false,
            jit_provisioning: true,
            created_at: now,
            updated_at: now,
        });

        assert!(service.is_enabled(&tenant_id));
        service.disable(&tenant_id);
        assert!(!service.is_enabled(&tenant_id));
    }

    #[test]
    fn test_sso_callback_all_fields() {
        let callback = SsoCallback {
            code: Some("auth_code_123".to_string()),
            saml_response: None,
            state: "state_xyz".to_string(),
            error: None,
            error_description: None,
        };
        assert_eq!(callback.code.unwrap(), "auth_code_123");
        assert!(callback.error.is_none());
    }

    #[test]
    fn test_sso_auth_request() {
        let request = SsoAuthRequest {
            tenant_id: TenantId::new("t1"),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "random_state".to_string(),
            nonce: Some("random_nonce".to_string()),
        };
        assert_eq!(request.state, "random_state");
        assert!(request.nonce.is_some());
    }

    #[test]
    fn test_role_mapping_empty_groups() {
        let mappings = vec![
            RoleMapping {
                idp_group: "Admins".to_string(),
                ramp_role: RampRole::TenantAdmin,
                priority: 1,
            },
        ];

        let groups: Vec<String> = vec![];
        let roles = SsoService::map_roles(&groups, &mappings, &RampRole::Viewer);

        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], RampRole::Viewer);
    }

    #[test]
    fn test_role_mapping_empty_mappings() {
        let mappings: Vec<RoleMapping> = vec![];
        let groups = vec!["Admins".to_string()];
        let roles = SsoService::map_roles(&groups, &mappings, &RampRole::Viewer);

        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], RampRole::Viewer);
    }
}
