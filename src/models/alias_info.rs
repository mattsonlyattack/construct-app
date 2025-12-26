use std::fmt;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::TagId;

/// Information about a tag alias mapping.
///
/// Captures the alias text, its canonical tag target, provenance metadata
/// (source, confidence, model version), and creation timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AliasInfo {
    alias: String,
    canonical_tag_id: TagId,
    source: String,
    confidence: f64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    model_version: Option<String>,
}

impl AliasInfo {
    /// Creates a new AliasInfo.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{AliasInfo, TagId};
    /// use time::OffsetDateTime;
    ///
    /// let now = OffsetDateTime::now_utc();
    /// let alias = AliasInfo::new(
    ///     "ml",
    ///     TagId::new(42),
    ///     "llm",
    ///     0.85,
    ///     now,
    ///     Some("deepseek-r1:8b".to_string()),
    /// );
    ///
    /// assert_eq!(alias.alias(), "ml");
    /// assert_eq!(alias.canonical_tag_id(), TagId::new(42));
    /// assert_eq!(alias.source(), "llm");
    /// assert_eq!(alias.confidence(), 0.85);
    /// ```
    pub fn new(
        alias: impl Into<String>,
        canonical_tag_id: TagId,
        source: impl Into<String>,
        confidence: f64,
        created_at: OffsetDateTime,
        model_version: Option<String>,
    ) -> Self {
        Self {
            alias: alias.into(),
            canonical_tag_id,
            source: source.into(),
            confidence,
            created_at,
            model_version,
        }
    }

    /// Returns the alias text.
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Returns the canonical tag ID this alias resolves to.
    pub fn canonical_tag_id(&self) -> TagId {
        self.canonical_tag_id
    }

    /// Returns the source of this alias ('user' or 'llm').
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the confidence score (0.0-1.0).
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Returns when this alias was created.
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Returns the model version if this is an LLM-created alias.
    pub fn model_version(&self) -> Option<&str> {
        self.model_version.as_deref()
    }
}

impl fmt::Display for AliasInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} -> {} (source: {}, confidence: {:.0}%)",
            self.alias,
            self.canonical_tag_id.get(),
            self.source,
            self.confidence * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_alias_info() {
        let now = OffsetDateTime::now_utc();
        let alias = AliasInfo::new(
            "ml",
            TagId::new(42),
            "llm",
            0.85,
            now,
            Some("deepseek-r1:8b".to_string()),
        );

        assert_eq!(alias.alias(), "ml");
        assert_eq!(alias.canonical_tag_id(), TagId::new(42));
        assert_eq!(alias.source(), "llm");
        assert_eq!(alias.confidence(), 0.85);
        assert_eq!(alias.created_at(), now);
        assert_eq!(alias.model_version(), Some("deepseek-r1:8b"));
    }

    #[test]
    fn user_alias_has_no_model_version() {
        let now = OffsetDateTime::now_utc();
        let alias = AliasInfo::new("machine-learning", TagId::new(42), "user", 1.0, now, None);

        assert_eq!(alias.source(), "user");
        assert_eq!(alias.model_version(), None);
    }

    #[test]
    fn display_formats_correctly() {
        let now = OffsetDateTime::now_utc();
        let alias = AliasInfo::new(
            "ml",
            TagId::new(42),
            "llm",
            0.85,
            now,
            Some("model".to_string()),
        );

        let display = format!("{}", alias);
        assert!(display.contains("ml -> 42"));
        assert!(display.contains("source: llm"));
        assert!(display.contains("confidence: 85%"));
    }

    #[test]
    fn serialization_roundtrip() {
        let now = OffsetDateTime::now_utc();
        let alias = AliasInfo::new(
            "ml",
            TagId::new(42),
            "llm",
            0.85,
            now,
            Some("deepseek-r1:8b".to_string()),
        );

        let json = serde_json::to_string(&alias).unwrap();
        let deserialized: AliasInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(alias, deserialized);
    }
}
