#[derive(Debug, PartialEq)]
pub struct Note {
    pub id: u64,
    pub content: String,
}

#[derive(Default)]
pub struct NoteStore {
    notes: Vec<Note>,
}

impl NoteStore {
    pub fn new() -> Self {
        Self::default()
    }

    // read all notes
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn add_note(&mut self, content: String) {
        let note = Note {
            id: self.notes.len() as u64,
            content,
        };
        self.notes.push(note);
    }

    // read a specifc note
    pub fn get_note(&self, id: u64) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn update_note(&mut self, id: u64, new_content: String) -> Option<&Note> {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.content = new_content;
            Some(note)
        } else {
            None
        }
    }

    pub fn delete_note(&mut self, id: u64) -> Option<Note> {
        if let Some(pos) = self.notes.iter().position(|note| note.id == id) {
            Some(self.notes.remove(pos))
        } else {
            None
        }
    }
}

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
