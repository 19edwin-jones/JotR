use chrono::Utc;
use rusqlite::OptionalExtension;
use rusqlite::params;
use rusqlite::{Connection, Result};

use crate::note::Note;

pub fn open_database() -> Result<Connection> {
    let connection = Connection::open("jotr.db")?;

    initialize_database(&connection)?;

    Ok(connection)
}

pub fn initialize_database(connection: &Connection) -> Result<()> {
    connection.execute(
        "
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        ",
        [],
    )?;

    Ok(())
}

pub fn add_note(connection: &Connection, content: &str) -> Result<i64> {
    let current_time = Utc::now();
    let updated_time = current_time; // New note, so updated time is the same as created time
    connection.execute(
        "
        INSERT INTO notes (content, created_at, updated_at)
        VALUES (?1, ?2, ?3)
        ",
        params![content, current_time, updated_time],
    )?;

    Ok(connection.last_insert_rowid())
}

pub fn get_note_by_id(connection: &Connection, id: i64) -> Result<Option<Note>> {
    let mut stmt = connection
        .prepare("SELECT id, content, created_at, updated_at FROM notes WHERE id = ?1")?;

    let note = stmt
        .query_row([id], |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .optional()?;

    Ok(note)
}

pub fn get_all_notes(connection: &Connection) -> Result<Vec<Note>> {
    let mut stmt =
        connection.prepare("SELECT id, content, created_at, updated_at FROM notes ORDER BY id")?;
    let note_iter = stmt.query_map([], |row| {
        Ok(Note {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;

    let mut notes = Vec::new();
    for note in note_iter {
        notes.push(note?);
    }

    Ok(notes)
}

pub fn update_note(connection: &Connection, id: i64, new_content: &str) -> Result<bool> {
    let updated_time = Utc::now();

    let rows_updated = connection.execute(
        "
        UPDATE notes
        SET content = ?1,
            updated_at = ?2
        WHERE id = ?3
        ",
        params![new_content, updated_time, id],
    )?;

    Ok(rows_updated > 0)
}

pub fn delete_note(connection: &Connection, id: i64) -> Result<bool> {
    let rows_deleted = connection.execute(
        "
        DELETE FROM notes
        WHERE id = ?1
        ",
        [id],
    )?;

    Ok(rows_deleted > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Result<Connection> {
        let connection = Connection::open_in_memory()?;

        initialize_database(&connection)?;

        Ok(connection)
    }

    #[test] // CREATE
    fn can_add_note() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        assert_eq!(id, 1);

        Ok(())
    }

    #[test] // READ: Single Note
    fn can_get_note_by_id() -> Result<()> {
        let connection = setup_db()?;

        let before = Utc::now();

        let id = add_note(&connection, "Hello JotR")?;

        let after = Utc::now();

        let note = get_note_by_id(&connection, id)?.expect("Note should exist");

        assert_eq!(note.id, id);
        assert_eq!(note.content, "Hello JotR");

        // Check that creation time (add_note call) is between `before` and `after` time
        assert!(note.created_at >= before);
        assert!(note.created_at <= after);

        // Note was only created, therefore no updates made
        assert!(note.updated_at == note.created_at);

        Ok(())
    }

    #[test] // READ: All Notes
    fn can_get_all_notes() -> Result<()> {
        let connection = setup_db()?;

        let id1 = add_note(&connection, "First note")?;
        let id2 = add_note(&connection, "Second note")?;

        let notes = get_all_notes(&connection)?;

        assert_eq!(notes.len(), 2);

        assert_eq!(notes[0].id, id1);
        assert_eq!(notes[0].content, "First note");

        assert_eq!(notes[1].id, id2);
        assert_eq!(notes[1].content, "Second note");

        assert!(notes[0].created_at == notes[0].updated_at);
        assert!(notes[1].created_at == notes[1].updated_at);

        Ok(())
    }

    #[test] // UPDATE
    fn can_update_note() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        let updated = update_note(&connection, id, "Updated content")?;

        assert!(updated);

        let note = get_note_by_id(&connection, id)?.expect("Note should exist");

        assert_eq!(note.content, "Updated content");

        Ok(())
    }

    #[test] // DELETE
    fn can_delete_note() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        let deleted = delete_note(&connection, id)?;

        assert!(deleted);

        let content = get_note_by_id(&connection, id)?;
        assert_eq!(content, None);

        Ok(())
    }

    #[test] // READ: Missing Note
    fn missing_note_returns_none() -> Result<()> {
        let connection = setup_db()?;

        let note = get_note_by_id(&connection, 999)?;

        assert_eq!(note, None);

        Ok(())
    }

    #[test] // UPDATE: Missing Note
    fn updating_missing_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let updated = update_note(&connection, 999, "Updated content")?;

        assert!(!updated);

        Ok(())
    }

    #[test] // DELETE: Missing Note
    fn deleting_missing_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let deleted = delete_note(&connection, 999)?;

        assert!(!deleted);

        Ok(())
    }
}
