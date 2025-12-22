use serde::{Deserialize, Serialize};
use std::fmt;

/// Source of a tag assignment.
///
/// Distinguishes between tags explicitly created by users and those inferred by LLM.
/// The `Llm` variant carries provenance metadata (model and confidence) intrinsic
/// to LLM-inferred tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
pub enum TagSource {
    /// Tag explicitly created or assigned by the user.
    /// User tags are implicitly 100% confidence with no model.
    User,
    /// Tag inferred by the language model.
    Llm {
        /// The model identifier that produced this tag (e.g., "deepseek-r1:8b").
        model: String,
        /// Confidence score (0-100 percentage).
        confidence: u8,
    },
}

impl TagSource {
    /// Creates an LLM tag source with the given model and confidence.
    pub fn llm(model: impl Into<String>, confidence: u8) -> Self {
        Self::Llm {
            model: model.into(),
            confidence,
        }
    }

    /// Returns the confidence score for this tag source.
    /// User tags always return 100.
    pub fn confidence(&self) -> u8 {
        match self {
            Self::User => 100,
            Self::Llm { confidence, .. } => *confidence,
        }
    }

    /// Returns the model identifier if this is an LLM-inferred tag.
    pub fn model(&self) -> Option<&str> {
        match self {
            Self::User => None,
            Self::Llm { model, .. } => Some(model),
        }
    }

    /// Returns true if this tag was created by a user.
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User)
    }

    /// Returns true if this tag was inferred by an LLM.
    pub fn is_llm(&self) -> bool {
        matches!(self, Self::Llm { .. })
    }
}

impl fmt::Display for TagSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Llm { model, confidence } => {
                write!(f, "llm({model}, {confidence}%)")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_serializes_correctly() {
        let source = TagSource::User;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, r#"{"type":"user"}"#);

        let deserialized: TagSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, source);
    }

    #[test]
    fn llm_serializes_with_metadata() {
        let source = TagSource::llm("deepseek-r1:8b", 85);
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(
            json,
            r#"{"type":"llm","model":"deepseek-r1:8b","confidence":85}"#
        );

        let deserialized: TagSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, source);
    }

    #[test]
    fn deserialization_fails_on_unknown_variant() {
        let invalid_json = r#"{"type":"unknown"}"#;
        let result: Result<TagSource, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn confidence_returns_correct_values() {
        assert_eq!(TagSource::User.confidence(), 100);
        assert_eq!(TagSource::llm("model", 75).confidence(), 75);
    }

    #[test]
    fn model_returns_correct_values() {
        assert_eq!(TagSource::User.model(), None);
        assert_eq!(
            TagSource::llm("deepseek-r1:8b", 85).model(),
            Some("deepseek-r1:8b")
        );
    }

    #[test]
    fn display_formats_correctly() {
        assert_eq!(format!("{}", TagSource::User), "user");
        assert_eq!(
            format!("{}", TagSource::llm("gpt-4", 92)),
            "llm(gpt-4, 92%)"
        );
    }
}
