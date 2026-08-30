pub mod database;
pub mod note;
pub mod store;

pub use note::Note;
pub use store::NoteStore;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use rusqlite::{Connection, Result};

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
}
