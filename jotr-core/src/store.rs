use chrono::Utc;
use rusqlite::{Connection, Result};

use crate::database;
use crate::note::Note;

/// Public entry point for note storage. Wraps a `database` connection,
/// translating its low-level results into richer return types and adding
/// the domain policy (soft delete, retention) that `database` stays
/// agnostic of.
pub struct NoteStore {
    connection: Connection,
}

impl NoteStore {
    /// Opens the app's real, on-disk database.
    pub fn new() -> Result<Self> {
        let connection = database::open_database()?;

        Ok(Self { connection })
    }

    /// Builds a store around an existing connection — used by tests to run
    /// against an in-memory database instead of the real `jotr.db` file.
    pub fn from_connection(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn add_note(&self, content: &str) -> Result<i64> {
        database::add_note(&self.connection, content)
    }

    pub fn get_note(&self, id: i64) -> Result<Option<Note>> {
        database::get_note_by_id(&self.connection, id)
    }

    pub fn notes(&self) -> Result<Vec<Note>> {
        database::get_all_notes(&self.connection)
    }

    /// `database::update_note` only reports whether a row changed; re-fetch
    /// so callers get the updated note (with its bumped `updated_at`) directly.
    pub fn update_note(&self, id: i64, content: &str) -> Result<Option<Note>> {
        let updated = database::update_note(&self.connection, id, content)?;

        if updated { self.get_note(id) } else { Ok(None) }
    }

    /// Notes currently in the trash (soft-deleted, not yet purged).
    pub fn deleted_notes(&self) -> Result<Vec<Note>> {
        database::get_deleted_notes(&self.connection)
    }

    /// Soft-deletes a note: the row still exists and can be brought back
    /// with `restore_note`, until `cleanup_deleted_notes` or
    /// `permanently_delete_note` removes it for good.
    pub fn delete_note(&self, id: i64) -> Result<Option<Note>> {
        database::soft_delete_note(&self.connection, id)
    }

    /// Permanently purges trashed notes older than `retention_days` — this
    /// is where the retention policy lives; `database` only compares
    /// timestamps. A negative `retention_days` (handy in tests) puts the
    /// cutoff in the future, making every trashed note eligible immediately.
    pub fn cleanup_deleted_notes(&self, retention_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);

        database::hard_delete_expired_notes(&self.connection, cutoff)
    }

    /// Deletes a note immediately, bypassing the trash — "delete forever",
    /// as opposed to `delete_note`'s everyday soft delete.
    pub fn permanently_delete_note(&self, id: i64) -> Result<bool> {
        database::hard_delete_note(&self.connection, id)
    }

    /// Undoes a soft delete, returning the note to normal (active) reads.
    pub fn restore_note(&self, id: i64) -> Result<bool> {
        database::restore_note(&self.connection, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_store() -> Result<NoteStore> {
        let connection = Connection::open_in_memory()?;
        database::initialize_database(&connection)?;

        Ok(NoteStore::from_connection(connection))
    }

    #[test]
    fn update_returns_updated_note() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        let original = store.get_note(id)?.expect("Note should exist");

        let updated = store
            .update_note(id, "Updated content")?
            .expect("Updated note should exist");

        assert_eq!(updated.id, id);
        assert_eq!(updated.content, "Updated content");

        assert_eq!(updated.created_at, original.created_at);
        assert!(updated.updated_at > original.updated_at);

        Ok(())
    }

    #[test]
    fn get_missing_note_returns_none() -> Result<()> {
        let store = setup_store()?;

        let note = store.get_note(999)?;

        assert_eq!(note, None);

        Ok(())
    }

    #[test]
    fn notes_returns_only_active_notes() -> Result<()> {
        let store = setup_store()?;

        let active_id = store.add_note("Active note")?;
        let deleted_id = store.add_note("Deleted note")?;
        store.delete_note(deleted_id)?;

        let notes = store.notes()?;

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, active_id);

        Ok(())
    }

    #[test]
    fn delete_returns_deleted_note() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        let original = store.get_note(id)?.expect("Note should exist");

        let deleted = store
            .delete_note(id)?
            .expect("Deleted note should be returned");

        assert_eq!(deleted.id, id);
        assert_eq!(deleted.content, "Hello JotR");
        assert_eq!(deleted.created_at, original.created_at);
        assert_eq!(deleted.updated_at, original.updated_at);

        assert_eq!(store.get_note(id)?, None);

        Ok(())
    }

    #[test]
    fn update_missing_note_returns_none() -> Result<()> {
        let store = setup_store()?;

        let note = store.update_note(999, "Updated content")?;

        assert_eq!(note, None);

        Ok(())
    }

    #[test]
    fn delete_missing_note_returns_none() -> Result<()> {
        let store = setup_store()?;

        let note = store.delete_note(999)?;

        assert_eq!(note, None);

        Ok(())
    }

    #[test]
    fn deleted_notes_returns_soft_deleted_notes() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        store.delete_note(id)?;

        let deleted = store.deleted_notes()?;

        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, id);
        assert!(deleted[0].deleted_at.is_some());

        Ok(())
    }

    #[test]
    fn restore_returns_note_to_active_notes() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        store.delete_note(id)?;

        let restored = store.restore_note(id)?;

        assert!(restored);

        let note = store
            .get_note(id)?
            .expect("Restored note should be active again");

        assert_eq!(note.id, id);
        assert_eq!(note.content, "Hello JotR");
        assert_eq!(note.deleted_at, None);

        Ok(())
    }

    #[test]
    fn restoring_missing_note_returns_false() -> Result<()> {
        let store = setup_store()?;

        let restored = store.restore_note(999)?;

        assert!(!restored);

        Ok(())
    }

    #[test]
    fn cleanup_deleted_notes_removes_expired_notes() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        store.delete_note(id)?;

        let deleted = store.cleanup_deleted_notes(-1)?;

        assert_eq!(deleted, 1);
        assert!(store.deleted_notes()?.is_empty());

        Ok(())
    }

    // Regression guard for the retention cutoff's date-math direction: a large
    // retention window should push the cutoff into the past, not the future,
    // so a just-deleted note isn't swept up immediately.
    #[test]
    fn cleanup_deleted_notes_keeps_notes_within_retention() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        store.delete_note(id)?;

        let deleted = store.cleanup_deleted_notes(30)?;

        assert_eq!(deleted, 0);
        assert_eq!(store.deleted_notes()?.len(), 1);

        Ok(())
    }

    #[test]
    fn permanently_delete_note_removes_note() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        store.delete_note(id)?;

        let deleted = store.permanently_delete_note(id)?;

        assert!(deleted);
        assert!(store.deleted_notes()?.is_empty());

        Ok(())
    }

    #[test]
    fn permanently_delete_missing_note_returns_false() -> Result<()> {
        let store = setup_store()?;

        let deleted = store.permanently_delete_note(999)?;

        assert!(!deleted);

        Ok(())
    }
}
