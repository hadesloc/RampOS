use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId},
    Result,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    case::{AmlCase, CaseNote, NoteType},
    types::{CaseSeverity, CaseStatus, CaseType},
};

#[async_trait]
pub trait CaseStore: Send + Sync {
    async fn create_case(&self, case: &AmlCase) -> Result<String>;
    async fn get_case(&self, case_id: &str) -> Result<Option<AmlCase>>;
    async fn update_status(
        &self,
        case_id: &str,
        status: CaseStatus,
        resolved_at: Option<DateTime<Utc>>,
        resolution: Option<String>,
    ) -> Result<()>;
    async fn assign_case(&self, case_id: &str, assigned_to: &str) -> Result<()>;
    async fn add_note(&self, note: &CaseNote) -> Result<()>;
    async fn get_notes(&self, case_id: &str) -> Result<Vec<CaseNote>>;
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
            INSERT INTO compliance_cases (
                id, tenant_id, user_id, intent_id, case_type, severity, status,
                detection_data, assigned_to, resolution, created_at, updated_at, resolved_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&case.id)
        .bind(&case.tenant_id.to_string())
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

    async fn get_case(&self, case_id: &str) -> Result<Option<AmlCase>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, intent_id, case_type, severity, status,
                detection_data, assigned_to, resolution, created_at, updated_at, resolved_at
            FROM compliance_cases
            WHERE id = $1
            "#,
        )
        .bind(case_id)
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

            let severity: CaseSeverity = serde_json::from_str(&severity_str)
                .unwrap_or(CaseSeverity::Low);

            let status: CaseStatus = serde_json::from_str(&status_str)
                .unwrap_or(CaseStatus::Open);

            Ok(Some(AmlCase {
                id: row.try_get("id")?,
                tenant_id: TenantId::from(row.try_get::<String, _>("tenant_id")?),
                user_id: row.try_get::<Option<String>, _>("user_id")?.map(UserId::from),
                intent_id: row.try_get::<Option<String>, _>("intent_id")?.map(IntentId::from),
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

    async fn update_status(
        &self,
        case_id: &str,
        status: CaseStatus,
        resolved_at: Option<DateTime<Utc>>,
        resolution: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE compliance_cases
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
            UPDATE compliance_cases
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

    async fn add_note(&self, note: &CaseNote) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO case_notes (
                id, case_id, author_id, content, note_type, is_internal, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(note.id)
        .bind(&note.case_id)
        .bind(&note.author_id)
        .bind(&note.content)
        .bind(serde_json::to_string(&note.note_type).unwrap_or_default())
        .bind(note.is_internal)
        .bind(note.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_notes(&self, case_id: &str) -> Result<Vec<CaseNote>> {
        let rows = sqlx::query(
            r#"
            SELECT id, case_id, author_id, content, note_type, is_internal, created_at
            FROM case_notes
            WHERE case_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(case_id)
        .fetch_all(&self.pool)
        .await?;

        let mut notes = Vec::new();
        for row in rows {
            let note_type_str: String = row.try_get("note_type")?;
            let note_type: NoteType = serde_json::from_str(&note_type_str)
                .unwrap_or(NoteType::Comment);

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
            FROM compliance_cases
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
            FROM compliance_cases
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

            let severity: CaseSeverity = serde_json::from_str(&severity_str)
                .unwrap_or(CaseSeverity::Low);

            let status: CaseStatus = serde_json::from_str(&status_str)
                .unwrap_or(CaseStatus::Open);

            cases.push(AmlCase {
                id: row.try_get("id")?,
                tenant_id: TenantId::from(row.try_get::<String, _>("tenant_id")?),
                user_id: row.try_get::<Option<String>, _>("user_id")?.map(UserId::from),
                intent_id: row.try_get::<Option<String>, _>("intent_id")?.map(IntentId::from),
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
