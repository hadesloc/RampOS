use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::travel_rule::{TravelRuleCounterparty, VaspInteroperabilityStatus, VaspReviewStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaspRegistryRecordInput {
    pub vasp_code: String,
    pub legal_name: String,
    pub display_name: Option<String>,
    pub jurisdiction_code: Option<String>,
    pub registration_number: Option<String>,
    pub travel_rule_profile: Option<String>,
    pub transport_profile: Option<String>,
    pub endpoint_uri: Option<String>,
    pub endpoint_public_key: Option<String>,
    pub supports_inbound: bool,
    pub supports_outbound: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaspReviewState {
    pub status: VaspReviewStatus,
    pub reviewed_by: Option<String>,
    pub reason_code: Option<String>,
    pub notes: Option<String>,
}

impl Default for VaspReviewState {
    fn default() -> Self {
        Self {
            status: VaspReviewStatus::Pending,
            reviewed_by: None,
            reason_code: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaspInteroperabilityState {
    pub status: VaspInteroperabilityStatus,
    pub error_code: Option<String>,
    pub notes: Option<String>,
}

impl Default for VaspInteroperabilityState {
    fn default() -> Self {
        Self {
            status: VaspInteroperabilityStatus::Unknown,
            error_code: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaspRegistryRecord {
    pub vasp_code: String,
    pub legal_name: String,
    pub display_name: Option<String>,
    pub jurisdiction_code: Option<String>,
    pub registration_number: Option<String>,
    pub travel_rule_profile: Option<String>,
    pub transport_profile: Option<String>,
    pub endpoint_uri: Option<String>,
    pub endpoint_public_key: Option<String>,
    pub review: VaspReviewState,
    pub interoperability: VaspInteroperabilityState,
    pub supports_inbound: bool,
    pub supports_outbound: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaspReviewUpdate {
    pub status: VaspReviewStatus,
    pub reviewed_by: Option<String>,
    pub reason_code: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaspInteroperabilityUpdate {
    pub status: VaspInteroperabilityStatus,
    pub transport_profile: Option<String>,
    pub endpoint_uri: Option<String>,
    pub endpoint_public_key: Option<String>,
    pub error_code: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VaspRegistryError {
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("metadata must be a JSON object")]
    InvalidMetadata,
    #[error("reason_code is required when status is {status:?}")]
    ReasonCodeRequired { status: VaspReviewStatus },
    #[error("transport_profile is required when interoperability status is {status:?}")]
    TransportProfileRequired { status: VaspInteroperabilityStatus },
    #[error("endpoint_uri is required when interoperability status is {status:?}")]
    EndpointUriRequired { status: VaspInteroperabilityStatus },
}

pub struct VaspRegistryService;

impl VaspRegistryService {
    pub fn register(
        input: VaspRegistryRecordInput,
    ) -> Result<VaspRegistryRecord, VaspRegistryError> {
        validate_object(&input.metadata)?;

        Ok(VaspRegistryRecord {
            vasp_code: normalize_required(input.vasp_code, "vasp_code", true)?,
            legal_name: normalize_required(input.legal_name, "legal_name", false)?,
            display_name: normalize_optional(input.display_name, false),
            jurisdiction_code: normalize_optional(input.jurisdiction_code, true),
            registration_number: normalize_optional(input.registration_number, false),
            travel_rule_profile: normalize_optional(input.travel_rule_profile, false),
            transport_profile: normalize_optional(input.transport_profile, false),
            endpoint_uri: normalize_optional(input.endpoint_uri, false),
            endpoint_public_key: normalize_optional(input.endpoint_public_key, false),
            review: VaspReviewState::default(),
            interoperability: VaspInteroperabilityState::default(),
            supports_inbound: input.supports_inbound,
            supports_outbound: input.supports_outbound,
            metadata: input.metadata,
        })
    }

    pub fn apply_review_update(
        record: &VaspRegistryRecord,
        update: &VaspReviewUpdate,
    ) -> Result<VaspRegistryRecord, VaspRegistryError> {
        let reason_code = normalize_optional(update.reason_code.clone(), false);
        if matches!(
            update.status,
            VaspReviewStatus::Rejected | VaspReviewStatus::Suspended
        ) && reason_code.is_none()
        {
            return Err(VaspRegistryError::ReasonCodeRequired {
                status: update.status,
            });
        }

        let mut next = record.clone();
        next.review = VaspReviewState {
            status: update.status,
            reviewed_by: normalize_optional(update.reviewed_by.clone(), false),
            reason_code: if matches!(
                update.status,
                VaspReviewStatus::Rejected | VaspReviewStatus::Suspended
            ) {
                reason_code
            } else {
                None
            },
            notes: normalize_optional(update.notes.clone(), false),
        };

        Ok(next)
    }

    pub fn apply_interoperability_update(
        record: &VaspRegistryRecord,
        update: &VaspInteroperabilityUpdate,
    ) -> Result<VaspRegistryRecord, VaspRegistryError> {
        let transport_profile = normalize_optional(update.transport_profile.clone(), false)
            .or_else(|| record.transport_profile.clone());
        let endpoint_uri = normalize_optional(update.endpoint_uri.clone(), false)
            .or_else(|| record.endpoint_uri.clone());

        if matches!(
            update.status,
            VaspInteroperabilityStatus::Ready | VaspInteroperabilityStatus::Limited
        ) {
            if transport_profile.is_none() {
                return Err(VaspRegistryError::TransportProfileRequired {
                    status: update.status,
                });
            }

            if endpoint_uri.is_none() {
                return Err(VaspRegistryError::EndpointUriRequired {
                    status: update.status,
                });
            }
        }

        let mut next = record.clone();
        next.transport_profile = transport_profile;
        next.endpoint_uri = endpoint_uri;
        next.endpoint_public_key = normalize_optional(update.endpoint_public_key.clone(), false)
            .or_else(|| record.endpoint_public_key.clone());
        next.interoperability = VaspInteroperabilityState {
            status: update.status,
            error_code: if update.status.is_usable() {
                None
            } else {
                normalize_optional(update.error_code.clone(), false)
            },
            notes: normalize_optional(update.notes.clone(), false),
        };

        Ok(next)
    }

    pub fn counterparty(record: &VaspRegistryRecord) -> TravelRuleCounterparty {
        TravelRuleCounterparty {
            vasp_code: Some(record.vasp_code.clone()),
            jurisdiction_code: record.jurisdiction_code.clone(),
            travel_rule_profile: record.travel_rule_profile.clone(),
            transport_profile: record.transport_profile.clone(),
            review_status: Some(record.review.status),
            interoperability_status: Some(record.interoperability.status),
            supports_inbound: Some(record.supports_inbound),
            supports_outbound: Some(record.supports_outbound),
        }
    }
}

fn normalize_required(
    value: String,
    field: &'static str,
    lowercase: bool,
) -> Result<String, VaspRegistryError> {
    normalize_optional(Some(value), lowercase).ok_or(VaspRegistryError::EmptyField { field })
}

fn normalize_optional(value: Option<String>, lowercase: bool) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else if lowercase {
            Some(trimmed.to_ascii_lowercase())
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validate_object(value: &Value) -> Result<(), VaspRegistryError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(VaspRegistryError::InvalidMetadata)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::travel_rule::{VaspInteroperabilityStatus, VaspReviewStatus};

    #[test]
    fn register_normalizes_registry_fields_and_defaults_states() {
        let record = VaspRegistryService::register(VaspRegistryRecordInput {
            vasp_code: "  vasp-sg-1  ".to_string(),
            legal_name: "  Example VASP Ltd  ".to_string(),
            display_name: Some("   ".to_string()),
            jurisdiction_code: Some(" sg ".to_string()),
            registration_number: Some(" REG-123 ".to_string()),
            travel_rule_profile: Some(" trp-bridge ".to_string()),
            transport_profile: Some(" https-api ".to_string()),
            endpoint_uri: Some(" https://vasp.example/travel-rule ".to_string()),
            endpoint_public_key: Some(" pub-key ".to_string()),
            supports_inbound: true,
            supports_outbound: false,
            metadata: json!({ "network": "trp-bridge" }),
        })
        .expect("registry record should be created");

        assert_eq!(record.vasp_code, "vasp-sg-1");
        assert_eq!(record.legal_name, "Example VASP Ltd");
        assert_eq!(record.display_name, None);
        assert_eq!(record.jurisdiction_code.as_deref(), Some("sg"));
        assert_eq!(record.review.status, VaspReviewStatus::Pending);
        assert_eq!(
            record.interoperability.status,
            VaspInteroperabilityStatus::Unknown
        );
    }

    #[test]
    fn rejected_review_requires_reason_code() {
        let record = sample_record();

        let error = VaspRegistryService::apply_review_update(
            &record,
            &VaspReviewUpdate {
                status: VaspReviewStatus::Rejected,
                reviewed_by: Some("analyst-1".to_string()),
                reason_code: None,
                notes: Some("missing due diligence artifacts".to_string()),
            },
        )
        .expect_err("rejection without reason should fail");

        assert_eq!(
            error,
            VaspRegistryError::ReasonCodeRequired {
                status: VaspReviewStatus::Rejected,
            }
        );
    }

    #[test]
    fn ready_interoperability_persists_endpoint_and_counterparty_snapshot() {
        let updated = VaspRegistryService::apply_interoperability_update(
            &sample_record(),
            &VaspInteroperabilityUpdate {
                status: VaspInteroperabilityStatus::Ready,
                transport_profile: Some("trp-bridge".to_string()),
                endpoint_uri: Some("https://vasp.example/travel-rule".to_string()),
                endpoint_public_key: Some("rotated-key".to_string()),
                error_code: Some("timeout".to_string()),
                notes: Some("connectivity restored".to_string()),
            },
        )
        .expect("ready interoperability update should succeed");

        assert_eq!(updated.transport_profile.as_deref(), Some("trp-bridge"));
        assert_eq!(
            updated.endpoint_uri.as_deref(),
            Some("https://vasp.example/travel-rule")
        );
        assert_eq!(updated.endpoint_public_key.as_deref(), Some("rotated-key"));
        assert_eq!(
            updated.interoperability.status,
            VaspInteroperabilityStatus::Ready
        );
        assert_eq!(updated.interoperability.error_code, None);

        let counterparty = VaspRegistryService::counterparty(&updated);
        assert_eq!(counterparty.vasp_code.as_deref(), Some("vasp-sg-1"));
        assert_eq!(counterparty.review_status, Some(VaspReviewStatus::Approved));
        assert_eq!(
            counterparty.interoperability_status,
            Some(VaspInteroperabilityStatus::Ready)
        );
        assert_eq!(
            counterparty.transport_profile.as_deref(),
            Some("trp-bridge")
        );
    }

    fn sample_record() -> VaspRegistryRecord {
        VaspRegistryRecord {
            vasp_code: "vasp-sg-1".to_string(),
            legal_name: "Example VASP Ltd".to_string(),
            display_name: Some("Example".to_string()),
            jurisdiction_code: Some("SG".to_string()),
            registration_number: Some("REG-123".to_string()),
            travel_rule_profile: Some("trp-bridge".to_string()),
            transport_profile: Some("trp-bridge".to_string()),
            endpoint_uri: Some("https://vasp.example/travel-rule".to_string()),
            endpoint_public_key: Some("pub-key".to_string()),
            review: VaspReviewState {
                status: VaspReviewStatus::Approved,
                reviewed_by: Some("analyst-1".to_string()),
                reason_code: None,
                notes: None,
            },
            interoperability: VaspInteroperabilityState {
                status: VaspInteroperabilityStatus::Limited,
                error_code: Some("stale-ack".to_string()),
                notes: Some("temporary partial outage".to_string()),
            },
            supports_inbound: true,
            supports_outbound: true,
            metadata: json!({ "source": "internal-review" }),
        }
    }
}
