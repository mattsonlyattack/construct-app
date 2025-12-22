use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a note.
///
/// Wraps a database ID to provide type safety and prevent accidental
/// mixing of different ID types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NoteId(i64);

impl NoteId {
    /// Creates a new note ID.
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// Returns the underlying ID value.
    pub fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a tag.
///
/// Wraps a database ID to provide type safety and prevent accidental
/// mixing of different ID types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagId(i64);

impl TagId {
    /// Creates a new tag ID.
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// Returns the underlying ID value.
    pub fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_id_serializes_as_raw_integer() {
        let id = NoteId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        let deserialized: NoteId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn tag_id_serializes_as_raw_integer() {
        let id = TagId::new(99);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "99");

        let deserialized: TagId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn ids_are_not_interchangeable() {
        // This test documents the type safety - these lines would fail to compile:
        // let note_id: NoteId = TagId::new(1); // Error: mismatched types
        // let tag_id: TagId = NoteId::new(1);  // Error: mismatched types

        let note_id = NoteId::new(1);
        let tag_id = TagId::new(1);

        // Same underlying value, but different types
        assert_eq!(note_id.get(), tag_id.get());
    }
}
