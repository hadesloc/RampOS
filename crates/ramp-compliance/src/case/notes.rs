use chrono::{DateTime, Utc};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{store::CaseStore, types::CaseStatus};

/// Note Type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteType {
    Comment,
    StatusChange,
    EvidenceAdded,
    Decision,
    Escalation,
}

impl std::fmt::Display for NoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NoteType::Comment => "Comment",
            NoteType::StatusChange => "StatusChange",
            NoteType::EvidenceAdded => "EvidenceAdded",
            NoteType::Decision => "Decision",
            NoteType::Escalation => "Escalation",
        };
        write!(f, "{}", s)
    }
}

/// Case Note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseNote {
    pub id: Uuid,
    pub case_id: String,
    pub author_id: Option<String>,
    pub content: String,
    pub note_type: NoteType,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

/// Case Note Manager
pub struct CaseNoteManager {
    store: Arc<dyn CaseStore>,
}

impl CaseNoteManager {
    pub fn new(store: Arc<dyn CaseStore>) -> Self {
        Self { store }
    }

    /// Add a note to a case
    pub async fn add_note(
        &self,
        case_id: &str,
        author_id: Option<String>,
        content: String,
        note_type: NoteType,
        is_internal: bool,
    ) -> Result<CaseNote> {
        let note = CaseNote {
            id: Uuid::now_v7(),
            case_id: case_id.to_string(),
            author_id: author_id.clone(),
            content: content.clone(),
            note_type: note_type.clone(),
            is_internal,
            created_at: Utc::now(),
        };

        self.store.add_note(&note).await?;

        info!(
            case_id = %case_id,
            note_type = ?note_type,
            author_id = ?author_id,
            "Case note added"
        );

        Ok(note)
    }

    /// Get all notes for a case
    pub async fn get_notes(&self, case_id: &str) -> Result<Vec<CaseNote>> {
        self.store.get_notes(case_id).await
    }

    /// Get public notes for a case (visible to customer)
    pub async fn get_public_notes(&self, case_id: &str) -> Result<Vec<CaseNote>> {
        let notes = self.store.get_notes(case_id).await?;
        Ok(notes.into_iter().filter(|n| !n.is_internal).collect())
    }

    /// Auto-create note on status change
    pub async fn on_status_change(
        &self,
        case_id: &str,
        old_status: CaseStatus,
        new_status: CaseStatus,
        author_id: Option<String>,
    ) -> Result<CaseNote> {
        let content = format!("Status changed from {:?} to {:?}", old_status, new_status);
        self.add_note(
            case_id,
            author_id,
            content,
            NoteType::StatusChange,
            true, // Status changes are usually internal unless explicitly notified
        )
        .await
    }

    /// Auto-create note on assignment change
    pub async fn on_assignment_change(
        &self,
        case_id: &str,
        assigned_to: Option<String>,
        author_id: Option<String>,
    ) -> Result<CaseNote> {
        let content = match assigned_to {
            Some(assignee) => format!("Case assigned to {}", assignee),
            None => "Case unassigned".to_string(),
        };

        self.add_note(
            case_id,
            author_id,
            content,
            NoteType::StatusChange, // Fits loosely or we could add AssignmentChange
            true,
        )
        .await
    }

    /// Auto-create note on resolution
    pub async fn on_resolution(
        &self,
        case_id: &str,
        resolution: &str,
        author_id: Option<String>,
    ) -> Result<CaseNote> {
        let content = format!("Case resolved: {}", resolution);
        self.add_note(case_id, author_id, content, NoteType::Decision, true)
            .await
    }
}
