/// Post-processing layer for tag normalization.
///
/// Ensures consistent tag formatting regardless of LLM output quality.
/// All tags are normalized to lowercase, kebab-case format with only
/// alphanumeric characters and hyphens.
pub struct TagNormalizer;

impl TagNormalizer {
    /// Normalizes a single tag to lowercase kebab-case format.
    ///
    /// # Normalization rules
    ///
    /// - Converts to lowercase
    /// - Replaces spaces with hyphens
    /// - Removes all characters except alphanumeric and hyphens
    /// - Trims leading/trailing whitespace and hyphens
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::autotagger::TagNormalizer;
    ///
    /// assert_eq!(TagNormalizer::normalize_tag("RUST"), "rust");
    /// assert_eq!(TagNormalizer::normalize_tag("machine learning"), "machine-learning");
    /// assert_eq!(TagNormalizer::normalize_tag("C++"), "c");
    /// assert_eq!(TagNormalizer::normalize_tag("rust!"), "rust");
    /// assert_eq!(TagNormalizer::normalize_tag("  --rust--  "), "rust");
    /// assert_eq!(TagNormalizer::normalize_tag("Machine Learning!"), "machine-learning");
    /// ```
    #[must_use]
    pub fn normalize_tag(tag: &str) -> String {
        let normalized = tag
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        // Collapse consecutive hyphens into a single hyphen
        let collapsed = normalized
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        collapsed
            .trim_matches(|c: char| c.is_whitespace() || c == '-')
            .to_string()
    }

    /// Normalizes a collection of tags, removing duplicates and empty strings.
    ///
    /// # Normalization rules
    ///
    /// - Applies `normalize_tag` to each tag
    /// - Deduplicates case-insensitively (keeps first occurrence)
    /// - Filters out empty strings after normalization
    /// - Preserves order of first occurrence
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::autotagger::TagNormalizer;
    ///
    /// let tags = vec!["Rust".to_string(), "rust".to_string(), "RUST".to_string()];
    /// assert_eq!(TagNormalizer::normalize_tags(tags), vec!["rust"]);
    ///
    /// let tags = vec!["Machine Learning".to_string(), "AI".to_string(), "   ".to_string()];
    /// assert_eq!(TagNormalizer::normalize_tags(tags), vec!["machine-learning", "ai"]);
    /// ```
    #[must_use]
    pub fn normalize_tags(tags: Vec<String>) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        tags.into_iter()
            .map(|tag| Self::normalize_tag(&tag))
            .filter(|tag| !tag.is_empty() && seen.insert(tag.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowercase_conversion() {
        assert_eq!(TagNormalizer::normalize_tag("RUST"), "rust");
        assert_eq!(TagNormalizer::normalize_tag("RuSt"), "rust");
        assert_eq!(TagNormalizer::normalize_tag("rust"), "rust");
    }

    #[test]
    fn test_space_to_hyphen_replacement() {
        assert_eq!(
            TagNormalizer::normalize_tag("machine learning"),
            "machine-learning"
        );
        assert_eq!(
            TagNormalizer::normalize_tag("deep neural networks"),
            "deep-neural-networks"
        );
        assert_eq!(
            TagNormalizer::normalize_tag("web development"),
            "web-development"
        );
    }

    #[test]
    fn test_special_character_removal() {
        assert_eq!(TagNormalizer::normalize_tag("c++"), "c");
        assert_eq!(TagNormalizer::normalize_tag("rust!"), "rust");
        assert_eq!(TagNormalizer::normalize_tag("c#"), "c");
        assert_eq!(TagNormalizer::normalize_tag("node.js"), "nodejs");
        assert_eq!(TagNormalizer::normalize_tag("@mentions"), "mentions");
    }

    #[test]
    fn test_deduplication_case_insensitive() {
        let tags = vec!["Rust".to_string(), "rust".to_string(), "RUST".to_string()];
        assert_eq!(TagNormalizer::normalize_tags(tags), vec!["rust"]);

        let tags = vec![
            "Machine Learning".to_string(),
            "machine-learning".to_string(),
            "MACHINE-LEARNING".to_string(),
        ];
        assert_eq!(
            TagNormalizer::normalize_tags(tags),
            vec!["machine-learning"]
        );
    }

    #[test]
    fn test_trimming_whitespace_and_hyphens() {
        assert_eq!(TagNormalizer::normalize_tag("  rust  "), "rust");
        assert_eq!(TagNormalizer::normalize_tag("--rust--"), "rust");
        assert_eq!(TagNormalizer::normalize_tag("  --rust--  "), "rust");
        assert_eq!(TagNormalizer::normalize_tag("-web-"), "web");
    }

    #[test]
    fn test_combined_normalization() {
        assert_eq!(
            TagNormalizer::normalize_tag("Machine Learning!"),
            "machine-learning"
        );
        assert_eq!(
            TagNormalizer::normalize_tag("  C++ Programming  "),
            "c-programming"
        );
        assert_eq!(
            TagNormalizer::normalize_tag("Node.js & Express"),
            "nodejs-express"
        );
        assert_eq!(TagNormalizer::normalize_tag("--WEB 2.0--"), "web-20");
    }

    #[test]
    fn test_empty_strings_filtered() {
        let tags = vec![
            "rust".to_string(),
            "   ".to_string(),
            "ai".to_string(),
            "".to_string(),
            "---".to_string(),
        ];
        assert_eq!(TagNormalizer::normalize_tags(tags), vec!["rust", "ai"]);
    }

    #[test]
    fn test_preserve_order_of_first_occurrence() {
        let tags = vec![
            "Rust".to_string(),
            "AI".to_string(),
            "rust".to_string(),
            "Web".to_string(),
        ];
        assert_eq!(
            TagNormalizer::normalize_tags(tags),
            vec!["rust", "ai", "web"]
        );
    }
}
