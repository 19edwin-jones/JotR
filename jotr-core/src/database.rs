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
            content TEXT NOT NULL
        )
        ",
        [],
    )?;

    Ok(())
}

pub fn add_note(connection: &Connection, content: &str) -> Result<i64> {
    connection.execute(
        "
        INSERT INTO notes (content)
        VALUES (?1)
        ",
        [content],
    )?;

    Ok(connection.last_insert_rowid())
}

pub fn get_note_by_id(connection: &Connection, id: i64) -> Result<Option<Note>> {
    let mut stmt = connection.prepare("SELECT id, content FROM notes WHERE id = ?1")?;

    let note = stmt
        .query_row([id], |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
            })
        })
        .optional()?;

    Ok(note)
}

pub fn get_all_notes(connection: &Connection) -> Result<Vec<Note>> {
    let mut stmt = connection.prepare("SELECT id, content FROM notes")?;
    let note_iter = stmt.query_map([], |row| {
        Ok(Note {
            id: row.get(0)?,
            content: row.get(1)?,
        })
    })?;

    let mut notes = Vec::new();
    for note in note_iter {
        notes.push(note?);
    }

    Ok(notes)
}

pub fn update_note(connection: &Connection, id: i64, new_content: &str) -> Result<bool> {
    let rows_updated = connection.execute(
        "
        UPDATE notes
        SET content = ?1
        WHERE id = ?2
        ",
        params![new_content, id],
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

    #[test] // READ
    fn can_get_note_by_id() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        let content = get_note_by_id(&connection, id)?;

        assert_eq!(
            content,
            Some(Note {
                id,
                content: "Hello JotR".to_string()
            })
        );

        Ok(())
    }

    #[test] // UPDATE
    fn can_update_note() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        let updated = update_note(&connection, id, "Updated content")?;

        assert!(updated);

        let content = get_note_by_id(&connection, id)?;
        assert_eq!(
            content,
            Some(Note {
                id,
                content: "Updated content".to_string()
            })
        );

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
