use chrono::Utc;
use ramp_common::{
    types::{IntentId, TenantId, UserId},
    Result,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    store::postgres::CaseStore,
    types::{CaseSeverity, CaseStatus, CaseType},
};
pub mod notes;
pub use notes::{CaseNote, CaseNoteManager, NoteType};

/// AML Case
#[derive(Debug, Clone)]
pub struct AmlCase {
    pub id: String,
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub intent_id: Option<IntentId>,
    pub case_type: CaseType,
    pub severity: CaseSeverity,
    pub status: CaseStatus,
    pub detection_data: serde_json::Value,
    pub assigned_to: Option<String>,
    pub resolution: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub resolved_at: Option<chrono::DateTime<Utc>>,
}

/// Case Manager - handles AML case lifecycle
pub struct CaseManager {
    store: Arc<dyn CaseStore>,
    pub note_manager: CaseNoteManager,
}

impl CaseManager {
    pub fn new(store: Arc<dyn CaseStore>) -> Self {
        Self {
            store: store.clone(),
            note_manager: CaseNoteManager::new(store),
        }
    }

    /// Create a new AML case
    pub async fn create_case(
        &self,
        tenant_id: &TenantId,
        user_id: Option<&UserId>,
        intent_id: Option<&IntentId>,
        case_type: CaseType,
        severity: CaseSeverity,
        detection_data: serde_json::Value,
    ) -> Result<String> {
        let case_id = format!("case_{}", Uuid::now_v7());
        let now = Utc::now();

        let case = AmlCase {
            id: case_id.clone(),
            tenant_id: tenant_id.clone(),
            user_id: user_id.cloned(),
            intent_id: intent_id.cloned(),
            case_type: case_type.clone(),
            severity,
            status: CaseStatus::Open,
            detection_data,
            assigned_to: None,
            resolution: None,
            created_at: now,
            updated_at: now,
            resolved_at: None,
        };

        self.store.create_case(&case).await?;

        info!(
            case_id = %case_id,
            case_type = ?case_type,
            severity = ?severity,
            "AML case created"
        );

        Ok(case_id)
    }

    /// Update case status
    pub async fn update_status(
        &self,
        case_id: &str,
        new_status: CaseStatus,
        author_id: Option<String>,
    ) -> Result<()> {
        let old_case = self.store.get_case(case_id).await?;
        let old_status = match old_case {
            Some(c) => c.status,
            None => return Ok(()), // Or error if case not found
        };

        if old_status != new_status {
            self.store
                .update_status(case_id, new_status, None, None)
                .await?;

            info!(
                case_id = case_id,
                new_status = ?new_status,
                "Case status updated"
            );

            // Auto-create note
            self.note_manager
                .on_status_change(case_id, old_status, new_status, author_id)
                .await?;
        }

        Ok(())
    }

    /// Assign case to analyst
    pub async fn assign_case(
        &self,
        case_id: &str,
        analyst_id: &str,
        author_id: Option<String>,
    ) -> Result<()> {
        self.store.assign_case(case_id, analyst_id).await?;

        info!(case_id = case_id, analyst_id = analyst_id, "Case assigned");

        // Auto-create note
        self.note_manager
            .on_assignment_change(case_id, Some(analyst_id.to_string()), author_id)
            .await?;

        Ok(())
    }

    /// Resolve case
    pub async fn resolve_case(
        &self,
        case_id: &str,
        resolution: &str,
        new_status: CaseStatus,
        author_id: Option<String>,
    ) -> Result<()> {
        self.store
            .update_status(
                case_id,
                new_status,
                Some(Utc::now()),
                Some(resolution.to_string()),
            )
            .await?;

        info!(
            case_id = case_id,
            resolution = resolution,
            status = ?new_status,
            "Case resolved"
        );

        // Auto-create note
        self.note_manager
            .on_resolution(case_id, resolution, author_id)
            .await?;

        Ok(())
    }

    /// Get cases for a user
    pub async fn get_user_cases(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<AmlCase>> {
        self.store.get_user_cases(tenant_id, user_id).await
    }

    /// Get open cases
    pub async fn get_open_cases(&self, tenant_id: &TenantId) -> Result<Vec<AmlCase>> {
        self.store.get_open_cases(tenant_id).await
    }
}
