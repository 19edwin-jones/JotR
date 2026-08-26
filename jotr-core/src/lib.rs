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
    pub fn add_note(&mut self, content: String) {
        let note = Note {
            id: self.notes.len() as u64,
            content,
        };
        self.notes.push(note);
    }

    // read all notes
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    // read a specifc note
    pub fn get_note(&self, id: u64) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
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
        let  store = NoteStore::new();

        assert_eq!(store.get_note(999), None);
    }
}
