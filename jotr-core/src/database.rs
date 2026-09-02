//! Raw SQLite access for notes. Every function here maps 1:1 to a statement;
//! domain rules (soft delete, retention policy, etc.) live one layer up in
//! `store`. Keeping that split means the SQL shape can change without the
//! rest of the app knowing.

use chrono::DateTime;
use chrono::Utc;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use rusqlite::params;
use rusqlite::{Connection, Result};

use crate::note::Note;

// Column order must match every query's SELECT list below; centralizing the
// mapping here means a schema change only needs one update, not one per query.
fn note_from_row(row: &Row) -> Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        content: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        deleted_at: row.get(4)?,
    })
}

/// Opens the on-disk database the real app uses; tests use in-memory
/// connections via `NoteStore::from_connection` instead.
pub fn open_database() -> Result<Connection> {
    let connection = Connection::open("jotr.db")?;

    initialize_database(&connection)?;

    Ok(connection)
}

/// Idempotent (`IF NOT EXISTS` everywhere), so it can run on every startup
/// without a separate migration step.
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

    // Partial index: only deleted rows are ever queried by `deleted_at`
    // (trash list, retention sweep), and they should stay a small minority —
    // indexing just that slice avoids bloating the index on every write.
    connection.execute(
        "
        CREATE INDEX IF NOT EXISTS idx_notes_deleted_at
        ON notes (deleted_at)
        WHERE deleted_at IS NOT NULL
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

/// Active-note lookup by id. Soft-deleted notes count as not found; use
/// `get_deleted_notes` to see trashed ones.
pub fn get_note_by_id(connection: &Connection, id: i64) -> Result<Option<Note>> {
    let mut stmt = connection.prepare(
        "
        SELECT id, content, created_at, updated_at, deleted_at
        FROM notes
        WHERE id = ?1
        AND deleted_at IS NULL
        ",
    )?;

    let note = stmt.query_row([id], note_from_row).optional()?;

    Ok(note)
}

/// All active (non-deleted) notes, oldest first.
pub fn get_all_notes(connection: &Connection) -> Result<Vec<Note>> {
    let mut stmt = connection.prepare(
        "
        SELECT id, content, created_at, updated_at, deleted_at
        FROM notes
        WHERE deleted_at IS NULL
        ORDER BY id
        ",
    )?;

    let notes = stmt
        .query_map([], note_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(notes)
}

/// Updates content and `updated_at`. Returns whether a row changed, so
/// callers can tell "updated" apart from "no such note".
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

/// Soft-deletes a note (stamps `deleted_at`) and returns it, or `None` if
/// it doesn't exist or is already deleted.
///
/// Uses `UPDATE ... RETURNING` to combine the write and the read into one
/// round trip. The `deleted_at IS NULL` guard makes repeat calls a no-op
/// instead of overwriting the original deletion timestamp.
pub fn soft_delete_note(connection: &Connection, id: i64) -> Result<Option<Note>> {
    let deleted_time = Utc::now();

    let mut stmt = connection.prepare(
        "
        UPDATE notes
        SET deleted_at = ?1
        WHERE id = ?2
        AND deleted_at IS NULL
        RETURNING id, content, created_at, updated_at, deleted_at
        ",
    )?;

    let note = stmt
        .query_row(params![deleted_time, id], note_from_row)
        .optional()?;

    Ok(note)
}

/// Permanently removes notes soft-deleted before `cutoff` — the retention
/// sweep. Takes a plain timestamp rather than a duration, so the "keep for
/// N days" policy stays in `NoteStore::cleanup_deleted_notes`.
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

/// The trash: soft-deleted notes not yet purged, most recently deleted
/// first (the likeliest "undo" candidate).
pub fn get_deleted_notes(connection: &Connection) -> Result<Vec<Note>> {
    let mut stmt = connection.prepare(
        "
        SELECT id, content, created_at, updated_at, deleted_at
        FROM notes
        WHERE deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        ",
    )?;

    let notes = stmt
        .query_map([], note_from_row)?
        .collect::<Result<Vec<_>>>()?;

    Ok(notes)
}

/// Unconditionally deletes a note — the "delete forever" action. Unlike
/// `hard_delete_expired_notes`, it ignores `deleted_at` entirely.
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

/// Reverses a soft delete. Guarded by `deleted_at IS NOT NULL` so
/// restoring an already-active note is a no-op, not a false success.
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

    #[test] // Startup calls this unconditionally, so it must tolerate an existing schema
    fn initializing_database_twice_is_idempotent() -> Result<()> {
        let connection = setup_db()?;

        initialize_database(&connection)?;

        Ok(())
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

    #[test] // READ: All Notes, none present
    fn get_all_notes_returns_empty_when_none_exist() -> Result<()> {
        let connection = setup_db()?;

        let notes = get_all_notes(&connection)?;

        assert!(notes.is_empty());

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

    #[test] // UPDATE: content can still be edited while a note is in the trash
    fn can_update_soft_deleted_note() -> Result<()> {
        let connection = setup_db()?;
        let id = add_note(&connection, "Hello JotR")?;
        soft_delete_note(&connection, id)?;

        let updated = update_note(&connection, id, "Updated content")?;

        assert!(updated);

        let note = get_deleted_notes(&connection)?
            .into_iter()
            .find(|note| note.id == id)
            .expect("Note should still be in the trash");

        assert_eq!(note.content, "Updated content");

        Ok(())
    }

    #[test] // SOFT DELETE
    fn can_soft_delete_note() -> Result<()> {
        let connection = setup_db()?;
        let id = add_note(&connection, "Hello JotR")?;
        let deleted = soft_delete_note(&connection, id)?;

        assert!(deleted.is_some());

        let note = get_note_by_id(&connection, id)?;

        assert!(note.is_none());

        Ok(())
    }

    #[test] // SOFT DELETE: Already-deleted note is a no-op, not a re-delete
    fn soft_deleting_already_deleted_note_is_noop() -> Result<()> {
        let connection = setup_db()?;
        let id = add_note(&connection, "Hello JotR")?;

        soft_delete_note(&connection, id)?;
        let second_delete = soft_delete_note(&connection, id)?;

        assert!(second_delete.is_none());

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

    #[test] // HARD DELETE: mixed cutoff — only the note deleted before the cutoff is purged
    fn hard_delete_expired_notes_only_removes_notes_past_cutoff() -> Result<()> {
        let connection = setup_db()?;

        let expired_id = add_note(&connection, "Expired")?;
        let recent_id = add_note(&connection, "Recently deleted")?;
        let active_id = add_note(&connection, "Still active")?;

        soft_delete_note(&connection, expired_id)?;
        soft_delete_note(&connection, recent_id)?;

        // Backdate the "expired" note's deletion directly so its timestamp is
        // reliably older than `recent_id`'s, instead of relying on real time
        // passing between the two `soft_delete_note` calls above.
        connection.execute(
            "UPDATE notes SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now() - chrono::Duration::days(10), expired_id],
        )?;

        let cutoff = Utc::now() - chrono::Duration::days(1);
        let rows_deleted = hard_delete_expired_notes(&connection, cutoff)?;

        assert_eq!(rows_deleted, 1);

        // The recently-deleted note is still in the trash...
        let deleted_notes = get_deleted_notes(&connection)?;
        assert_eq!(deleted_notes.len(), 1);
        assert_eq!(deleted_notes[0].id, recent_id);

        // ...and the active note was never touched, regardless of the cutoff.
        assert!(get_note_by_id(&connection, active_id)?.is_some());

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

        assert!(deleted.is_none());

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

    #[test] // HARD DELETE: existing note removed outright, regardless of soft-delete state
    fn can_hard_delete_note() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        let deleted = hard_delete_note(&connection, id)?;

        assert!(deleted);
        assert_eq!(get_note_by_id(&connection, id)?, None);

        Ok(())
    }

    #[test] // HARD DELETE: Missing note that doesn't exist
    fn hard_deleting_missing_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let deleted = hard_delete_note(&connection, 999)?;

        assert!(!deleted);

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

    #[test] // READ: Get deleted notes, none present
    fn get_deleted_notes_returns_empty_when_none_deleted() -> Result<()> {
        let connection = setup_db()?;

        add_note(&connection, "Hello JotR")?;

        let deleted_notes = get_deleted_notes(&connection)?;

        assert!(deleted_notes.is_empty());

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

    #[test] // RESTORE: Note that was never soft-deleted
    fn restoring_active_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let id = add_note(&connection, "Hello JotR")?;

        let restored = restore_note(&connection, id)?;

        assert!(!restored);

        Ok(())
    }

    #[test] // RESTORE: Missing note that doesn't exist
    fn restoring_missing_note_returns_false() -> Result<()> {
        let connection = setup_db()?;

        let restored = restore_note(&connection, 999)?;

        assert!(!restored);

        Ok(())
    }
}
