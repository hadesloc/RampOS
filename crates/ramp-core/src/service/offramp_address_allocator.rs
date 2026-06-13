use crate::chain::{Chain, SolanaChain, SolanaChainConfig};
use crate::repository::{PartnerCapabilityRecord, PartnerRegistryRecord};
use crate::service::partner_registry::PartnerRegistrySnapshot;
use crate::service::{extract_offramp_bundle_config, ConfigBundleArtifact};
use ramp_common::{onchain_gate, Error, Result};
use std::collections::BTreeSet;
use std::sync::Arc;
use tracing::warn;

const SOLANA_CHAIN_ID: i64 = 101;
const SOLANA_CUSTODY_CREDENTIAL_KIND: &str = "offramp_deposit_address_solana";
const ETHEREUM_CHAIN_ID: i64 = 1;
const BNB_CHAIN_ID: i64 = 56;
const POLYGON_CHAIN_ID: i64 = 137;
const AVALANCHE_CHAIN_ID: i64 = 43114;

#[derive(Clone, Copy)]
struct GovernedEvmLane {
    chain_id: i64,
    display_name: &'static str,
    rail_aliases: &'static [&'static str],
    corridor_aliases: &'static [&'static str],
    credential_aliases: &'static [&'static str],
}

const ETHEREUM_GOVERNED_EVM_LANE: GovernedEvmLane = GovernedEvmLane {
    chain_id: ETHEREUM_CHAIN_ID,
    display_name: "Ethereum",
    rail_aliases: &["ethereum", "eth"],
    corridor_aliases: &["ethereum", "eth"],
    credential_aliases: &["ethereum", "eth"],
};

const BNB_GOVERNED_EVM_LANE: GovernedEvmLane = GovernedEvmLane {
    chain_id: BNB_CHAIN_ID,
    display_name: "BNB",
    rail_aliases: &["bnb", "bsc"],
    corridor_aliases: &["bnb", "bsc"],
    credential_aliases: &["bnb", "bsc", "binance"],
};

const POLYGON_GOVERNED_EVM_LANE: GovernedEvmLane = GovernedEvmLane {
    chain_id: POLYGON_CHAIN_ID,
    display_name: "Polygon",
    rail_aliases: &["polygon", "matic"],
    corridor_aliases: &["polygon", "matic"],
    credential_aliases: &["polygon", "matic"],
};

const AVALANCHE_GOVERNED_EVM_LANE: GovernedEvmLane = GovernedEvmLane {
    chain_id: AVALANCHE_CHAIN_ID,
    display_name: "Avalanche",
    rail_aliases: &["avalanche", "avax"],
    corridor_aliases: &["avalanche", "avax"],
    credential_aliases: &["avalanche", "avax"],
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfframpDepositAddressRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub chain_id: Option<i64>,
}

pub trait OfframpDepositAddressProvider: Send + Sync {
    fn supports_chain(&self, chain_id: Option<i64>) -> bool;
    fn issue(&self, request: &OfframpDepositAddressRequest) -> Result<String>;
}

pub struct EvmPlaceholderDepositAddressProvider;

impl OfframpDepositAddressProvider for EvmPlaceholderDepositAddressProvider {
    fn supports_chain(&self, chain_id: Option<i64>) -> bool {
        matches!(
            chain_id,
            None | Some(1 | 137 | 56 | 42161 | 10 | 8453 | 43114)
        )
    }

    fn issue(&self, request: &OfframpDepositAddressRequest) -> Result<String> {
        if is_governed_deposit_address_chain(request.chain_id) {
            if onchain_gate::is_production() {
                return Err(Error::Conflict(format!(
                    "Governed off-ramp deposit address issuance for chain_id={} requires an approved registry or bundle custody configuration in production; refusing placeholder address issuance",
                    display_chain_id(request.chain_id)
                )));
            }

            warn!(
                tenant_id = %request.tenant_id,
                user_id = %request.user_id,
                chain_id = %display_chain_id(request.chain_id),
                "Issuing placeholder off-ramp deposit address without approved governed custody configuration; this is forbidden in production"
            );
        }

        Ok(format!(
            "0x{:040x}",
            uuid::Uuid::now_v7().as_u128() & u128::MAX
        ))
    }
}

pub struct UnsupportedSolanaDepositAddressProvider;

impl OfframpDepositAddressProvider for UnsupportedSolanaDepositAddressProvider {
    fn supports_chain(&self, chain_id: Option<i64>) -> bool {
        chain_id == Some(101)
    }

    fn issue(&self, _request: &OfframpDepositAddressRequest) -> Result<String> {
        Err(Error::Conflict(
            "Solana deposit address issuance is unavailable in this runtime".to_string(),
        ))
    }
}

pub struct OfframpDepositAddressAllocator {
    providers: Vec<Arc<dyn OfframpDepositAddressProvider>>,
}

impl OfframpDepositAddressAllocator {
    pub fn new() -> Self {
        Self::with_providers(vec![
            Arc::new(UnsupportedSolanaDepositAddressProvider),
            Arc::new(EvmPlaceholderDepositAddressProvider),
        ])
    }

    pub fn with_providers(providers: Vec<Arc<dyn OfframpDepositAddressProvider>>) -> Self {
        Self { providers }
    }

    pub fn issue(&self, request: &OfframpDepositAddressRequest) -> Result<String> {
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.supports_chain(request.chain_id))
            .ok_or_else(|| {
                Error::Validation(format!(
                    "Off-ramp deposit address issuance is unsupported for chain_id={}",
                    request
                        .chain_id
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "none".to_string())
                ))
            })?;

        provider.issue(request)
    }

    pub fn configured_address_from_bundle(
        request: &OfframpDepositAddressRequest,
        bundle: &ConfigBundleArtifact,
    ) -> Result<Option<String>> {
        let Some(chain_id) = request.chain_id else {
            return Ok(None);
        };
        if !is_supported_governed_bundle_chain(chain_id) {
            return Ok(None);
        }

        if bundle.source.as_deref() != Some("registry")
            || !bundle
                .approval_status
                .as_deref()
                .map(|status| status.eq_ignore_ascii_case("approved"))
                .unwrap_or(false)
        {
            return Ok(None);
        }

        let Some(config) = extract_offramp_bundle_config(&bundle.payload)? else {
            return Ok(None);
        };
        let chain_key = chain_id.to_string();
        let Some(address) = config
            .deposit_addresses_by_chain
            .get(&chain_key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };

        validate_bundle_address(chain_id, address)?;
        Ok(Some(address.to_string()))
    }

    pub fn configured_address_from_registry(
        request: &OfframpDepositAddressRequest,
        snapshot: &PartnerRegistrySnapshot,
    ) -> Result<Option<String>> {
        if !snapshot.source.eq_ignore_ascii_case("registry") {
            return Ok(None);
        }

        match request.chain_id {
            Some(SOLANA_CHAIN_ID) => configured_solana_address_from_registry(request, snapshot),
            Some(ETHEREUM_CHAIN_ID) => configured_ethereum_address_from_registry(request, snapshot),
            Some(BNB_CHAIN_ID) => configured_bnb_address_from_registry(request, snapshot),
            Some(POLYGON_CHAIN_ID) => configured_polygon_address_from_registry(request, snapshot),
            Some(AVALANCHE_CHAIN_ID) => {
                configured_avalanche_address_from_registry(request, snapshot)
            }
            _ => Ok(None),
        }
    }
}

fn configured_ethereum_address_from_registry(
    request: &OfframpDepositAddressRequest,
    snapshot: &PartnerRegistrySnapshot,
) -> Result<Option<String>> {
    configured_governed_evm_address_from_registry(request, snapshot, &ETHEREUM_GOVERNED_EVM_LANE)
}

fn configured_bnb_address_from_registry(
    request: &OfframpDepositAddressRequest,
    snapshot: &PartnerRegistrySnapshot,
) -> Result<Option<String>> {
    configured_governed_evm_address_from_registry(request, snapshot, &BNB_GOVERNED_EVM_LANE)
}

fn configured_polygon_address_from_registry(
    request: &OfframpDepositAddressRequest,
    snapshot: &PartnerRegistrySnapshot,
) -> Result<Option<String>> {
    configured_governed_evm_address_from_registry(request, snapshot, &POLYGON_GOVERNED_EVM_LANE)
}

fn configured_avalanche_address_from_registry(
    request: &OfframpDepositAddressRequest,
    snapshot: &PartnerRegistrySnapshot,
) -> Result<Option<String>> {
    configured_governed_evm_address_from_registry(request, snapshot, &AVALANCHE_GOVERNED_EVM_LANE)
}

fn configured_governed_evm_address_from_registry(
    request: &OfframpDepositAddressRequest,
    snapshot: &PartnerRegistrySnapshot,
    lane: &GovernedEvmLane,
) -> Result<Option<String>> {
    let eligible_partners: Vec<&PartnerRegistryRecord> = snapshot
        .partners
        .iter()
        .filter(|partner| {
            partner.approval_status.eq_ignore_ascii_case("approved")
                && partner.lifecycle_state.eq_ignore_ascii_case("active")
        })
        .filter(|partner| {
            partner.capabilities.iter().any(|capability| {
                is_eligible_governed_evm_custody_capability(capability, &request.tenant_id, lane)
            })
        })
        .collect();
    if eligible_partners.is_empty() {
        return Ok(None);
    }

    let mut resolved_addresses: BTreeSet<String> = BTreeSet::new();
    for partner in eligible_partners {
        let partner_locators: BTreeSet<String> = partner
            .credential_references
            .iter()
            .filter(|credential| credential.environment.eq_ignore_ascii_case("production"))
            .filter(|credential| {
                is_eligible_governed_evm_custody_credential_kind(&credential.credential_kind, lane)
            })
            .filter_map(|credential| {
                let locator = credential.locator.trim();
                if locator.is_empty() || parse_env_locator(locator).is_none() {
                    return None;
                }
                Some(locator.to_string())
            })
            .collect();

        if partner_locators.is_empty() {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' is missing {} env locator for chain_id={}",
                partner.partner_id, lane.display_name, lane.chain_id
            )));
        }
        if partner_locators.len() > 1 {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' has ambiguous {} env locators for chain_id={}",
                partner.partner_id, lane.display_name, lane.chain_id
            )));
        }

        let locator = partner_locators.into_iter().next().ok_or_else(|| {
            Error::Internal(format!("Missing {} custody env locator", lane.display_name))
        })?;
        let env_key = parse_env_locator(&locator).ok_or_else(|| {
            Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' has invalid {} env locator '{}' for chain_id={}",
                partner.partner_id, lane.display_name, locator, lane.chain_id
            ))
        })?;

        let env_value = std::env::var(&env_key).map_err(|_| {
            Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' has unresolved {} env locator '{}' for chain_id={}",
                partner.partner_id, lane.display_name, env_key, lane.chain_id
            ))
        })?;

        let address = env_value.trim();
        if address.is_empty() {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' resolved {} env locator '{}' to an empty value for chain_id={}",
                partner.partner_id, lane.display_name, env_key, lane.chain_id
            )));
        }
        if !is_valid_evm_address(address) {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' resolved {} env locator '{}' to an invalid address: {}",
                partner.partner_id, lane.display_name, env_key, address
            )));
        }
        resolved_addresses.insert(address.to_string());
    }

    if resolved_addresses.len() > 1 {
        return Err(Error::Conflict(format!(
            "Approved healthy custody registry env locator resolution is ambiguous across eligible {} partners for chain_id={}",
            lane.display_name, lane.chain_id
        )));
    }

    Ok(resolved_addresses.into_iter().next())
}

fn configured_solana_address_from_registry(
    request: &OfframpDepositAddressRequest,
    snapshot: &PartnerRegistrySnapshot,
) -> Result<Option<String>> {
    let eligible_partners: Vec<&PartnerRegistryRecord> = snapshot
        .partners
        .iter()
        .filter(|partner| {
            partner.approval_status.eq_ignore_ascii_case("approved")
                && partner.lifecycle_state.eq_ignore_ascii_case("active")
        })
        .filter(|partner| {
            partner
                .capabilities
                .iter()
                .any(|capability| is_eligible_custody_capability(capability, &request.tenant_id))
        })
        .collect();
    if eligible_partners.is_empty() {
        return Ok(None);
    }

    let validator = SolanaChain::new(SolanaChainConfig::mainnet(""))
        .map_err(|error| Error::Validation(error.to_string()))?;
    let mut resolved_addresses: BTreeSet<String> = BTreeSet::new();

    for partner in eligible_partners {
        let partner_locators: BTreeSet<String> = partner
            .credential_references
            .iter()
            .filter(|credential| {
                credential
                    .credential_kind
                    .eq_ignore_ascii_case(SOLANA_CUSTODY_CREDENTIAL_KIND)
                    && credential.environment.eq_ignore_ascii_case("production")
            })
            .map(|credential| credential.locator.trim().to_string())
            .filter(|locator| !locator.is_empty())
            .collect();

        if partner_locators.is_empty() {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' is missing Solana env locator for chain_id=101",
                partner.partner_id
            )));
        }
        if partner_locators.len() > 1 {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' has ambiguous Solana env locators for chain_id=101",
                partner.partner_id
            )));
        }

        let locator = partner_locators
            .into_iter()
            .next()
            .ok_or_else(|| Error::Internal("Missing Solana custody env locator".to_string()))?;
        let env_key = parse_env_locator(&locator).ok_or_else(|| {
            Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' has invalid Solana env locator '{}' for chain_id=101",
                partner.partner_id, locator
            ))
        })?;

        let env_value = std::env::var(&env_key).map_err(|_| {
            Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' has unresolved Solana env locator '{}' for chain_id=101",
                partner.partner_id, env_key
            ))
        })?;

        let address = env_value.trim();
        if address.is_empty() {
            return Err(Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' resolved Solana env locator '{}' to an empty value for chain_id=101",
                partner.partner_id, env_key
            )));
        }

        validator.validate_address(address).map_err(|error| {
            Error::Conflict(format!(
                "Approved healthy custody registry partner '{}' resolved Solana env locator '{}' to an invalid address: {}",
                partner.partner_id, env_key, error
            ))
        })?;
        resolved_addresses.insert(address.to_string());
    }

    if resolved_addresses.len() > 1 {
        return Err(Error::Conflict(
            "Approved healthy custody registry env locator resolution is ambiguous across eligible Solana partners for chain_id=101".to_string(),
        ));
    }

    Ok(resolved_addresses.into_iter().next())
}

fn validate_bundle_address(chain_id: i64, address: &str) -> Result<()> {
    match chain_id {
        SOLANA_CHAIN_ID => {
            let validator = SolanaChain::new(SolanaChainConfig::mainnet(""))
                .map_err(|error| Error::Validation(error.to_string()))?;
            validator
                .validate_address(address)
                .map_err(|error| Error::Validation(error.to_string()))?;
            Ok(())
        }
        chain_id if is_supported_governed_evm_chain(chain_id) => {
            if !is_valid_evm_address(address) {
                return Err(Error::Validation(format!(
                    "Invalid {} address '{}'",
                    governed_evm_display_name(chain_id).unwrap_or("EVM"),
                    address,
                )));
            }
            Ok(())
        }
        other => Err(Error::Validation(format!(
            "Custody lane address validation is unsupported for chain_id={}",
            other
        ))),
    }
}

fn parse_env_locator(locator: &str) -> Option<String> {
    let trimmed = locator.trim();
    let key = trimmed
        .strip_prefix("env://")
        .or_else(|| trimmed.strip_prefix("env:"))?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    if !key
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    Some(key.to_string())
}

fn is_eligible_custody_capability(capability: &PartnerCapabilityRecord, tenant_id: &str) -> bool {
    capability.capability_family.eq_ignore_ascii_case("custody")
        && capability.approval_status.eq_ignore_ascii_case("approved")
        && capability.environment.eq_ignore_ascii_case("production")
        && capability
            .supported_rails
            .iter()
            .any(|rail| normalize_lane_value(rail) == "solana")
        && capability
            .supported_methods
            .iter()
            .any(|method| normalize_lane_value(method) == "depositaddress")
        && capability.health_signals.iter().any(|signal| {
            signal.status.eq_ignore_ascii_case("healthy")
                || signal.status.eq_ignore_ascii_case("ok")
        })
        && capability.rollout_scopes.iter().any(|scope| {
            scope.rollout_state.eq_ignore_ascii_case("approved")
                && scope.environment.eq_ignore_ascii_case("production")
                && scope
                    .method_family
                    .as_deref()
                    .map(|method| normalize_lane_value(method) == "depositaddress")
                    .unwrap_or(false)
                && scope
                    .corridor_code
                    .as_deref()
                    .map(is_solana_offramp_corridor)
                    .unwrap_or(false)
                && scope
                    .tenant_id
                    .as_deref()
                    .map(|scope_tenant| scope_tenant == tenant_id)
                    .unwrap_or(true)
        })
}

fn is_eligible_governed_evm_custody_capability(
    capability: &PartnerCapabilityRecord,
    tenant_id: &str,
    lane: &GovernedEvmLane,
) -> bool {
    capability.capability_family.eq_ignore_ascii_case("custody")
        && capability.approval_status.eq_ignore_ascii_case("approved")
        && capability.environment.eq_ignore_ascii_case("production")
        && capability
            .supported_rails
            .iter()
            .any(|rail| lane_matches_aliases(rail, lane.rail_aliases))
        && capability
            .supported_methods
            .iter()
            .any(|method| normalize_lane_value(method) == "depositaddress")
        && capability.health_signals.iter().any(|signal| {
            signal.status.eq_ignore_ascii_case("healthy")
                || signal.status.eq_ignore_ascii_case("ok")
        })
        && capability.rollout_scopes.iter().any(|scope| {
            scope.rollout_state.eq_ignore_ascii_case("approved")
                && scope.environment.eq_ignore_ascii_case("production")
                && scope
                    .method_family
                    .as_deref()
                    .map(|method| normalize_lane_value(method) == "depositaddress")
                    .unwrap_or(false)
                && scope
                    .corridor_code
                    .as_deref()
                    .map(|corridor_code| is_governed_evm_offramp_corridor(corridor_code, lane))
                    .unwrap_or(false)
                && scope
                    .tenant_id
                    .as_deref()
                    .map(|scope_tenant| scope_tenant == tenant_id)
                    .unwrap_or(true)
        })
}

fn normalize_lane_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn is_solana_offramp_corridor(corridor_code: &str) -> bool {
    let normalized = normalize_lane_value(corridor_code);
    normalized.contains("solana") && normalized.contains("offramp")
}

fn is_governed_evm_offramp_corridor(corridor_code: &str, lane: &GovernedEvmLane) -> bool {
    let normalized = normalize_lane_value(corridor_code);
    normalized.contains("offramp")
        && lane
            .corridor_aliases
            .iter()
            .any(|alias| normalized.contains(&normalize_lane_value(alias)))
}

fn is_eligible_governed_evm_custody_credential_kind(
    credential_kind: &str,
    lane: &GovernedEvmLane,
) -> bool {
    let normalized = normalize_lane_value(credential_kind);
    normalized == "offrampdepositaddress"
        || normalized == "offrampdepositaddressevm"
        || (normalized.contains("offramp")
            && normalized.contains("depositaddress")
            && (lane
                .credential_aliases
                .iter()
                .any(|alias| normalized.contains(&normalize_lane_value(alias)))
                || normalized.contains(&lane.chain_id.to_string())))
}

fn lane_matches_aliases(value: &str, aliases: &[&str]) -> bool {
    let normalized = normalize_lane_value(value);
    aliases
        .iter()
        .any(|alias| normalized.contains(&normalize_lane_value(alias)))
}

fn is_supported_governed_evm_chain(chain_id: i64) -> bool {
    governed_evm_lane(chain_id).is_some()
}

fn is_supported_governed_bundle_chain(chain_id: i64) -> bool {
    chain_id == SOLANA_CHAIN_ID || is_supported_governed_evm_chain(chain_id)
}

fn is_governed_deposit_address_chain(chain_id: Option<i64>) -> bool {
    chain_id
        .map(|chain_id| chain_id == SOLANA_CHAIN_ID || is_supported_governed_evm_chain(chain_id))
        .unwrap_or(false)
}

fn display_chain_id(chain_id: Option<i64>) -> String {
    chain_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn governed_evm_lane(chain_id: i64) -> Option<&'static GovernedEvmLane> {
    match chain_id {
        ETHEREUM_CHAIN_ID => Some(&ETHEREUM_GOVERNED_EVM_LANE),
        BNB_CHAIN_ID => Some(&BNB_GOVERNED_EVM_LANE),
        POLYGON_CHAIN_ID => Some(&POLYGON_GOVERNED_EVM_LANE),
        AVALANCHE_CHAIN_ID => Some(&AVALANCHE_GOVERNED_EVM_LANE),
        _ => None,
    }
}

fn governed_evm_display_name(chain_id: i64) -> Option<&'static str> {
    governed_evm_lane(chain_id).map(|lane| lane.display_name)
}

fn is_valid_evm_address(address: &str) -> bool {
    let Some(hex) = address.strip_prefix("0x") else {
        return false;
    };
    hex.len() == 40 && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

impl Default for OfframpDepositAddressAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        CredentialReferenceRecord, PartnerCapabilityRecord, PartnerHealthSignalRecord,
        PartnerRegistryRecord, PartnerRolloutScopeRecord,
    };
    use crate::service::PartnerRegistrySnapshot;
    use chrono::Utc;
    use serde_json::json;

    fn configured_bundle(
        approval_status: &str,
        source: &str,
        payload: serde_json::Value,
    ) -> ConfigBundleArtifact {
        ConfigBundleArtifact {
            bundle_id: "cfg_bundle_test".to_string(),
            tenant_name: "Tenant".to_string(),
            exported_at: "2026-03-18T00:00:00Z".to_string(),
            action_mode: "whitelisted_only".to_string(),
            sections: vec!["offramp".to_string()],
            payload,
            approval_status: Some(approval_status.to_string()),
            rollout_scope: Some(json!({"scope":"tenant"})),
            provenance: Some(json!({"mode":"registry"})),
            source: Some(source.to_string()),
        }
    }

    fn test_request(chain_id: Option<i64>) -> OfframpDepositAddressRequest {
        OfframpDepositAddressRequest {
            tenant_id: "tenant_allocator".to_string(),
            user_id: "user_allocator".to_string(),
            chain_id,
        }
    }

    fn clear_production_env() {
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
    }

    fn env_credential_reference(credential_id: &str, locator: &str) -> CredentialReferenceRecord {
        env_credential_reference_with_kind(credential_id, "offramp_deposit_address_solana", locator)
    }

    fn env_credential_reference_with_kind(
        credential_id: &str,
        credential_kind: &str,
        locator: &str,
    ) -> CredentialReferenceRecord {
        CredentialReferenceRecord {
            credential_id: credential_id.to_string(),
            credential_kind: credential_kind.to_string(),
            locator: locator.to_string(),
            environment: "production".to_string(),
            approval_reference: None,
            rotation_metadata: json!({}),
        }
    }

    fn custody_partner(
        partner_id: &str,
        credential_references: Vec<CredentialReferenceRecord>,
    ) -> PartnerRegistryRecord {
        custody_partner_with_rollout(
            partner_id,
            credential_references,
            vec!["solana".to_string()],
            vec!["deposit_address".to_string()],
            Some("OFFRAMP_SOLANA"),
            Some("deposit_address"),
        )
    }

    fn custody_avalanche_partner(
        partner_id: &str,
        credential_references: Vec<CredentialReferenceRecord>,
    ) -> PartnerRegistryRecord {
        custody_partner_with_rollout(
            partner_id,
            credential_references,
            vec!["avalanche".to_string()],
            vec!["deposit_address".to_string()],
            Some("OFFRAMP_AVALANCHE"),
            Some("deposit_address"),
        )
    }

    fn custody_partner_with_rollout(
        partner_id: &str,
        credential_references: Vec<CredentialReferenceRecord>,
        supported_rails: Vec<String>,
        supported_methods: Vec<String>,
        corridor_code: Option<&str>,
        method_family: Option<&str>,
    ) -> PartnerRegistryRecord {
        PartnerRegistryRecord {
            partner_id: partner_id.to_string(),
            tenant_id: Some("tenant_allocator".to_string()),
            partner_class: "custodian".to_string(),
            code: partner_id.to_string(),
            display_name: format!("Custodian {partner_id}"),
            legal_name: None,
            market: Some("VN".to_string()),
            jurisdiction: Some("VN".to_string()),
            service_domain: "custody".to_string(),
            lifecycle_state: "active".to_string(),
            approval_status: "approved".to_string(),
            metadata: json!({}),
            capabilities: vec![PartnerCapabilityRecord {
                capability_id: format!("capability_{partner_id}"),
                capability_family: "custody".to_string(),
                environment: "production".to_string(),
                adapter_key: None,
                provider_key: None,
                supported_rails,
                supported_methods,
                approval_status: "approved".to_string(),
                metadata: json!({}),
                rollout_scopes: vec![PartnerRolloutScopeRecord {
                    scope_id: format!("scope_{partner_id}"),
                    tenant_id: Some("tenant_allocator".to_string()),
                    environment: "production".to_string(),
                    corridor_code: corridor_code.map(|value| value.to_string()),
                    geography: Some("VN".to_string()),
                    method_family: method_family.map(|value| value.to_string()),
                    rollout_state: "approved".to_string(),
                    rollback_target: None,
                    approval_reference: None,
                }],
                health_signals: vec![PartnerHealthSignalRecord {
                    health_signal_id: format!("signal_{partner_id}"),
                    status: "healthy".to_string(),
                    source: "allocator_test".to_string(),
                    score: Some(99),
                    incident_summary: None,
                    evidence: json!({}),
                    observed_at: Utc::now(),
                }],
            }],
            credential_references,
        }
    }

    fn registry_snapshot(partners: Vec<PartnerRegistryRecord>) -> PartnerRegistrySnapshot {
        PartnerRegistrySnapshot {
            action_mode: "registry_backed".to_string(),
            source: "registry".to_string(),
            partners,
        }
    }

    #[test]
    fn issues_evm_placeholder_for_supported_evm_chains_outside_production() {
        let _guard = onchain_gate::test_env_lock();
        clear_production_env();
        let allocator = OfframpDepositAddressAllocator::new();

        for chain_id in [
            None,
            Some(1),
            Some(137),
            Some(56),
            Some(42161),
            Some(10),
            Some(8453),
            Some(43114),
        ] {
            let address = allocator.issue(&test_request(chain_id)).unwrap();
            assert!(address.starts_with("0x"));
            assert_eq!(address.len(), 42);
        }
    }

    #[test]
    fn production_fails_closed_for_governed_evm_placeholder_without_config() {
        let _guard = onchain_gate::test_env_lock();
        std::env::set_var("RUST_ENV", "production");
        std::env::remove_var("RAMPOS_ENV");
        let allocator = OfframpDepositAddressAllocator::new();

        for chain_id in [Some(1), Some(56), Some(137), Some(43114)] {
            let error = allocator.issue(&test_request(chain_id)).unwrap_err();
            match error {
                Error::Conflict(message) => {
                    assert!(message
                        .contains("requires an approved registry or bundle custody configuration"));
                    assert!(message.contains("refusing placeholder address issuance"));
                }
                other => panic!("unexpected error for chain_id={chain_id:?}: {other:?}"),
            }
        }

        clear_production_env();
    }

    #[test]
    fn production_detection_for_placeholder_uses_rampos_env_when_rust_env_unset_or_empty() {
        let _guard = onchain_gate::test_env_lock();
        let allocator = OfframpDepositAddressAllocator::new();

        std::env::remove_var("RUST_ENV");
        std::env::set_var("RAMPOS_ENV", "production");
        let missing_rust_env_error = allocator.issue(&test_request(Some(1))).unwrap_err();
        assert!(matches!(missing_rust_env_error, Error::Conflict(_)));

        std::env::set_var("RUST_ENV", "   ");
        std::env::set_var("RAMPOS_ENV", "PrOdUcTiOn");
        let empty_rust_env_error = allocator.issue(&test_request(Some(56))).unwrap_err();
        assert!(matches!(empty_rust_env_error, Error::Conflict(_)));

        clear_production_env();
    }

    #[test]
    fn fails_closed_for_solana() {
        let allocator = OfframpDepositAddressAllocator::new();
        let error = allocator.issue(&test_request(Some(101))).unwrap_err();
        match error {
            Error::Conflict(message) => {
                assert!(message.contains("Solana deposit address issuance is unavailable"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_chain_ids() {
        let allocator = OfframpDepositAddressAllocator::new();
        let error = allocator.issue(&test_request(Some(999999))).unwrap_err();
        match error {
            Error::Validation(message) => {
                assert!(message.contains("unsupported for chain_id=999999"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn extracts_configured_solana_address_from_approved_registry_bundle() {
        let bundle = configured_bundle(
            "approved",
            "registry",
            json!({
                "offramp": {
                    "depositAddressesByChain": {
                        "101": "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
                    }
                }
            }),
        );

        let address = OfframpDepositAddressAllocator::configured_address_from_bundle(
            &test_request(Some(101)),
            &bundle,
        )
        .unwrap();

        assert_eq!(
            address.as_deref(),
            Some("7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy")
        );
    }

    #[test]
    fn extracts_configured_avalanche_address_from_approved_registry_bundle() {
        let bundle = configured_bundle(
            "approved",
            "registry",
            json!({
                "offramp": {
                    "depositAddressesByChain": {
                        "43114": "0x1234567890123456789012345678901234567890"
                    }
                }
            }),
        );

        let address = OfframpDepositAddressAllocator::configured_address_from_bundle(
            &test_request(Some(43114)),
            &bundle,
        )
        .unwrap();

        assert_eq!(
            address.as_deref(),
            Some("0x1234567890123456789012345678901234567890")
        );
    }

    #[test]
    fn extracts_configured_supported_evm_addresses_from_approved_registry_bundle() {
        for chain_id in [1, 56, 137] {
            let bundle = configured_bundle(
                "approved",
                "registry",
                json!({
                    "offramp": {
                        "depositAddressesByChain": {
                            chain_id.to_string(): "0x1234567890123456789012345678901234567890"
                        }
                    }
                }),
            );

            let address = OfframpDepositAddressAllocator::configured_address_from_bundle(
                &test_request(Some(chain_id)),
                &bundle,
            )
            .unwrap();

            assert_eq!(
                address.as_deref(),
                Some("0x1234567890123456789012345678901234567890"),
                "expected configured bundle address for chain_id={chain_id}"
            );
        }
    }

    #[test]
    fn ignores_non_registry_or_non_approved_bundle_for_configured_solana_address() {
        let bundle = configured_bundle(
            "fallback",
            "fallback",
            json!({
                "offramp": {
                    "depositAddressesByChain": {
                        "101": "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy"
                    }
                }
            }),
        );

        let address = OfframpDepositAddressAllocator::configured_address_from_bundle(
            &test_request(Some(101)),
            &bundle,
        )
        .unwrap();

        assert!(address.is_none());
    }

    #[test]
    fn rejects_invalid_configured_solana_address() {
        let bundle = configured_bundle(
            "approved",
            "registry",
            json!({
                "offramp": {
                    "depositAddressesByChain": {
                        "101": "not-a-valid-solana-address"
                    }
                }
            }),
        );

        let error = OfframpDepositAddressAllocator::configured_address_from_bundle(
            &test_request(Some(101)),
            &bundle,
        )
        .unwrap_err();

        match error {
            Error::Validation(message) => {
                assert!(message.contains("Solana"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn extracts_registry_backed_solana_address_from_env_locator() {
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR",
            "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy",
        );
        let snapshot = registry_snapshot(vec![custody_partner(
            "partner_custody_env",
            vec![env_credential_reference(
                "cred_solana_addr",
                "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR",
            )],
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap();

        assert_eq!(
            address.as_deref(),
            Some("7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy")
        );

        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR");
    }

    #[test]
    fn extracts_registry_backed_avalanche_address_from_env_locator() {
        std::env::set_var(
            "RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR",
            "0x1234567890123456789012345678901234567890",
        );
        let snapshot = registry_snapshot(vec![custody_avalanche_partner(
            "partner_avalanche_custody_env",
            vec![env_credential_reference_with_kind(
                "cred_avalanche_addr",
                "offramp_deposit_address",
                "env://RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR",
            )],
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(43114)),
            &snapshot,
        )
        .unwrap();

        assert_eq!(
            address.as_deref(),
            Some("0x1234567890123456789012345678901234567890")
        );

        std::env::remove_var("RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR");
    }

    #[test]
    fn extracts_registry_backed_ethereum_address_from_env_locator() {
        std::env::set_var(
            "RAMPOS_TEST_ETHEREUM_CUSTODY_ADDR",
            "0x1234567890123456789012345678901234567890",
        );
        let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
            "partner_ethereum_custody_env",
            vec![env_credential_reference_with_kind(
                "cred_ethereum_addr",
                "offramp_deposit_address_ethereum",
                "env://RAMPOS_TEST_ETHEREUM_CUSTODY_ADDR",
            )],
            vec!["ethereum".to_string()],
            vec!["deposit_address".to_string()],
            Some("OFFRAMP_ETHEREUM"),
            Some("deposit_address"),
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(1)),
            &snapshot,
        )
        .unwrap();

        assert_eq!(
            address.as_deref(),
            Some("0x1234567890123456789012345678901234567890")
        );

        std::env::remove_var("RAMPOS_TEST_ETHEREUM_CUSTODY_ADDR");
    }

    #[test]
    fn extracts_registry_backed_supported_additional_evm_addresses_from_env_locator() {
        for (chain_id, rail, corridor_code, env_key, credential_kind) in [
            (
                56,
                "bnb",
                "OFFRAMP_BNB",
                "RAMPOS_TEST_BNB_CUSTODY_ADDR",
                "offramp_deposit_address_bnb",
            ),
            (
                137,
                "polygon",
                "OFFRAMP_POLYGON",
                "RAMPOS_TEST_POLYGON_CUSTODY_ADDR",
                "offramp_deposit_address_polygon",
            ),
        ] {
            std::env::set_var(env_key, "0x1234567890123456789012345678901234567890");
            let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
                &format!("partner_{rail}_custody_env"),
                vec![env_credential_reference_with_kind(
                    &format!("cred_{rail}_addr"),
                    credential_kind,
                    &format!("env://{env_key}"),
                )],
                vec![rail.to_string()],
                vec!["deposit_address".to_string()],
                Some(corridor_code),
                Some("deposit_address"),
            )]);

            let address = OfframpDepositAddressAllocator::configured_address_from_registry(
                &test_request(Some(chain_id)),
                &snapshot,
            )
            .unwrap();

            assert_eq!(
                address.as_deref(),
                Some("0x1234567890123456789012345678901234567890"),
                "expected registry-backed address for chain_id={chain_id}"
            );

            std::env::remove_var(env_key);
        }
    }

    #[test]
    fn fails_closed_when_registry_ethereum_match_has_no_env_locator() {
        let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
            "partner_ethereum_missing_locator",
            vec![env_credential_reference_with_kind(
                "cred_ethereum_no_env",
                "offramp_deposit_address_ethereum",
                "vault://partners/custody/ethereum",
            )],
            vec!["ethereum".to_string()],
            vec!["deposit_address".to_string()],
            Some("OFFRAMP_ETHEREUM"),
            Some("deposit_address"),
        )]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(1)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("missing Ethereum env locator"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn fails_closed_when_registry_supported_additional_evm_match_has_no_env_locator() {
        for (chain_id, rail, corridor_code, credential_kind, expected_message) in [
            (
                56,
                "bnb",
                "OFFRAMP_BNB",
                "offramp_deposit_address_bnb",
                "missing BNB env locator",
            ),
            (
                137,
                "polygon",
                "OFFRAMP_POLYGON",
                "offramp_deposit_address_polygon",
                "missing Polygon env locator",
            ),
        ] {
            let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
                &format!("partner_{rail}_missing_locator"),
                vec![env_credential_reference_with_kind(
                    &format!("cred_{rail}_no_env"),
                    credential_kind,
                    &format!("vault://partners/custody/{rail}"),
                )],
                vec![rail.to_string()],
                vec!["deposit_address".to_string()],
                Some(corridor_code),
                Some("deposit_address"),
            )]);

            let error = OfframpDepositAddressAllocator::configured_address_from_registry(
                &test_request(Some(chain_id)),
                &snapshot,
            )
            .unwrap_err();

            match error {
                Error::Conflict(message) => {
                    assert!(
                        message.contains(expected_message),
                        "unexpected conflict message for chain_id={chain_id}: {message}"
                    );
                }
                other => panic!("unexpected error for chain_id={chain_id}: {other:?}"),
            }
        }
    }

    #[test]
    fn ignores_unrelated_avalanche_env_credentials_when_resolving_registry_locator() {
        std::env::set_var(
            "RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_OK",
            "0x1234567890123456789012345678901234567890",
        );
        std::env::set_var("RAMPOS_TEST_AVALANCHE_API_KEY", "secret-token");
        let snapshot = registry_snapshot(vec![custody_avalanche_partner(
            "partner_avalanche_with_extra_credentials",
            vec![
                env_credential_reference_with_kind(
                    "cred_avalanche_addr_ok",
                    "offramp_deposit_address",
                    "env://RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_OK",
                ),
                env_credential_reference_with_kind(
                    "cred_avalanche_api_key",
                    "api_key",
                    "env://RAMPOS_TEST_AVALANCHE_API_KEY",
                ),
            ],
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(43114)),
            &snapshot,
        )
        .unwrap();

        assert_eq!(
            address.as_deref(),
            Some("0x1234567890123456789012345678901234567890")
        );

        std::env::remove_var("RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_OK");
        std::env::remove_var("RAMPOS_TEST_AVALANCHE_API_KEY");
    }

    #[test]
    fn fails_closed_when_registry_avalanche_match_has_no_env_locator() {
        let snapshot = registry_snapshot(vec![custody_avalanche_partner(
            "partner_avalanche_missing_locator",
            vec![env_credential_reference_with_kind(
                "cred_avalanche_no_env",
                "offramp_deposit_address",
                "vault://partners/custody/avalanche",
            )],
        )]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(43114)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("missing Avalanche env locator"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn fails_closed_when_registry_avalanche_env_value_is_invalid_evm_address() {
        std::env::set_var(
            "RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_INVALID",
            "not-an-address",
        );
        let snapshot = registry_snapshot(vec![custody_avalanche_partner(
            "partner_avalanche_invalid_address",
            vec![env_credential_reference_with_kind(
                "cred_avalanche_invalid",
                "offramp_deposit_address",
                "env://RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_INVALID",
            )],
        )]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(43114)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("invalid address"));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        std::env::remove_var("RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_INVALID");
    }

    #[test]
    fn ignores_avalanche_custody_partner_without_avalanche_offramp_corridor_scope() {
        std::env::set_var(
            "RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_WRONG_SCOPE",
            "0x1234567890123456789012345678901234567890",
        );
        let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
            "partner_avalanche_wrong_scope",
            vec![env_credential_reference_with_kind(
                "cred_avalanche_addr_wrong_scope",
                "offramp_deposit_address",
                "env://RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_WRONG_SCOPE",
            )],
            vec!["avalanche".to_string()],
            vec!["deposit_address".to_string()],
            Some("OFFRAMP_SOLANA"),
            Some("deposit_address"),
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(43114)),
            &snapshot,
        )
        .unwrap();

        assert!(address.is_none());
        std::env::remove_var("RAMPOS_TEST_AVALANCHE_CUSTODY_ADDR_WRONG_SCOPE");
    }

    #[test]
    fn fails_closed_when_registry_custody_match_has_missing_locator() {
        let snapshot = registry_snapshot(vec![custody_partner("partner_custody_missing", vec![])]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("locator"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn fails_closed_when_registry_custody_locators_are_ambiguous() {
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR_A",
            "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy",
        );
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR_B",
            "11111111111111111111111111111111",
        );
        let snapshot = registry_snapshot(vec![custody_partner(
            "partner_custody_ambiguous",
            vec![
                env_credential_reference(
                    "cred_solana_addr_a",
                    "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_A",
                ),
                env_credential_reference(
                    "cred_solana_addr_b",
                    "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_B",
                ),
            ],
        )]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("ambiguous"));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_A");
        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_B");
    }

    #[test]
    fn fails_closed_when_one_eligible_partner_has_valid_locator_and_another_is_missing_locator() {
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR_OK",
            "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy",
        );
        let snapshot = registry_snapshot(vec![
            custody_partner(
                "partner_custody_ok",
                vec![env_credential_reference(
                    "cred_solana_addr_ok",
                    "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_OK",
                )],
            ),
            custody_partner("partner_custody_missing", vec![]),
        ]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("missing Solana env locator"));
                assert!(message.contains("partner_custody_missing"));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_OK");
    }

    #[test]
    fn ignores_custody_partner_without_solana_lane_metadata_even_with_solana_credential() {
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR_WRONG_LANE",
            "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy",
        );
        let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
            "partner_custody_wrong_lane",
            vec![env_credential_reference(
                "cred_solana_addr_wrong_lane",
                "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_WRONG_LANE",
            )],
            vec!["evm".to_string()],
            vec!["withdraw".to_string()],
            Some("OFFRAMP_SOLANA"),
            Some("deposit_address"),
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap();

        assert!(address.is_none());
        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_WRONG_LANE");
    }

    #[test]
    fn ignores_custody_partner_without_solana_rollout_scope_lane_even_with_solana_capability() {
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR_WRONG_SCOPE",
            "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy",
        );
        let snapshot = registry_snapshot(vec![custody_partner_with_rollout(
            "partner_custody_wrong_scope",
            vec![env_credential_reference(
                "cred_solana_addr_wrong_scope",
                "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_WRONG_SCOPE",
            )],
            vec!["solana".to_string()],
            vec!["deposit_address".to_string()],
            Some("USDT_VN_OFFRAMP"),
            Some("push_transfer"),
        )]);

        let address = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap();

        assert!(address.is_none());
        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_WRONG_SCOPE");
    }

    #[test]
    fn fails_closed_when_registry_partner_has_unresolved_env_locator() {
        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_UNRESOLVED");
        let snapshot = registry_snapshot(vec![custody_partner(
            "partner_custody_unresolved",
            vec![env_credential_reference(
                "cred_solana_addr_unresolved",
                "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_UNRESOLVED",
            )],
        )]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("unresolved"));
                assert!(message.contains("partner_custody_unresolved"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn fails_closed_when_registry_partner_env_value_is_invalid_solana_address() {
        std::env::set_var(
            "RAMPOS_TEST_SOLANA_CUSTODY_ADDR_INVALID",
            "not-a-valid-solana-address",
        );
        let snapshot = registry_snapshot(vec![custody_partner(
            "partner_custody_invalid",
            vec![env_credential_reference(
                "cred_solana_addr_invalid",
                "env://RAMPOS_TEST_SOLANA_CUSTODY_ADDR_INVALID",
            )],
        )]);

        let error = OfframpDepositAddressAllocator::configured_address_from_registry(
            &test_request(Some(101)),
            &snapshot,
        )
        .unwrap_err();

        match error {
            Error::Conflict(message) => {
                assert!(message.contains("invalid address"));
                assert!(message.contains("partner_custody_invalid"));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        std::env::remove_var("RAMPOS_TEST_SOLANA_CUSTODY_ADDR_INVALID");
    }
}
