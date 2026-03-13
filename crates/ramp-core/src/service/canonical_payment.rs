use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalPaymentDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalPaymentStatusFamily {
    Review,
    Hold,
    Cleared,
    Failed,
    Returned,
    Settled,
}

impl CanonicalPaymentStatusFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Review => "review",
            Self::Hold => "hold",
            Self::Cleared => "cleared",
            Self::Failed => "failed",
            Self::Returned => "returned",
            Self::Settled => "settled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPaymentParty {
    pub account_id: Option<String>,
    pub bank_identifier: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPaymentRecord {
    pub provider: String,
    pub provider_reference: String,
    pub reference_code: String,
    pub tenant_id: String,
    pub direction: CanonicalPaymentDirection,
    pub amount: Decimal,
    pub currency: String,
    pub raw_status: Option<String>,
    pub status_family: CanonicalPaymentStatusFamily,
    pub payer: CanonicalPaymentParty,
    pub beneficiary: CanonicalPaymentParty,
    pub occurred_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPaymentInput {
    pub provider: String,
    pub provider_reference: String,
    pub reference_code: String,
    pub tenant_id: String,
    pub direction: CanonicalPaymentDirection,
    pub amount: Decimal,
    pub currency: String,
    pub raw_status: Option<String>,
    pub payer: CanonicalPaymentParty,
    pub beneficiary: CanonicalPaymentParty,
    pub occurred_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl CanonicalPaymentRecord {
    pub fn from_input(
        input: CanonicalPaymentInput,
        status_family: CanonicalPaymentStatusFamily,
    ) -> Self {
        Self {
            provider: input.provider,
            provider_reference: input.provider_reference,
            reference_code: input.reference_code,
            tenant_id: input.tenant_id,
            direction: input.direction,
            amount: input.amount,
            currency: input.currency,
            raw_status: input.raw_status,
            status_family,
            payer: input.payer,
            beneficiary: input.beneficiary,
            occurred_at: input.occurred_at,
            metadata: input.metadata,
        }
    }
}

pub fn map_vietqr_status(status: &str) -> CanonicalPaymentStatusFamily {
    map_status_token(status, CanonicalPaymentStatusFamily::Hold)
}

pub fn map_napas_status(status: &str) -> CanonicalPaymentStatusFamily {
    match status.trim().to_ascii_uppercase().as_str() {
        "00" => CanonicalPaymentStatusFamily::Settled,
        "01" | "03" | "09" | "68" | "91" => CanonicalPaymentStatusFamily::Hold,
        "94" => CanonicalPaymentStatusFamily::Review,
        "06" | "12" | "30" | "96" => CanonicalPaymentStatusFamily::Failed,
        other => map_status_token(other, CanonicalPaymentStatusFamily::Hold),
    }
}

pub fn map_generic_bank_status(
    status: Option<&str>,
    metadata: &serde_json::Value,
) -> CanonicalPaymentStatusFamily {
    if let Some(status) = status {
        return map_status_token(status, CanonicalPaymentStatusFamily::Cleared);
    }

    if let Some(status) = metadata.get("status").and_then(|value| value.as_str()) {
        return map_status_token(status, CanonicalPaymentStatusFamily::Cleared);
    }

    CanonicalPaymentStatusFamily::Cleared
}

fn map_status_token(
    raw_status: &str,
    default_status: CanonicalPaymentStatusFamily,
) -> CanonicalPaymentStatusFamily {
    let normalized = raw_status.trim().to_ascii_uppercase();

    if normalized.is_empty() {
        return default_status;
    }

    match normalized.as_str() {
        "SUCCESS" | "SUCCEEDED" | "COMPLETED" | "PAID" | "SETTLED" => {
            CanonicalPaymentStatusFamily::Settled
        }
        "CLEARED" | "POSTED" | "BOOKED" => CanonicalPaymentStatusFamily::Cleared,
        "PENDING" | "PROCESSING" | "IN_PROGRESS" | "QUEUED" | "WAITING" | "ON_HOLD" => {
            CanonicalPaymentStatusFamily::Hold
        }
        "REVIEW" | "MANUAL_REVIEW" | "REVIEW_REQUIRED" | "AML_REVIEW" | "COMPLIANCE_HOLD" => {
            CanonicalPaymentStatusFamily::Review
        }
        "FAILED" | "FAIL" | "REJECTED" | "DECLINED" | "ERROR" | "CANCELLED" => {
            CanonicalPaymentStatusFamily::Failed
        }
        "RETURNED" | "RETURN" | "REVERSED" | "REVERSAL" | "REFUNDED" => {
            CanonicalPaymentStatusFamily::Returned
        }
        _ => default_status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vietqr_success_maps_to_settled() {
        assert_eq!(
            map_vietqr_status("SUCCESS"),
            CanonicalPaymentStatusFamily::Settled
        );
    }

    #[test]
    fn napas_code_00_maps_to_settled() {
        assert_eq!(
            map_napas_status("00"),
            CanonicalPaymentStatusFamily::Settled
        );
    }

    #[test]
    fn generic_defaults_to_cleared_without_status() {
        assert_eq!(
            map_generic_bank_status(None, &serde_json::json!({})),
            CanonicalPaymentStatusFamily::Cleared
        );
    }
}
