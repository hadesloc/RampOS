use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryEvidenceImportRecord {
    pub evidence_import_id: String,
    pub tenant_id: String,
    pub source_family: String,
    pub source_ref: String,
    pub account_scope: String,
    pub asset_code: String,
    pub idempotency_key: String,
    pub snapshot_at: DateTime<Utc>,
    pub available_balance: Decimal,
    pub reserved_balance: Decimal,
    pub source_lineage: serde_json::Value,
    pub metadata: serde_json::Value,
    pub imported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertTreasuryEvidenceImportRequest {
    pub evidence_import_id: String,
    pub tenant_id: String,
    pub source_family: String,
    pub source_ref: String,
    pub account_scope: String,
    pub asset_code: String,
    pub idempotency_key: String,
    pub snapshot_at: DateTime<Utc>,
    pub available_balance: Decimal,
    pub reserved_balance: Decimal,
    pub source_lineage: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryEvidenceImportQuery {
    pub tenant_id: String,
    pub source_family: Option<String>,
    pub asset_code: Option<String>,
    pub account_scope: Option<String>,
}

#[derive(Clone)]
pub struct TreasuryEvidenceImportStore {
    pool: PgPool,
}

impl TreasuryEvidenceImportStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn import_evidence(
        &self,
        request: &UpsertTreasuryEvidenceImportRequest,
    ) -> Result<TreasuryEvidenceImportRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, TreasuryEvidenceImportRow>(
            r#"
            INSERT INTO treasury_evidence_imports (
                id, tenant_id, source_family, source_ref, account_scope, asset_code,
                idempotency_key, snapshot_at, available_balance, reserved_balance,
                source_lineage, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (tenant_id, idempotency_key) DO UPDATE SET
                source_family = EXCLUDED.source_family,
                source_ref = EXCLUDED.source_ref,
                account_scope = EXCLUDED.account_scope,
                asset_code = EXCLUDED.asset_code,
                snapshot_at = EXCLUDED.snapshot_at,
                available_balance = EXCLUDED.available_balance,
                reserved_balance = EXCLUDED.reserved_balance,
                source_lineage = EXCLUDED.source_lineage,
                metadata = EXCLUDED.metadata,
                imported_at = NOW()
            RETURNING id, tenant_id, source_family, source_ref, account_scope, asset_code,
                      idempotency_key, snapshot_at, available_balance, reserved_balance,
                      source_lineage, metadata, imported_at
            "#,
        )
        .bind(&request.evidence_import_id)
        .bind(&request.tenant_id)
        .bind(&request.source_family)
        .bind(&request.source_ref)
        .bind(&request.account_scope)
        .bind(&request.asset_code)
        .bind(&request.idempotency_key)
        .bind(request.snapshot_at)
        .bind(request.available_balance)
        .bind(request.reserved_balance)
        .bind(&request.source_lineage)
        .bind(&request.metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into_record())
    }

    pub async fn list_imports(
        &self,
        query: &TreasuryEvidenceImportQuery,
    ) -> Result<Vec<TreasuryEvidenceImportRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, TreasuryEvidenceImportRow>(
            r#"
            SELECT id, tenant_id, source_family, source_ref, account_scope, asset_code,
                   idempotency_key, snapshot_at, available_balance, reserved_balance,
                   source_lineage, metadata, imported_at
            FROM treasury_evidence_imports
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR source_family = $2)
              AND ($3::text IS NULL OR asset_code = $3)
              AND ($4::text IS NULL OR account_scope = $4)
            ORDER BY snapshot_at DESC, imported_at DESC, id ASC
            "#
        )
        .bind(&query.tenant_id)
        .bind(&query.source_family)
        .bind(&query.asset_code)
        .bind(&query.account_scope)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(TreasuryEvidenceImportRow::into_record).collect())
    }
}

pub fn normalize_treasury_balances(
    available_balance: Decimal,
    reserved_balance: Decimal,
) -> (Decimal, Decimal) {
    let available = available_balance.max(Decimal::ZERO);
    let reserved = reserved_balance.max(Decimal::ZERO);
    (available, reserved)
}

#[derive(Debug, Clone, FromRow)]
struct TreasuryEvidenceImportRow {
    id: String,
    tenant_id: String,
    source_family: String,
    source_ref: String,
    account_scope: String,
    asset_code: String,
    idempotency_key: String,
    snapshot_at: DateTime<Utc>,
    available_balance: Decimal,
    reserved_balance: Decimal,
    source_lineage: serde_json::Value,
    metadata: serde_json::Value,
    imported_at: DateTime<Utc>,
}

impl TreasuryEvidenceImportRow {
    fn into_record(self) -> TreasuryEvidenceImportRecord {
        TreasuryEvidenceImportRecord {
            evidence_import_id: self.id,
            tenant_id: self.tenant_id,
            source_family: self.source_family,
            source_ref: self.source_ref,
            account_scope: self.account_scope,
            asset_code: self.asset_code,
            idempotency_key: self.idempotency_key,
            snapshot_at: self.snapshot_at,
            available_balance: self.available_balance,
            reserved_balance: self.reserved_balance,
            source_lineage: self.source_lineage,
            metadata: self.metadata,
            imported_at: self.imported_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_treasury_balances_clamps_negative_values() {
        let (available, reserved) = normalize_treasury_balances(
            Decimal::new(-500, 0),
            Decimal::new(-250, 0),
        );

        assert_eq!(available, Decimal::ZERO);
        assert_eq!(reserved, Decimal::ZERO);
    }

    #[tokio::test]
    async fn db_gated_import_is_replay_safe_by_idempotency_key() {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };

        let pool = PgPool::connect(&database_url)
            .await
            .expect("database connection should succeed");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrations should succeed");

        let tenant_id = "tenant_treasury_evidence";
        sqlx::query(
            r#"
            INSERT INTO tenants (
                id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
            ) VALUES ($1, 'Treasury Evidence Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("seed tenant");

        let store = TreasuryEvidenceImportStore::new(pool);
        let request = UpsertTreasuryEvidenceImportRequest {
            evidence_import_id: "tei_001".to_string(),
            tenant_id: tenant_id.to_string(),
            source_family: "bank".to_string(),
            source_ref: "bank://vcb/main".to_string(),
            account_scope: "bank:vcb/vnd".to_string(),
            asset_code: "VND".to_string(),
            idempotency_key: "import_001".to_string(),
            snapshot_at: Utc::now(),
            available_balance: Decimal::new(5000000, 0),
            reserved_balance: Decimal::new(1000000, 0),
            source_lineage: serde_json::json!({"statementId":"stmt_001"}),
            metadata: serde_json::json!({"source":"bank_statement"}),
        };

        let first = store.import_evidence(&request).await.expect("first import");
        let second = store.import_evidence(&request).await.expect("replayed import");
        let rows = store
            .list_imports(&TreasuryEvidenceImportQuery {
                tenant_id: tenant_id.to_string(),
                source_family: Some("bank".to_string()),
                asset_code: Some("VND".to_string()),
                account_scope: Some("bank:vcb/vnd".to_string()),
            })
            .await
            .expect("list imports");

        assert_eq!(first.idempotency_key, "import_001");
        assert_eq!(second.idempotency_key, "import_001");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source_ref, "bank://vcb/main");
    }
}
