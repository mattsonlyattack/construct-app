//! Hierarchy suggester for identifying tag relationships using LLMs.
//!
//! This module provides the `HierarchySuggester` struct which uses an Ollama-compatible
//! LLM to analyze existing tags and suggest broader/narrower relationships with
//! XKOS-compliant hierarchy types (generic vs partitive).

use std::sync::Arc;

use crate::ollama::{OllamaClientTrait, OllamaError};

/// Prompt template for tag relationship extraction.
///
/// Designed for model-agnostic compatibility with clear, explicit instructions.
/// Includes XKOS semantics explanation and few-shot examples demonstrating both
/// generic (is-a) and partitive (part-of) hierarchy types.
const PROMPT_TEMPLATE: &str = r#"Analyze the following tags and identify hierarchical relationships between them. Return ONLY a JSON array of relationship objects. Do not include any explanatory text.

XKOS HIERARCHY TYPES:

1. GENERIC (is-a): Specialization relationships where the narrower concept is a type of the broader concept
   - Example: "transformer" is a type of "neural-network"
   - Use hierarchy_type: "generic"

2. PARTITIVE (part-of): Compositional relationships where the narrower concept is a component of the broader concept
   - Example: "attention" is a component of "transformer"
   - Use hierarchy_type: "partitive"

INSTRUCTIONS:
1. Identify pairs of tags with clear hierarchical relationships
2. For each relationship, specify:
   - source_tag: the narrower/more specific concept (child)
   - target_tag: the broader/more general concept (parent)
   - hierarchy_type: either "generic" or "partitive"
   - confidence: score from 0.0 to 1.0 based on how clear the relationship is
3. Only include relationships where you are confident (>= 0.7)
4. Use exact tag names from the input list
5. Edges point "up" the hierarchy (from specific to general)

EXAMPLES:

Input: ["transformer", "neural-network", "machine-learning", "attention", "deep-learning"]
Output: [
  {"source_tag": "transformer", "target_tag": "neural-network", "hierarchy_type": "generic", "confidence": 0.95},
  {"source_tag": "neural-network", "target_tag": "machine-learning", "hierarchy_type": "generic", "confidence": 0.9},
  {"source_tag": "attention", "target_tag": "transformer", "hierarchy_type": "partitive", "confidence": 0.85},
  {"source_tag": "deep-learning", "target_tag": "machine-learning", "hierarchy_type": "generic", "confidence": 0.9}
]

Input: ["python", "django", "flask", "web-framework", "programming-language"]
Output: [
  {"source_tag": "django", "target_tag": "web-framework", "hierarchy_type": "generic", "confidence": 0.95},
  {"source_tag": "flask", "target_tag": "web-framework", "hierarchy_type": "generic", "confidence": 0.95},
  {"source_tag": "python", "target_tag": "programming-language", "hierarchy_type": "generic", "confidence": 0.95}
]

TAGS TO ANALYZE:
{tags}

JSON OUTPUT:"#;

/// Represents a suggested hierarchical relationship between two tags.
///
/// # Fields
///
/// - `source_tag`: The narrower/more specific concept (child in the hierarchy)
/// - `target_tag`: The broader/more general concept (parent in the hierarchy)
/// - `hierarchy_type`: Either "generic" (is-a) or "partitive" (part-of)
/// - `confidence`: Confidence score from 0.0 to 1.0
///
/// # Edge Direction Convention
///
/// Edges point "up" the hierarchy:
/// - `source_tag` = narrower/child concept (more specific)
/// - `target_tag` = broader/parent concept (more general)
///
/// # Examples
///
/// ```
/// use cons::hierarchy::RelationshipSuggestion;
///
/// // Generic (is-a) relationship
/// let suggestion = RelationshipSuggestion {
///     source_tag: "transformer".to_string(),
///     target_tag: "neural-network".to_string(),
///     hierarchy_type: "generic".to_string(),
///     confidence: 0.95,
/// };
///
/// // Partitive (part-of) relationship
/// let suggestion = RelationshipSuggestion {
///     source_tag: "attention".to_string(),
///     target_tag: "transformer".to_string(),
///     hierarchy_type: "partitive".to_string(),
///     confidence: 0.85,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipSuggestion {
    pub source_tag: String,
    pub target_tag: String,
    pub hierarchy_type: String,
    pub confidence: f64,
}

/// Builder for constructing `HierarchySuggester` instances.
///
/// This builder provides an ergonomic way to construct `HierarchySuggester` instances,
/// following the same pattern as `AutoTaggerBuilder`.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::hierarchy::HierarchySuggesterBuilder;
/// use cons::ollama::{OllamaClientBuilder, OllamaClientTrait};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// let suggester = HierarchySuggesterBuilder::new()
///     .client(Arc::new(client))
///     .build();
///
/// let tags = vec!["transformer".to_string(), "neural-network".to_string()];
/// let suggestions = suggester.suggest_relationships("deepseek-r1:8b", tags)?;
///
/// for suggestion in suggestions {
///     println!("{} -> {}: {:.2}", suggestion.source_tag, suggestion.target_tag, suggestion.confidence);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct HierarchySuggesterBuilder {
    client: Option<Arc<dyn OllamaClientTrait>>,
}

impl HierarchySuggesterBuilder {
    /// Creates a new `HierarchySuggesterBuilder` with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Ollama client to use for relationship suggestion.
    ///
    /// # Arguments
    ///
    /// * `client` - An Arc-wrapped implementation of `OllamaClientTrait`
    pub fn client(mut self, client: Arc<dyn OllamaClientTrait>) -> Self {
        self.client = Some(client);
        self
    }

    /// Builds the `HierarchySuggester` with the configured settings.
    ///
    /// # Panics
    ///
    /// Panics if `client()` was not called before `build()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use cons::hierarchy::HierarchySuggesterBuilder;
    /// use cons::ollama::OllamaClientBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClientBuilder::new().build()?;
    /// let suggester = HierarchySuggesterBuilder::new()
    ///     .client(Arc::new(client))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn build(self) -> HierarchySuggester {
        HierarchySuggester {
            client: self.client.expect("client must be set via client() method"),
        }
    }
}

/// Suggests hierarchical relationships between tags using LLM-based analysis.
///
/// # Examples
///
/// ## Using HierarchySuggesterBuilder (Recommended)
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::hierarchy::HierarchySuggesterBuilder;
/// use cons::ollama::OllamaClientBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create Ollama client
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// // Create HierarchySuggester using builder
/// let suggester = HierarchySuggesterBuilder::new()
///     .client(Arc::new(client))
///     .build();
///
/// // Suggest relationships for tags
/// let tags = vec![
///     "transformer".to_string(),
///     "neural-network".to_string(),
///     "attention".to_string(),
/// ];
/// let suggestions = suggester.suggest_relationships("deepseek-r1:8b", tags)?;
///
/// // Process the suggestions (Vec<RelationshipSuggestion>)
/// // Note: Only suggestions with confidence >= 0.7 are returned
/// for suggestion in suggestions {
///     println!(
///         "{} -> {} ({}, {:.2})",
///         suggestion.source_tag,
///         suggestion.target_tag,
///         suggestion.hierarchy_type,
///         suggestion.confidence
///     );
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Using HierarchySuggester::new() directly
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::hierarchy::HierarchySuggester;
/// use cons::ollama::OllamaClientBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// let suggester = HierarchySuggester::new(Arc::new(client));
/// let tags = vec!["rust".to_string(), "programming-language".to_string()];
/// let suggestions = suggester.suggest_relationships("deepseek-r1:8b", tags)?;
///
/// for suggestion in suggestions {
///     println!("{} -> {}", suggestion.source_tag, suggestion.target_tag);
/// }
/// # Ok(())
/// # }
/// ```
pub struct HierarchySuggester {
    client: Arc<dyn OllamaClientTrait>,
}

impl HierarchySuggester {
    /// Creates a new `HierarchySuggester` with the specified Ollama client.
    ///
    /// # Arguments
    ///
    /// * `client` - An Arc-wrapped implementation of `OllamaClientTrait`
    ///
    /// # Note
    ///
    /// Prefer using `HierarchySuggesterBuilder` for more ergonomic construction.
    #[must_use]
    pub fn new(client: Arc<dyn OllamaClientTrait>) -> Self {
        Self { client }
    }

    /// Suggests hierarchical relationships for the given tags using the specified model.
    ///
    /// # Arguments
    ///
    /// * `model` - The name of the Ollama model to use (e.g., "deepseek-r1:8b")
    /// * `tag_names` - Vector of tag names to analyze for relationships
    ///
    /// # Returns
    ///
    /// Returns a `Vec<RelationshipSuggestion>` containing only suggestions with confidence >= 0.7.
    /// Returns an empty `Vec` if JSON parsing fails (fail-safe behavior).
    ///
    /// # Errors
    ///
    /// Returns `OllamaError` if the LLM request fails (network, timeout, API errors).
    /// JSON parsing errors do not cause failures; they return empty results instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use cons::hierarchy::HierarchySuggester;
    /// use cons::ollama::OllamaClientBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClientBuilder::new().build()?;
    /// let suggester = HierarchySuggester::new(Arc::new(client));
    ///
    /// let tags = vec![
    ///     "python".to_string(),
    ///     "django".to_string(),
    ///     "web-framework".to_string(),
    /// ];
    ///
    /// let suggestions = suggester.suggest_relationships("deepseek-r1:8b", tags)?;
    ///
    /// for suggestion in &suggestions {
    ///     println!(
    ///         "{} -> {} ({}, confidence: {:.2})",
    ///         suggestion.source_tag,
    ///         suggestion.target_tag,
    ///         suggestion.hierarchy_type,
    ///         suggestion.confidence
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn suggest_relationships(
        &self,
        model: &str,
        tag_names: Vec<String>,
    ) -> Result<Vec<RelationshipSuggestion>, OllamaError> {
        // Format tags as JSON array
        let tags_json = serde_json::to_string(&tag_names).map_err(OllamaError::Serialization)?;

        // Construct prompt with tag names
        let prompt = PROMPT_TEMPLATE.replace("{tags}", &tags_json);

        // Call LLM
        let response = self.client.generate(model, &prompt)?;

        // Extract JSON from response (handles various output formats)
        let Some(json_str) = extract_json(&response) else {
            return Ok(Vec::new()); // Fail-safe: empty on extraction failure
        };

        // Parse and filter suggestions
        Ok(parse_suggestions(&json_str))
    }
}

/// Extracts JSON array from model response, handling various output formats.
///
/// Handles:
/// - Clean JSON response (no wrapping)
/// - Markdown code block wrapping (```json ... ```)
/// - Explanatory text before/after JSON
///
/// # Arguments
///
/// * `response` - The raw model response text
///
/// # Returns
///
/// Returns `Some(String)` containing the extracted JSON array, or `None` if no JSON found.
fn extract_json(response: &str) -> Option<String> {
    let trimmed = response.trim();

    // Try to find JSON array boundaries
    let start = trimmed.find('[')?;
    let end = trimmed.rfind(']')?;

    if start <= end {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

/// Parses JSON string into a `Vec` of `RelationshipSuggestion` objects.
///
/// # Arguments
///
/// * `json_str` - JSON array string to parse
///
/// # Returns
///
/// Returns a `Vec<RelationshipSuggestion>` with:
/// - Confidence scores clamped to 0.0-1.0 range
/// - Only suggestions with confidence >= 0.7
///
/// Returns an empty `Vec` if parsing fails (fail-safe behavior).
///
/// # Filtering
///
/// - Clamps confidence scores to 0.0-1.0 range
/// - Filters out suggestions with confidence < 0.7
/// - Filters out malformed suggestions (missing fields, invalid types)
fn parse_suggestions(json_str: &str) -> Vec<RelationshipSuggestion> {
    // Parse JSON
    let json_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(), // Fail-safe
    };

    // Extract array
    let Some(arr) = json_value.as_array() else {
        return Vec::new(); // Fail-safe
    };

    // Parse suggestions with validation and filtering
    let mut suggestions = Vec::new();
    for item in arr {
        let Some(obj) = item.as_object() else {
            continue;
        };

        // Extract required fields
        let Some(source_tag) = obj.get("source_tag").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(target_tag) = obj.get("target_tag").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(hierarchy_type) = obj.get("hierarchy_type").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(confidence) = obj.get("confidence").and_then(|v| v.as_f64()) else {
            continue;
        };

        // Clamp confidence to valid range
        let confidence = confidence.clamp(0.0, 1.0);

        // Filter by confidence threshold
        if confidence < 0.7 {
            continue;
        }

        suggestions.push(RelationshipSuggestion {
            source_tag: source_tag.to_string(),
            target_tag: target_tag.to_string(),
            hierarchy_type: hierarchy_type.to_string(),
            confidence,
        });
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockOllamaClient {
        response: String,
    }

    impl OllamaClientTrait for MockOllamaClient {
        fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_prompt_template_includes_xkos_semantics() {
        // Verify that the prompt includes XKOS explanation
        assert!(PROMPT_TEMPLATE.contains("GENERIC (is-a)"));
        assert!(PROMPT_TEMPLATE.contains("PARTITIVE (part-of)"));
        assert!(PROMPT_TEMPLATE.contains("Specialization"));
        assert!(PROMPT_TEMPLATE.contains("Compositional"));

        // Verify few-shot examples demonstrate both hierarchy types
        assert!(PROMPT_TEMPLATE.contains("\"hierarchy_type\": \"generic\""));
        assert!(PROMPT_TEMPLATE.contains("\"hierarchy_type\": \"partitive\""));
        assert!(PROMPT_TEMPLATE.contains("transformer"));
        assert!(PROMPT_TEMPLATE.contains("neural-network"));
        assert!(PROMPT_TEMPLATE.contains("attention"));
    }

    #[test]
    fn test_suggest_relationships_returns_parsed_suggestions() {
        let mock = MockOllamaClient {
            response: r#"[
                {"source_tag": "transformer", "target_tag": "neural-network", "hierarchy_type": "generic", "confidence": 0.95},
                {"source_tag": "attention", "target_tag": "transformer", "hierarchy_type": "partitive", "confidence": 0.85}
            ]"#.to_string(),
        };
        let suggester = HierarchySuggester::new(Arc::new(mock));

        let tags = vec![
            "transformer".to_string(),
            "neural-network".to_string(),
            "attention".to_string(),
        ];
        let result = suggester.suggest_relationships("test-model", tags);

        assert!(result.is_ok());
        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 2);

        assert_eq!(suggestions[0].source_tag, "transformer");
        assert_eq!(suggestions[0].target_tag, "neural-network");
        assert_eq!(suggestions[0].hierarchy_type, "generic");
        assert_eq!(suggestions[0].confidence, 0.95);

        assert_eq!(suggestions[1].source_tag, "attention");
        assert_eq!(suggestions[1].target_tag, "transformer");
        assert_eq!(suggestions[1].hierarchy_type, "partitive");
        assert_eq!(suggestions[1].confidence, 0.85);
    }

    #[test]
    fn test_extract_json_handles_markdown_wrapped_responses() {
        // Test with markdown code block
        let response = r#"```json
[
    {"source_tag": "rust", "target_tag": "programming-language", "hierarchy_type": "generic", "confidence": 0.95}
]
```"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        let json = extracted.unwrap();
        let suggestions = parse_suggestions(&json);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].source_tag, "rust");
        assert_eq!(suggestions[0].target_tag, "programming-language");

        // Test with preamble and postamble
        let response = r#"Here are the relationships I found:

[
    {"source_tag": "python", "target_tag": "programming-language", "hierarchy_type": "generic", "confidence": 0.9}
]

I hope this helps!"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        let json = extracted.unwrap();
        let suggestions = parse_suggestions(&json);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].source_tag, "python");
    }

    #[test]
    fn test_parse_suggestions_filters_by_confidence_threshold() {
        // Test that suggestions with confidence >= 0.7 are included
        let json_high = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 0.7},
            {"source_tag": "c", "target_tag": "d", "hierarchy_type": "generic", "confidence": 0.95}
        ]"#;
        let suggestions = parse_suggestions(json_high);
        assert_eq!(suggestions.len(), 2);

        // Test that suggestions with confidence < 0.7 are filtered out
        let json_low = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 0.69},
            {"source_tag": "c", "target_tag": "d", "hierarchy_type": "generic", "confidence": 0.5}
        ]"#;
        let suggestions = parse_suggestions(json_low);
        assert_eq!(suggestions.len(), 0);

        // Test mixed confidence values
        let json_mixed = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 0.95},
            {"source_tag": "c", "target_tag": "d", "hierarchy_type": "generic", "confidence": 0.65},
            {"source_tag": "e", "target_tag": "f", "hierarchy_type": "partitive", "confidence": 0.8}
        ]"#;
        let suggestions = parse_suggestions(json_mixed);
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].source_tag, "a");
        assert_eq!(suggestions[1].source_tag, "e");
    }

    #[test]
    fn test_fail_safe_behavior_returns_empty_vec_on_parse_failure() {
        // Test with invalid JSON
        let invalid_json = "This is not JSON at all";
        let suggestions = parse_suggestions(invalid_json);
        assert!(suggestions.is_empty());

        // Test with extraction failure
        let no_json = "No square brackets here";
        let extracted = extract_json(no_json);
        assert!(extracted.is_none());

        // Test with malformed JSON object
        let malformed = r#"[{"incomplete": "object"}]"#;
        let suggestions = parse_suggestions(malformed);
        assert!(suggestions.is_empty());

        // Test with non-array JSON
        let not_array = r#"{"key": "value"}"#;
        let suggestions = parse_suggestions(not_array);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_confidence_clamping_to_valid_range() {
        // Test clamping of out-of-range values
        let json_high = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 1.5},
            {"source_tag": "c", "target_tag": "d", "hierarchy_type": "generic", "confidence": 2.0}
        ]"#;
        let suggestions = parse_suggestions(json_high);
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].confidence, 1.0);
        assert_eq!(suggestions[1].confidence, 1.0);

        let json_low = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": -0.5}
        ]"#;
        let suggestions = parse_suggestions(json_low);
        // This should be filtered out because after clamping to 0.0, it's < 0.7
        assert_eq!(suggestions.len(), 0);

        // Test valid range values
        let json_valid = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 0.7},
            {"source_tag": "c", "target_tag": "d", "hierarchy_type": "generic", "confidence": 1.0},
            {"source_tag": "e", "target_tag": "f", "hierarchy_type": "partitive", "confidence": 0.85}
        ]"#;
        let suggestions = parse_suggestions(json_valid);
        assert_eq!(suggestions.len(), 3);
        assert_eq!(suggestions[0].confidence, 0.7);
        assert_eq!(suggestions[1].confidence, 1.0);
        assert_eq!(suggestions[2].confidence, 0.85);
    }

    #[test]
    fn test_ollama_error_propagates_correctly() {
        struct FailingMockClient;

        impl OllamaClientTrait for FailingMockClient {
            fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
                Err(OllamaError::Http { status: 500 })
            }
        }

        let suggester = HierarchySuggester::new(Arc::new(FailingMockClient));
        let tags = vec!["test".to_string()];
        let result = suggester.suggest_relationships("test-model", tags);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OllamaError::Http { status: 500 }
        ));
    }

    #[test]
    fn test_full_workflow_with_mock_client() {
        let mock = MockOllamaClient {
            response: r#"Based on the tags, here are the relationships:

```json
[
    {"source_tag": "transformer", "target_tag": "neural-network", "hierarchy_type": "generic", "confidence": 0.95},
    {"source_tag": "attention", "target_tag": "transformer", "hierarchy_type": "partitive", "confidence": 0.85},
    {"source_tag": "neural-network", "target_tag": "machine-learning", "hierarchy_type": "generic", "confidence": 0.9}
]
```

These represent clear hierarchical relationships."#
                .to_string(),
        };
        let suggester = HierarchySuggester::new(Arc::new(mock));

        let tags = vec![
            "transformer".to_string(),
            "neural-network".to_string(),
            "attention".to_string(),
            "machine-learning".to_string(),
        ];
        let result = suggester.suggest_relationships("test-model", tags);

        assert!(result.is_ok());
        let suggestions = result.unwrap();

        assert_eq!(suggestions.len(), 3);
        assert_eq!(suggestions[0].source_tag, "transformer");
        assert_eq!(suggestions[0].target_tag, "neural-network");
        assert_eq!(suggestions[0].hierarchy_type, "generic");
        assert_eq!(suggestions[0].confidence, 0.95);

        assert_eq!(suggestions[1].source_tag, "attention");
        assert_eq!(suggestions[1].target_tag, "transformer");
        assert_eq!(suggestions[1].hierarchy_type, "partitive");
        assert_eq!(suggestions[1].confidence, 0.85);

        assert_eq!(suggestions[2].source_tag, "neural-network");
        assert_eq!(suggestions[2].target_tag, "machine-learning");
        assert_eq!(suggestions[2].hierarchy_type, "generic");
        assert_eq!(suggestions[2].confidence, 0.9);
    }

    #[test]
    fn test_hierarchy_suggester_builder_construction() {
        let mock = MockOllamaClient {
            response: r#"[{"source_tag": "rust", "target_tag": "programming-language", "hierarchy_type": "generic", "confidence": 0.95}]"#.to_string(),
        };

        let suggester = HierarchySuggesterBuilder::new()
            .client(Arc::new(mock))
            .build();

        let tags = vec!["rust".to_string(), "programming-language".to_string()];
        let result = suggester.suggest_relationships("test-model", tags);
        assert!(result.is_ok());
        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].source_tag, "rust");
        assert_eq!(suggestions[0].target_tag, "programming-language");
    }

    #[test]
    fn test_extract_json_handles_nested_arrays() {
        let response = r#"[[{"inner": "array"}], [{"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 0.9}]]"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        // Should extract the outermost brackets
        assert_eq!(extracted.unwrap(), response);
    }

    #[test]
    fn test_parse_suggestions_filters_malformed_objects() {
        let json = r#"[
            {"source_tag": "a", "target_tag": "b", "hierarchy_type": "generic", "confidence": 0.95},
            {"source_tag": "c", "confidence": 0.9},
            {"target_tag": "d", "hierarchy_type": "generic", "confidence": 0.85},
            {"source_tag": "e", "target_tag": "f", "hierarchy_type": "generic", "confidence": "not-a-number"},
            {"source_tag": "g", "target_tag": "h", "hierarchy_type": "generic", "confidence": 0.8}
        ]"#;
        let suggestions = parse_suggestions(json);

        // Only first and last objects are valid and meet confidence threshold
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].source_tag, "a");
        assert_eq!(suggestions[1].source_tag, "g");
    }
}
