use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KybEvidenceSourceRecord {
    pub evidence_source_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub document_id: Option<String>,
    pub collected_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KybUboEvidenceLinkRecord {
    pub ubo_link_id: String,
    pub owner_entity_id: String,
    pub ownership_pct: Option<Decimal>,
    pub evidence_source_ref: Option<String>,
    pub review_state: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KybEvidencePackageRecord {
    pub package_id: String,
    pub tenant_id: String,
    pub institution_entity_id: String,
    pub institution_legal_name: String,
    pub provider_family: String,
    pub provider_policy_id: Option<String>,
    pub corridor_code: Option<String>,
    pub review_status: String,
    pub review_notes: Option<String>,
    pub export_status: String,
    pub export_artifact_uri: Option<String>,
    pub metadata: serde_json::Value,
    pub exported_at: Option<DateTime<Utc>>,
    pub evidence_sources: Vec<KybEvidenceSourceRecord>,
    pub ubo_links: Vec<KybUboEvidenceLinkRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertKybEvidenceSourceRequest {
    pub evidence_source_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub document_id: Option<String>,
    pub collected_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertKybUboEvidenceLinkRequest {
    pub ubo_link_id: String,
    pub owner_entity_id: String,
    pub ownership_pct: Option<Decimal>,
    pub evidence_source_ref: Option<String>,
    pub review_state: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertKybEvidencePackageGraphRequest {
    pub package_id: String,
    pub tenant_id: String,
    pub institution_entity_id: String,
    pub institution_legal_name: String,
    pub provider_family: String,
    pub provider_policy_id: Option<String>,
    pub corridor_code: Option<String>,
    pub review_status: String,
    pub review_notes: Option<String>,
    pub export_status: String,
    pub export_artifact_uri: Option<String>,
    pub metadata: serde_json::Value,
    pub exported_at: Option<DateTime<Utc>>,
    pub evidence_sources: Vec<UpsertKybEvidenceSourceRequest>,
    pub ubo_links: Vec<UpsertKybUboEvidenceLinkRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KybEvidencePackageQuery {
    pub tenant_id: String,
    pub institution_entity_id: Option<String>,
    pub corridor_code: Option<String>,
    pub review_status: Option<String>,
}

#[derive(Clone)]
pub struct KybEvidencePackageStore {
    pool: PgPool,
}

impl KybEvidencePackageStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_package_graph(
        &self,
        request: &UpsertKybEvidencePackageGraphRequest,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO kyb_evidence_packages (
                id, tenant_id, institution_entity_id, institution_legal_name, provider_family,
                provider_policy_id, corridor_code, review_status, review_notes, export_status,
                export_artifact_uri, metadata, exported_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                institution_entity_id = EXCLUDED.institution_entity_id,
                institution_legal_name = EXCLUDED.institution_legal_name,
                provider_family = EXCLUDED.provider_family,
                provider_policy_id = EXCLUDED.provider_policy_id,
                corridor_code = EXCLUDED.corridor_code,
                review_status = EXCLUDED.review_status,
                review_notes = EXCLUDED.review_notes,
                export_status = EXCLUDED.export_status,
                export_artifact_uri = EXCLUDED.export_artifact_uri,
                metadata = EXCLUDED.metadata,
                exported_at = EXCLUDED.exported_at
            "#,
        )
        .bind(&request.package_id)
        .bind(&request.tenant_id)
        .bind(&request.institution_entity_id)
        .bind(&request.institution_legal_name)
        .bind(&request.provider_family)
        .bind(&request.provider_policy_id)
        .bind(&request.corridor_code)
        .bind(&request.review_status)
        .bind(&request.review_notes)
        .bind(&request.export_status)
        .bind(&request.export_artifact_uri)
        .bind(&request.metadata)
        .bind(request.exported_at)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM kyb_evidence_sources WHERE package_id = $1")
            .bind(&request.package_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM kyb_ubo_evidence_links WHERE package_id = $1")
            .bind(&request.package_id)
            .execute(&mut *tx)
            .await?;

        for source in &request.evidence_sources {
            sqlx::query(
                r#"
                INSERT INTO kyb_evidence_sources (
                    id, package_id, source_kind, source_ref, document_id, collected_at, metadata
                ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(&source.evidence_source_id)
            .bind(&request.package_id)
            .bind(&source.source_kind)
            .bind(&source.source_ref)
            .bind(&source.document_id)
            .bind(source.collected_at)
            .bind(&source.metadata)
            .execute(&mut *tx)
            .await?;
        }

        for link in &request.ubo_links {
            sqlx::query(
                r#"
                INSERT INTO kyb_ubo_evidence_links (
                    id, package_id, owner_entity_id, ownership_pct, evidence_source_ref,
                    review_state, metadata
                ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(&link.ubo_link_id)
            .bind(&request.package_id)
            .bind(&link.owner_entity_id)
            .bind(link.ownership_pct)
            .bind(&link.evidence_source_ref)
            .bind(&link.review_state)
            .bind(&link.metadata)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_packages(
        &self,
        query: &KybEvidencePackageQuery,
    ) -> Result<Vec<KybEvidencePackageRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, KybEvidencePackageRow>(
            r#"
            SELECT
                id, tenant_id, institution_entity_id, institution_legal_name, provider_family,
                provider_policy_id, corridor_code, review_status, review_notes, export_status,
                export_artifact_uri, metadata, exported_at
            FROM kyb_evidence_packages
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR institution_entity_id = $2)
              AND ($3::text IS NULL OR corridor_code = $3)
              AND ($4::text IS NULL OR review_status = $4)
            ORDER BY created_at DESC, id ASC
            "#,
        )
        .bind(&query.tenant_id)
        .bind(&query.institution_entity_id)
        .bind(&query.corridor_code)
        .bind(&query.review_status)
        .fetch_all(&self.pool)
        .await?;

        const MAX_ITEMS: usize = 256;
        let mut records = Vec::with_capacity(rows.len().min(MAX_ITEMS));
        for row in rows.into_iter().take(MAX_ITEMS) {
            records.push(self.load_package_graph(row).await?);
        }
        Ok(records)
    }

    pub async fn get_package(
        &self,
        tenant_id: &str,
        package_id: &str,
    ) -> Result<Option<KybEvidencePackageRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, KybEvidencePackageRow>(
            r#"
            SELECT
                id, tenant_id, institution_entity_id, institution_legal_name, provider_family,
                provider_policy_id, corridor_code, review_status, review_notes, export_status,
                export_artifact_uri, metadata, exported_at
            FROM kyb_evidence_packages
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(package_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.load_package_graph(row).await?)),
            None => Ok(None),
        }
    }

    async fn load_package_graph(
        &self,
        row: KybEvidencePackageRow,
    ) -> Result<KybEvidencePackageRecord, sqlx::Error> {
        let evidence_sources = sqlx::query_as::<_, KybEvidenceSourceRow>(
            r#"
            SELECT id, source_kind, source_ref, document_id, collected_at, metadata
            FROM kyb_evidence_sources
            WHERE package_id = $1
            ORDER BY collected_at DESC, id ASC
            "#,
        )
        .bind(&row.id)
        .fetch_all(&self.pool)
        .await?;

        let ubo_links = sqlx::query_as::<_, KybUboEvidenceLinkRow>(
            r#"
            SELECT id, owner_entity_id, ownership_pct, evidence_source_ref, review_state, metadata
            FROM kyb_ubo_evidence_links
            WHERE package_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(&row.id)
        .fetch_all(&self.pool)
        .await?;

        Ok(KybEvidencePackageRecord {
            package_id: row.id,
            tenant_id: row.tenant_id,
            institution_entity_id: row.institution_entity_id,
            institution_legal_name: row.institution_legal_name,
            provider_family: row.provider_family,
            provider_policy_id: row.provider_policy_id,
            corridor_code: row.corridor_code,
            review_status: row.review_status,
            review_notes: row.review_notes,
            export_status: row.export_status,
            export_artifact_uri: row.export_artifact_uri,
            metadata: row.metadata,
            exported_at: row.exported_at,
            evidence_sources: evidence_sources
                .into_iter()
                .map(|source| KybEvidenceSourceRecord {
                    evidence_source_id: source.id,
                    source_kind: source.source_kind,
                    source_ref: source.source_ref,
                    document_id: source.document_id,
                    collected_at: source.collected_at,
                    metadata: source.metadata,
                })
                .collect(),
            ubo_links: ubo_links
                .into_iter()
                .map(|link| KybUboEvidenceLinkRecord {
                    ubo_link_id: link.id,
                    owner_entity_id: link.owner_entity_id,
                    ownership_pct: link.ownership_pct,
                    evidence_source_ref: link.evidence_source_ref,
                    review_state: link.review_state,
                    metadata: link.metadata,
                })
                .collect(),
        })
    }
}

#[derive(Debug, Clone, FromRow)]
struct KybEvidencePackageRow {
    id: String,
    tenant_id: String,
    institution_entity_id: String,
    institution_legal_name: String,
    provider_family: String,
    provider_policy_id: Option<String>,
    corridor_code: Option<String>,
    review_status: String,
    review_notes: Option<String>,
    export_status: String,
    export_artifact_uri: Option<String>,
    metadata: serde_json::Value,
    exported_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct KybEvidenceSourceRow {
    id: String,
    source_kind: String,
    source_ref: String,
    document_id: Option<String>,
    collected_at: DateTime<Utc>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct KybUboEvidenceLinkRow {
    id: String,
    owner_entity_id: String,
    ownership_pct: Option<Decimal>,
    evidence_source_ref: Option<String>,
    review_state: String,
    metadata: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_query_can_filter_by_review_status() {
        let query = KybEvidencePackageQuery {
            tenant_id: "tenant_kyb".to_string(),
            institution_entity_id: None,
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            review_status: Some("approved".to_string()),
        };

        assert_eq!(query.corridor_code.as_deref(), Some("VN_SG_PAYOUT"));
        assert_eq!(query.review_status.as_deref(), Some("approved"));
    }

    #[tokio::test]
    async fn db_gated_upsert_and_load_package_graph() {
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

        sqlx::query(
            r#"
            INSERT INTO kyb_entities (
                id, tenant_id, entity_type, display_name, jurisdiction, status, metadata
            ) VALUES
                ('kyb_pkg_business', 'tenant_kyb_pkg', 'business', 'Ramp Ops SG', 'SG', 'needs_review', '{}'::jsonb),
                ('kyb_pkg_ubo', 'tenant_kyb_pkg', 'ubo', 'UBO HoldCo', 'SG', 'verified', '{}'::jsonb)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .execute(&pool)
        .await
        .expect("kyb entities seed");

        let store = KybEvidencePackageStore::new(pool);
        let request = UpsertKybEvidencePackageGraphRequest {
            package_id: "pkg_kyb_sg_001".to_string(),
            tenant_id: "tenant_kyb_pkg".to_string(),
            institution_entity_id: "kyb_pkg_business".to_string(),
            institution_legal_name: "Ramp Ops SG".to_string(),
            provider_family: "kyb".to_string(),
            provider_policy_id: Some("policy_kyb_default".to_string()),
            corridor_code: Some("VN_SG_PAYOUT".to_string()),
            review_status: "approved".to_string(),
            review_notes: Some("manual review completed".to_string()),
            export_status: "ready".to_string(),
            export_artifact_uri: Some("s3://evidence/pkg_kyb_sg_001.json".to_string()),
            metadata: serde_json::json!({"entityType":"business"}),
            exported_at: None,
            evidence_sources: vec![UpsertKybEvidenceSourceRequest {
                evidence_source_id: "source_registry_extract".to_string(),
                source_kind: "registry_extract".to_string(),
                source_ref: "registry://sg/acra/123".to_string(),
                document_id: None,
                collected_at: Utc::now(),
                metadata: serde_json::json!({"freshnessDays": 7}),
            }],
            ubo_links: vec![UpsertKybUboEvidenceLinkRequest {
                ubo_link_id: "ubo_link_001".to_string(),
                owner_entity_id: "kyb_pkg_ubo".to_string(),
                ownership_pct: Some(Decimal::new(7500, 2)),
                evidence_source_ref: Some("registry://sg/acra/123".to_string()),
                review_state: "verified".to_string(),
                metadata: serde_json::json!({"source":"registry"}),
            }],
        };

        store
            .upsert_package_graph(&request)
            .await
            .expect("package graph should persist");

        let list = store
            .list_packages(&KybEvidencePackageQuery {
                tenant_id: "tenant_kyb_pkg".to_string(),
                institution_entity_id: Some("kyb_pkg_business".to_string()),
                corridor_code: Some("VN_SG_PAYOUT".to_string()),
                review_status: Some("approved".to_string()),
            })
            .await
            .expect("package list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].evidence_sources.len(), 1);
        assert_eq!(list[0].ubo_links.len(), 1);

        let detail = store
            .get_package("tenant_kyb_pkg", "pkg_kyb_sg_001")
            .await
            .expect("package detail load")
            .expect("package should exist");
        assert_eq!(detail.provider_policy_id.as_deref(), Some("policy_kyb_default"));
        assert_eq!(detail.ubo_links[0].review_state, "verified");
    }
}
