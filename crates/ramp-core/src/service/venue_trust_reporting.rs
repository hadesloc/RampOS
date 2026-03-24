use std::collections::BTreeMap;
use std::sync::Arc;

use ramp_common::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::repository::{
    BeneficiaryProfileFilter, SourceOfFundsPackageFilter, VenueAccountFilter, VenueConnectionFilter,
    VenueTransferFilter, VenueTransferRecord, VenueTrustRepository, WalletAttestationFilter,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTrustDirectionSummary {
    pub transfer_direction: String,
    pub transfer_count: usize,
    pub total_amount: Decimal,
    pub status_counts: BTreeMap<String, usize>,
    pub asset_symbols: Vec<String>,
    pub networks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTrustEvidenceReference {
    pub kind: String,
    pub reference_id: String,
    pub status: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTrustReport {
    pub source: String,
    pub subject_type: String,
    pub subject_id: String,
    pub connection_ids: Vec<String>,
    pub account_ids: Vec<String>,
    pub wallet_attestation_ids: Vec<String>,
    pub beneficiary_profile_ids: Vec<String>,
    pub source_of_funds_package_ids: Vec<String>,
    pub cash_in: VenueTrustDirectionSummary,
    pub cash_out: VenueTrustDirectionSummary,
    pub evidence_references: Vec<VenueTrustEvidenceReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTrustEvidenceExportArtifact {
    pub file_name: String,
    pub media_type: String,
    pub contents: Vec<u8>,
}

pub struct VenueTrustReportingService {
    repository: Arc<dyn VenueTrustRepository>,
}

impl VenueTrustReportingService {
    pub fn new(repository: Arc<dyn VenueTrustRepository>) -> Self {
        Self { repository }
    }

    pub async fn build_subject_trust_report(
        &self,
        tenant_id: &str,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<VenueTrustReport> {
        let connections = self
            .repository
            .list_connections(&VenueConnectionFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: Some(subject_type.to_string()),
                subject_id: Some(subject_id.to_string()),
                venue_key: None,
                status: None,
            })
            .await?;

        let mut account_ids = Vec::new();
        for connection in &connections {
            let accounts = self
                .repository
                .list_accounts(&VenueAccountFilter {
                    tenant_id: tenant_id.to_string(),
                    venue_connection_id: Some(connection.connection_id.clone()),
                    venue_key: None,
                    status: None,
                    wallet_address: None,
                })
                .await?;
            for account in accounts {
                account_ids.push(account.account_id);
            }
        }
        account_ids.sort();
        account_ids.dedup();

        let beneficiaries = self
            .repository
            .list_beneficiary_profiles(&BeneficiaryProfileFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: Some(subject_type.to_string()),
                subject_id: Some(subject_id.to_string()),
                destination_type: None,
                verification_status: None,
            })
            .await?;

        let attestations = self
            .repository
            .list_wallet_attestations(&WalletAttestationFilter {
                tenant_id: tenant_id.to_string(),
                user_id: Some(subject_id.to_string()),
                wallet_address: None,
                chain_id: None,
                attestation_status: None,
            })
            .await?;

        let transfers = self
            .repository
            .list_transfers(&VenueTransferFilter {
                tenant_id: tenant_id.to_string(),
                user_id: Some(subject_id.to_string()),
                venue_connection_id: None,
                venue_account_id: None,
                status: None,
            })
            .await?;

        let source_of_funds_packages = self
            .repository
            .list_source_of_funds_packages(&SourceOfFundsPackageFilter {
                tenant_id: tenant_id.to_string(),
                subject_type: Some(subject_type.to_string()),
                subject_id: Some(subject_id.to_string()),
                venue_transfer_id: None,
                venue_account_id: None,
                review_status: None,
            })
            .await?;

        let evidence_references = build_evidence_references(
            &connections,
            &account_ids,
            &beneficiaries,
            &attestations,
            &source_of_funds_packages,
            &transfers,
        );

        Ok(VenueTrustReport {
            source: "venue_trust_repository".to_string(),
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            connection_ids: connections.iter().map(|record| record.connection_id.clone()).collect(),
            account_ids,
            wallet_attestation_ids: attestations
                .iter()
                .map(|record| record.attestation_id.to_string())
                .collect(),
            beneficiary_profile_ids: beneficiaries
                .iter()
                .map(|record| record.beneficiary_profile_id.clone())
                .collect(),
            source_of_funds_package_ids: source_of_funds_packages
                .iter()
                .map(|record| record.package_id.clone())
                .collect(),
            cash_in: summarize_direction(&transfers, "wallet_to_venue"),
            cash_out: summarize_direction(&transfers, "venue_to_wallet"),
            evidence_references,
        })
    }

    pub fn export_evidence_json(
        &self,
        report: &VenueTrustReport,
    ) -> Result<VenueTrustEvidenceExportArtifact> {
        let contents = serde_json::to_vec_pretty(&json!({
            "subjectType": report.subject_type,
            "subjectId": report.subject_id,
            "cashIn": report.cash_in,
            "cashOut": report.cash_out,
            "evidenceReferences": report.evidence_references,
        }))
        .map_err(|error| ramp_common::Error::Internal(format!(
            "failed to serialize venue trust evidence export: {}",
            error
        )))?;

        Ok(VenueTrustEvidenceExportArtifact {
            file_name: format!(
                "venue_trust_evidence_{}_{}.json",
                report.subject_type, report.subject_id
            ),
            media_type: "application/json".to_string(),
            contents,
        })
    }
}

fn summarize_direction(
    transfers: &[VenueTransferRecord],
    direction: &str,
) -> VenueTrustDirectionSummary {
    let mut status_counts = BTreeMap::new();
    let mut total_amount = Decimal::ZERO;
    let mut asset_symbols = Vec::new();
    let mut networks = Vec::new();
    let filtered: Vec<&VenueTransferRecord> = transfers
        .iter()
        .filter(|record| record.transfer_direction == direction)
        .collect();

    for transfer in &filtered {
        *status_counts.entry(transfer.status.clone()).or_insert(0) += 1;
        total_amount += transfer.amount;
        asset_symbols.push(transfer.asset_symbol.clone());
        networks.push(transfer.network.clone());
    }

    asset_symbols.sort();
    asset_symbols.dedup();
    networks.sort();
    networks.dedup();

    VenueTrustDirectionSummary {
        transfer_direction: direction.to_string(),
        transfer_count: filtered.len(),
        total_amount,
        status_counts,
        asset_symbols,
        networks,
    }
}

fn build_evidence_references(
    connections: &[crate::repository::VenueConnectionRecord],
    account_ids: &[String],
    beneficiaries: &[crate::repository::BeneficiaryProfileRecord],
    attestations: &[crate::repository::WalletAttestationRecord],
    source_of_funds_packages: &[crate::repository::SourceOfFundsPackageRecord],
    transfers: &[VenueTransferRecord],
) -> Vec<VenueTrustEvidenceReference> {
    let mut evidence = Vec::new();

    for connection in connections {
        evidence.push(VenueTrustEvidenceReference {
            kind: "connection".to_string(),
            reference_id: connection.connection_id.clone(),
            status: Some(connection.status.clone()),
            source: "venue_connections".to_string(),
        });
    }

    for account_id in account_ids {
        evidence.push(VenueTrustEvidenceReference {
            kind: "account".to_string(),
            reference_id: account_id.clone(),
            status: None,
            source: "venue_accounts".to_string(),
        });
    }

    for attestation in attestations {
        evidence.push(VenueTrustEvidenceReference {
            kind: "wallet_attestation".to_string(),
            reference_id: attestation.attestation_id.to_string(),
            status: Some(attestation.attestation_status.clone()),
            source: "wallet_attestations".to_string(),
        });
    }

    for beneficiary in beneficiaries {
        evidence.push(VenueTrustEvidenceReference {
            kind: "beneficiary_profile".to_string(),
            reference_id: beneficiary.beneficiary_profile_id.clone(),
            status: Some(beneficiary.verification_status.clone()),
            source: "beneficiary_profiles".to_string(),
        });
    }

    for package in source_of_funds_packages {
        evidence.push(VenueTrustEvidenceReference {
            kind: "source_of_funds_package".to_string(),
            reference_id: package.package_id.clone(),
            status: Some(package.review_status.clone()),
            source: "source_of_funds_packages".to_string(),
        });
    }

    for transfer in transfers {
        evidence.push(VenueTrustEvidenceReference {
            kind: "transfer".to_string(),
            reference_id: transfer.transfer_id.clone(),
            status: Some(transfer.status.clone()),
            source: "venue_transfers".to_string(),
        });
    }

    evidence
}
