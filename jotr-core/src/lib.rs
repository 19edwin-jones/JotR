//! Core note-taking library, split into two layers so callers (CLI, GUI, tests)
//! never need to touch SQL directly:
//!   - `database`: raw SQLite access — one function per statement.
//!   - `store`: the public `NoteStore` API that wraps `database` with the
//!     domain rules (e.g. soft delete, retention cleanup).
//!
//! Only `Note` and `NoteStore` are re-exported: everything else is an
//! implementation detail callers shouldn't depend on directly.

pub mod database;
pub mod note;
pub mod store;

pub use note::Note;
pub use store::NoteStore;
