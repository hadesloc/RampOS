use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{TenantId, UserId},
    Error, Result,
};
use tokio::sync::Mutex;

use crate::case::{AmlCase, CaseNote};
use crate::store::CaseStore;
use crate::types::{CaseSeverity, CaseStatus};

#[derive(Default)]
pub struct InMemoryCaseStore {
    cases: Arc<Mutex<HashMap<String, AmlCase>>>,
    notes: Arc<Mutex<Vec<CaseNote>>>,
}

impl InMemoryCaseStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CaseStore for InMemoryCaseStore {
    async fn create_case(&self, case: &AmlCase) -> Result<String> {
        self.cases
            .lock()
            .await
            .insert(case.id.clone(), case.clone());
        Ok(case.id.clone())
    }

    async fn get_case(&self, tenant_id: &TenantId, case_id: &str) -> Result<Option<AmlCase>> {
        let cases = self.cases.lock().await;
        Ok(cases
            .get(case_id)
            .filter(|case| &case.tenant_id == tenant_id)
            .cloned())
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
        let mut cases: Vec<AmlCase> = self
            .cases
            .lock()
            .await
            .values()
            .filter(|case| &case.tenant_id == tenant_id)
            .filter(|case| status.is_none_or(|status| case.status == status))
            .filter(|case| severity.is_none_or(|severity| case.severity == severity))
            .filter(|case| {
                assigned_to.is_none_or(|assignee| case.assigned_to.as_deref() == Some(assignee))
            })
            .filter(|case| user_id.is_none_or(|user_id| case.user_id.as_ref() == Some(user_id)))
            .cloned()
            .collect();

        cases.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let start = offset.max(0) as usize;
        let end = (start + limit.max(0) as usize).min(cases.len());

        if start >= cases.len() {
            return Ok(vec![]);
        }

        Ok(cases[start..end].to_vec())
    }

    async fn count_cases(
        &self,
        tenant_id: &TenantId,
        status: Option<CaseStatus>,
        severity: Option<CaseSeverity>,
        assigned_to: Option<&str>,
        user_id: Option<&UserId>,
    ) -> Result<i64> {
        let count = self
            .cases
            .lock()
            .await
            .values()
            .filter(|case| &case.tenant_id == tenant_id)
            .filter(|case| status.is_none_or(|status| case.status == status))
            .filter(|case| severity.is_none_or(|severity| case.severity == severity))
            .filter(|case| {
                assigned_to.is_none_or(|assignee| case.assigned_to.as_deref() == Some(assignee))
            })
            .filter(|case| user_id.is_none_or(|user_id| case.user_id.as_ref() == Some(user_id)))
            .count();
        Ok(count as i64)
    }

    async fn avg_resolution_hours(&self, tenant_id: &TenantId) -> Result<f64> {
        let cases = self.cases.lock().await;
        let mut total = 0.0;
        let mut count = 0.0;

        for case in cases.values() {
            if &case.tenant_id == tenant_id {
                if let Some(resolved_at) = case.resolved_at {
                    let diff = resolved_at - case.created_at;
                    total += diff.num_seconds() as f64 / 3600.0;
                    count += 1.0;
                }
            }
        }

        if count == 0.0 {
            Ok(0.0)
        } else {
            Ok(total / count)
        }
    }

    async fn update_status(
        &self,
        tenant_id: &TenantId,
        case_id: &str,
        status: CaseStatus,
        resolved_at: Option<DateTime<Utc>>,
        resolution: Option<String>,
    ) -> Result<()> {
        if let Some(case) = self.cases.lock().await.get_mut(case_id) {
            if &case.tenant_id != tenant_id {
                return Err(Error::NotFound("Case not found".to_string()));
            }
            case.status = status;
            case.resolved_at = resolved_at;
            case.resolution = resolution;
            case.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn assign_case(
        &self,
        tenant_id: &TenantId,
        case_id: &str,
        assigned_to: &str,
    ) -> Result<()> {
        if let Some(case) = self.cases.lock().await.get_mut(case_id) {
            if &case.tenant_id != tenant_id {
                return Err(Error::NotFound("Case not found".to_string()));
            }
            case.assigned_to = Some(assigned_to.to_string());
            case.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn add_note(&self, tenant_id: &TenantId, note: &CaseNote) -> Result<()> {
        let cases = self.cases.lock().await;
        let case = cases
            .get(&note.case_id)
            .filter(|case| &case.tenant_id == tenant_id);

        if case.is_none() {
            return Err(Error::NotFound(
                "Case not found for note insertion".to_string(),
            ));
        }

        drop(cases);
        self.notes.lock().await.push(note.clone());
        Ok(())
    }

    async fn get_notes(&self, tenant_id: &TenantId, case_id: &str) -> Result<Vec<CaseNote>> {
        let cases = self.cases.lock().await;
        let case = cases
            .get(case_id)
            .filter(|case| &case.tenant_id == tenant_id);

        if case.is_none() {
            return Ok(vec![]);
        }

        drop(cases);
        Ok(self
            .notes
            .lock()
            .await
            .iter()
            .filter(|note| note.case_id == case_id)
            .cloned()
            .collect())
    }

    async fn get_user_cases(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<Vec<AmlCase>> {
        Ok(self
            .cases
            .lock()
            .await
            .values()
            .filter(|case| &case.tenant_id == tenant_id && case.user_id.as_ref() == Some(user_id))
            .cloned()
            .collect())
    }

    async fn get_open_cases(&self, tenant_id: &TenantId) -> Result<Vec<AmlCase>> {
        Ok(self
            .cases
            .lock()
            .await
            .values()
            .filter(|case| {
                &case.tenant_id == tenant_id
                    && matches!(
                        case.status,
                        CaseStatus::Open | CaseStatus::Review | CaseStatus::Hold
                    )
            })
            .cloned()
            .collect())
    }
}
