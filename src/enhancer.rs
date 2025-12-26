//! Note text enhancement using LLMs.
//!
//! This module provides the `NoteEnhancer` struct which uses an Ollama-compatible
//! LLM to expand abbreviated notes, complete fragments, and clarify implicit context
//! while preserving the original intent.

use std::sync::Arc;

use crate::ollama::{OllamaClientTrait, OllamaError};

/// Prompt template for note enhancement.
///
/// Designed to expand abbreviations, complete fragments, and clarify context
/// while preserving the original intent. Returns JSON with enhanced content
/// and confidence score.
const PROMPT_TEMPLATE: &str = r#"You are a note enhancement assistant. Your task is to expand abbreviated notes, complete sentence fragments, and clarify implicit context while preserving the original intent.

CRITICAL RULES:
1. PRESERVE INTENT: Do not add information not implied by the original text
2. EXPAND thoughtfully: Fix abbreviations, complete fragments, add implied context
3. PRESERVE VERBATIM: Keep code blocks, URLs, and proper nouns exactly as written
4. CONFIDENCE: Return a score (0.0-1.0) reflecting enhancement quality
   - High (>0.8): Straightforward expansion with clear intent
   - Medium (0.5-0.8): Some interpretation required
   - Low (<0.5): Significant guesswork or ambiguity
5. COMPLETE NOTES: If the note is already a complete thought, return it unchanged with high confidence

EXAMPLES:

Input: "buy milk"
Output: {"enhanced_content": "Buy milk from the grocery store.", "confidence": 0.7}

Input: "async rust tokio - easier than manual threads"
Output: {"enhanced_content": "Learning async Rust. The tokio runtime makes concurrent programming much easier than manual thread management.", "confidence": 0.85}

Input: "mtg notes Q4 roadmap - auth feature + db migration prio"
Output: {"enhanced_content": "Meeting notes: discussed Q4 roadmap. Need to prioritize authentication feature and database migration.", "confidence": 0.75}

Input: "The quick brown fox jumps over the lazy dog"
Output: {"enhanced_content": "The quick brown fox jumps over the lazy dog", "confidence": 0.95}

Input: "debug py script - print() -> logging"
Output: {"enhanced_content": "Debugging a Python script. Used print statements but should switch to proper logging.", "confidence": 0.8}

Input: "```python\nprint('hello')\n```"
Output: {"enhanced_content": "Code snippet:\n```python\nprint('hello')\n```", "confidence": 0.9}

NOTE CONTENT:
{content}

Return ONLY a JSON object with two fields:
- "enhanced_content": The expanded note text (string)
- "confidence": Your confidence in the enhancement quality (float 0.0-1.0)

JSON OUTPUT:"#;

/// Result of note enhancement operation.
///
/// Contains the enhanced note content and a confidence score
/// indicating the quality of the enhancement.
#[derive(Debug, Clone, PartialEq)]
pub struct EnhancementResult {
    /// The enhanced note content
    enhanced_content: String,
    /// Confidence score (0.0-1.0) in the enhancement quality
    confidence: f64,
}

impl EnhancementResult {
    /// Creates a new `EnhancementResult`.
    ///
    /// # Arguments
    ///
    /// * `enhanced_content` - The enhanced note text
    /// * `confidence` - Confidence score (will be clamped to 0.0-1.0)
    pub fn new(enhanced_content: String, confidence: f64) -> Self {
        Self {
            enhanced_content,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Returns the enhanced note content.
    pub fn enhanced_content(&self) -> &str {
        &self.enhanced_content
    }

    /// Returns the confidence score (0.0-1.0).
    pub fn confidence(&self) -> f64 {
        self.confidence
    }
}

/// Builder for constructing `NoteEnhancer` instances.
///
/// This builder provides an ergonomic way to construct `NoteEnhancer` instances,
/// following the same pattern as `AutoTaggerBuilder`.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::enhancer::NoteEnhancerBuilder;
/// use cons::ollama::{OllamaClientBuilder, OllamaClientTrait};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// let enhancer = NoteEnhancerBuilder::new()
///     .client(Arc::new(client))
///     .build();
///
/// let result = enhancer.enhance_content("deepseek-r1:8b", "buy milk")?;
/// println!("Enhanced: {}", result.enhanced_content());
/// println!("Confidence: {:.2}", result.confidence());
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct NoteEnhancerBuilder {
    client: Option<Arc<dyn OllamaClientTrait>>,
}

impl NoteEnhancerBuilder {
    /// Creates a new `NoteEnhancerBuilder` with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Ollama client to use for note enhancement.
    ///
    /// # Arguments
    ///
    /// * `client` - An Arc-wrapped implementation of `OllamaClientTrait`
    pub fn client(mut self, client: Arc<dyn OllamaClientTrait>) -> Self {
        self.client = Some(client);
        self
    }

    /// Builds the `NoteEnhancer` with the configured settings.
    ///
    /// # Panics
    ///
    /// Panics if `client()` was not called before `build()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use cons::enhancer::NoteEnhancerBuilder;
    /// use cons::ollama::OllamaClientBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OllamaClientBuilder::new().build()?;
    /// let enhancer = NoteEnhancerBuilder::new()
    ///     .client(Arc::new(client))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn build(self) -> NoteEnhancer {
        NoteEnhancer {
            client: self.client.expect("client must be set via client() method"),
        }
    }
}

/// Enhances note content using LLM-based text expansion.
///
/// # Examples
///
/// ## Using NoteEnhancerBuilder (Recommended)
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::enhancer::NoteEnhancerBuilder;
/// use cons::ollama::OllamaClientBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create Ollama client
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// // Create NoteEnhancer using builder
/// let enhancer = NoteEnhancerBuilder::new()
///     .client(Arc::new(client))
///     .build();
///
/// // Enhance note content
/// let result = enhancer.enhance_content("deepseek-r1:8b", "buy milk")?;
///
/// println!("Enhanced: {}", result.enhanced_content());
/// println!("Confidence: {:.2}", result.confidence());
/// # Ok(())
/// # }
/// ```
///
/// ## Using NoteEnhancer::new() directly
///
/// ```no_run
/// use std::sync::Arc;
/// use cons::enhancer::NoteEnhancer;
/// use cons::ollama::OllamaClientBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()?;
///
/// let enhancer = NoteEnhancer::new(Arc::new(client));
/// let result = enhancer.enhance_content("deepseek-r1:8b", "buy milk")?;
///
/// println!("Enhanced: {}", result.enhanced_content());
/// println!("Confidence: {:.2}", result.confidence());
/// # Ok(())
/// # }
/// ```
pub struct NoteEnhancer {
    client: Arc<dyn OllamaClientTrait>,
}

impl NoteEnhancer {
    /// Creates a new `NoteEnhancer` with the specified Ollama client.
    ///
    /// # Arguments
    ///
    /// * `client` - An Arc-wrapped implementation of `OllamaClientTrait`
    ///
    /// # Note
    ///
    /// Prefer using `NoteEnhancerBuilder` for more ergonomic construction.
    #[must_use]
    pub fn new(client: Arc<dyn OllamaClientTrait>) -> Self {
        Self { client }
    }

    /// Enhances the given note content using the specified model.
    ///
    /// # Arguments
    ///
    /// * `model` - The name of the Ollama model to use (e.g., "deepseek-r1:8b")
    /// * `content` - The note content to enhance
    ///
    /// # Returns
    ///
    /// Returns an `EnhancementResult` containing the enhanced content and confidence score.
    ///
    /// # Errors
    ///
    /// Returns `OllamaError` if:
    /// - The LLM request fails (network, timeout, API errors)
    /// - JSON parsing fails (malformed response from LLM)
    pub fn enhance_content(
        &self,
        model: &str,
        content: &str,
    ) -> Result<EnhancementResult, OllamaError> {
        // Construct prompt with note content
        let prompt = PROMPT_TEMPLATE.replace("{content}", content);

        // Call LLM
        let response = self.client.generate(model, &prompt)?;

        // Extract JSON from response (handles various output formats)
        let json_str = extract_json(&response).ok_or_else(|| OllamaError::Api {
            message: "Failed to extract JSON from LLM response".to_string(),
        })?;

        // Parse enhancement result
        parse_enhancement_result(&json_str)
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

/// Parses JSON string into an `EnhancementResult`.
///
/// # Arguments
///
/// * `json_str` - JSON string to parse
///
/// # Returns
///
/// Returns `EnhancementResult` with enhanced content and clamped confidence score.
///
/// # Errors
///
/// Returns `OllamaError::Api` if:
/// - JSON parsing fails
/// - Required fields are missing
/// - Fields have wrong types
fn parse_enhancement_result(json_str: &str) -> Result<EnhancementResult, OllamaError> {
    // Parse JSON
    let json_value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| OllamaError::Api {
            message: format!("Failed to parse JSON: {}", e),
        })?;

    // Extract object
    let obj = json_value.as_object().ok_or_else(|| OllamaError::Api {
        message: "Expected JSON object".to_string(),
    })?;

    // Extract enhanced_content field
    let enhanced_content = obj
        .get("enhanced_content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OllamaError::Api {
            message: "Missing or invalid 'enhanced_content' field".to_string(),
        })?
        .to_string();

    // Extract confidence field
    let confidence = obj
        .get("confidence")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| OllamaError::Api {
            message: "Missing or invalid 'confidence' field".to_string(),
        })?;

    // Clamp confidence and create result
    Ok(EnhancementResult::new(enhanced_content, confidence))
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

    // Task 3.1: Six focused tests for NoteEnhancer

    #[test]
    fn test_note_enhancer_builder_constructs_note_enhancer_with_client() {
        let mock = MockOllamaClient {
            response: r#"{"enhanced_content": "Test note.", "confidence": 0.9}"#.to_string(),
        };

        let enhancer = NoteEnhancerBuilder::new().client(Arc::new(mock)).build();

        let result = enhancer.enhance_content("test-model", "test content");
        assert!(result.is_ok());
        let enhancement = result.unwrap();
        assert_eq!(enhancement.enhanced_content(), "Test note.");
        assert_eq!(enhancement.confidence(), 0.9);
    }

    #[test]
    fn test_enhance_content_returns_enhancement_result_with_content_and_confidence() {
        let mock = MockOllamaClient {
            response: r#"{"enhanced_content": "Buy milk from the store.", "confidence": 0.85}"#
                .to_string(),
        };
        let enhancer = NoteEnhancer::new(Arc::new(mock));

        let result = enhancer.enhance_content("deepseek-r1:8b", "buy milk");

        assert!(result.is_ok());
        let enhancement = result.unwrap();
        assert_eq!(enhancement.enhanced_content(), "Buy milk from the store.");
        assert_eq!(enhancement.confidence(), 0.85);
    }

    #[test]
    fn test_json_response_parsing_extracts_enhanced_content_and_confidence() {
        let json = r#"{"enhanced_content": "Enhanced note text.", "confidence": 0.75}"#;
        let result = parse_enhancement_result(json);

        assert!(result.is_ok());
        let enhancement = result.unwrap();
        assert_eq!(enhancement.enhanced_content(), "Enhanced note text.");
        assert_eq!(enhancement.confidence(), 0.75);
    }

    #[test]
    fn test_extract_json_handles_markdown_code_blocks_and_preamble() {
        // Test markdown code block
        let response_markdown = r#"Here is the enhanced note:

```json
{"enhanced_content": "Test note.", "confidence": 0.9}
```

Hope this helps!"#;
        let extracted = extract_json(response_markdown);
        assert!(extracted.is_some());
        let json = extracted.unwrap();
        assert!(json.contains("enhanced_content"));
        assert!(json.contains("confidence"));

        // Test preamble without code block
        let response_preamble = r#"Based on the input, here's the result:
{"enhanced_content": "Test note.", "confidence": 0.9}
That's my enhancement."#;
        let extracted = extract_json(response_preamble);
        assert!(extracted.is_some());
        let json = extracted.unwrap();
        assert!(json.contains("enhanced_content"));
    }

    #[test]
    fn test_confidence_clamping_to_valid_range() {
        // Test clamping above 1.0
        let result_high = EnhancementResult::new("Test".to_string(), 1.5);
        assert_eq!(result_high.confidence(), 1.0);

        // Test clamping below 0.0
        let result_low = EnhancementResult::new("Test".to_string(), -0.5);
        assert_eq!(result_low.confidence(), 0.0);

        // Test valid range values
        let result_valid = EnhancementResult::new("Test".to_string(), 0.7);
        assert_eq!(result_valid.confidence(), 0.7);

        // Test parsing with out-of-range values
        let json_high = r#"{"enhanced_content": "Test", "confidence": 2.0}"#;
        let parsed = parse_enhancement_result(json_high).unwrap();
        assert_eq!(parsed.confidence(), 1.0);

        let json_low = r#"{"enhanced_content": "Test", "confidence": -1.0}"#;
        let parsed = parse_enhancement_result(json_low).unwrap();
        assert_eq!(parsed.confidence(), 0.0);
    }

    #[test]
    fn test_fail_safe_behavior_returns_error_on_parse_failure() {
        // Test missing enhanced_content field
        let json_missing_content = r#"{"confidence": 0.9}"#;
        let result = parse_enhancement_result(json_missing_content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OllamaError::Api { .. }));

        // Test missing confidence field
        let json_missing_confidence = r#"{"enhanced_content": "Test"}"#;
        let result = parse_enhancement_result(json_missing_confidence);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OllamaError::Api { .. }));

        // Test invalid JSON
        let invalid_json = "This is not JSON";
        let result = parse_enhancement_result(invalid_json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OllamaError::Api { .. }));

        // Test extraction failure (no JSON)
        let no_json = "No curly braces here";
        let extracted = extract_json(no_json);
        assert!(extracted.is_none());

        // Test enhance_content returns error when extraction fails
        let mock = MockOllamaClient {
            response: "No JSON in this response".to_string(),
        };
        let enhancer = NoteEnhancer::new(Arc::new(mock));
        let result = enhancer.enhance_content("test-model", "test content");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OllamaError::Api { .. }));
    }

    // Additional tests for completeness

    #[test]
    fn test_extract_json_handles_nested_objects() {
        let response =
            r#"{"enhanced_content": "Test", "metadata": {"key": "value"}, "confidence": 0.9}"#;
        let extracted = extract_json(response);

        assert!(extracted.is_some());
        // Should extract the outermost braces
        assert_eq!(extracted.unwrap(), response);
    }

    #[test]
    fn test_parse_enhancement_result_wrong_field_types() {
        // Test enhanced_content as non-string
        let json_wrong_type = r#"{"enhanced_content": 123, "confidence": 0.9}"#;
        let result = parse_enhancement_result(json_wrong_type);
        assert!(result.is_err());

        // Test confidence as non-number
        let json_wrong_confidence = r#"{"enhanced_content": "Test", "confidence": "high"}"#;
        let result = parse_enhancement_result(json_wrong_confidence);
        assert!(result.is_err());
    }

    #[test]
    fn test_ollama_error_propagates_from_client() {
        struct FailingMockClient;

        impl OllamaClientTrait for FailingMockClient {
            fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
                Err(OllamaError::Http { status: 500 })
            }
        }

        let enhancer = NoteEnhancer::new(Arc::new(FailingMockClient));
        let result = enhancer.enhance_content("test-model", "test content");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OllamaError::Http { status: 500 }
        ));
    }

    #[test]
    fn test_full_workflow_with_mock_client() {
        let mock = MockOllamaClient {
            response: r#"I'll enhance this note for you:

```json
{"enhanced_content": "Learning async Rust. The tokio runtime makes concurrent programming much easier than manual thread management.", "confidence": 0.88}
```

This expansion clarifies the abbreviated input."#
                .to_string(),
        };
        let enhancer = NoteEnhancer::new(Arc::new(mock));

        let result = enhancer.enhance_content(
            "deepseek-r1:8b",
            "async rust tokio - easier than manual threads",
        );

        assert!(result.is_ok());
        let enhancement = result.unwrap();

        assert_eq!(
            enhancement.enhanced_content(),
            "Learning async Rust. The tokio runtime makes concurrent programming much easier than manual thread management."
        );
        assert_eq!(enhancement.confidence(), 0.88);
    }

    #[test]
    fn test_enhancement_result_equality() {
        let result1 = EnhancementResult::new("Test content".to_string(), 0.9);
        let result2 = EnhancementResult::new("Test content".to_string(), 0.9);
        let result3 = EnhancementResult::new("Different content".to_string(), 0.9);

        assert_eq!(result1, result2);
        assert_ne!(result1, result3);
    }

    #[test]
    fn test_enhancement_result_debug_format() {
        let result = EnhancementResult::new("Test content".to_string(), 0.9);
        let debug_str = format!("{:?}", result);

        assert!(debug_str.contains("Test content"));
        assert!(debug_str.contains("0.9"));
    }

    #[test]
    fn test_enhancement_result_clone() {
        let result1 = EnhancementResult::new("Test content".to_string(), 0.9);
        let result2 = result1.clone();

        assert_eq!(result1, result2);
        assert_eq!(result1.enhanced_content(), result2.enhanced_content());
        assert_eq!(result1.confidence(), result2.confidence());
    }
}
