use serde::{Deserialize, Serialize};
use std::fmt;

/// Source of a tag assignment.
///
/// Distinguishes between tags explicitly created by users and those inferred by LLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagSource {
    /// Tag explicitly created or assigned by the user.
    User,
    /// Tag inferred by the language model.
    Llm,
}

impl fmt::Display for TagSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Llm => write!(f, "llm"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_source_serializes_to_json_correctly() {
        let user = TagSource::User;
        let llm = TagSource::Llm;

        let user_json = serde_json::to_string(&user).unwrap();
        let llm_json = serde_json::to_string(&llm).unwrap();

        assert_eq!(user_json, r#""user""#);
        assert_eq!(llm_json, r#""llm""#);

        // Test roundtrip
        let user_deserialized: TagSource = serde_json::from_str(&user_json).unwrap();
        let llm_deserialized: TagSource = serde_json::from_str(&llm_json).unwrap();

        assert_eq!(user_deserialized, user);
        assert_eq!(llm_deserialized, llm);
    }

    #[test]
    fn test_tag_source_deserialization_fails_on_unknown_variant() {
        let invalid_json = r#""unknown""#;
        let result: Result<TagSource, _> = serde_json::from_str(invalid_json);

        assert!(result.is_err());
    }

    #[test]
    fn test_tag_source_display_user() {
        let source = TagSource::User;
        assert_eq!(format!("{}", source), "user");
    }

    #[test]
    fn test_tag_source_display_llm() {
        let source = TagSource::Llm;
        assert_eq!(format!("{}", source), "llm");
    }
}
