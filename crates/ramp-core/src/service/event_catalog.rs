use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStability {
    Active,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventDeprecationMarker {
    pub deprecated_since_version: String,
    pub replacement_event: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventPayloadFieldDescriptor {
    pub path: String,
    pub value_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventCatalogEntry {
    pub event_name: String,
    pub version: String,
    pub stability: EventStability,
    pub payload_wrapper: String,
    pub internal_subject: Option<String>,
    pub payload_fields: Vec<EventPayloadFieldDescriptor>,
    pub deprecation: Option<EventDeprecationMarker>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventCatalog {
    pub entries: Vec<EventCatalogEntry>,
}

impl EventCatalog {
    pub fn current() -> Self {
        Self {
            entries: vec![
                EventCatalogEntry {
                    event_name: "intent.status.changed".to_string(),
                    version: "v1".to_string(),
                    stability: EventStability::Active,
                    payload_wrapper: "webhook_event".to_string(),
                    internal_subject: Some("intent.status_changed".to_string()),
                    payload_fields: vec![
                        field("id", "string", true, "Webhook event identifier"),
                        field("type", "string", true, "Public event type name"),
                        field("created_at", "string", true, "RFC3339 creation timestamp"),
                        field("data.intentId", "string", false, "Intent identifier"),
                        field("data.newStatus", "string", false, "Latest intent status"),
                    ],
                    deprecation: None,
                },
                EventCatalogEntry {
                    event_name: "risk.review.required".to_string(),
                    version: "v1".to_string(),
                    stability: EventStability::Active,
                    payload_wrapper: "webhook_event".to_string(),
                    internal_subject: Some("risk.review_required".to_string()),
                    payload_fields: vec![
                        field("id", "string", true, "Webhook event identifier"),
                        field("type", "string", true, "Public event type name"),
                        field("created_at", "string", true, "RFC3339 creation timestamp"),
                        field("data.intentId", "string", false, "Intent identifier"),
                    ],
                    deprecation: None,
                },
                EventCatalogEntry {
                    event_name: "kyc.flagged".to_string(),
                    version: "v1".to_string(),
                    stability: EventStability::Active,
                    payload_wrapper: "webhook_event".to_string(),
                    internal_subject: Some("kyc.flagged".to_string()),
                    payload_fields: vec![
                        field("id", "string", true, "Webhook event identifier"),
                        field("type", "string", true, "Public event type name"),
                        field("created_at", "string", true, "RFC3339 creation timestamp"),
                        field("data.userId", "string", false, "Flagged user identifier"),
                        field("data.reason", "string", false, "KYC flag reason"),
                    ],
                    deprecation: None,
                },
                EventCatalogEntry {
                    event_name: "recon.batch.ready".to_string(),
                    version: "v1".to_string(),
                    stability: EventStability::Active,
                    payload_wrapper: "webhook_event".to_string(),
                    internal_subject: Some("recon.batch.ready".to_string()),
                    payload_fields: vec![
                        field("id", "string", true, "Webhook event identifier"),
                        field("type", "string", true, "Public event type name"),
                        field("created_at", "string", true, "RFC3339 creation timestamp"),
                        field(
                            "data.batchId",
                            "string",
                            false,
                            "Reconciliation batch identifier",
                        ),
                        field("data.status", "string", false, "Ready batch status"),
                    ],
                    deprecation: None,
                },
            ],
        }
    }

    pub fn find(&self, event_name: &str) -> Option<&EventCatalogEntry> {
        self.entries
            .iter()
            .find(|entry| entry.event_name == event_name)
    }
}

fn field(
    path: &str,
    value_type: &str,
    required: bool,
    description: &str,
) -> EventPayloadFieldDescriptor {
    EventPayloadFieldDescriptor {
        path: path.to_string(),
        value_type: value_type.to_string(),
        required,
        description: description.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_catalog_contains_live_webhook_event_types() {
        let catalog = EventCatalog::current();

        assert!(catalog.find("intent.status.changed").is_some());
        assert!(catalog.find("risk.review.required").is_some());
        assert!(catalog.find("kyc.flagged").is_some());
        assert!(catalog.find("recon.batch.ready").is_some());
    }

    #[test]
    fn intent_status_changed_catalog_entry_preserves_wrapper_contract() {
        let catalog = EventCatalog::current();
        let entry = catalog
            .find("intent.status.changed")
            .expect("intent.status.changed should be cataloged");

        assert_eq!(entry.version, "v1");
        assert_eq!(entry.payload_wrapper, "webhook_event");
        assert_eq!(
            entry.internal_subject.as_deref(),
            Some("intent.status_changed")
        );
        assert!(entry
            .payload_fields
            .iter()
            .any(|field| field.path == "id" && field.required));
        assert!(entry
            .payload_fields
            .iter()
            .any(|field| field.path == "data.intentId"));
        assert!(entry.deprecation.is_none());
    }
}
