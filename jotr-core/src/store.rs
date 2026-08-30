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
