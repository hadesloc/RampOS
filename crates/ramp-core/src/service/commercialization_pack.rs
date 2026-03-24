use ramp_common::Result;
use ramp_compliance::provider_routing::{ProviderFamily, ProviderRoutingPolicyStore};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::repository::{
    CommercializationPackReferenceRecord, CommercializationPackRepository, CorridorPackRecord,
    CorridorPackRepository, PartnerCapabilityRecord, PartnerRegistryRecord,
    PartnerRegistryRepository, PaymentMethodCapabilityRecord, PgCommercializationPackRepository,
    PgCorridorPackRepository, PgPartnerRegistryRepository, PgPaymentMethodCapabilityRepository,
};
use crate::service::{
    CommercialReadinessService, CommercialReadinessSnapshot, PaymentMethodCapabilityService,
    ProviderRoutingContext, ProviderRoutingService,
};

pub use crate::repository::UpsertCommercializationPackRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackSnapshot {
    pub action_mode: String,
    pub source: String,
    pub packs: Vec<CommercializationPack>,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPack {
    pub commercialization_pack_id: String,
    pub tenant_id: Option<String>,
    pub pack_code: String,
    pub lifecycle_state: String,
    pub rollout_state: String,
    pub partner: CommercializationPackPartner,
    pub capability: CommercializationPackCapability,
    pub commercial_readiness: CommercializationPackReadiness,
    pub corridor: CommercializationPackCorridor,
    pub lanes: Vec<CommercializationPackLane>,
    pub runtime_reference: CommercializationPackRuntimeReference,
    pub metadata: serde_json::Value,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackPartner {
    pub partner_id: String,
    pub display_name: String,
    pub partner_class: String,
    pub approval_status: String,
    pub lifecycle_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackCapability {
    pub capability_id: String,
    pub capability_family: String,
    pub environment: String,
    pub approval_status: String,
    pub supported_rails: Vec<String>,
    pub supported_methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackReadiness {
    pub extension_id: String,
    pub extension_kind: String,
    pub enabled: bool,
    pub approval_reference: Option<String>,
    pub required_partner_capabilities: Vec<String>,
    pub required_corridor_packs: Vec<String>,
    pub required_compliance_checks: Vec<String>,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackCorridor {
    pub corridor_code: String,
    pub source_market: String,
    pub destination_market: String,
    pub settlement_direction: String,
    pub lifecycle_state: String,
    pub rollout_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackLane {
    pub lane_reference: String,
    pub settlement_direction: String,
    pub method_family: String,
    pub funding_source: Option<String>,
    pub presentment_model: Option<String>,
    pub runtime_target: String,
    pub compliance_bindings: Vec<CommercializationPackComplianceBinding>,
    pub policy_flags: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackComplianceBinding {
    pub hook_kind: String,
    pub required: bool,
    pub selected_provider_key: String,
    pub selected_provider_class: String,
    pub selection_source: String,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercializationPackRuntimeReference {
    pub pack_reference: String,
    pub corridor_reference: String,
    pub lane_references: Vec<String>,
    pub default_lane_reference: Option<String>,
    pub eligible: bool,
    pub fallback_behavior: String,
}

#[derive(Clone)]
pub struct CommercializationPackService {
    pack_repository: Option<Arc<dyn CommercializationPackRepository>>,
    partner_repository: Option<Arc<dyn PartnerRegistryRepository>>,
    corridor_repository: Option<Arc<dyn CorridorPackRepository>>,
    payment_method_capability_service: PaymentMethodCapabilityService,
    provider_routing_service: ProviderRoutingService,
    readiness_service: CommercialReadinessService,
}

impl CommercializationPackService {
    pub fn new() -> Self {
        Self {
            pack_repository: None,
            partner_repository: None,
            corridor_repository: None,
            payment_method_capability_service: PaymentMethodCapabilityService::new(),
            provider_routing_service: ProviderRoutingService::new(),
            readiness_service: CommercialReadinessService::new(),
        }
    }

    pub fn with_pack_repository(repository: Arc<dyn CommercializationPackRepository>) -> Self {
        Self {
            pack_repository: Some(repository),
            partner_repository: None,
            corridor_repository: None,
            payment_method_capability_service: PaymentMethodCapabilityService::new(),
            provider_routing_service: ProviderRoutingService::new(),
            readiness_service: CommercialReadinessService::new(),
        }
    }

    pub fn with_dependencies(
        pack_repository: Arc<dyn CommercializationPackRepository>,
        partner_repository: Arc<dyn PartnerRegistryRepository>,
        corridor_repository: Arc<dyn CorridorPackRepository>,
        readiness_service: CommercialReadinessService,
    ) -> Self {
        Self {
            pack_repository: Some(pack_repository),
            partner_repository: Some(partner_repository),
            corridor_repository: Some(corridor_repository),
            payment_method_capability_service: PaymentMethodCapabilityService::new(),
            provider_routing_service: ProviderRoutingService::new(),
            readiness_service,
        }
    }

    pub fn with_runtime_dependencies(
        pack_repository: Arc<dyn CommercializationPackRepository>,
        partner_repository: Arc<dyn PartnerRegistryRepository>,
        corridor_repository: Arc<dyn CorridorPackRepository>,
        payment_method_capability_service: PaymentMethodCapabilityService,
        provider_routing_service: ProviderRoutingService,
        readiness_service: CommercialReadinessService,
    ) -> Self {
        Self {
            pack_repository: Some(pack_repository),
            partner_repository: Some(partner_repository),
            corridor_repository: Some(corridor_repository),
            payment_method_capability_service,
            provider_routing_service,
            readiness_service,
        }
    }

    pub async fn from_pool_for_tenant(pool: PgPool, tenant_id: Option<&str>) -> Result<Self> {
        let store = ProviderRoutingPolicyStore::new(pool.clone());
        let mut policies = Vec::new();
        for family in all_provider_families() {
            let mut family_policies = store
                .list_policies(tenant_id, family)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
            policies.append(&mut family_policies);
        }

        Ok(Self {
            pack_repository: Some(Arc::new(PgCommercializationPackRepository::new(
                pool.clone(),
            ))),
            partner_repository: Some(Arc::new(PgPartnerRegistryRepository::new(pool.clone()))),
            corridor_repository: Some(Arc::new(PgCorridorPackRepository::new(pool.clone()))),
            payment_method_capability_service: PaymentMethodCapabilityService::with_repository(
                Arc::new(PgPaymentMethodCapabilityRepository::new(pool.clone())),
            ),
            provider_routing_service: ProviderRoutingService::from_policies(policies),
            readiness_service: CommercialReadinessService::from_pool_for_tenant(pool, tenant_id)
                .await?,
        })
    }

    pub async fn list_packs(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<CommercializationPackSnapshot> {
        let Some(pack_repository) = &self.pack_repository else {
            return Ok(fallback_snapshot("repository_not_configured"));
        };

        let records = pack_repository
            .list_commercialization_packs(tenant_id)
            .await?;
        if records.is_empty() {
            return Ok(fallback_snapshot("no_pack_records"));
        }

        let partner_repository = self.partner_repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal(
                "Partner registry repository is required to compose commercialization packs"
                    .to_string(),
            )
        })?;
        let corridor_repository = self.corridor_repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal(
                "Corridor pack repository is required to compose commercialization packs"
                    .to_string(),
            )
        })?;

        let partners = partner_repository.list_registry_records(tenant_id).await?;
        let corridors = corridor_repository.list_corridor_packs(tenant_id).await?;
        let readiness_snapshot = self.readiness_service.snapshot();

        let mut packs = Vec::with_capacity(records.len());
        for record in &records {
            packs.push(
                compose_pack(
                    record,
                    &partners,
                    &corridors,
                    &self.payment_method_capability_service,
                    &self.provider_routing_service,
                    &readiness_snapshot,
                )
                .await?,
            );
        }

        Ok(CommercializationPackSnapshot {
            action_mode: "commercialization_pack_registry".to_string(),
            source: "registry".to_string(),
            provenance: serde_json::json!({
                "sourceClass": "governed_registry",
                "packCount": packs.len(),
                "sources": [
                    "commercialization_packs",
                    "partners",
                    "corridor_packs",
                    "commercial_readiness"
                ],
            }),
            packs,
        })
    }

    pub async fn upsert_pack(
        &self,
        request: &UpsertCommercializationPackRequest,
    ) -> Result<CommercializationPackSnapshot> {
        let pack_repository = self.pack_repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal(
                "Commercialization pack repository is not configured".to_string(),
            )
        })?;
        let partner_repository = self.partner_repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal(
                "Partner registry repository is not configured".to_string(),
            )
        })?;
        let corridor_repository = self.corridor_repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal("Corridor pack repository is not configured".to_string())
        })?;

        let partners = partner_repository
            .list_registry_records(request.tenant_id.as_deref())
            .await?;
        let partner = partners
            .iter()
            .find(|partner| partner.partner_id == request.partner_id)
            .ok_or_else(|| {
                ramp_common::Error::Validation(format!(
                    "Partner '{}' not found for commercialization pack",
                    request.partner_id
                ))
            })?;
        let capability = partner
            .capabilities
            .iter()
            .find(|capability| capability.capability_id == request.partner_capability_id)
            .ok_or_else(|| {
                ramp_common::Error::Validation(format!(
                    "Partner capability '{}' does not belong to partner '{}'",
                    request.partner_capability_id, request.partner_id
                ))
            })?;
        let corridor = corridor_repository
            .get_corridor_pack(request.tenant_id.as_deref(), &request.corridor_code)
            .await?
            .ok_or_else(|| {
                ramp_common::Error::Validation(format!(
                    "Corridor '{}' not found for commercialization pack",
                    request.corridor_code
                ))
            })?;
        let readiness_snapshot = self.readiness_service.snapshot();
        let readiness_extension = readiness_snapshot
            .extensions
            .iter()
            .find(|extension| extension.extension_id == request.commercial_extension_id)
            .ok_or_else(|| {
                ramp_common::Error::Validation(format!(
                    "Commercial readiness extension '{}' not found",
                    request.commercial_extension_id
                ))
            })?;

        validate_extension_alignment(
            capability,
            corridor.corridor_code.as_str(),
            readiness_extension,
        )?;

        let normalized_request = UpsertCommercializationPackRequest {
            approval_reference: request
                .approval_reference
                .clone()
                .or_else(|| readiness_extension.approval_reference.clone()),
            ..request.clone()
        };

        pack_repository
            .upsert_commercialization_pack(&normalized_request)
            .await?;

        self.list_packs(normalized_request.tenant_id.as_deref())
            .await
    }

    pub async fn resolve_requested_provider(
        &self,
        tenant_id: Option<&str>,
        requested_provider: &str,
    ) -> Result<String> {
        let Some(pack_code) = requested_provider.strip_prefix("pack:") else {
            return Ok(requested_provider.to_string());
        };
        let Some(pack_repository) = &self.pack_repository else {
            return Ok(requested_provider.to_string());
        };

        let Some(pack) = pack_repository
            .get_commercialization_pack(tenant_id, pack_code)
            .await?
        else {
            return Ok(requested_provider.to_string());
        };

        if !is_runtime_eligible(&pack.lifecycle_state, &pack.rollout_state) {
            return Ok(requested_provider.to_string());
        }

        Ok(format!("corridor:{}", pack.corridor_code))
    }

    pub async fn resolve_runtime_target(
        &self,
        tenant_id: Option<&str>,
        requested_provider: &str,
        settlement_direction: &str,
        method_family: &str,
    ) -> Result<String> {
        let Some(pack_code) = requested_provider.strip_prefix("pack:") else {
            return Ok(requested_provider.to_string());
        };
        let Some(pack_repository) = &self.pack_repository else {
            return Ok(requested_provider.to_string());
        };

        let Some(record) = pack_repository
            .get_commercialization_pack(tenant_id, pack_code)
            .await?
        else {
            return Ok(requested_provider.to_string());
        };

        if !is_runtime_eligible(&record.lifecycle_state, &record.rollout_state) {
            return Ok(requested_provider.to_string());
        }

        let Some(partner_repository) = &self.partner_repository else {
            return Ok(format!("corridor:{}", record.corridor_code));
        };
        let Some(corridor_repository) = &self.corridor_repository else {
            return Ok(format!("corridor:{}", record.corridor_code));
        };
        let partners = partner_repository.list_registry_records(tenant_id).await?;
        let corridors = corridor_repository.list_corridor_packs(tenant_id).await?;
        let readiness_snapshot = self.readiness_service.snapshot();
        let pack = compose_pack(
            &record,
            &partners,
            &corridors,
            &self.payment_method_capability_service,
            &self.provider_routing_service,
            &readiness_snapshot,
        )
        .await?;

        Ok(pack
            .lanes
            .iter()
            .find(|lane| {
                lane.settlement_direction
                    .eq_ignore_ascii_case(settlement_direction)
                    && lane.method_family.eq_ignore_ascii_case(method_family)
            })
            .map(|lane| lane.runtime_target.clone())
            .or_else(|| pack.runtime_reference.default_lane_reference.clone())
            .unwrap_or_else(|| format!("corridor:{}", record.corridor_code)))
    }
}

impl Default for CommercializationPackService {
    fn default() -> Self {
        Self::new()
    }
}

fn fallback_snapshot(reason: &str) -> CommercializationPackSnapshot {
    CommercializationPackSnapshot {
        action_mode: "commercialization_pack_registry".to_string(),
        source: "fallback".to_string(),
        packs: Vec::new(),
        provenance: serde_json::json!({
            "sourceClass": "bounded_fallback",
            "reason": reason,
            "packCount": 0,
        }),
    }
}

async fn compose_pack(
    record: &CommercializationPackReferenceRecord,
    partners: &[PartnerRegistryRecord],
    corridors: &[CorridorPackRecord],
    payment_method_capability_service: &PaymentMethodCapabilityService,
    provider_routing_service: &ProviderRoutingService,
    readiness_snapshot: &CommercialReadinessSnapshot,
) -> Result<CommercializationPack> {
    let partner = partners
        .iter()
        .find(|partner| partner.partner_id == record.partner_id)
        .ok_or_else(|| {
            ramp_common::Error::Validation(format!(
                "Partner '{}' referenced by pack '{}' was not found",
                record.partner_id, record.pack_code
            ))
        })?;
    let capability = partner
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == record.partner_capability_id)
        .ok_or_else(|| {
            ramp_common::Error::Validation(format!(
                "Capability '{}' referenced by pack '{}' was not found",
                record.partner_capability_id, record.pack_code
            ))
        })?;
    let corridor = corridors
        .iter()
        .find(|corridor| corridor.corridor_code == record.corridor_code)
        .ok_or_else(|| {
            ramp_common::Error::Validation(format!(
                "Corridor '{}' referenced by pack '{}' was not found",
                record.corridor_code, record.pack_code
            ))
        })?;
    let readiness_extension = readiness_snapshot
        .extensions
        .iter()
        .find(|extension| extension.extension_id == record.commercial_extension_id)
        .ok_or_else(|| {
            ramp_common::Error::Validation(format!(
                "Commercial readiness extension '{}' referenced by pack '{}' was not found",
                record.commercial_extension_id, record.pack_code
            ))
        })?;

    let lanes = build_lanes(
        record,
        capability,
        corridor,
        payment_method_capability_service,
        provider_routing_service,
    )
    .await?;
    let runtime_reference = runtime_reference_for(record, &lanes);

    Ok(CommercializationPack {
        commercialization_pack_id: record.commercialization_pack_id.clone(),
        tenant_id: record.tenant_id.clone(),
        pack_code: record.pack_code.clone(),
        lifecycle_state: record.lifecycle_state.clone(),
        rollout_state: record.rollout_state.clone(),
        partner: CommercializationPackPartner {
            partner_id: partner.partner_id.clone(),
            display_name: partner.display_name.clone(),
            partner_class: partner.partner_class.clone(),
            approval_status: partner.approval_status.clone(),
            lifecycle_state: partner.lifecycle_state.clone(),
        },
        capability: CommercializationPackCapability {
            capability_id: capability.capability_id.clone(),
            capability_family: capability.capability_family.clone(),
            environment: capability.environment.clone(),
            approval_status: capability.approval_status.clone(),
            supported_rails: capability.supported_rails.clone(),
            supported_methods: capability.supported_methods.clone(),
        },
        commercial_readiness: CommercializationPackReadiness {
            extension_id: readiness_extension.extension_id.clone(),
            extension_kind: readiness_extension.extension_kind.clone(),
            enabled: readiness_extension.enabled,
            approval_reference: record
                .approval_reference
                .clone()
                .or_else(|| readiness_extension.approval_reference.clone()),
            required_partner_capabilities: readiness_extension
                .required_partner_capabilities
                .clone(),
            required_corridor_packs: readiness_extension.required_corridor_packs.clone(),
            required_compliance_checks: readiness_extension.required_compliance_checks.clone(),
            provenance: readiness_extension.metadata.clone(),
        },
        corridor: CommercializationPackCorridor {
            corridor_code: corridor.corridor_code.clone(),
            source_market: corridor.source_market.clone(),
            destination_market: corridor.destination_market.clone(),
            settlement_direction: corridor.settlement_direction.clone(),
            lifecycle_state: corridor.lifecycle_state.clone(),
            rollout_state: corridor.rollout_state.clone(),
        },
        lanes,
        runtime_reference,
        metadata: record.metadata.clone(),
        provenance: serde_json::json!({
            "sourceClass": "runtime_contract",
            "commercializationPackId": record.commercialization_pack_id,
            "partnerId": partner.partner_id,
            "partnerCapabilityId": capability.capability_id,
            "commercialExtensionId": readiness_extension.extension_id,
            "corridorCode": corridor.corridor_code,
        }),
    })
}

fn runtime_reference_for(
    record: &CommercializationPackReferenceRecord,
    lanes: &[CommercializationPackLane],
) -> CommercializationPackRuntimeReference {
    CommercializationPackRuntimeReference {
        pack_reference: format!("pack:{}", record.pack_code),
        corridor_reference: format!("corridor:{}", record.corridor_code),
        lane_references: lanes
            .iter()
            .map(|lane| lane.lane_reference.clone())
            .collect(),
        default_lane_reference: lanes.first().map(|lane| lane.lane_reference.clone()),
        eligible: is_runtime_eligible(&record.lifecycle_state, &record.rollout_state),
        fallback_behavior: "keep_requested_provider_when_pack_unresolved".to_string(),
    }
}

async fn build_lanes(
    record: &CommercializationPackReferenceRecord,
    capability: &PartnerCapabilityRecord,
    corridor: &CorridorPackRecord,
    payment_method_capability_service: &PaymentMethodCapabilityService,
    provider_routing_service: &ProviderRoutingService,
) -> Result<Vec<CommercializationPackLane>> {
    let capabilities = payment_method_capability_service
        .list_capabilities(
            Some(&corridor.corridor_pack_id),
            Some(&capability.capability_id),
        )
        .await?;

    let mut lanes = Vec::new();
    for method_capability in capabilities.capabilities {
        let lane_reference = format!(
            "lane:{}:{}:{}",
            record.pack_code,
            method_capability.settlement_direction.to_ascii_lowercase(),
            method_capability.method_family
        );
        lanes.push(CommercializationPackLane {
            lane_reference,
            settlement_direction: method_capability.settlement_direction.clone(),
            method_family: method_capability.method_family.clone(),
            funding_source: method_capability.funding_source.clone(),
            presentment_model: method_capability.presentment_model.clone(),
            runtime_target: resolve_lane_runtime_target(corridor, &method_capability)
                .unwrap_or_else(|| format!("corridor:{}", corridor.corridor_code)),
            compliance_bindings: build_compliance_bindings(
                corridor,
                &method_capability,
                provider_routing_service,
            ),
            policy_flags: method_capability.policy_flags.clone(),
            metadata: method_capability.metadata.clone(),
        });
    }

    Ok(lanes)
}

fn resolve_lane_runtime_target(
    corridor: &CorridorPackRecord,
    method_capability: &PaymentMethodCapabilityRecord,
) -> Option<String> {
    corridor
        .endpoints
        .iter()
        .find(|endpoint| {
            endpoint.endpoint_role.eq_ignore_ascii_case(
                if method_capability
                    .settlement_direction
                    .eq_ignore_ascii_case("payout")
                {
                    "destination"
                } else {
                    "source"
                },
            ) && endpoint
                .method_family
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(&method_capability.method_family))
                .unwrap_or(true)
        })
        .and_then(|endpoint| {
            endpoint
                .adapter_key
                .clone()
                .or_else(|| endpoint.provider_key.clone())
                .or_else(|| endpoint.partner_id.clone())
        })
        .filter(|value| !value.is_empty())
}

fn build_compliance_bindings(
    corridor: &CorridorPackRecord,
    method_capability: &PaymentMethodCapabilityRecord,
    provider_routing_service: &ProviderRoutingService,
) -> Vec<CommercializationPackComplianceBinding> {
    corridor
        .compliance_hooks
        .iter()
        .filter(|hook| hook.required)
        .map(|hook| {
            let decision = provider_routing_service.evaluate(&ProviderRoutingContext {
                provider_family: Some(hook.hook_kind.clone()),
                corridor_code: Some(corridor.corridor_code.clone()),
                entity_type: corridor
                    .eligibility_rules
                    .iter()
                    .find(|rule| {
                        rule.method_family
                            .as_deref()
                            .map(|value| {
                                value.eq_ignore_ascii_case(&method_capability.method_family)
                            })
                            .unwrap_or(true)
                    })
                    .and_then(|rule| rule.entity_type.clone())
                    .or_else(|| {
                        corridor
                            .endpoints
                            .iter()
                            .find(|endpoint| {
                                endpoint.endpoint_role.eq_ignore_ascii_case(
                                    if method_capability
                                        .settlement_direction
                                        .eq_ignore_ascii_case("payout")
                                    {
                                        "destination"
                                    } else {
                                        "source"
                                    },
                                ) && endpoint
                                    .method_family
                                    .as_deref()
                                    .map(|value| {
                                        value.eq_ignore_ascii_case(&method_capability.method_family)
                                    })
                                    .unwrap_or(true)
                            })
                            .map(|endpoint| endpoint.entity_type.clone())
                    }),
                risk_tier: None,
                amount: None,
                asset: None,
                partner_id: corridor
                    .eligibility_rules
                    .iter()
                    .find(|rule| {
                        rule.method_family
                            .as_deref()
                            .map(|value| {
                                value.eq_ignore_ascii_case(&method_capability.method_family)
                            })
                            .unwrap_or(true)
                    })
                    .and_then(|rule| rule.partner_id.clone())
                    .or_else(|| {
                        corridor
                            .endpoints
                            .iter()
                            .find(|endpoint| {
                                endpoint.endpoint_role.eq_ignore_ascii_case(
                                    if method_capability
                                        .settlement_direction
                                        .eq_ignore_ascii_case("payout")
                                    {
                                        "destination"
                                    } else {
                                        "source"
                                    },
                                ) && endpoint
                                    .method_family
                                    .as_deref()
                                    .map(|value| {
                                        value.eq_ignore_ascii_case(&method_capability.method_family)
                                    })
                                    .unwrap_or(true)
                            })
                            .and_then(|endpoint| endpoint.partner_id.clone())
                    }),
                tenant_id: None,
            });

            match decision {
                Ok(decision) if !decision.fallback_used => CommercializationPackComplianceBinding {
                    hook_kind: hook.hook_kind.clone(),
                    required: hook.required,
                    selected_provider_key: decision.selected_provider_key,
                    selected_provider_class: decision.selected_provider_class,
                    selection_source: "provider_routing_policy".to_string(),
                    provenance: decision.provenance,
                },
                _ => CommercializationPackComplianceBinding {
                    hook_kind: hook.hook_kind.clone(),
                    required: hook.required,
                    selected_provider_key: hook
                        .provider_key
                        .clone()
                        .unwrap_or_else(|| "unassigned".to_string()),
                    selected_provider_class: hook.hook_kind.clone(),
                    selection_source: "corridor_compliance_hook".to_string(),
                    provenance: serde_json::json!({
                        "sourceClass": "corridor_pack",
                        "hookKind": hook.hook_kind,
                    }),
                },
            }
        })
        .collect()
}

fn all_provider_families() -> [ProviderFamily; 6] {
    [
        ProviderFamily::Kyc,
        ProviderFamily::Kyb,
        ProviderFamily::Kyt,
        ProviderFamily::Sanctions,
        ProviderFamily::AdverseMedia,
        ProviderFamily::TravelRule,
    ]
}

fn validate_extension_alignment(
    capability: &PartnerCapabilityRecord,
    corridor_code: &str,
    readiness_extension: &crate::service::CommercialExtensionRecord,
) -> Result<()> {
    if !readiness_extension.required_partner_capabilities.is_empty()
        && !readiness_extension
            .required_partner_capabilities
            .iter()
            .any(|required| required.eq_ignore_ascii_case(&capability.capability_family))
    {
        return Err(ramp_common::Error::Validation(format!(
            "Capability family '{}' does not satisfy commercial extension '{}'",
            capability.capability_family, readiness_extension.extension_id
        )));
    }

    if !readiness_extension.required_corridor_packs.is_empty()
        && !readiness_extension
            .required_corridor_packs
            .iter()
            .any(|required| required.eq_ignore_ascii_case(corridor_code))
    {
        return Err(ramp_common::Error::Validation(format!(
            "Corridor '{}' does not satisfy commercial extension '{}'",
            corridor_code, readiness_extension.extension_id
        )));
    }

    Ok(())
}

fn is_runtime_eligible(lifecycle_state: &str, rollout_state: &str) -> bool {
    matches!(
        lifecycle_state.to_ascii_lowercase().as_str(),
        "active" | "pilot"
    ) && matches!(
        rollout_state.to_ascii_lowercase().as_str(),
        "approved" | "active"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        CommercializationPackReferenceRecord, CorridorPackRepository,
        PaymentMethodCapabilityRecord, PaymentMethodCapabilityRepository,
        UpsertApprovalReferenceRequest, UpsertCommercializationPackRequest,
        UpsertCorridorComplianceHookRequest, UpsertCorridorCutoffPolicyRequest,
        UpsertCorridorEligibilityRuleRequest, UpsertCorridorEndpointRequest,
        UpsertCorridorFeeProfileRequest, UpsertCorridorPackRequest,
        UpsertCorridorRolloutScopeRequest, UpsertCredentialReferenceRequest,
        UpsertPartnerCapabilityRequest, UpsertPartnerHealthSignalRequest, UpsertPartnerRequest,
        UpsertPartnerRolloutScopeRequest,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockCommercializationPackRepository {
        records: Mutex<Vec<CommercializationPackReferenceRecord>>,
    }

    #[async_trait]
    impl CommercializationPackRepository for MockCommercializationPackRepository {
        async fn upsert_commercialization_pack(
            &self,
            request: &UpsertCommercializationPackRequest,
        ) -> Result<()> {
            let mut records = self.records.lock().expect("records lock");
            records.retain(|record| {
                record.commercialization_pack_id != request.commercialization_pack_id
            });
            records.push(CommercializationPackReferenceRecord {
                commercialization_pack_id: request.commercialization_pack_id.clone(),
                tenant_id: request.tenant_id.clone(),
                pack_code: request.pack_code.clone(),
                partner_id: request.partner_id.clone(),
                partner_capability_id: request.partner_capability_id.clone(),
                commercial_extension_id: request.commercial_extension_id.clone(),
                corridor_code: request.corridor_code.clone(),
                approval_reference: request.approval_reference.clone(),
                lifecycle_state: request.lifecycle_state.clone(),
                rollout_state: request.rollout_state.clone(),
                metadata: request.metadata.clone(),
            });
            Ok(())
        }

        async fn list_commercialization_packs(
            &self,
            tenant_id: Option<&str>,
        ) -> Result<Vec<CommercializationPackReferenceRecord>> {
            Ok(self
                .records
                .lock()
                .expect("records lock")
                .iter()
                .filter(|record| {
                    tenant_id
                        .map(|value| record.tenant_id.as_deref() == Some(value))
                        .unwrap_or(true)
                })
                .cloned()
                .collect())
        }

        async fn get_commercialization_pack(
            &self,
            tenant_id: Option<&str>,
            pack_code: &str,
        ) -> Result<Option<CommercializationPackReferenceRecord>> {
            Ok(self
                .records
                .lock()
                .expect("records lock")
                .iter()
                .find(|record| {
                    record.pack_code == pack_code
                        && tenant_id
                            .map(|value| record.tenant_id.as_deref() == Some(value))
                            .unwrap_or(true)
                })
                .cloned())
        }
    }

    #[derive(Clone)]
    struct MockPartnerRepository {
        partners: Vec<PartnerRegistryRecord>,
    }

    #[async_trait]
    impl PartnerRegistryRepository for MockPartnerRepository {
        async fn upsert_approval_reference(
            &self,
            _request: &UpsertApprovalReferenceRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_partner(&self, _request: &UpsertPartnerRequest) -> Result<()> {
            Ok(())
        }
        async fn upsert_capability(&self, _request: &UpsertPartnerCapabilityRequest) -> Result<()> {
            Ok(())
        }
        async fn upsert_rollout_scope(
            &self,
            _request: &UpsertPartnerRolloutScopeRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_health_signal(
            &self,
            _request: &UpsertPartnerHealthSignalRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_credential_reference(
            &self,
            _request: &UpsertCredentialReferenceRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn list_registry_records(
            &self,
            _tenant_id: Option<&str>,
        ) -> Result<Vec<PartnerRegistryRecord>> {
            Ok(self.partners.clone())
        }
    }

    #[derive(Clone)]
    struct MockCorridorRepository {
        corridors: Vec<CorridorPackRecord>,
    }

    #[derive(Clone)]
    struct MockPaymentMethodCapabilityRepository {
        capabilities: Vec<PaymentMethodCapabilityRecord>,
    }

    #[async_trait]
    impl CorridorPackRepository for MockCorridorRepository {
        async fn upsert_corridor_pack(&self, _request: &UpsertCorridorPackRequest) -> Result<()> {
            Ok(())
        }
        async fn upsert_endpoint(&self, _request: &UpsertCorridorEndpointRequest) -> Result<()> {
            Ok(())
        }
        async fn upsert_fee_profile(
            &self,
            _request: &UpsertCorridorFeeProfileRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_cutoff_policy(
            &self,
            _request: &UpsertCorridorCutoffPolicyRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_compliance_hook(
            &self,
            _request: &UpsertCorridorComplianceHookRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_rollout_scope(
            &self,
            _request: &UpsertCorridorRolloutScopeRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn upsert_eligibility_rule(
            &self,
            _request: &UpsertCorridorEligibilityRuleRequest,
        ) -> Result<()> {
            Ok(())
        }
        async fn list_corridor_packs(
            &self,
            _tenant_id: Option<&str>,
        ) -> Result<Vec<CorridorPackRecord>> {
            Ok(self.corridors.clone())
        }
        async fn get_corridor_pack(
            &self,
            _tenant_id: Option<&str>,
            corridor_code: &str,
        ) -> Result<Option<CorridorPackRecord>> {
            Ok(self
                .corridors
                .iter()
                .find(|corridor| corridor.corridor_code == corridor_code)
                .cloned())
        }
    }

    #[async_trait]
    impl PaymentMethodCapabilityRepository for MockPaymentMethodCapabilityRepository {
        async fn upsert_payment_method_capability(
            &self,
            _request: &crate::repository::UpsertPaymentMethodCapabilityRequest,
        ) -> Result<()> {
            Ok(())
        }

        async fn list_payment_method_capabilities(
            &self,
            corridor_pack_id: Option<&str>,
            partner_capability_id: Option<&str>,
        ) -> Result<Vec<PaymentMethodCapabilityRecord>> {
            Ok(self
                .capabilities
                .iter()
                .filter(|capability| {
                    corridor_pack_id
                        .map(|value| capability.corridor_pack_id == value)
                        .unwrap_or(true)
                        && partner_capability_id
                            .map(|value| capability.partner_capability_id.as_deref() == Some(value))
                            .unwrap_or(true)
                })
                .cloned()
                .collect())
        }
    }

    fn readiness_service() -> CommercialReadinessService {
        CommercialReadinessService::with_extensions(vec![
            crate::service::CommercialExtensionRecord {
                extension_id: "card_payout".to_string(),
                extension_kind: "card_adjacent".to_string(),
                label: "Card Payout".to_string(),
                description: "Pilot card payout".to_string(),
                enabled: false,
                approval_reference: None,
                required_partner_capabilities: vec!["card_issuing".to_string()],
                required_corridor_packs: vec![],
                required_compliance_checks: vec![],
                metadata: serde_json::json!({"sourceClass":"governed_registry"}),
            },
        ])
    }

    fn sample_partner() -> PartnerRegistryRecord {
        PartnerRegistryRecord {
            partner_id: "partner_card_hk".to_string(),
            tenant_id: Some("tenant-a".to_string()),
            partner_class: "issuer".to_string(),
            code: "card-hk".to_string(),
            display_name: "Card Partner HK".to_string(),
            legal_name: None,
            market: Some("HK".to_string()),
            jurisdiction: None,
            service_domain: "card_distribution".to_string(),
            lifecycle_state: "active".to_string(),
            approval_status: "approved".to_string(),
            metadata: serde_json::json!({}),
            capabilities: vec![PartnerCapabilityRecord {
                capability_id: "capability_card_hk".to_string(),
                capability_family: "card_issuing".to_string(),
                environment: "production".to_string(),
                adapter_key: None,
                provider_key: None,
                supported_rails: vec!["fps".to_string()],
                supported_methods: vec!["push_transfer".to_string()],
                approval_status: "approved".to_string(),
                metadata: serde_json::json!({}),
                rollout_scopes: vec![],
                health_signals: vec![],
            }],
            credential_references: vec![],
        }
    }

    fn sample_corridor() -> CorridorPackRecord {
        CorridorPackRecord {
            corridor_pack_id: "corridor_vn_hk".to_string(),
            tenant_id: Some("tenant-a".to_string()),
            corridor_code: "VN_HK_PAYOUT".to_string(),
            source_market: "VN".to_string(),
            destination_market: "HK".to_string(),
            source_currency: "VND".to_string(),
            destination_currency: "HKD".to_string(),
            settlement_direction: "payout".to_string(),
            fee_model: "shared".to_string(),
            lifecycle_state: "active".to_string(),
            rollout_state: "approved".to_string(),
            eligibility_state: "eligible".to_string(),
            metadata: serde_json::json!({}),
            endpoints: vec![crate::repository::CorridorEndpointRecord {
                endpoint_id: "endpoint_hk_destination".to_string(),
                endpoint_role: "destination".to_string(),
                partner_id: Some("partner_card_hk".to_string()),
                provider_key: None,
                adapter_key: Some("mock".to_string()),
                entity_type: "individual".to_string(),
                rail: "fps".to_string(),
                method_family: Some("push_transfer".to_string()),
                settlement_mode: Some("same_day".to_string()),
                instrument_family: Some("bank_transfer".to_string()),
                metadata: serde_json::json!({}),
            }],
            fee_profiles: vec![],
            cutoff_policies: vec![],
            compliance_hooks: vec![crate::repository::CorridorComplianceHookRecord {
                compliance_hook_id: "hook_travel_rule".to_string(),
                hook_kind: "travel_rule".to_string(),
                provider_key: Some("travel_rule_provider".to_string()),
                required: true,
                config: serde_json::json!({}),
                metadata: serde_json::json!({}),
            }],
            rollout_scopes: vec![],
            eligibility_rules: vec![crate::repository::CorridorEligibilityRuleRecord {
                eligibility_rule_id: "eligibility_hk".to_string(),
                partner_id: Some("partner_card_hk".to_string()),
                entity_type: Some("individual".to_string()),
                method_family: Some("push_transfer".to_string()),
                amount_bounds: serde_json::json!({"min":"100","max":"10000"}),
                compliance_requirements: serde_json::json!({"travelRule":true}),
                metadata: serde_json::json!({}),
            }],
        }
    }

    fn sample_payment_method_capability() -> PaymentMethodCapabilityRecord {
        PaymentMethodCapabilityRecord {
            payment_method_capability_id: "pmc_vn_hk_push_transfer".to_string(),
            corridor_pack_id: "corridor_vn_hk".to_string(),
            partner_capability_id: Some("capability_card_hk".to_string()),
            method_family: "push_transfer".to_string(),
            funding_source: Some("bank_account".to_string()),
            settlement_direction: "payout".to_string(),
            presentment_model: Some("server_driven".to_string()),
            card_funding_enabled: false,
            policy_flags: serde_json::json!({"travelRule":true}),
            metadata: serde_json::json!({"pilotLane":"default"}),
        }
    }

    fn provider_routing_service() -> crate::service::ProviderRoutingService {
        crate::service::ProviderRoutingService::with_rules(vec![
            crate::service::ProviderRoutingRule {
                rule_id: "provider_policy_vn_hk_travel_rule".to_string(),
                priority: 1,
                corridor_code: Some("VN_HK_PAYOUT".to_string()),
                entity_type: Some("individual".to_string()),
                risk_tier: None,
                amount_min: None,
                amount_max: None,
                asset: None,
                partner_id: Some("partner_card_hk".to_string()),
                provider_key: "notabene".to_string(),
                provider_class: "travel_rule".to_string(),
                enabled: true,
                metadata: serde_json::json!({
                    "policyId": "provider_policy_vn_hk_travel_rule",
                    "sourceClass": "persisted_registry",
                }),
            },
        ])
    }

    #[tokio::test]
    async fn list_packs_composes_runtime_reference_from_governed_records() {
        let pack_repository = Arc::new(MockCommercializationPackRepository::default());
        pack_repository
            .upsert_commercialization_pack(&UpsertCommercializationPackRequest {
                commercialization_pack_id: "pack_vn_hk".to_string(),
                tenant_id: Some("tenant-a".to_string()),
                pack_code: "pilot_vn_hk".to_string(),
                partner_id: "partner_card_hk".to_string(),
                partner_capability_id: "capability_card_hk".to_string(),
                commercial_extension_id: "card_payout".to_string(),
                corridor_code: "VN_HK_PAYOUT".to_string(),
                approval_reference: None,
                lifecycle_state: "active".to_string(),
                rollout_state: "approved".to_string(),
                metadata: serde_json::json!({"pilot":true}),
            })
            .await
            .expect("pack should persist");

        let service = CommercializationPackService::with_dependencies(
            pack_repository,
            Arc::new(MockPartnerRepository {
                partners: vec![sample_partner()],
            }),
            Arc::new(MockCorridorRepository {
                corridors: vec![sample_corridor()],
            }),
            readiness_service(),
        );

        let snapshot = service
            .list_packs(Some("tenant-a"))
            .await
            .expect("snapshot should load");

        assert_eq!(snapshot.source, "registry");
        assert_eq!(
            snapshot.packs[0].runtime_reference.pack_reference,
            "pack:pilot_vn_hk"
        );
        assert_eq!(
            snapshot.packs[0].runtime_reference.corridor_reference,
            "corridor:VN_HK_PAYOUT"
        );
        assert_eq!(
            snapshot.packs[0].capability.capability_family,
            "card_issuing"
        );
    }

    #[tokio::test]
    async fn list_packs_composes_explicit_lane_contract_from_current_records() {
        let pack_repository = Arc::new(MockCommercializationPackRepository::default());
        pack_repository
            .upsert_commercialization_pack(&UpsertCommercializationPackRequest {
                commercialization_pack_id: "pack_vn_hk".to_string(),
                tenant_id: Some("tenant-a".to_string()),
                pack_code: "pilot_vn_hk".to_string(),
                partner_id: "partner_card_hk".to_string(),
                partner_capability_id: "capability_card_hk".to_string(),
                commercial_extension_id: "card_payout".to_string(),
                corridor_code: "VN_HK_PAYOUT".to_string(),
                approval_reference: Some("approval_card_pack_ops".to_string()),
                lifecycle_state: "active".to_string(),
                rollout_state: "approved".to_string(),
                metadata: serde_json::json!({"pilot":true}),
            })
            .await
            .expect("pack should persist");

        let service = CommercializationPackService::with_runtime_dependencies(
            pack_repository,
            Arc::new(MockPartnerRepository {
                partners: vec![sample_partner()],
            }),
            Arc::new(MockCorridorRepository {
                corridors: vec![sample_corridor()],
            }),
            crate::service::PaymentMethodCapabilityService::with_repository(Arc::new(
                MockPaymentMethodCapabilityRepository {
                    capabilities: vec![sample_payment_method_capability()],
                },
            )),
            provider_routing_service(),
            readiness_service(),
        );

        let snapshot = service
            .list_packs(Some("tenant-a"))
            .await
            .expect("snapshot should load");
        let payload = serde_json::to_value(snapshot).expect("snapshot should serialize");

        assert_eq!(
            payload["packs"][0]["runtimeReference"]["defaultLaneReference"],
            "lane:pilot_vn_hk:payout:push_transfer"
        );
        assert_eq!(
            payload["packs"][0]["runtimeReference"]["laneReferences"][0],
            "lane:pilot_vn_hk:payout:push_transfer"
        );
        assert_eq!(
            payload["packs"][0]["lanes"][0]["laneReference"],
            "lane:pilot_vn_hk:payout:push_transfer"
        );
        assert_eq!(payload["packs"][0]["lanes"][0]["runtimeTarget"], "mock");
        assert_eq!(
            payload["packs"][0]["lanes"][0]["complianceBindings"][0]["selectedProviderKey"],
            "notabene"
        );
    }

    #[tokio::test]
    async fn resolve_runtime_target_prefers_explicit_lane_binding() {
        let pack_repository = Arc::new(MockCommercializationPackRepository::default());
        pack_repository
            .upsert_commercialization_pack(&UpsertCommercializationPackRequest {
                commercialization_pack_id: "pack_vn_hk".to_string(),
                tenant_id: Some("tenant-a".to_string()),
                pack_code: "pilot_vn_hk".to_string(),
                partner_id: "partner_card_hk".to_string(),
                partner_capability_id: "capability_card_hk".to_string(),
                commercial_extension_id: "card_payout".to_string(),
                corridor_code: "VN_HK_PAYOUT".to_string(),
                approval_reference: Some("approval_card_pack_ops".to_string()),
                lifecycle_state: "active".to_string(),
                rollout_state: "approved".to_string(),
                metadata: serde_json::json!({"pilot":true}),
            })
            .await
            .expect("pack should persist");

        let service = CommercializationPackService::with_runtime_dependencies(
            pack_repository,
            Arc::new(MockPartnerRepository {
                partners: vec![sample_partner()],
            }),
            Arc::new(MockCorridorRepository {
                corridors: vec![sample_corridor()],
            }),
            crate::service::PaymentMethodCapabilityService::with_repository(Arc::new(
                MockPaymentMethodCapabilityRepository {
                    capabilities: vec![sample_payment_method_capability()],
                },
            )),
            provider_routing_service(),
            readiness_service(),
        );

        let resolved = service
            .resolve_runtime_target(
                Some("tenant-a"),
                "pack:pilot_vn_hk",
                "payout",
                "push_transfer",
            )
            .await
            .expect("resolution should succeed");

        assert_eq!(resolved, "mock");
    }

    #[tokio::test]
    async fn resolve_requested_provider_keeps_fallback_for_unknown_pack() {
        let service = CommercializationPackService::with_pack_repository(Arc::new(
            MockCommercializationPackRepository::default(),
        ));

        let resolved = service
            .resolve_requested_provider(Some("tenant-a"), "pack:missing")
            .await
            .expect("resolution should succeed");

        assert_eq!(resolved, "pack:missing");
    }
}
