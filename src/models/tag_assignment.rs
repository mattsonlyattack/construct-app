use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::TagSource;

/// Assignment of a tag to a note with AI-first metadata.
///
/// Tracks confidence, source, verification status, and model version for each
/// tag-note relationship to distinguish LLM-inferred tags from user-created ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagAssignment {
    /// The ID of the tag being assigned.
    pub tag_id: i64,
    /// Confidence score (0-100 percentage).
    pub confidence: u8,
    /// Source of the tag assignment (user or LLM).
    pub source: TagSource,
    /// When this tag assignment was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Whether this assignment has been verified by the user.
    pub verified: bool,
    /// LLM model version that produced this assignment (None for user-created).
    pub model_version: Option<String>,
}

impl TagAssignment {
    /// Creates a new tag assignment with default verified=false.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{TagAssignment, TagSource};
    /// use time::OffsetDateTime;
    ///
    /// let now = OffsetDateTime::now_utc();
    /// let assignment = TagAssignment::new(
    ///     1,
    ///     85,
    ///     TagSource::Llm,
    ///     now,
    ///     Some("deepseek-r1:8b".to_string())
    /// );
    ///
    /// assert_eq!(assignment.tag_id, 1);
    /// assert_eq!(assignment.confidence, 85);
    /// assert_eq!(assignment.source, TagSource::Llm);
    /// assert!(!assignment.verified);
    /// assert_eq!(assignment.model_version, Some("deepseek-r1:8b".to_string()));
    /// ```
    pub fn new(
        tag_id: i64,
        confidence: u8,
        source: TagSource,
        created_at: OffsetDateTime,
        model_version: Option<String>,
    ) -> Self {
        Self {
            tag_id,
            confidence,
            source,
            created_at,
            verified: false,
            model_version,
        }
    }

    /// Creates a user-created tag assignment with confidence=100 and no model version.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{TagAssignment, TagSource};
    /// use time::OffsetDateTime;
    ///
    /// let now = OffsetDateTime::now_utc();
    /// let assignment = TagAssignment::user_created(42, now);
    ///
    /// assert_eq!(assignment.tag_id, 42);
    /// assert_eq!(assignment.confidence, 100);
    /// assert_eq!(assignment.source, TagSource::User);
    /// assert!(!assignment.verified);
    /// assert_eq!(assignment.model_version, None);
    /// ```
    pub fn user_created(tag_id: i64, created_at: OffsetDateTime) -> Self {
        Self {
            tag_id,
            confidence: 100,
            source: TagSource::User,
            created_at,
            verified: false,
            model_version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_assignment_serialization_roundtrip() {
        let now = OffsetDateTime::now_utc();
        let assignment = TagAssignment::new(
            1,
            85,
            TagSource::Llm,
            now,
            Some("deepseek-r1:8b".to_string()),
        );

        let json = serde_json::to_string(&assignment).unwrap();
        let deserialized: TagAssignment = serde_json::from_str(&json).unwrap();

        assert_eq!(assignment, deserialized);
    }

    #[test]
    fn test_tag_assignment_equality_with_same_confidence() {
        let now = OffsetDateTime::now_utc();
        let assignment1 = TagAssignment::new(1, 85, TagSource::Llm, now, Some("model".to_string()));
        let assignment2 = TagAssignment::new(1, 85, TagSource::Llm, now, Some("model".to_string()));

        assert_eq!(assignment1, assignment2);
    }

    #[test]
    fn test_tag_assignment_user_created_constructor() {
        let now = OffsetDateTime::now_utc();
        let assignment = TagAssignment::user_created(42, now);

        assert_eq!(assignment.tag_id, 42);
        assert_eq!(assignment.confidence, 100);
        assert_eq!(assignment.source, TagSource::User);
        assert_eq!(assignment.created_at, now);
        assert!(!assignment.verified);
        assert_eq!(assignment.model_version, None);
    }
}
