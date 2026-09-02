use chrono::DateTime;
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
            updated_at TEXT NOT NULL,
            deleted_at TEXT
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
    let mut stmt = connection.prepare(
        "
        SELECT id, content, created_at, updated_at, deleted_at
        FROM notes
        WHERE id = ?1
        AND deleted_at IS NULL
        ",
    )?;

    let note = stmt
        .query_row([id], |row| {
            Ok(Note {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                deleted_at: row.get(4)?,
            })
        })
        .optional()?;

    Ok(note)
}

pub fn get_all_notes(connection: &Connection) -> Result<Vec<Note>> {
    let mut stmt = connection.prepare(
        "
        SELECT id, content, created_at, updated_at, deleted_at
        FROM notes
        WHERE deleted_at IS NULL
        ORDER BY id
        ",
    )?;

    let note_iter = stmt.query_map([], |row| {
        Ok(Note {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
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

pub fn soft_delete_note(connection: &Connection, id: i64) -> Result<bool> {
    let deleted_time = Utc::now();

    let rows_deleted = connection.execute(
        "
        UPDATE notes
        SET deleted_at = ?1
        WHERE id = ?2
        AND deleted_at IS NULL
        ",
        params![deleted_time, id],
    )?;

    Ok(rows_deleted > 0)
}

pub fn hard_delete_expired_notes(connection: &Connection, cutoff: DateTime<Utc>) -> Result<usize> {
    let rows_deleted = connection.execute(
        "
        DELETE FROM notes
        WHERE deleted_at IS NOT NULL
        AND deleted_at < ?1
        ",
        [cutoff],
    )?;

    Ok(rows_deleted)
}

pub fn get_deleted_notes(connection: &Connection) -> Result<Vec<Note>> {
    let mut stmt = connection.prepare(
        "
        SELECT id, content, created_at, updated_at, deleted_at
        FROM notes
        WHERE deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        ",
    )?;

    let note_iter = stmt.query_map([], |row| {
        Ok(Note {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    })?;

    let mut notes = Vec::new();

    for note in note_iter {
        notes.push(note?);
    }

    Ok(notes)
}

pub fn hard_delete_note(connection: &Connection, id: i64) -> Result<bool> {
    let rows_deleted = connection.execute(
        "
        DELETE FROM notes
        WHERE id = ?1
        ",
        [id],
    )?;

    Ok(rows_deleted > 0)
}

pub fn restore_note(connection: &Connection, id: i64) -> Result<bool> {
    let rows_restored = connection.execute(
        "
        UPDATE notes
        SET deleted_at = NULL
        WHERE id = ?1
        AND deleted_at IS NOT NULL
        ",
        [id],
    )?;

    Ok(rows_restored > 0)
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
    fn get_all_notes_excludes_soft_deleted_notes() -> Result<()> {
        let connection = setup_db()?;

        let id1 = add_note(&connection, "First note")?;
        let id2 = add_note(&connection, "Second note")?;

        soft_delete_note(&connection, id1)?;

        let notes = get_all_notes(&connection)?;

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, id2);
        assert_eq!(notes[0].content, "Second note");

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

    #[test] // SOFT DELETE
    fn can_soft_delete_note() -> Result<()> {
        let connection = setup_db()?;
        let id = add_note(&connection, "Hello JotR")?;
        let deleted = soft_delete_note(&connection, id)?;

        assert!(deleted);

        let note = get_note_by_id(&connection, id)?;

        assert!(note.is_none());

        Ok(())
    }

    #[test] // HARD DELETE
    fn can_hard_delete_expired_notes() -> Result<()> {
        let connection = setup_db()?;

        let id1 = add_note(&connection, "First note")?;
        let id2 = add_note(&connection, "Second note")?;

        soft_delete_note(&connection, id1)?;
        soft_delete_note(&connection, id2)?;

        let cutoff = Utc::now() + chrono::Duration::seconds(1);

        let rows_deleted = hard_delete_expired_notes(&connection, cutoff)?;

        assert_eq!(rows_deleted, 2);

        let remaining: i64 = connection.query_row(
            "SELECT COUNT(*) FROM notes WHERE id IN (?1, ?2)",
            params![id1, id2],
            |row| row.get(0),
        )?;

        assert_eq!(remaining, 0);

        Ok(())
    }

    #[test] // READ: Missing note that doesn't exist
    fn getting_missing_note_returns_none() -> Result<()> {
        let connection = setup_db()?;

        let note = get_note_by_id(&connection, 999)?;

        assert_eq!(note, None);

        Ok(())
    }

    #[test] // UPDATE: Missing note that doesn't exist
    fn updating_missing_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let updated = update_note(&connection, 999, "Updated content")?;

        assert!(!updated);

        Ok(())
    }

    #[test] // SOFT DELETE: Missing note that doesn't exist
    fn soft_deleting_missing_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let deleted = soft_delete_note(&connection, 999)?;

        assert!(!deleted);

        Ok(())
    }

    #[test] // HARD DELETE: No notes to delete
    fn hard_deleting_with_no_expired_notes_returns_zero() -> Result<()> {
        let connection = setup_db()?;

        let cutoff = Utc::now();

        let rows_deleted = hard_delete_expired_notes(&connection, cutoff)?;

        assert_eq!(rows_deleted, 0);

        Ok(())
    }

    #[test] // READ: Get deleted notes
    fn can_get_deleted_notes() -> Result<()> {
        let connection = setup_db()?;

        let id1 = add_note(&connection, "First note")?;
        let id2 = add_note(&connection, "Second note")?;

        soft_delete_note(&connection, id1)?;
        soft_delete_note(&connection, id2)?;

        let deleted_notes = get_deleted_notes(&connection)?;

        assert_eq!(deleted_notes.len(), 2);
        assert_eq!(deleted_notes[0].id, id2);
        assert_eq!(deleted_notes[1].id, id1);

        Ok(())
    }

    #[test]
    fn can_restore_note() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;
        soft_delete_note(&connection, id)?;

        let restored = restore_note(&connection, id)?;

        assert!(restored);

        let note = get_note_by_id(&connection, id)?.expect("Note should exist");

        assert_eq!(note.id, id);
        assert_eq!(note.content, "Hello JotR");
        assert!(note.deleted_at.is_none());

        Ok(())
    }
}
