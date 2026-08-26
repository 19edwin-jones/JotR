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
        Self { notes: Vec::new() }
    }
    pub fn add_note(&mut self, content: String) {
        let note = Note {
            id: self.notes.len() as u64,
            content,
        };
        self.notes.push(note);
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
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
}
