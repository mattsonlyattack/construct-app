use serde::{Deserialize, Serialize};

use super::TagId;

/// A tag with optional aliases for knowledge organization.
///
/// Tags use a SKOS-inspired vocabulary model where the `name` field represents
/// the preferred label and `aliases` represent alternative labels for the same concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    id: TagId,
    name: String,
    aliases: Vec<String>,
}

impl Tag {
    /// Creates a new tag with empty aliases.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Tag, TagId};
    ///
    /// let tag = Tag::new(TagId::new(1), "rust");
    /// assert_eq!(tag.id(), TagId::new(1));
    /// assert_eq!(tag.name(), "rust");
    /// assert!(tag.aliases().is_empty());
    /// ```
    pub fn new(id: TagId, name: impl Into<String>) -> Self {
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
    /// use cons::{Tag, TagId};
    ///
    /// let tag = Tag::with_aliases(TagId::new(1), "rust", vec!["rust-lang".to_string()]);
    /// assert_eq!(tag.id(), TagId::new(1));
    /// assert_eq!(tag.name(), "rust");
    /// assert_eq!(tag.aliases(), &["rust-lang"]);
    /// ```
    pub fn with_aliases(id: TagId, name: impl Into<String>, aliases: Vec<String>) -> Self {
        Self {
            id,
            name: name.into(),
            aliases,
        }
    }

    /// Returns the tag's unique identifier.
    pub fn id(&self) -> TagId {
        self.id
    }

    /// Returns the preferred label for this tag.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the alternative labels for this tag.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Adds an alias to this tag.
    pub fn add_alias(&mut self, alias: impl Into<String>) {
        self.aliases.push(alias.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_tag_with_empty_aliases() {
        let tag = Tag::new(TagId::new(1), "rust");

        assert_eq!(tag.id(), TagId::new(1));
        assert_eq!(tag.name(), "rust");
        assert!(tag.aliases().is_empty());
    }

    #[test]
    fn with_aliases_creates_tag_with_aliases() {
        let aliases = vec!["rust-lang".to_string(), "rustlang".to_string()];
        let tag = Tag::with_aliases(TagId::new(42), "rust", aliases.clone());

        assert_eq!(tag.id(), TagId::new(42));
        assert_eq!(tag.name(), "rust");
        assert_eq!(tag.aliases(), &aliases);
    }

    #[test]
    fn add_alias_appends_to_list() {
        let mut tag = Tag::new(TagId::new(1), "machine-learning");
        tag.add_alias("ML");
        tag.add_alias("ml");

        assert_eq!(tag.aliases(), &["ML", "ml"]);
    }
}
