use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::{NoteId, TagAssignment};

/// A note with its content and tag assignments.
///
/// Notes are the primary unit of knowledge capture in the system. Each note
/// contains freeform text content and zero or more tag assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    id: NoteId,
    content: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
    tags: Vec<TagAssignment>,
}

impl Note {
    /// Returns the note's unique identifier.
    pub fn id(&self) -> NoteId {
        self.id
    }

    /// Returns the note's content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns when this note was created.
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Returns when this note was last updated.
    pub fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }

    /// Returns the tag assignments for this note.
    pub fn tags(&self) -> &[TagAssignment] {
        &self.tags
    }

    /// Returns a mutable reference to the tag assignments.
    pub fn tags_mut(&mut self) -> &mut Vec<TagAssignment> {
        &mut self.tags
    }

    /// Adds a tag assignment to this note.
    pub fn add_tag(&mut self, tag: TagAssignment) {
        self.tags.push(tag);
    }
}

/// Builder for constructing `Note` instances.
///
/// # Examples
///
/// ```
/// use cons::{NoteBuilder, NoteId};
///
/// let note = NoteBuilder::new()
///     .id(NoteId::new(1))
///     .content("My first note")
///     .build();
///
/// assert_eq!(note.id(), NoteId::new(1));
/// assert_eq!(note.content(), "My first note");
/// assert!(note.tags().is_empty());
/// ```
#[derive(Debug, Default)]
pub struct NoteBuilder {
    id: Option<NoteId>,
    content: Option<String>,
    created_at: Option<OffsetDateTime>,
    updated_at: Option<OffsetDateTime>,
    tags: Option<Vec<TagAssignment>>,
}

impl NoteBuilder {
    /// Creates a new `NoteBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the note ID (required).
    pub fn id(mut self, id: NoteId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the note content (required).
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the created timestamp (defaults to now).
    pub fn created_at(mut self, created_at: OffsetDateTime) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Sets the updated timestamp (defaults to now).
    pub fn updated_at(mut self, updated_at: OffsetDateTime) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Sets the tag assignments (defaults to empty).
    pub fn tags(mut self, tags: Vec<TagAssignment>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Builds the `Note`, using defaults for optional fields.
    ///
    /// # Panics
    ///
    /// Panics if `id` or `content` have not been set.
    pub fn build(self) -> Note {
        let now = OffsetDateTime::now_utc();
        Note {
            id: self.id.expect("id is required"),
            content: self.content.expect("content is required"),
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            tags: self.tags.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TagId;

    #[test]
    fn builder_creates_note_with_defaults() {
        let note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content("Test note")
            .build();

        assert_eq!(note.id(), NoteId::new(1));
        assert_eq!(note.content(), "Test note");
        assert!(note.tags().is_empty());
    }

    #[test]
    fn builder_allows_setting_all_fields() {
        let now = OffsetDateTime::now_utc();
        let tag = TagAssignment::user(TagId::new(1), now);

        let note = NoteBuilder::new()
            .id(NoteId::new(42))
            .content("Complete note")
            .created_at(now)
            .updated_at(now)
            .tags(vec![tag.clone()])
            .build();

        assert_eq!(note.id(), NoteId::new(42));
        assert_eq!(note.content(), "Complete note");
        assert_eq!(note.created_at(), now);
        assert_eq!(note.updated_at(), now);
        assert_eq!(note.tags().len(), 1);
    }

    #[test]
    fn serialization_roundtrip() {
        let now = OffsetDateTime::now_utc();
        let note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content("Test content")
            .created_at(now)
            .updated_at(now)
            .build();

        let json = serde_json::to_string(&note).unwrap();
        let deserialized: Note = serde_json::from_str(&json).unwrap();

        assert_eq!(note, deserialized);
    }

    #[test]
    fn note_with_mixed_tag_sources() {
        let now = OffsetDateTime::now_utc();

        let user_tag = TagAssignment::user(TagId::new(1), now);
        let llm_tag = TagAssignment::llm(TagId::new(2), "deepseek-r1:8b", 85, now);

        let note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content("Note with mixed sources")
            .tags(vec![user_tag, llm_tag])
            .build();

        assert_eq!(note.tags().len(), 2);
        assert!(note.tags()[0].source().is_user());
        assert!(note.tags()[1].source().is_llm());
        assert_eq!(note.tags()[0].confidence(), 100);
        assert_eq!(note.tags()[1].confidence(), 85);
    }

    #[test]
    fn add_tag_appends_to_list() {
        let now = OffsetDateTime::now_utc();
        let mut note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content("Test")
            .build();

        note.add_tag(TagAssignment::user(TagId::new(1), now));
        note.add_tag(TagAssignment::llm(TagId::new(2), "model", 75, now));

        assert_eq!(note.tags().len(), 2);
    }
}
