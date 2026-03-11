use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use ramp_compliance::{
    TransportRetryPolicy, TravelRuleExchangeDispatch, TravelRuleExchangeError,
    TravelRuleExchangeRequest, TravelRuleExchangeResponse, TravelRuleExchangeService,
    TravelRuleTransport, TravelRuleTransportError, TravelRuleTransportFactory,
    TravelRuleTransportProfile, TransportAttemptStatus,
};

#[derive(Clone)]
struct ScriptedTransport {
    responses: Arc<Mutex<VecDeque<Result<TravelRuleExchangeResponse, TravelRuleTransportError>>>>,
}

impl ScriptedTransport {
    fn new(
        responses: Vec<Result<TravelRuleExchangeResponse, TravelRuleTransportError>>,
    ) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
        }
    }
}

#[async_trait]
impl TravelRuleTransport for ScriptedTransport {
    fn transport_kind(&self) -> &str {
        "HTTPS_API"
    }

    async fn send(
        &self,
        _profile: &TravelRuleTransportProfile,
        _request: &TravelRuleExchangeRequest,
        _attempt_number: u32,
    ) -> Result<TravelRuleExchangeResponse, TravelRuleTransportError> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Ok(TravelRuleExchangeResponse {
                    status: TransportAttemptStatus::Acknowledged,
                    response_status_code: Some(200),
                    response_payload: Some(serde_json::json!({ "ok": true })),
                    error_code: None,
                    error_message: None,
                    metadata: serde_json::json!({}),
                })
            })
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

fn sample_profile() -> TravelRuleTransportProfile {
    TravelRuleTransportProfile {
        profile_code: "trp-bridge".to_string(),
        transport_kind: "HTTPS_API".to_string(),
        endpoint_uri: Some("https://vasp.example/travel-rule".to_string()),
        retry_policy: TransportRetryPolicy::default(),
        metadata: serde_json::json!({}),
    }
}

fn sample_request() -> TravelRuleExchangeRequest {
    TravelRuleExchangeRequest {
        disclosure_id: "trd_001".to_string(),
        transport_profile: "trp-bridge".to_string(),
        endpoint_uri: None,
        payload: serde_json::json!({ "travelRule": true }),
        correlation_id: Some("corr_001".to_string()),
        metadata: serde_json::json!({}),
    }
}

#[tokio::test]
async fn exchange_service_schedules_retry_for_timeout() {
    let service = TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(Arc::new(
        ScriptedTransport::new(vec![Ok(TravelRuleExchangeResponse {
            status: TransportAttemptStatus::Timeout,
            response_status_code: Some(504),
            response_payload: None,
            error_code: Some("timeout".to_string()),
            error_message: Some("gateway timeout".to_string()),
            metadata: serde_json::json!({}),
        })]),
    ))));

    let dispatch = service
        .dispatch(&sample_profile(), &sample_request(), &[])
        .await
        .unwrap();

    assert_eq!(dispatch.attempt.attempt_number, 1);
    assert!(dispatch.retry.should_retry);
    assert_eq!(dispatch.retry.next_attempt_number, Some(2));
}

#[tokio::test]
async fn exchange_service_marks_rejected_transport_as_terminal() {
    let service = TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(Arc::new(
        ScriptedTransport::new(vec![Err(TravelRuleTransportError::permanent(
            "rejected",
            "counterparty rejected disclosure",
        ))]),
    ))));

    let dispatch: TravelRuleExchangeDispatch = service
        .dispatch(&sample_profile(), &sample_request(), &[])
        .await
        .unwrap();

    assert_eq!(dispatch.attempt.status, TransportAttemptStatus::Rejected);
    assert!(dispatch.retry.terminal);
    assert!(!dispatch.retry.should_retry);
}

#[tokio::test]
async fn exchange_service_rejects_request_endpoint_override() {
    let service = TravelRuleExchangeService::new(Arc::new(FixedTransportFactory::new(Arc::new(
        ScriptedTransport::new(Vec::new()),
    ))));

    let mut request = sample_request();
    request.endpoint_uri = Some("https://attacker.invalid/collect".to_string());

    let error = service
        .dispatch(&sample_profile(), &request, &[])
        .await
        .expect_err("endpoint override must be rejected");

    assert_eq!(error, TravelRuleExchangeError::EndpointOverrideRejected);
}
