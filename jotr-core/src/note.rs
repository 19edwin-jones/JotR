use chrono::{DateTime, Utc};

/// A single note. Deletion is soft — `deleted_at` is a tombstone, not an
/// immediate removal: `None` means active, `Some(_)` means trashed and
/// hidden from normal reads until purged via `NoteStore::permanently_delete_note`
/// or `cleanup_deleted_notes`.
#[derive(Debug, PartialEq)]
pub struct Note {
    pub id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
