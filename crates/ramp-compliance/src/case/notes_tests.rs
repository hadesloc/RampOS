#[cfg(test)]
mod tests {
    use crate::case::notes::{CaseNoteManager, NoteType};
    use crate::types::CaseStatus;

    #[tokio::test]
    async fn test_case_notes() {
        let manager = CaseNoteManager::new();
        let case_id = "case_123";

        // Test add note
        let note = manager
            .add_note(
                case_id,
                Some("analyst_1".to_string()),
                "Investigation started".to_string(),
                NoteType::Comment,
                true,
            )
            .await
            .unwrap();

        assert_eq!(note.case_id, case_id);
        assert_eq!(note.content, "Investigation started");
        assert_eq!(note.note_type, NoteType::Comment);
        assert!(note.is_internal);

        // Test status change note
        let note = manager
            .on_status_change(
                case_id,
                CaseStatus::Open,
                CaseStatus::Review,
                Some("system".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(note.note_type, NoteType::StatusChange);
        assert!(note.content.contains("Open"));
        assert!(note.content.contains("Review"));

        // Test assignment change note
        let note = manager
            .on_assignment_change(
                case_id,
                Some("analyst_2".to_string()),
                Some("admin".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(note.note_type, NoteType::StatusChange); // We used StatusChange for assignment
        assert!(note.content.contains("assigned to analyst_2"));

        // Test resolution note
        let note = manager
            .on_resolution(case_id, "False positive", Some("manager".to_string()))
            .await
            .unwrap();

        assert_eq!(note.note_type, NoteType::Decision);
        assert!(note.content.contains("False positive"));
    }
}
