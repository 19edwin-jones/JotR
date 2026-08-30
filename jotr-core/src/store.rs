use crate::note::Note;

#[derive(Default)]
pub struct NoteStore {
    notes: Vec<Note>,
}

impl NoteStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn add_note(&mut self, content: String) {
        let note = Note {
            id: self.notes.len() as i64,
            content,
        };

        self.notes.push(note);
    }

    pub fn get_note(&self, id: i64) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn update_note(&mut self, id: i64, new_content: String) -> Option<&Note> {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.content = new_content;
            Some(note)
        } else {
            None
        }
    }

    pub fn delete_note(&mut self, id: i64) -> Option<Note> {
        if let Some(pos) = self.notes.iter().position(|note| note.id == id) {
            Some(self.notes.remove(pos))
        } else {
            None
        }
    }
}
