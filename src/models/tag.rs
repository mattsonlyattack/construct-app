use serde::{Deserialize, Serialize};

/// A tag with optional aliases for knowledge organization.
///
/// Tags use a SKOS-inspired vocabulary model where the `name` field represents
/// the preferred label and `aliases` represent alternative labels for the same concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier from the database.
    pub id: i64,
    /// Preferred label for this tag (canonical name).
    pub name: String,
    /// Alternative labels for the same concept (SKOS alternate labels).
    pub aliases: Vec<String>,
}

impl Tag {
    /// Creates a new tag with empty aliases.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::Tag;
    ///
    /// let tag = Tag::new(1, "rust");
    /// assert_eq!(tag.id, 1);
    /// assert_eq!(tag.name, "rust");
    /// assert!(tag.aliases.is_empty());
    /// ```
    pub fn new(id: i64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            aliases: Vec::new(),
        }
    }

    /// Creates a new tag with the specified aliases.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::Tag;
    ///
    /// let tag = Tag::with_aliases(1, "rust", vec!["rust-lang".to_string()]);
    /// assert_eq!(tag.id, 1);
    /// assert_eq!(tag.name, "rust");
    /// assert_eq!(tag.aliases, vec!["rust-lang"]);
    /// ```
    pub fn with_aliases(id: i64, name: impl Into<String>, aliases: Vec<String>) -> Self {
        Self {
            id,
            name: name.into(),
            aliases,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new_creates_tag_with_empty_aliases() {
        let tag = Tag::new(1, "rust");

        assert_eq!(tag.id, 1);
        assert_eq!(tag.name, "rust");
        assert!(tag.aliases.is_empty());
    }

    #[test]
    fn test_tag_with_aliases_creates_tag_with_aliases() {
        let aliases = vec!["rust-lang".to_string(), "rustlang".to_string()];
        let tag = Tag::with_aliases(42, "rust", aliases.clone());

        assert_eq!(tag.id, 42);
        assert_eq!(tag.name, "rust");
        assert_eq!(tag.aliases, aliases);
    }
}
