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
}
