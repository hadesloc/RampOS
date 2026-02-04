use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId},
    Result,
};
use sqlx::{PgPool, QueryBuilder, Row};

use crate::{
    case::{AmlCase, CaseNote, NoteType},
    types::{CaseSeverity, CaseStatus, CaseType},
};

#[async_trait]
pub trait CaseStore: Send + Sync {
    async fn create_case(&self, case: &AmlCase) -> Result<String>;
    async fn get_case(&self, tenant_id: &TenantId, case_id: &str) -> Result<Option<AmlCase>>;
    async fn list_cases(
        &self,
        tenant_id: &TenantId,
        status: Option<CaseStatus>,
        severity: Option<CaseSeverity>,
        assigned_to: Option<&str>,
        user_id: Option<&UserId>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AmlCase>>;
    async fn count_cases(
        &self,
        tenant_id: &TenantId,
        status: Option<CaseStatus>,
        severity: Option<CaseSeverity>,
        assigned_to: Option<&str>,
        user_id: Option<&UserId>,
    ) -> Result<i64>;
    async fn avg_resolution_hours(&self, tenant_id: &TenantId) -> Result<f64>;
    async fn update_status(
        &self,
        case_id: &str,
        status: CaseStatus,
        resolved_at: Option<DateTime<Utc>>,
        resolution: Option<String>,
    ) -> Result<()>;
    async fn assign_case(&self, case_id: &str, assigned_to: &str) -> Result<()>;
    async fn add_note(&self, tenant_id: &TenantId, note: &CaseNote) -> Result<()>;
    async fn get_notes(&self, tenant_id: &TenantId, case_id: &str) -> Result<Vec<CaseNote>>;
    async fn get_user_cases(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<Vec<AmlCase>>;
    async fn get_open_cases(&self, tenant_id: &TenantId) -> Result<Vec<AmlCase>>;
}

pub struct PostgresCaseStore {
    pool: PgPool,
}

impl PostgresCaseStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CaseStore for PostgresCaseStore {
    async fn create_case(&self, case: &AmlCase) -> Result<String> {
        sqlx::query(
            r#"
            INSERT INTO aml_cases (
                id, tenant_id, user_id, intent_id, case_type, severity, status,
                detection_data, assigned_to, resolution, created_at, updated_at, resolved_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&case.id)
        .bind(case.tenant_id.to_string())
        .bind(case.user_id.as_ref().map(|u| u.to_string()))
        .bind(case.intent_id.as_ref().map(|i| i.to_string()))
        .bind(serde_json::to_string(&case.case_type).unwrap_or_default())
        .bind(serde_json::to_string(&case.severity).unwrap_or_default())
        .bind(serde_json::to_string(&case.status).unwrap_or_default())
        .bind(&case.detection_data)
        .bind(&case.assigned_to)
        .bind(&case.resolution)
        .bind(case.created_at)
        .bind(case.updated_at)
        .bind(case.resolved_at)
        .execute(&self.pool)
        .await?;

        Ok(case.id.clone())
    }

    async fn get_case(&self, tenant_id: &TenantId, case_id: &str) -> Result<Option<AmlCase>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, intent_id, case_type, severity, status,
                detection_data, assigned_to, resolution, created_at, updated_at, resolved_at
            FROM aml_cases
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(case_id)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let case_type_str: String = row.try_get("case_type")?;
            let severity_str: String = row.try_get("severity")?;
            let status_str: String = row.try_get("status")?;

            // Handle parsing JSON strings to enums
            // Since we stored them as JSON strings, we need to deserialize them properly
            // The format stored by serde_json::to_string is usually quoted like "Velocity"
            // So we can try serde_json::from_str

            let case_type: CaseType = serde_json::from_str(&case_type_str)
                .unwrap_or(CaseType::Other("Unknown".to_string()));

            let severity: CaseSeverity =
                serde_json::from_str(&severity_str).unwrap_or(CaseSeverity::Low);

            let status: CaseStatus = serde_json::from_str(&status_str).unwrap_or(CaseStatus::Open);

            Ok(Some(AmlCase {
                id: row.try_get("id")?,
                tenant_id: TenantId::new(row.try_get::<String, _>("tenant_id")?),
                user_id: row
                    .try_get::<Option<String>, _>("user_id")?
                    .map(UserId::new),
                intent_id: row
                    .try_get::<Option<String>, _>("intent_id")?
                    .map(IntentId::new),
                case_type,
                severity,
                status,
                detection_data: row.try_get("detection_data")?,
                assigned_to: row.try_get("assigned_to")?,
                resolution: row.try_get("resolution")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                resolved_at: row.try_get("resolved_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_cases(
        &self,
        tenant_id: &TenantId,
        status: Option<CaseStatus>,
        severity: Option<CaseSeverity>,
        assigned_to: Option<&str>,
        user_id: Option<&UserId>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AmlCase>> {
        let mut builder = QueryBuilder::new(
            "SELECT id, tenant_id, user_id, intent_id, case_type, severity, status, \
             detection_data, assigned_to, resolution, created_at, updated_at, resolved_at \
             FROM aml_cases WHERE tenant_id = ",
        );
        builder.push_bind(tenant_id.to_string());

        if let Some(status) = status {
            let status_str = serde_json::to_string(&status).unwrap_or_default();
            builder.push(" AND status = ").push_bind(status_str);
        }

        if let Some(severity) = severity {
            let severity_str = serde_json::to_string(&severity).unwrap_or_default();
            builder.push(" AND severity = ").push_bind(severity_str);
        }

        if let Some(assigned_to) = assigned_to {
            builder.push(" AND assigned_to = ").push_bind(assigned_to);
        }

        if let Some(user_id) = user_id {
            builder
                .push(" AND user_id = ")
                .push_bind(user_id.to_string());
        }

        builder
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        self.map_rows_to_cases(rows)
    }

    async fn count_cases(
        &self,
        tenant_id: &TenantId,
        status: Option<CaseStatus>,
        severity: Option<CaseSeverity>,
        assigned_to: Option<&str>,
        user_id: Option<&UserId>,
    ) -> Result<i64> {
        let mut builder =
            QueryBuilder::new("SELECT COUNT(*) as count FROM aml_cases WHERE tenant_id = ");
        builder.push_bind(tenant_id.to_string());

        if let Some(status) = status {
            let status_str = serde_json::to_string(&status).unwrap_or_default();
            builder.push(" AND status = ").push_bind(status_str);
        }

        if let Some(severity) = severity {
            let severity_str = serde_json::to_string(&severity).unwrap_or_default();
            builder.push(" AND severity = ").push_bind(severity_str);
        }

        if let Some(assigned_to) = assigned_to {
            builder.push(" AND assigned_to = ").push_bind(assigned_to);
        }

        if let Some(user_id) = user_id {
            builder
                .push(" AND user_id = ")
                .push_bind(user_id.to_string());
        }

        let row = builder
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        let count: i64 = row.try_get("count").unwrap_or(0);
        Ok(count)
    }

    async fn avg_resolution_hours(&self, tenant_id: &TenantId) -> Result<f64> {
        let row = sqlx::query(
            r#"
            SELECT AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) as avg_hours
            FROM aml_cases
            WHERE tenant_id = $1 AND resolved_at IS NOT NULL
            "#,
        )
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        let avg: Option<f64> = row.try_get("avg_hours").unwrap_or(None);
        Ok(avg.unwrap_or(0.0))
    }

    async fn update_status(
        &self,
        case_id: &str,
        status: CaseStatus,
        resolved_at: Option<DateTime<Utc>>,
        resolution: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE aml_cases
            SET status = $1, resolved_at = $2, resolution = $3, updated_at = NOW()
            WHERE id = $4
            "#,
        )
        .bind(serde_json::to_string(&status).unwrap_or_default())
        .bind(resolved_at)
        .bind(resolution)
        .bind(case_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn assign_case(&self, case_id: &str, assigned_to: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE aml_cases
            SET assigned_to = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(assigned_to)
        .bind(case_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn add_note(&self, tenant_id: &TenantId, note: &CaseNote) -> Result<()> {
        let result = sqlx::query(
            r#"
            INSERT INTO case_notes (
                id, case_id, tenant_id, author_id, content, note_type, is_internal, created_at
            )
            SELECT $1, $2, tenant_id, $3, $4, $5, $6, $7
            FROM aml_cases
            WHERE id = $2 AND tenant_id = $8
            "#,
        )
        .bind(note.id)
        .bind(&note.case_id)
        .bind(&note.author_id)
        .bind(&note.content)
        .bind(serde_json::to_string(&note.note_type).unwrap_or_default())
        .bind(note.is_internal)
        .bind(note.created_at)
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ramp_common::Error::Database(
                "Case not found for note insertion".to_string(),
            ));
        }

        Ok(())
    }

    async fn get_notes(&self, tenant_id: &TenantId, case_id: &str) -> Result<Vec<CaseNote>> {
        let rows = sqlx::query(
            r#"
            SELECT id, case_id, author_id, content, note_type, is_internal, created_at
            FROM case_notes
            WHERE case_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(case_id)
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut notes = Vec::new();
        for row in rows {
            let note_type_str: String = row.try_get("note_type")?;
            let note_type: NoteType =
                serde_json::from_str(&note_type_str).unwrap_or(NoteType::Comment);

            notes.push(CaseNote {
                id: row.try_get("id")?,
                case_id: row.try_get("case_id")?,
                author_id: row.try_get("author_id")?,
                content: row.try_get("content")?,
                note_type,
                is_internal: row.try_get("is_internal")?,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(notes)
    }

    async fn get_user_cases(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<Vec<AmlCase>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, intent_id, case_type, severity, status,
                detection_data, assigned_to, resolution, created_at, updated_at, resolved_at
            FROM aml_cases
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        self.map_rows_to_cases(rows)
    }

    async fn get_open_cases(&self, tenant_id: &TenantId) -> Result<Vec<AmlCase>> {
        let open_status = serde_json::to_string(&CaseStatus::Open).unwrap_or_default();
        let review_status = serde_json::to_string(&CaseStatus::Review).unwrap_or_default();
        let hold_status = serde_json::to_string(&CaseStatus::Hold).unwrap_or_default();

        let rows = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, intent_id, case_type, severity, status,
                detection_data, assigned_to, resolution, created_at, updated_at, resolved_at
            FROM aml_cases
            WHERE tenant_id = $1 AND status IN ($2, $3, $4)
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(open_status)
        .bind(review_status)
        .bind(hold_status)
        .fetch_all(&self.pool)
        .await?;

        self.map_rows_to_cases(rows)
    }
}

impl PostgresCaseStore {
    fn map_rows_to_cases(&self, rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<AmlCase>> {
        let mut cases = Vec::new();
        for row in rows {
            let case_type_str: String = row.try_get("case_type")?;
            let severity_str: String = row.try_get("severity")?;
            let status_str: String = row.try_get("status")?;

            let case_type: CaseType = serde_json::from_str(&case_type_str)
                .unwrap_or(CaseType::Other("Unknown".to_string()));

            let severity: CaseSeverity =
                serde_json::from_str(&severity_str).unwrap_or(CaseSeverity::Low);

            let status: CaseStatus = serde_json::from_str(&status_str).unwrap_or(CaseStatus::Open);

            cases.push(AmlCase {
                id: row.try_get("id")?,
                tenant_id: TenantId::new(row.try_get::<String, _>("tenant_id")?),
                user_id: row
                    .try_get::<Option<String>, _>("user_id")?
                    .map(UserId::new),
                intent_id: row
                    .try_get::<Option<String>, _>("intent_id")?
                    .map(IntentId::new),
                case_type,
                severity,
                status,
                detection_data: row.try_get("detection_data")?,
                assigned_to: row.try_get("assigned_to")?,
                resolution: row.try_get("resolution")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                resolved_at: row.try_get("resolved_at")?,
            });
        }
        Ok(cases)
    }
}

