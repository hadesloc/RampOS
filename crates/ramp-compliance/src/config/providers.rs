use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub kyc: KycProviderConfig,
    #[serde(default)]
    pub kyt: KytProviderConfig,
    #[serde(default)]
    pub sanctions: SanctionsProviderConfig,
    #[serde(default)]
    pub document_storage: DocumentStorageConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KycProviderConfig {
    pub provider: KycProviderType,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
}

impl Default for KycProviderConfig {
    fn default() -> Self {
        Self {
            provider: KycProviderType::Mock,
            api_key: None,
            api_url: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KycProviderType {
    Mock,
    Onfido,
    Jumio,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KytProviderConfig {
    pub provider: KytProviderType,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
}

impl Default for KytProviderConfig {
    fn default() -> Self {
        Self {
            provider: KytProviderType::Mock,
            api_key: None,
            api_url: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KytProviderType {
    Mock,
    Chainalysis,
    Elliptic,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SanctionsProviderConfig {
    pub provider: SanctionsProviderType,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
}

impl Default for SanctionsProviderConfig {
    fn default() -> Self {
        Self {
            provider: SanctionsProviderType::Mock,
            api_key: None,
            api_url: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SanctionsProviderType {
    Mock,
    OpenSanctions,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentStorageConfig {
    pub provider: DocumentStorageType,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
}

impl Default for DocumentStorageConfig {
    fn default() -> Self {
        Self {
            provider: DocumentStorageType::Mock,
            bucket: None,
            region: None,
            endpoint: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStorageType {
    Mock,
    S3,
}
