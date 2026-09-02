use chrono::Utc;
use rusqlite::{Connection, Result};

use crate::database;
use crate::note::Note;

pub struct NoteStore {
    connection: Connection,
}

impl NoteStore {
    pub fn new() -> Result<Self> {
        let connection = database::open_database()?;

        Ok(Self { connection })
    }

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

    pub fn update_note(&self, id: i64, content: &str) -> Result<Option<Note>> {
        let updated = database::update_note(&self.connection, id, content)?;

        if updated { self.get_note(id) } else { Ok(None) }
    }

    pub fn deleted_notes(&self) -> Result<Vec<Note>> {
        database::get_deleted_notes(&self.connection)
    }

    pub fn delete_note(&self, id: i64) -> Result<Option<Note>> {
        let note = self.get_note(id)?;

        if note.is_some() {
            database::soft_delete_note(&self.connection, id)?;
        }

        Ok(note)
    }

    pub fn cleanup_deleted_notes(&self, retention_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);

        database::hard_delete_expired_notes(&self.connection, cutoff)
    }

    pub fn permanently_delete_note(&self, id: i64) -> Result<bool> {
        database::hard_delete_note(&self.connection, id)
    }

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
        assert!(updated.updated_at >= original.updated_at);

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
    fn cleanup_deleted_notes_removes_expired_notes() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        store.delete_note(id)?;

        let deleted = store.cleanup_deleted_notes(-1)?;

        assert_eq!(deleted, 1);
        assert!(store.deleted_notes()?.is_empty());

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
}
