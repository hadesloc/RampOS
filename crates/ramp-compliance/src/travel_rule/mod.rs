use serde::{Deserialize, Serialize};

pub mod disclosure;
pub mod exchange;
pub mod policy;
pub mod registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TravelRuleDirection {
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TravelRuleDirectionScope {
    Outbound,
    Inbound,
    Both,
}

impl TravelRuleDirectionScope {
    pub fn matches(self, direction: TravelRuleDirection) -> bool {
        matches!(self, Self::Both)
            || matches!(
                (self, direction),
                (Self::Outbound, TravelRuleDirection::Outbound)
                    | (Self::Inbound, TravelRuleDirection::Inbound)
            )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TravelRuleAction {
    Allow,
    ReviewRequired,
    DiscloseBeforeSettlement,
    DiscloseAfterSettlement,
    Block,
}

impl TravelRuleAction {
    pub fn requires_disclosure(self) -> bool {
        matches!(
            self,
            Self::DiscloseBeforeSettlement | Self::DiscloseAfterSettlement
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VaspReviewStatus {
    Pending,
    Approved,
    Rejected,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VaspInteroperabilityStatus {
    Unknown,
    Ready,
    Limited,
    Degraded,
    Disabled,
}

impl VaspInteroperabilityStatus {
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Ready | Self::Limited)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TravelRuleRequirement {
    Jurisdiction,
    AssetNetwork,
    Counterparty,
    CounterpartyJurisdiction,
    CounterpartyVaspCode,
    CounterpartyTravelRuleProfile,
    TransportProfile,
    ApprovedCounterparty,
    InteroperableCounterparty,
    DirectionalSupport,
}

pub use disclosure::{
    DisclosureLifecycleEvent, DisclosureLifecycleEventKind, DisclosureLifecycleStage,
    DisclosureStateMachine, DisclosureTransitionError, DisclosureTransitionRequest,
    DisclosureTransitionResult, ExceptionQueueStatus, TransportAttemptStatus,
};
pub use exchange::{
    TransportRetryPolicy, TravelRuleExchangeDispatch, TravelRuleExchangeError,
    TravelRuleExchangeRequest, TravelRuleExchangeResponse, TravelRuleExchangeService,
    TravelRuleRetryDecision, TravelRuleTransport, TravelRuleTransportAttempt,
    TravelRuleTransportError, TravelRuleTransportFactory, TravelRuleTransportProfile,
};
pub use policy::{
    AmountThreshold, AssetScope, CounterpartyScope, TravelRuleCounterparty,
    TravelRuleEvaluationRequest, TravelRuleEvaluationResult, TravelRulePolicy,
    TravelRulePolicyEngine,
};
pub use registry::{
    VaspInteroperabilityState, VaspInteroperabilityUpdate, VaspRegistryError, VaspRegistryRecord,
    VaspRegistryRecordInput, VaspRegistryService, VaspReviewState, VaspReviewUpdate,
};
