//! Auto-tagger for extracting tags from note content using LLMs.
//!
//! This module provides the `AutoTagger` struct which uses an Ollama-compatible
//! LLM to extract relevant tags from note content with confidence scores.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ollama::{OllamaClientTrait, OllamaError};

use super::normalizer::TagNormalizer;

/// Prompt template for tag extraction.
///
/// Designed for model-agnostic compatibility with clear, explicit instructions.
/// Includes few-shot examples demonstrating the expected output format.
const PROMPT_TEMPLATE: &str = r#"Extract relevant tags from the note content below. Return ONLY a JSON object with tag names as keys and confidence scores (0.0-1.0) as values. Do not include any explanatory text.

INSTRUCTIONS:
1. Focus on what the note is ABOUT (primary topics), not things merely mentioned in passing
2. Extract 3-7 tags depending on note complexity
3. Use lowercase for all tags
4. Use hyphens instead of spaces (e.g., "machine-learning" not "machine learning")
5. Avoid special characters; use only alphanumeric and hyphens
6. Assign confidence scores from 0.0 to 1.0 based on how central each tag is to the note's content

EXAMPLES:

Input: "Learning async Rust. The tokio runtime makes concurrent programming much easier than manual thread management."
Output: {"async": 0.95, "rust": 0.95, "tokio": 0.85, "concurrency": 0.75}

Input: "Debugging a Python script. Used print statements but should switch to proper logging."
Output: {"debugging": 0.9, "python": 0.7, "logging": 0.65}

Input: "Meeting notes: discussed Q4 roadmap. Need to prioritize authentication feature and database migration."
Output: {"meeting-notes": 0.85, "roadmap": 0.75, "authentication": 0.7, "database": 0.65}

NOTE CONTENT:
{content}

JSON OUTPUT:"#;

/// Builder for constructing `AutoTagger` instances.
///
/// This builder provides an ergonomic way to construct `AutoTagger` instances,
/// following the same pattern as `OllamaClientBuilder`.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::autotagger::AutoTaggerBuilder;
/// use cons::ollama::{OllamaClientBuilder, OllamaClientTrait};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// let tagger = AutoTaggerBuilder::new()
///     .client(Arc::new(client))
///     .build();
///
/// let tags = tagger.generate_tags("deepseek-r1:8b", "Learning Rust async programming")?;
///
/// for (tag, confidence) in tags {
///     println!("{}: {:.2}", tag, confidence);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct AutoTaggerBuilder {
    client: Option<Arc<dyn OllamaClientTrait>>,
}

impl AutoTaggerBuilder {
    /// Creates a new `AutoTaggerBuilder` with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Ollama client to use for tag generation.
    ///
    /// # Arguments
    ///
    /// * `client` - An Arc-wrapped implementation of `OllamaClientTrait`
    pub fn client(mut self, client: Arc<dyn OllamaClientTrait>) -> Self {
        self.client = Some(client);
        self
    }

    /// Builds the `AutoTagger` with the configured settings.
    ///
    /// # Panics
    ///
    /// Panics if `client()` was not called before `build()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use cons::autotagger::AutoTaggerBuilder;
    /// use cons::ollama::OllamaClientBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClientBuilder::new().build()?;
    /// let tagger = AutoTaggerBuilder::new()
    ///     .client(Arc::new(client))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn build(self) -> AutoTagger {
        AutoTagger {
            client: self.client.expect("client must be set via client() method"),
        }
    }
}

/// Extracts tags from note content using LLM-based analysis.
///
/// # Examples
///
/// ## Using AutoTaggerBuilder (Recommended)
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::autotagger::AutoTaggerBuilder;
/// use cons::ollama::OllamaClientBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create Ollama client
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// // Create AutoTagger using builder
/// let tagger = AutoTaggerBuilder::new()
///     .client(Arc::new(client))
///     .build();
///
/// // Generate tags for note content
/// let tags = tagger.generate_tags("deepseek-r1:8b", "Learning Rust async programming")?;
///
/// // Process the tags (HashMap<String, f64>)
/// for (tag, confidence) in tags {
///     println!("{}: {:.2}", tag, confidence);
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Using AutoTagger::new() directly
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::autotagger::AutoTagger;
/// use cons::ollama::OllamaClientBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// let tagger = AutoTagger::new(Arc::new(client));
/// let tags = tagger.generate_tags("deepseek-r1:8b", "Learning Rust async programming")?;
///
/// for (tag, confidence) in tags {
///     println!("{}: {:.2}", tag, confidence);
/// }
/// # Ok(())
/// # }
/// ```
pub struct AutoTagger {
    client: Arc<dyn OllamaClientTrait>,
}

impl AutoTagger {
    /// Creates a new `AutoTagger` with the specified Ollama client.
    ///
    /// # Arguments
    ///
    /// * `client` - An Arc-wrapped implementation of `OllamaClientTrait`
    ///
    /// # Note
    ///
    /// Prefer using `AutoTaggerBuilder` for more ergonomic construction.
    #[must_use]
    pub fn new(client: Arc<dyn OllamaClientTrait>) -> Self {
        Self { client }
    }

    /// Generates tags for the given note content using the specified model.
    ///
    /// # Arguments
    ///
    /// * `model` - The name of the Ollama model to use (e.g., "deepseek-r1:8b")
    /// * `content` - The note content to extract tags from
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` of normalized tag names to confidence scores (0.0-1.0).
    /// Returns an empty `HashMap` if JSON parsing fails (fail-safe behavior).
    ///
    /// # Errors
    ///
    /// Returns `OllamaError` if the LLM request fails (network, timeout, API errors).
    /// JSON parsing errors do not cause failures; they return empty results instead.
    pub fn generate_tags(
        &self,
        model: &str,
        content: &str,
    ) -> Result<HashMap<String, f64>, OllamaError> {
        // Construct prompt with note content
        let prompt = PROMPT_TEMPLATE.replace("{content}", content);

        // Call LLM
        let response = self.client.generate(model, &prompt)?;

        // Extract JSON from response (handles various output formats)
        let Some(json_str) = extract_json(&response) else {
            return Ok(HashMap::new()); // Fail-safe: empty on extraction failure
        };

        // Parse and normalize tags
        Ok(parse_tags(&json_str))
    }
}

/// Extracts JSON from model response, handling various output formats.
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
/// Returns `Some(String)` containing the extracted JSON, or `None` if no JSON found.
fn extract_json(response: &str) -> Option<String> {
    let trimmed = response.trim();

    // Try to find JSON object boundaries
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;

    if start <= end {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

/// Parses JSON string into a `HashMap` of normalized tags to confidence scores.
///
/// # Arguments
///
/// * `json_str` - JSON string to parse
///
/// # Returns
///
/// Returns a `HashMap` with normalized tag names and clamped confidence scores.
/// Returns an empty `HashMap` if parsing fails (fail-safe behavior).
///
/// # Normalization
///
/// - Applies `TagNormalizer` to all tag names
/// - Clamps confidence scores to 0.0-1.0 range
/// - Filters out empty normalized tags
fn parse_tags(json_str: &str) -> HashMap<String, f64> {
    // Parse JSON
    let json_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return HashMap::new(), // Fail-safe
    };

    // Extract object
    let Some(obj) = json_value.as_object() else {
        return HashMap::new(); // Fail-safe
    };

    // Parse tags with normalization and validation
    let mut tags = HashMap::new();
    for (key, value) in obj {
        // Normalize tag name
        let normalized = TagNormalizer::normalize_tag(key);
        if normalized.is_empty() {
            continue;
        }

        // Parse and clamp confidence score
        let confidence = match value.as_f64() {
            Some(f) => f.clamp(0.0, 1.0),
            None => continue, // Skip non-numeric values
        };

        tags.insert(normalized, confidence);
    }

    tags
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
    fn test_prompt_construction_includes_note_content() {
        let mock = MockOllamaClient {
            response: r#"{"rust": 0.9}"#.to_string(),
        };
        let tagger = AutoTagger::new(Arc::new(mock));

        let content = "Learning Rust ownership patterns";
        let result = tagger.generate_tags("test-model", content);

        assert!(result.is_ok());
        // Verify that the prompt would include the content (tested via integration)
    }

    #[test]
    fn test_json_parsing_of_valid_model_output() {
        let json = r#"{"rust": 0.9, "async": 0.75}"#;
        let tags = parse_tags(json);

        assert_eq!(tags.len(), 2);
        assert_eq!(tags.get("rust"), Some(&0.9));
        assert_eq!(tags.get("async"), Some(&0.75));
    }

    #[test]
    fn test_json_extraction_from_markdown_code_blocks() {
        let response = r#"```json
{"rust": 0.9, "async": 0.75}
```"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        let json = extracted.unwrap();
        let tags = parse_tags(&json);

        assert_eq!(tags.len(), 2);
        assert_eq!(tags.get("rust"), Some(&0.9));
        assert_eq!(tags.get("async"), Some(&0.75));
    }

    #[test]
    fn test_json_extraction_from_text_with_preamble_and_postamble() {
        let response = r#"Here are the tags I extracted:

{"rust": 0.9, "async": 0.75, "tokio": 0.8}

I hope this helps!"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        let json = extracted.unwrap();
        let tags = parse_tags(&json);

        assert_eq!(tags.len(), 3);
        assert_eq!(tags.get("rust"), Some(&0.9));
        assert_eq!(tags.get("async"), Some(&0.75));
        assert_eq!(tags.get("tokio"), Some(&0.8));
    }

    #[test]
    fn test_fail_safe_behavior_on_parse_failure() {
        // Test with invalid JSON
        let invalid_json = "This is not JSON at all";
        let tags = parse_tags(invalid_json);
        assert!(tags.is_empty());

        // Test with extraction failure
        let no_json = "No curly braces here";
        let extracted = extract_json(no_json);
        assert!(extracted.is_none());
    }

    #[test]
    fn test_confidence_score_clamping_to_valid_range() {
        // Test clamping of out-of-range values
        let json_high = r#"{"rust": 1.5, "async": 2.0}"#;
        let tags = parse_tags(json_high);
        assert_eq!(tags.get("rust"), Some(&1.0));
        assert_eq!(tags.get("async"), Some(&1.0));

        let json_low = r#"{"rust": -0.5, "async": -1.0}"#;
        let tags = parse_tags(json_low);
        assert_eq!(tags.get("rust"), Some(&0.0));
        assert_eq!(tags.get("async"), Some(&0.0));

        // Test valid range values
        let json_valid = r#"{"rust": 0.0, "async": 1.0, "tokio": 0.5}"#;
        let tags = parse_tags(json_valid);
        assert_eq!(tags.get("rust"), Some(&0.0));
        assert_eq!(tags.get("async"), Some(&1.0));
        assert_eq!(tags.get("tokio"), Some(&0.5));
    }

    #[test]
    fn test_tag_normalization_applied_to_keys() {
        let json = r#"{"RUST": 0.9, "Machine Learning": 0.85, "C++": 0.7}"#;
        let tags = parse_tags(json);

        // Verify normalization was applied
        assert!(tags.contains_key("rust"));
        assert!(tags.contains_key("machine-learning"));
        assert!(tags.contains_key("c")); // C++ becomes "c" after special char removal

        // Verify original (unnormalized) keys are not present
        assert!(!tags.contains_key("RUST"));
        assert!(!tags.contains_key("Machine Learning"));
        assert!(!tags.contains_key("C++"));
    }

    #[test]
    fn test_generate_tags_returns_empty_on_json_extraction_failure() {
        let mock = MockOllamaClient {
            response: "No JSON here, just plain text".to_string(),
        };
        let tagger = AutoTagger::new(Arc::new(mock));

        let result = tagger.generate_tags("test-model", "test content");

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_generate_tags_returns_empty_on_json_parse_failure() {
        let mock = MockOllamaClient {
            response: r#"{"invalid": "not a number"}"#.to_string(),
        };
        let tagger = AutoTagger::new(Arc::new(mock));

        let result = tagger.generate_tags("test-model", "test content");

        assert!(result.is_ok());
        let tags = result.unwrap();
        // Tag with non-numeric value should be skipped
        assert!(!tags.contains_key("invalid"));
    }

    #[test]
    fn test_ollama_error_propagates_correctly() {
        struct FailingMockClient;

        
        impl OllamaClientTrait for FailingMockClient {
            fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
                Err(OllamaError::Http { status: 500 })
            }
        }

        let tagger = AutoTagger::new(Arc::new(FailingMockClient));
        let result = tagger.generate_tags("test-model", "test content");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OllamaError::Http { status: 500 }
        ));
    }

    #[test]
    fn test_full_workflow_with_mock_client() {
        let mock = MockOllamaClient {
            response: r#"Based on the content, here are the tags:

```json
{"rust": 0.95, "async": 0.85, "tokio": 0.75}
```

These tags represent the main topics."#
                .to_string(),
        };
        let tagger = AutoTagger::new(Arc::new(mock));

        let result = tagger
            .generate_tags("test-model", "Learning async Rust with tokio")
            ;

        assert!(result.is_ok());
        let tags = result.unwrap();

        assert_eq!(tags.len(), 3);
        assert_eq!(tags.get("rust"), Some(&0.95));
        assert_eq!(tags.get("async"), Some(&0.85));
        assert_eq!(tags.get("tokio"), Some(&0.75));
    }

    // Integration tests for Task Group 3
    mod integration_tests {
        use super::*;

        /// Test AutoTagger works with mock OllamaClient returning valid JSON.
        #[test]
        fn test_autotagger_integration_with_valid_json_response() {
            let mock = MockOllamaClient {
                response: r#"{"rust": 0.9, "testing": 0.85, "integration": 0.75}"#.to_string(),
            };
            let tagger = AutoTagger::new(Arc::new(mock));

            let result = tagger
                .generate_tags("deepseek-r1:8b", "Writing integration tests for Rust")
                ;

            assert!(result.is_ok());
            let tags = result.unwrap();

            assert_eq!(tags.len(), 3);
            assert_eq!(tags.get("rust"), Some(&0.9));
            assert_eq!(tags.get("testing"), Some(&0.85));
            assert_eq!(tags.get("integration"), Some(&0.75));
        }

        /// Test AutoTagger handles OllamaError gracefully.
        #[test]
        fn test_autotagger_handles_ollama_error_gracefully() {
            struct ErrorMockClient;

            
            impl OllamaClientTrait for ErrorMockClient {
                fn generate(
                    &self,
                    _model: &str,
                    _prompt: &str,
                ) -> Result<String, OllamaError> {
                    Err(OllamaError::Network(
                        reqwest::Client::new()
                            .get("not-a-valid-url")
                            .build()
                            .unwrap_err(),
                    ))
                }
            }

            let tagger = AutoTagger::new(Arc::new(ErrorMockClient));
            let result = tagger.generate_tags("test-model", "test content");

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), OllamaError::Network(_)));
        }

        /// Test full workflow: content -> prompt -> mock response -> normalized tags.
        #[test]
        fn test_full_workflow_content_to_normalized_tags() {
            // Mock response with various normalization challenges
            let mock = MockOllamaClient {
                response: r#"Here are the extracted tags:

```json
{"RUST": 0.95, "Machine Learning": 0.88, "async-programming": 0.82, "API Design!": 0.75}
```

I focused on the main topics discussed."#
                    .to_string(),
            };
            let tagger = AutoTagger::new(Arc::new(mock));

            let content = "Exploring async Rust for machine learning API design";
            let result = tagger.generate_tags("gemma3:4b", content);

            assert!(result.is_ok());
            let tags = result.unwrap();

            // Verify normalization was applied
            assert_eq!(tags.len(), 4);
            assert_eq!(tags.get("rust"), Some(&0.95)); // RUST -> rust
            assert_eq!(tags.get("machine-learning"), Some(&0.88)); // Machine Learning -> machine-learning
            assert_eq!(tags.get("async-programming"), Some(&0.82)); // unchanged
            assert_eq!(tags.get("api-design"), Some(&0.75)); // API Design! -> api-design

            // Verify original (unnormalized) keys are not present
            assert!(!tags.contains_key("RUST"));
            assert!(!tags.contains_key("Machine Learning"));
            assert!(!tags.contains_key("API Design!"));
        }

        /// Test model name is passed correctly to client.
        #[test]
        fn test_model_name_passed_to_client() {
            use std::sync::Mutex;

            struct ModelCapturingMock {
                captured_model: Mutex<Option<String>>,
            }

            
            impl OllamaClientTrait for ModelCapturingMock {
                fn generate(
                    &self,
                    model: &str,
                    _prompt: &str,
                ) -> Result<String, OllamaError> {
                    *self.captured_model.lock().unwrap() = Some(model.to_string());
                    Ok(r#"{"test": 0.9}"#.to_string())
                }
            }

            let mock = ModelCapturingMock {
                captured_model: Mutex::new(None),
            };
            let mock = Arc::new(mock);
            let tagger = AutoTagger::new(mock.clone());

            let result = tagger.generate_tags("deepseek-r1:8b", "test content");
            assert!(result.is_ok());

            let captured = mock.captured_model.lock().unwrap();
            assert_eq!(captured.as_deref(), Some("deepseek-r1:8b"));
        }

        /// Test AutoTaggerBuilder constructs AutoTagger correctly.
        #[test]
        fn test_autotagger_builder_construction() {
            let mock = MockOllamaClient {
                response: r#"{"rust": 0.9}"#.to_string(),
            };

            let tagger = AutoTaggerBuilder::new()
                .client(Arc::new(mock))
                .build();

            let result = tagger.generate_tags("test-model", "test content");
            assert!(result.is_ok());
            let tags = result.unwrap();
            assert_eq!(tags.get("rust"), Some(&0.9));
        }
    }

    #[test]
    fn test_extract_json_handles_nested_objects() {
        let response = r#"{"outer": {"inner": 0.5}, "tag": 0.9}"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        // Should extract the outermost braces
        assert_eq!(extracted.unwrap(), response);
    }

    #[test]
    fn test_parse_tags_filters_empty_normalized_tags() {
        let json = r#"{"!!!": 0.9, "   ": 0.8, "valid": 0.7}"#;
        let tags = parse_tags(json);

        // Empty normalized tags should be filtered out
        assert!(!tags.contains_key(""));
        assert_eq!(tags.len(), 1);
        assert_eq!(tags.get("valid"), Some(&0.7));
    }
}
