pub mod database;
pub mod note;
pub mod store;

pub use note::Note;
pub use store::NoteStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_add_note() {
        let mut store = NoteStore::new();

        store.add_note("Hello JotR".to_string());

        assert_eq!(store.notes().len(), 1);
        assert_eq!(store.notes()[0].content, "Hello JotR");
    }

    #[test]
    fn can_get_note_by_id() {
        let store = NoteStore::new();

        assert_eq!(store.get_note(999), None);
    }

    #[test]
    fn can_update_note() {
        let mut store = NoteStore::new();

        store.add_note("Hello JotR".to_string());

        assert_eq!(
            store.update_note(0, "Updated content".to_string()),
            Some(&Note {
                id: 0,
                content: "Updated content".to_string()
            })
        );

        assert_eq!(store.notes()[0].content, "Updated content");
    }

    #[test]
    fn can_delete_note() {
        let mut store = NoteStore::new();

        store.add_note("Hello JotR".to_string());

        assert_eq!(
            store.delete_note(0),
            Some(Note {
                id: 0,
                content: "Hello JotR".to_string()
            })
        );

        assert_eq!(store.notes().len(), 0);
    }
}
