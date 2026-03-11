use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::travel_rule::TransportAttemptStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportRetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub backoff_multiplier: u32,
    pub max_delay_ms: Option<u64>,
    pub retryable_statuses: Vec<TransportAttemptStatus>,
}

impl Default for TransportRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            backoff_multiplier: 2,
            max_delay_ms: Some(5_000),
            retryable_statuses: vec![
                TransportAttemptStatus::Failed,
                TransportAttemptStatus::Timeout,
            ],
        }
    }
}

impl TransportRetryPolicy {
    fn validate(&self) -> Result<(), TravelRuleExchangeError> {
        if self.max_attempts == 0 {
            return Err(TravelRuleExchangeError::InvalidRetryPolicy(
                "max_attempts must be at least one".to_string(),
            ));
        }

        if self.backoff_multiplier == 0 {
            return Err(TravelRuleExchangeError::InvalidRetryPolicy(
                "backoff_multiplier must be at least one".to_string(),
            ));
        }

        Ok(())
    }

    fn decision_for(
        &self,
        attempt_number: u32,
        status: TransportAttemptStatus,
    ) -> TravelRuleRetryDecision {
        if status == TransportAttemptStatus::Acknowledged {
            return TravelRuleRetryDecision::terminal();
        }

        if !self.retryable_statuses.contains(&status) {
            return TravelRuleRetryDecision {
                should_retry: false,
                terminal: matches!(
                    status,
                    TransportAttemptStatus::Failed
                        | TransportAttemptStatus::Timeout
                        | TransportAttemptStatus::Rejected
                ),
                next_attempt_number: None,
                next_delay_ms: None,
            };
        }

        if attempt_number >= self.max_attempts {
            return TravelRuleRetryDecision::terminal();
        }

        TravelRuleRetryDecision {
            should_retry: true,
            terminal: false,
            next_attempt_number: Some(attempt_number + 1),
            next_delay_ms: Some(self.delay_after(attempt_number)),
        }
    }

    fn delay_after(&self, attempt_number: u32) -> u64 {
        let multiplier =
            u64::from(self.backoff_multiplier).saturating_pow(attempt_number.saturating_sub(1));
        let delay = self.base_delay_ms.saturating_mul(multiplier);

        match self.max_delay_ms {
            Some(max_delay_ms) => delay.min(max_delay_ms),
            None => delay,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleTransportProfile {
    pub profile_code: String,
    pub transport_kind: String,
    pub endpoint_uri: Option<String>,
    pub retry_policy: TransportRetryPolicy,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleExchangeRequest {
    pub disclosure_id: String,
    pub transport_profile: String,
    pub endpoint_uri: Option<String>,
    pub payload: Value,
    pub correlation_id: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleExchangeResponse {
    pub status: TransportAttemptStatus,
    pub response_status_code: Option<u16>,
    pub response_payload: Option<Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub metadata: Value,
}

impl TravelRuleExchangeResponse {
    fn from_transport_error(error: TravelRuleTransportError) -> Self {
        Self {
            status: error.status(),
            response_status_code: None,
            response_payload: None,
            error_code: Some(error.code().to_string()),
            error_message: Some(error.message().to_string()),
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleTransportAttempt {
    pub attempt_number: u32,
    pub transport_kind: String,
    pub status: TransportAttemptStatus,
    pub endpoint_uri: Option<String>,
    pub request_payload: Value,
    pub response_payload: Option<Value>,
    pub response_status_code: Option<u16>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retry_scheduled_after_ms: Option<u64>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleRetryDecision {
    pub should_retry: bool,
    pub terminal: bool,
    pub next_attempt_number: Option<u32>,
    pub next_delay_ms: Option<u64>,
}

impl TravelRuleRetryDecision {
    fn terminal() -> Self {
        Self {
            should_retry: false,
            terminal: true,
            next_attempt_number: None,
            next_delay_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleExchangeDispatch {
    pub attempt: TravelRuleTransportAttempt,
    pub retry: TravelRuleRetryDecision,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum TravelRuleTransportError {
    #[error("retryable transport error {code}: {message}")]
    Retryable { code: String, message: String },
    #[error("permanent transport error {code}: {message}")]
    Permanent { code: String, message: String },
}

impl TravelRuleTransportError {
    pub fn retryable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Retryable {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn permanent(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Permanent {
            code: code.into(),
            message: message.into(),
        }
    }

    fn status(&self) -> TransportAttemptStatus {
        match self {
            Self::Retryable { .. } => TransportAttemptStatus::Failed,
            Self::Permanent { .. } => TransportAttemptStatus::Rejected,
        }
    }

    fn code(&self) -> &str {
        match self {
            Self::Retryable { code, .. } | Self::Permanent { code, .. } => code,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Retryable { message, .. } | Self::Permanent { message, .. } => message,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TravelRuleExchangeError {
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("{field} must be a JSON object")]
    InvalidJsonObject { field: &'static str },
    #[error("request transport_profile does not match profile_code")]
    TransportProfileMismatch,
    #[error("endpoint_uri must be configured on the request or transport profile")]
    MissingEndpointUri,
    #[error("request endpoint_uri does not match the approved transport profile endpoint")]
    EndpointOverrideRejected,
    #[error("retry policy is invalid: {0}")]
    InvalidRetryPolicy(String),
}

#[async_trait]
pub trait TravelRuleTransport: Send + Sync {
    fn transport_kind(&self) -> &str;

    async fn send(
        &self,
        profile: &TravelRuleTransportProfile,
        request: &TravelRuleExchangeRequest,
        attempt_number: u32,
    ) -> Result<TravelRuleExchangeResponse, TravelRuleTransportError>;
}

pub trait TravelRuleTransportFactory: Send + Sync {
    fn resolve(
        &self,
        profile: &TravelRuleTransportProfile,
    ) -> Result<Arc<dyn TravelRuleTransport>, TravelRuleExchangeError>;
}

pub struct TravelRuleExchangeService {
    transport_factory: Arc<dyn TravelRuleTransportFactory>,
}

impl TravelRuleExchangeService {
    pub fn new(transport_factory: Arc<dyn TravelRuleTransportFactory>) -> Self {
        Self { transport_factory }
    }

    pub async fn dispatch(
        &self,
        profile: &TravelRuleTransportProfile,
        request: &TravelRuleExchangeRequest,
        previous_attempts: &[TravelRuleTransportAttempt],
    ) -> Result<TravelRuleExchangeDispatch, TravelRuleExchangeError> {
        validate_profile(profile)?;
        validate_request(profile, request)?;

        let attempt_number = next_attempt_number(previous_attempts);
        let endpoint_uri = profile
            .endpoint_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or(TravelRuleExchangeError::MissingEndpointUri)?;

        let transport = self.transport_factory.resolve(profile)?;
        let response = match transport.send(profile, request, attempt_number).await {
            Ok(response) => {
                validate_optional_object(response.response_payload.as_ref(), "response_payload")?;
                validate_object(&response.metadata, "response_metadata")?;
                response
            }
            Err(error) => TravelRuleExchangeResponse::from_transport_error(error),
        };

        let retry = profile
            .retry_policy
            .decision_for(attempt_number, response.status);

        Ok(TravelRuleExchangeDispatch {
            attempt: TravelRuleTransportAttempt {
                attempt_number,
                transport_kind: transport.transport_kind().trim().to_string(),
                status: response.status,
                endpoint_uri: Some(endpoint_uri),
                request_payload: request.payload.clone(),
                response_payload: response.response_payload,
                response_status_code: response.response_status_code,
                error_code: response.error_code,
                error_message: response.error_message,
                retry_scheduled_after_ms: retry.next_delay_ms,
                metadata: response.metadata,
            },
            retry,
        })
    }
}

fn validate_profile(profile: &TravelRuleTransportProfile) -> Result<(), TravelRuleExchangeError> {
    profile.retry_policy.validate()?;
    validate_required(&profile.profile_code, "profile_code")?;
    validate_required(&profile.transport_kind, "transport_kind")?;
    validate_object(&profile.metadata, "profile_metadata")?;
    Ok(())
}

fn validate_request(
    profile: &TravelRuleTransportProfile,
    request: &TravelRuleExchangeRequest,
) -> Result<(), TravelRuleExchangeError> {
    validate_required(&request.disclosure_id, "disclosure_id")?;
    validate_required(&request.transport_profile, "transport_profile")?;
    validate_object(&request.payload, "payload")?;
    validate_object(&request.metadata, "request_metadata")?;

    if !request
        .transport_profile
        .eq_ignore_ascii_case(&profile.profile_code)
    {
        return Err(TravelRuleExchangeError::TransportProfileMismatch);
    }

    if let Some(request_endpoint) = request
        .endpoint_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let approved_endpoint = profile
            .endpoint_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(TravelRuleExchangeError::MissingEndpointUri)?;

        if request_endpoint != approved_endpoint {
            return Err(TravelRuleExchangeError::EndpointOverrideRejected);
        }
    }

    Ok(())
}

fn next_attempt_number(previous_attempts: &[TravelRuleTransportAttempt]) -> u32 {
    previous_attempts
        .iter()
        .map(|attempt| attempt.attempt_number)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn validate_required(value: &str, field: &'static str) -> Result<(), TravelRuleExchangeError> {
    if value.trim().is_empty() {
        Err(TravelRuleExchangeError::EmptyField { field })
    } else {
        Ok(())
    }
}

fn validate_object(value: &Value, field: &'static str) -> Result<(), TravelRuleExchangeError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(TravelRuleExchangeError::InvalidJsonObject { field })
    }
}

fn validate_optional_object(
    value: Option<&Value>,
    field: &'static str,
) -> Result<(), TravelRuleExchangeError> {
    match value {
        Some(value) => validate_object(value, field),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use serde_json::json;

    use super::*;
    use crate::travel_rule::TransportAttemptStatus;

    #[tokio::test]
    async fn dispatch_schedules_retry_for_retryable_timeout() {
        let service =
            TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(Arc::new(
                ScriptedTransport::new(vec![Ok(TravelRuleExchangeResponse {
                    status: TransportAttemptStatus::Timeout,
                    response_status_code: Some(504),
                    response_payload: None,
                    error_code: Some("timeout".to_string()),
                    error_message: Some("gateway timeout".to_string()),
                    metadata: json!({ "provider": "mock" }),
                })]),
            ))));

        let dispatch = service
            .dispatch(&sample_profile(), &sample_request(), &[])
            .await
            .expect("dispatch should succeed");

        assert_eq!(dispatch.attempt.attempt_number, 1);
        assert_eq!(dispatch.attempt.status, TransportAttemptStatus::Timeout);
        assert_eq!(
            dispatch.retry,
            TravelRuleRetryDecision {
                should_retry: true,
                terminal: false,
                next_attempt_number: Some(2),
                next_delay_ms: Some(500),
            }
        );
    }

    #[tokio::test]
    async fn dispatch_stops_retrying_after_retry_budget_is_used() {
        let service = TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(
            Arc::new(ScriptedTransport::new(vec![Err(
                TravelRuleTransportError::retryable("timeout", "connection dropped"),
            )])),
        )));

        let previous_attempts = vec![
            TravelRuleTransportAttempt {
                attempt_number: 1,
                transport_kind: "https".to_string(),
                status: TransportAttemptStatus::Timeout,
                endpoint_uri: Some("https://vasp.example/travel-rule".to_string()),
                request_payload: json!({ "disclosureId": "trd_123" }),
                response_payload: None,
                response_status_code: Some(504),
                error_code: Some("timeout".to_string()),
                error_message: Some("gateway timeout".to_string()),
                retry_scheduled_after_ms: Some(500),
                metadata: json!({}),
            },
            TravelRuleTransportAttempt {
                attempt_number: 2,
                transport_kind: "https".to_string(),
                status: TransportAttemptStatus::Failed,
                endpoint_uri: Some("https://vasp.example/travel-rule".to_string()),
                request_payload: json!({ "disclosureId": "trd_123" }),
                response_payload: None,
                response_status_code: None,
                error_code: Some("timeout".to_string()),
                error_message: Some("connection dropped".to_string()),
                retry_scheduled_after_ms: Some(1000),
                metadata: json!({}),
            },
        ];

        let dispatch = service
            .dispatch(&sample_profile(), &sample_request(), &previous_attempts)
            .await
            .expect("dispatch should succeed");

        assert_eq!(dispatch.attempt.attempt_number, 3);
        assert_eq!(dispatch.attempt.status, TransportAttemptStatus::Failed);
        assert_eq!(
            dispatch.retry,
            TravelRuleRetryDecision {
                should_retry: false,
                terminal: true,
                next_attempt_number: None,
                next_delay_ms: None,
            }
        );
    }

    #[tokio::test]
    async fn rejected_transport_response_is_terminal() {
        let service =
            TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(Arc::new(
                ScriptedTransport::new(vec![Ok(TravelRuleExchangeResponse {
                    status: TransportAttemptStatus::Rejected,
                    response_status_code: Some(422),
                    response_payload: Some(json!({ "error": "schema_mismatch" })),
                    error_code: Some("schema_mismatch".to_string()),
                    error_message: Some("payload rejected".to_string()),
                    metadata: json!({}),
                })]),
            ))));

        let dispatch = service
            .dispatch(&sample_profile(), &sample_request(), &[])
            .await
            .expect("dispatch should succeed");

        assert_eq!(dispatch.attempt.status, TransportAttemptStatus::Rejected);
        assert!(!dispatch.retry.should_retry);
        assert!(dispatch.retry.terminal);
    }

    #[tokio::test]
    async fn dispatch_uses_profile_endpoint_when_request_attempts_override() {
        let transport = Arc::new(ObservingTransport::default());
        let service =
            TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(transport.clone())));
        let mut request = sample_request();
        request.endpoint_uri = Some("https://attacker.example/collect".to_string());

        let dispatch = service
            .dispatch(&sample_profile(), &request, &[])
            .await
            .expect("dispatch should succeed");

        assert_eq!(
            dispatch.attempt.endpoint_uri.as_deref(),
            Some("https://vasp.example/travel-rule")
        );
        assert_eq!(
            transport
                .last_request_endpoint()
                .as_deref(),
            Some("https://vasp.example/travel-rule")
        );
    }

    fn sample_profile() -> TravelRuleTransportProfile {
        TravelRuleTransportProfile {
            profile_code: "trp-bridge".to_string(),
            transport_kind: "https".to_string(),
            endpoint_uri: Some("https://vasp.example/travel-rule".to_string()),
            retry_policy: TransportRetryPolicy {
                max_attempts: 3,
                base_delay_ms: 500,
                backoff_multiplier: 2,
                max_delay_ms: Some(5_000),
                retryable_statuses: vec![
                    TransportAttemptStatus::Failed,
                    TransportAttemptStatus::Timeout,
                ],
            },
            metadata: json!({ "tenant": "tenant_123" }),
        }
    }

    fn sample_request() -> TravelRuleExchangeRequest {
        TravelRuleExchangeRequest {
            disclosure_id: "trd_123".to_string(),
            transport_profile: "trp-bridge".to_string(),
            endpoint_uri: None,
            payload: json!({
                "originator": { "name": "Alice" },
                "beneficiary": { "name": "Bob" }
            }),
            correlation_id: Some("corr_123".to_string()),
            metadata: json!({ "policyCode": "global-outbound" }),
        }
    }

    struct FixedTransportFactory {
        transport: Arc<dyn TravelRuleTransport>,
    }

    impl FixedTransportFactory {
        fn new(transport: Arc<dyn TravelRuleTransport>) -> Self {
            Self { transport }
        }
    }

    impl TravelRuleTransportFactory for FixedTransportFactory {
        fn resolve(
            &self,
            _profile: &TravelRuleTransportProfile,
        ) -> Result<Arc<dyn TravelRuleTransport>, TravelRuleExchangeError> {
            Ok(self.transport.clone())
        }
    }

    struct ScriptedTransport {
        responses: Mutex<VecDeque<Result<TravelRuleExchangeResponse, TravelRuleTransportError>>>,
    }

    impl ScriptedTransport {
        fn new(
            responses: Vec<Result<TravelRuleExchangeResponse, TravelRuleTransportError>>,
        ) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
            }
        }
    }

    #[async_trait]
    impl TravelRuleTransport for ScriptedTransport {
        fn transport_kind(&self) -> &str {
            "https"
        }

        async fn send(
            &self,
            _profile: &TravelRuleTransportProfile,
            _request: &TravelRuleExchangeRequest,
            _attempt_number: u32,
        ) -> Result<TravelRuleExchangeResponse, TravelRuleTransportError> {
            self.responses
                .lock()
                .expect("scripted responses lock should not be poisoned")
                .pop_front()
                .expect("scripted transport should have a response")
        }
    }

    #[derive(Default)]
    struct ObservingTransport {
        last_request_endpoint: Mutex<Option<String>>,
    }

    impl ObservingTransport {
        fn last_request_endpoint(&self) -> Option<String> {
            self.last_request_endpoint
                .lock()
                .expect("observing transport lock should not be poisoned")
                .clone()
        }
    }

    #[async_trait]
    impl TravelRuleTransport for ObservingTransport {
        fn transport_kind(&self) -> &str {
            "https"
        }

        async fn send(
            &self,
            _profile: &TravelRuleTransportProfile,
            request: &TravelRuleExchangeRequest,
            _attempt_number: u32,
        ) -> Result<TravelRuleExchangeResponse, TravelRuleTransportError> {
            *self
                .last_request_endpoint
                .lock()
                .expect("observing transport lock should not be poisoned") =
                request.endpoint_uri.clone();

            Ok(TravelRuleExchangeResponse {
                status: TransportAttemptStatus::Acknowledged,
                response_status_code: Some(200),
                response_payload: Some(json!({ "accepted": true })),
                error_code: None,
                error_message: None,
                metadata: json!({}),
            })
        }
    }
}
