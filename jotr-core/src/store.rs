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
            database::delete_note(&self.connection, id)?;
        }

        Ok(note)
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
        let note = store.update_note(id, "Updated content")?;

        assert_eq!(
            note,
            Some(Note {
                id,
                content: "Updated content".to_string(),
            })
        );

        Ok(())
    }

    #[test]
    fn delete_returns_deleted_note() -> Result<()> {
        let store = setup_store()?;

        let id = store.add_note("Hello JotR")?;
        let note = store.delete_note(id)?;

        assert_eq!(
            note,
            Some(Note {
                id,
                content: "Hello JotR".to_string(),
            })
        );

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
