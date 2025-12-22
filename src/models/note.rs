use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::TagAssignment;

/// A note with its content and tag assignments.
///
/// Notes are the primary unit of knowledge capture in the system. Each note
/// contains freeform text content and zero or more tag assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Unique identifier from the database.
    pub id: i64,
    /// The note's content.
    pub content: String,
    /// When this note was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When this note was last updated.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// Tag assignments for this note.
    pub tags: Vec<TagAssignment>,
}

/// Builder for constructing `Note` instances with optional fields.
///
/// # Examples
///
/// ```
/// use cons::NoteBuilder;
/// use time::OffsetDateTime;
///
/// let note = NoteBuilder::new()
///     .id(1)
///     .content("My first note")
///     .build();
///
/// assert_eq!(note.id, 1);
/// assert_eq!(note.content, "My first note");
/// assert!(note.tags.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct NoteBuilder {
    id: Option<i64>,
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

    /// Sets the note ID.
    pub fn id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the note content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the created timestamp.
    pub fn created_at(mut self, created_at: OffsetDateTime) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Sets the updated timestamp.
    pub fn updated_at(mut self, updated_at: OffsetDateTime) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Sets the tag assignments.
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

    #[test]
    fn test_note_builder_creates_note_with_default_empty_tags() {
        let note = NoteBuilder::new().id(1).content("Test note").build();

        assert_eq!(note.id, 1);
        assert_eq!(note.content, "Test note");
        assert!(note.tags.is_empty());
    }

    #[test]
    fn test_note_builder_allows_setting_all_fields() {
        let now = OffsetDateTime::now_utc();
        let tag_assignment = TagAssignment::user_created(1, now);

        let note = NoteBuilder::new()
            .id(42)
            .content("Complete note")
            .created_at(now)
            .updated_at(now)
            .tags(vec![tag_assignment.clone()])
            .build();

        assert_eq!(note.id, 42);
        assert_eq!(note.content, "Complete note");
        assert_eq!(note.created_at, now);
        assert_eq!(note.updated_at, now);
        assert_eq!(note.tags.len(), 1);
        assert_eq!(note.tags[0], tag_assignment);
    }

    #[test]
    fn test_note_serialization_roundtrip() {
        let now = OffsetDateTime::now_utc();
        let note = NoteBuilder::new()
            .id(1)
            .content("Test content")
            .created_at(now)
            .updated_at(now)
            .build();

        let json = serde_json::to_string(&note).unwrap();
        let deserialized: Note = serde_json::from_str(&json).unwrap();

        assert_eq!(note, deserialized);
    }

    #[test]
    fn test_note_with_multiple_tag_assignments_from_different_sources() {
        use super::super::{TagAssignment, TagSource};

        let now = OffsetDateTime::now_utc();

        // Create tag assignments from different sources
        let user_tag = TagAssignment::user_created(1, now);
        let llm_tag = TagAssignment::new(
            2,
            85,
            TagSource::Llm,
            now,
            Some("deepseek-r1:8b".to_string()),
        );
        let another_llm_tag = TagAssignment::new(
            3,
            92,
            TagSource::Llm,
            now,
            Some("deepseek-r1:8b".to_string()),
        );

        let note = NoteBuilder::new()
            .id(1)
            .content("Note with mixed tag sources")
            .created_at(now)
            .updated_at(now)
            .tags(vec![
                user_tag.clone(),
                llm_tag.clone(),
                another_llm_tag.clone(),
            ])
            .build();

        // Verify note has all three tags
        assert_eq!(note.tags.len(), 3);

        // Verify tag sources are preserved
        assert_eq!(note.tags[0].source, TagSource::User);
        assert_eq!(note.tags[1].source, TagSource::Llm);
        assert_eq!(note.tags[2].source, TagSource::Llm);

        // Verify confidence values
        assert_eq!(note.tags[0].confidence, 100);
        assert_eq!(note.tags[1].confidence, 85);
        assert_eq!(note.tags[2].confidence, 92);

        // Verify model versions
        assert_eq!(note.tags[0].model_version, None);
        assert_eq!(
            note.tags[1].model_version,
            Some("deepseek-r1:8b".to_string())
        );
        assert_eq!(
            note.tags[2].model_version,
            Some("deepseek-r1:8b".to_string())
        );
    }
}
