use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::{TagId, TagSource};

/// Assignment of a tag to a note with AI-first metadata.
///
/// Tracks source (with embedded confidence/model for LLM), verification status,
/// and timestamps for each tag-note relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagAssignment {
    tag_id: TagId,
    source: TagSource,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    verified: bool,
}

impl TagAssignment {
    /// Creates a new LLM-inferred tag assignment.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{TagAssignment, TagId};
    /// use time::OffsetDateTime;
    ///
    /// let now = OffsetDateTime::now_utc();
    /// let assignment = TagAssignment::llm(
    ///     TagId::new(1),
    ///     "deepseek-r1:8b",
    ///     85,
    ///     now,
    /// );
    ///
    /// assert_eq!(assignment.tag_id(), TagId::new(1));
    /// assert_eq!(assignment.confidence(), 85);
    /// assert!(!assignment.verified());
    /// ```
    pub fn llm(
        tag_id: TagId,
        model: impl Into<String>,
        confidence: u8,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            tag_id,
            source: TagSource::llm(model, confidence),
            created_at,
            verified: false,
        }
    }

    /// Creates a user-created tag assignment with 100% confidence.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{TagAssignment, TagId, TagSource};
    /// use time::OffsetDateTime;
    ///
    /// let now = OffsetDateTime::now_utc();
    /// let assignment = TagAssignment::user(TagId::new(42), now);
    ///
    /// assert_eq!(assignment.tag_id(), TagId::new(42));
    /// assert_eq!(assignment.confidence(), 100);
    /// assert!(assignment.source().is_user());
    /// ```
    pub fn user(tag_id: TagId, created_at: OffsetDateTime) -> Self {
        Self {
            tag_id,
            source: TagSource::User,
            created_at,
            verified: false,
        }
    }

    /// Returns the tag ID.
    pub fn tag_id(&self) -> TagId {
        self.tag_id
    }

    /// Returns the source of this tag assignment.
    pub fn source(&self) -> &TagSource {
        &self.source
    }

    /// Returns the confidence score (0-100).
    /// User tags always return 100.
    pub fn confidence(&self) -> u8 {
        self.source.confidence()
    }

    /// Returns the model identifier if this is an LLM-inferred tag.
    pub fn model(&self) -> Option<&str> {
        self.source.model()
    }

    /// Returns when this tag assignment was created.
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Returns whether this assignment has been verified by the user.
    pub fn verified(&self) -> bool {
        self.verified
    }

    /// Marks this assignment as verified by the user.
    pub fn verify(&mut self) {
        self.verified = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_assignment_has_correct_metadata() {
        let now = OffsetDateTime::now_utc();
        let assignment = TagAssignment::llm(TagId::new(1), "deepseek-r1:8b", 85, now);

        assert_eq!(assignment.tag_id(), TagId::new(1));
        assert_eq!(assignment.confidence(), 85);
        assert_eq!(assignment.model(), Some("deepseek-r1:8b"));
        assert!(!assignment.verified());
    }

    #[test]
    fn user_assignment_has_full_confidence() {
        let now = OffsetDateTime::now_utc();
        let assignment = TagAssignment::user(TagId::new(42), now);

        assert_eq!(assignment.tag_id(), TagId::new(42));
        assert_eq!(assignment.confidence(), 100);
        assert_eq!(assignment.model(), None);
        assert!(assignment.source().is_user());
    }

    #[test]
    fn serialization_roundtrip() {
        let now = OffsetDateTime::now_utc();
        let assignment = TagAssignment::llm(TagId::new(1), "model", 75, now);

        let json = serde_json::to_string(&assignment).unwrap();
        let deserialized: TagAssignment = serde_json::from_str(&json).unwrap();

        assert_eq!(assignment, deserialized);
    }

    #[test]
    fn verify_marks_as_verified() {
        let now = OffsetDateTime::now_utc();
        let mut assignment = TagAssignment::llm(TagId::new(1), "model", 60, now);

        assert!(!assignment.verified());
        assignment.verify();
        assert!(assignment.verified());
    }
}
