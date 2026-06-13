use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueConnectionFilter {
    pub tenant_id: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub venue_key: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueAccountFilter {
    pub tenant_id: String,
    pub venue_connection_id: Option<String>,
    pub venue_key: Option<String>,
    pub status: Option<String>,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeneficiaryProfileFilter {
    pub tenant_id: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub destination_type: Option<String>,
    pub verification_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletAttestationFilter {
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub wallet_address: Option<String>,
    pub chain_id: Option<String>,
    pub attestation_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTransferFilter {
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub venue_connection_id: Option<String>,
    pub venue_account_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceOfFundsPackageFilter {
    pub tenant_id: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub venue_transfer_id: Option<String>,
    pub venue_account_id: Option<String>,
    pub review_status: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueConnectionRecord {
    pub connection_id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub user_id: Option<String>,
    pub venue_key: String,
    pub connection_mode: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueAccountRecord {
    pub account_id: String,
    pub tenant_id: String,
    pub venue_connection_id: String,
    pub venue_key: String,
    pub account_label: Option<String>,
    pub account_ref: Option<String>,
    pub wallet_address: Option<String>,
    pub subaccount_ref: Option<String>,
    pub api_scope_summary: serde_json::Value,
    pub status: String,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeneficiaryProfileRecord {
    pub beneficiary_profile_id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub user_id: Option<String>,
    pub destination_type: String,
    pub destination_ref: String,
    pub display_name: Option<String>,
    pub asset_symbol: Option<String>,
    pub network: Option<String>,
    pub verification_status: String,
    pub cooldown_ends_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletAttestationRecord {
    pub attestation_id: Uuid,
    pub tenant_id: String,
    pub user_id: String,
    pub wallet_address: String,
    pub chain_id: String,
    pub attestation_status: String,
    pub proof_kind: String,
    pub proof_artifact_uri: Option<String>,
    pub risk_state: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueTransferRecord {
    pub transfer_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub beneficiary_profile_id: Option<String>,
    pub wallet_attestation_id: Uuid,
    pub venue_connection_id: String,
    pub venue_account_id: String,
    pub transfer_direction: String,
    pub asset_symbol: String,
    pub network: String,
    pub amount: Decimal,
    pub origin_intent_id: Option<String>,
    pub rfq_id: Option<String>,
    pub status: String,
    pub wallet_tx_hash: Option<String>,
    pub venue_credit_ref: Option<String>,
    pub failure_code: Option<String>,
    pub metadata: serde_json::Value,
    pub submitted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceOfFundsPackageRecord {
    pub package_id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub wallet_attestation_id: Option<Uuid>,
    pub venue_account_id: Option<String>,
    pub venue_transfer_id: Option<String>,
    pub review_status: String,
    pub package_uri: Option<String>,
    pub metadata: serde_json::Value,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnsureWalletAttestationRequest {
    pub attestation_id: Uuid,
    pub tenant_id: String,
    pub user_id: String,
    pub wallet_address: String,
    pub chain_id: String,
    pub attestation_status: String,
    pub proof_kind: String,
    pub proof_artifact_uri: Option<String>,
    pub risk_state: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertVenueConnectionRequest {
    pub connection_id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub user_id: Option<String>,
    pub venue_key: String,
    pub connection_mode: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertVenueAccountRequest {
    pub account_id: String,
    pub tenant_id: String,
    pub venue_connection_id: String,
    pub venue_key: String,
    pub account_label: Option<String>,
    pub account_ref: Option<String>,
    pub wallet_address: Option<String>,
    pub subaccount_ref: Option<String>,
    pub api_scope_summary: serde_json::Value,
    pub status: String,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertBeneficiaryProfileRequest {
    pub beneficiary_profile_id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub user_id: Option<String>,
    pub destination_type: String,
    pub destination_ref: String,
    pub display_name: Option<String>,
    pub asset_symbol: Option<String>,
    pub network: Option<String>,
    pub verification_status: String,
    pub cooldown_ends_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub last_verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertVenueTransferRequest {
    pub transfer_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub beneficiary_profile_id: Option<String>,
    pub wallet_attestation_id: Uuid,
    pub venue_connection_id: String,
    pub venue_account_id: String,
    pub transfer_direction: String,
    pub asset_symbol: String,
    pub network: String,
    pub amount: Decimal,
    pub origin_intent_id: Option<String>,
    pub rfq_id: Option<String>,
    pub status: String,
    pub wallet_tx_hash: Option<String>,
    pub venue_credit_ref: Option<String>,
    pub failure_code: Option<String>,
    pub metadata: serde_json::Value,
    pub submitted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertSourceOfFundsPackageRequest {
    pub package_id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub wallet_attestation_id: Option<Uuid>,
    pub venue_account_id: Option<String>,
    pub venue_transfer_id: Option<String>,
    pub review_status: String,
    pub package_uri: Option<String>,
    pub metadata: serde_json::Value,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait VenueTrustRepository: Send + Sync {
    async fn ensure_wallet_attestation(
        &self,
        request: &EnsureWalletAttestationRequest,
    ) -> Result<()>;
    async fn get_wallet_attestation(
        &self,
        tenant_id: &str,
        attestation_id: Uuid,
    ) -> Result<Option<WalletAttestationRecord>>;
    async fn list_wallet_attestations(
        &self,
        filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>>;

    async fn upsert_connection(&self, request: &UpsertVenueConnectionRequest) -> Result<()>;
    async fn get_connection(
        &self,
        tenant_id: &str,
        connection_id: &str,
    ) -> Result<Option<VenueConnectionRecord>>;
    async fn list_connections(
        &self,
        filter: &VenueConnectionFilter,
    ) -> Result<Vec<VenueConnectionRecord>>;

    async fn upsert_account(&self, request: &UpsertVenueAccountRequest) -> Result<()>;
    async fn get_account(
        &self,
        tenant_id: &str,
        account_id: &str,
    ) -> Result<Option<VenueAccountRecord>>;
    async fn list_accounts(&self, filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>>;

    async fn upsert_beneficiary_profile(
        &self,
        request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()>;
    async fn get_beneficiary_profile(
        &self,
        tenant_id: &str,
        beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>>;
    async fn list_beneficiary_profiles(
        &self,
        filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>>;

    async fn upsert_transfer(&self, request: &UpsertVenueTransferRequest) -> Result<()>;
    async fn get_transfer(
        &self,
        tenant_id: &str,
        transfer_id: &str,
    ) -> Result<Option<VenueTransferRecord>>;
    async fn list_transfers(
        &self,
        filter: &VenueTransferFilter,
    ) -> Result<Vec<VenueTransferRecord>>;

    async fn upsert_source_of_funds_package(
        &self,
        request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()>;
    async fn get_source_of_funds_package(
        &self,
        tenant_id: &str,
        package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>>;
    async fn list_source_of_funds_packages(
        &self,
        filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>>;
}

pub struct PgVenueTrustRepository {
    pool: PgPool,
}

impl PgVenueTrustRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VenueTrustRepository for PgVenueTrustRepository {
    async fn ensure_wallet_attestation(
        &self,
        request: &EnsureWalletAttestationRequest,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO wallet_attestations (
                id, tenant_id, user_id, wallet_address, chain_id, attestation_status,
                proof_kind, proof_artifact_uri, risk_state, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                user_id = EXCLUDED.user_id,
                wallet_address = EXCLUDED.wallet_address,
                chain_id = EXCLUDED.chain_id,
                attestation_status = EXCLUDED.attestation_status,
                proof_kind = EXCLUDED.proof_kind,
                proof_artifact_uri = EXCLUDED.proof_artifact_uri,
                risk_state = EXCLUDED.risk_state,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(request.attestation_id)
        .bind(&request.tenant_id)
        .bind(&request.user_id)
        .bind(&request.wallet_address)
        .bind(&request.chain_id)
        .bind(&request.attestation_status)
        .bind(&request.proof_kind)
        .bind(&request.proof_artifact_uri)
        .bind(&request.risk_state)
        .bind(&request.metadata)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_wallet_attestation(
        &self,
        tenant_id: &str,
        attestation_id: Uuid,
    ) -> Result<Option<WalletAttestationRecord>> {
        sqlx::query_as::<_, WalletAttestationRecord>(
            r#"
            SELECT id AS attestation_id, tenant_id, user_id, wallet_address, chain_id,
                   attestation_status, proof_kind, proof_artifact_uri, risk_state, metadata,
                   created_at, updated_at
            FROM wallet_attestations
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(attestation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn list_wallet_attestations(
        &self,
        filter: &WalletAttestationFilter,
    ) -> Result<Vec<WalletAttestationRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id AS attestation_id, tenant_id, user_id, wallet_address, chain_id, attestation_status, proof_kind, proof_artifact_uri, risk_state, metadata, created_at, updated_at FROM wallet_attestations WHERE tenant_id = ",
        );
        builder.push_bind(&filter.tenant_id);
        if let Some(user_id) = &filter.user_id {
            builder.push(" AND user_id = ").push_bind(user_id);
        }
        if let Some(wallet_address) = &filter.wallet_address {
            builder
                .push(" AND wallet_address = ")
                .push_bind(wallet_address);
        }
        if let Some(chain_id) = &filter.chain_id {
            builder.push(" AND chain_id = ").push_bind(chain_id);
        }
        if let Some(status) = &filter.attestation_status {
            builder.push(" AND attestation_status = ").push_bind(status);
        }
        builder.push(" ORDER BY created_at DESC, id ASC");
        builder
            .build_query_as::<WalletAttestationRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }

    async fn upsert_connection(&self, request: &UpsertVenueConnectionRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO venue_connections (
                id, tenant_id, subject_type, subject_id, user_id, venue_key,
                connection_mode, status, metadata, last_verified_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                subject_type = EXCLUDED.subject_type,
                subject_id = EXCLUDED.subject_id,
                user_id = EXCLUDED.user_id,
                venue_key = EXCLUDED.venue_key,
                connection_mode = EXCLUDED.connection_mode,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata,
                last_verified_at = EXCLUDED.last_verified_at
            "#,
        )
        .bind(&request.connection_id)
        .bind(&request.tenant_id)
        .bind(&request.subject_type)
        .bind(&request.subject_id)
        .bind(&request.user_id)
        .bind(&request.venue_key)
        .bind(&request.connection_mode)
        .bind(&request.status)
        .bind(&request.metadata)
        .bind(request.last_verified_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_connection(
        &self,
        tenant_id: &str,
        connection_id: &str,
    ) -> Result<Option<VenueConnectionRecord>> {
        sqlx::query_as::<_, VenueConnectionRecord>(
            r#"
            SELECT id AS connection_id, tenant_id, subject_type, subject_id, user_id, venue_key,
                   connection_mode, status, metadata, last_verified_at, created_at, updated_at
            FROM venue_connections
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(connection_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn list_connections(
        &self,
        filter: &VenueConnectionFilter,
    ) -> Result<Vec<VenueConnectionRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id AS connection_id, tenant_id, subject_type, subject_id, user_id, venue_key, connection_mode, status, metadata, last_verified_at, created_at, updated_at FROM venue_connections WHERE tenant_id = ",
        );
        builder.push_bind(&filter.tenant_id);
        if let Some(subject_type) = &filter.subject_type {
            builder.push(" AND subject_type = ").push_bind(subject_type);
        }
        if let Some(subject_id) = &filter.subject_id {
            builder.push(" AND subject_id = ").push_bind(subject_id);
        }
        if let Some(venue_key) = &filter.venue_key {
            builder.push(" AND venue_key = ").push_bind(venue_key);
        }
        if let Some(status) = &filter.status {
            builder.push(" AND status = ").push_bind(status);
        }
        builder.push(" ORDER BY created_at DESC, id ASC");
        builder
            .build_query_as::<VenueConnectionRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }

    async fn upsert_account(&self, request: &UpsertVenueAccountRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO venue_accounts (
                id, tenant_id, venue_connection_id, venue_key, account_label, account_ref,
                wallet_address, subaccount_ref, api_scope_summary, status, metadata, last_verified_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                venue_connection_id = EXCLUDED.venue_connection_id,
                venue_key = EXCLUDED.venue_key,
                account_label = EXCLUDED.account_label,
                account_ref = EXCLUDED.account_ref,
                wallet_address = EXCLUDED.wallet_address,
                subaccount_ref = EXCLUDED.subaccount_ref,
                api_scope_summary = EXCLUDED.api_scope_summary,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata,
                last_verified_at = EXCLUDED.last_verified_at
            "#,
        )
        .bind(&request.account_id)
        .bind(&request.tenant_id)
        .bind(&request.venue_connection_id)
        .bind(&request.venue_key)
        .bind(&request.account_label)
        .bind(&request.account_ref)
        .bind(&request.wallet_address)
        .bind(&request.subaccount_ref)
        .bind(&request.api_scope_summary)
        .bind(&request.status)
        .bind(&request.metadata)
        .bind(request.last_verified_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_account(
        &self,
        tenant_id: &str,
        account_id: &str,
    ) -> Result<Option<VenueAccountRecord>> {
        sqlx::query_as::<_, VenueAccountRecord>(
            r#"
            SELECT id AS account_id, tenant_id, venue_connection_id, venue_key, account_label,
                   account_ref, wallet_address, subaccount_ref, api_scope_summary, status,
                   metadata, last_verified_at, created_at, updated_at
            FROM venue_accounts
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn list_accounts(&self, filter: &VenueAccountFilter) -> Result<Vec<VenueAccountRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id AS account_id, tenant_id, venue_connection_id, venue_key, account_label, account_ref, wallet_address, subaccount_ref, api_scope_summary, status, metadata, last_verified_at, created_at, updated_at FROM venue_accounts WHERE tenant_id = ",
        );
        builder.push_bind(&filter.tenant_id);
        if let Some(connection_id) = &filter.venue_connection_id {
            builder
                .push(" AND venue_connection_id = ")
                .push_bind(connection_id);
        }
        if let Some(venue_key) = &filter.venue_key {
            builder.push(" AND venue_key = ").push_bind(venue_key);
        }
        if let Some(status) = &filter.status {
            builder.push(" AND status = ").push_bind(status);
        }
        if let Some(wallet_address) = &filter.wallet_address {
            builder
                .push(" AND wallet_address = ")
                .push_bind(wallet_address);
        }
        builder.push(" ORDER BY created_at DESC, id ASC");
        builder
            .build_query_as::<VenueAccountRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }

    async fn upsert_beneficiary_profile(
        &self,
        request: &UpsertBeneficiaryProfileRequest,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO beneficiary_profiles (
                id, tenant_id, subject_type, subject_id, user_id, destination_type,
                destination_ref, display_name, asset_symbol, network, verification_status,
                cooldown_ends_at, metadata, last_verified_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                subject_type = EXCLUDED.subject_type,
                subject_id = EXCLUDED.subject_id,
                user_id = EXCLUDED.user_id,
                destination_type = EXCLUDED.destination_type,
                destination_ref = EXCLUDED.destination_ref,
                display_name = EXCLUDED.display_name,
                asset_symbol = EXCLUDED.asset_symbol,
                network = EXCLUDED.network,
                verification_status = EXCLUDED.verification_status,
                cooldown_ends_at = EXCLUDED.cooldown_ends_at,
                metadata = EXCLUDED.metadata,
                last_verified_at = EXCLUDED.last_verified_at
            "#,
        )
        .bind(&request.beneficiary_profile_id)
        .bind(&request.tenant_id)
        .bind(&request.subject_type)
        .bind(&request.subject_id)
        .bind(&request.user_id)
        .bind(&request.destination_type)
        .bind(&request.destination_ref)
        .bind(&request.display_name)
        .bind(&request.asset_symbol)
        .bind(&request.network)
        .bind(&request.verification_status)
        .bind(request.cooldown_ends_at)
        .bind(&request.metadata)
        .bind(request.last_verified_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_beneficiary_profile(
        &self,
        tenant_id: &str,
        beneficiary_profile_id: &str,
    ) -> Result<Option<BeneficiaryProfileRecord>> {
        sqlx::query_as::<_, BeneficiaryProfileRecord>(
            r#"
            SELECT id AS beneficiary_profile_id, tenant_id, subject_type, subject_id, user_id,
                   destination_type, destination_ref, display_name, asset_symbol, network,
                   verification_status, cooldown_ends_at, metadata, last_verified_at,
                   created_at, updated_at
            FROM beneficiary_profiles
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(beneficiary_profile_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn list_beneficiary_profiles(
        &self,
        filter: &BeneficiaryProfileFilter,
    ) -> Result<Vec<BeneficiaryProfileRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id AS beneficiary_profile_id, tenant_id, subject_type, subject_id, user_id, destination_type, destination_ref, display_name, asset_symbol, network, verification_status, cooldown_ends_at, metadata, last_verified_at, created_at, updated_at FROM beneficiary_profiles WHERE tenant_id = ",
        );
        builder.push_bind(&filter.tenant_id);
        if let Some(subject_type) = &filter.subject_type {
            builder.push(" AND subject_type = ").push_bind(subject_type);
        }
        if let Some(subject_id) = &filter.subject_id {
            builder.push(" AND subject_id = ").push_bind(subject_id);
        }
        if let Some(destination_type) = &filter.destination_type {
            builder
                .push(" AND destination_type = ")
                .push_bind(destination_type);
        }
        if let Some(status) = &filter.verification_status {
            builder
                .push(" AND verification_status = ")
                .push_bind(status);
        }
        builder.push(" ORDER BY created_at DESC, id ASC");
        builder
            .build_query_as::<BeneficiaryProfileRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }

    async fn upsert_transfer(&self, request: &UpsertVenueTransferRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO venue_transfers (
                id, tenant_id, user_id, beneficiary_profile_id, wallet_attestation_id,
                venue_connection_id, venue_account_id, transfer_direction, asset_symbol,
                network, amount, origin_intent_id, rfq_id, status, wallet_tx_hash,
                venue_credit_ref, failure_code, metadata, submitted_at, completed_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                user_id = EXCLUDED.user_id,
                beneficiary_profile_id = EXCLUDED.beneficiary_profile_id,
                wallet_attestation_id = EXCLUDED.wallet_attestation_id,
                venue_connection_id = EXCLUDED.venue_connection_id,
                venue_account_id = EXCLUDED.venue_account_id,
                transfer_direction = EXCLUDED.transfer_direction,
                asset_symbol = EXCLUDED.asset_symbol,
                network = EXCLUDED.network,
                amount = EXCLUDED.amount,
                origin_intent_id = EXCLUDED.origin_intent_id,
                rfq_id = EXCLUDED.rfq_id,
                status = EXCLUDED.status,
                wallet_tx_hash = EXCLUDED.wallet_tx_hash,
                venue_credit_ref = EXCLUDED.venue_credit_ref,
                failure_code = EXCLUDED.failure_code,
                metadata = EXCLUDED.metadata,
                submitted_at = EXCLUDED.submitted_at,
                completed_at = EXCLUDED.completed_at
            "#,
        )
        .bind(&request.transfer_id)
        .bind(&request.tenant_id)
        .bind(&request.user_id)
        .bind(&request.beneficiary_profile_id)
        .bind(request.wallet_attestation_id)
        .bind(&request.venue_connection_id)
        .bind(&request.venue_account_id)
        .bind(&request.transfer_direction)
        .bind(&request.asset_symbol)
        .bind(&request.network)
        .bind(request.amount)
        .bind(&request.origin_intent_id)
        .bind(&request.rfq_id)
        .bind(&request.status)
        .bind(&request.wallet_tx_hash)
        .bind(&request.venue_credit_ref)
        .bind(&request.failure_code)
        .bind(&request.metadata)
        .bind(request.submitted_at)
        .bind(request.completed_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_transfer(
        &self,
        tenant_id: &str,
        transfer_id: &str,
    ) -> Result<Option<VenueTransferRecord>> {
        sqlx::query_as::<_, VenueTransferRecord>(
            r#"
            SELECT id AS transfer_id, tenant_id, user_id, beneficiary_profile_id,
                   wallet_attestation_id, venue_connection_id, venue_account_id,
                   transfer_direction, asset_symbol, network, amount, origin_intent_id,
                   rfq_id, status, wallet_tx_hash, venue_credit_ref, failure_code, metadata,
                   submitted_at, completed_at, created_at, updated_at
            FROM venue_transfers
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn list_transfers(
        &self,
        filter: &VenueTransferFilter,
    ) -> Result<Vec<VenueTransferRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id AS transfer_id, tenant_id, user_id, beneficiary_profile_id, wallet_attestation_id, venue_connection_id, venue_account_id, transfer_direction, asset_symbol, network, amount, origin_intent_id, rfq_id, status, wallet_tx_hash, venue_credit_ref, failure_code, metadata, submitted_at, completed_at, created_at, updated_at FROM venue_transfers WHERE tenant_id = ",
        );
        builder.push_bind(&filter.tenant_id);
        if let Some(user_id) = &filter.user_id {
            builder.push(" AND user_id = ").push_bind(user_id);
        }
        if let Some(connection_id) = &filter.venue_connection_id {
            builder
                .push(" AND venue_connection_id = ")
                .push_bind(connection_id);
        }
        if let Some(account_id) = &filter.venue_account_id {
            builder
                .push(" AND venue_account_id = ")
                .push_bind(account_id);
        }
        if let Some(status) = &filter.status {
            builder.push(" AND status = ").push_bind(status);
        }
        builder.push(" ORDER BY created_at DESC, id ASC");
        builder
            .build_query_as::<VenueTransferRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }

    async fn upsert_source_of_funds_package(
        &self,
        request: &UpsertSourceOfFundsPackageRequest,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO source_of_funds_packages (
                id, tenant_id, subject_type, subject_id, wallet_attestation_id,
                venue_account_id, venue_transfer_id, review_status, package_uri, metadata, reviewed_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                subject_type = EXCLUDED.subject_type,
                subject_id = EXCLUDED.subject_id,
                wallet_attestation_id = EXCLUDED.wallet_attestation_id,
                venue_account_id = EXCLUDED.venue_account_id,
                venue_transfer_id = EXCLUDED.venue_transfer_id,
                review_status = EXCLUDED.review_status,
                package_uri = EXCLUDED.package_uri,
                metadata = EXCLUDED.metadata,
                reviewed_at = EXCLUDED.reviewed_at
            "#,
        )
        .bind(&request.package_id)
        .bind(&request.tenant_id)
        .bind(&request.subject_type)
        .bind(&request.subject_id)
        .bind(request.wallet_attestation_id)
        .bind(&request.venue_account_id)
        .bind(&request.venue_transfer_id)
        .bind(&request.review_status)
        .bind(&request.package_uri)
        .bind(&request.metadata)
        .bind(request.reviewed_at)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    async fn get_source_of_funds_package(
        &self,
        tenant_id: &str,
        package_id: &str,
    ) -> Result<Option<SourceOfFundsPackageRecord>> {
        sqlx::query_as::<_, SourceOfFundsPackageRecord>(
            r#"
            SELECT id AS package_id, tenant_id, subject_type, subject_id, wallet_attestation_id,
                   venue_account_id, venue_transfer_id, review_status, package_uri, metadata,
                   reviewed_at, created_at, updated_at
            FROM source_of_funds_packages
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn list_source_of_funds_packages(
        &self,
        filter: &SourceOfFundsPackageFilter,
    ) -> Result<Vec<SourceOfFundsPackageRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id AS package_id, tenant_id, subject_type, subject_id, wallet_attestation_id, venue_account_id, venue_transfer_id, review_status, package_uri, metadata, reviewed_at, created_at, updated_at FROM source_of_funds_packages WHERE tenant_id = ",
        );
        builder.push_bind(&filter.tenant_id);
        if let Some(subject_type) = &filter.subject_type {
            builder.push(" AND subject_type = ").push_bind(subject_type);
        }
        if let Some(subject_id) = &filter.subject_id {
            builder.push(" AND subject_id = ").push_bind(subject_id);
        }
        if let Some(transfer_id) = &filter.venue_transfer_id {
            builder
                .push(" AND venue_transfer_id = ")
                .push_bind(transfer_id);
        }
        if let Some(account_id) = &filter.venue_account_id {
            builder
                .push(" AND venue_account_id = ")
                .push_bind(account_id);
        }
        if let Some(status) = &filter.review_status {
            builder.push(" AND review_status = ").push_bind(status);
        }
        builder.push(" ORDER BY created_at DESC, id ASC");
        builder
            .build_query_as::<SourceOfFundsPackageRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }
}

fn database_error(error: sqlx::Error) -> ramp_common::Error {
    ramp_common::Error::Database(error.to_string())
}
