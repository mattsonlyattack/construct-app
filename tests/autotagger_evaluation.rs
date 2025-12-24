//! Auto-tagger evaluation tests.
//!
//! This test file provides evaluation tests for the auto-tagger prompt engineering.
//! Some tests may be ignored for CI and require running Ollama locally.
//!
//! To run ignored tests locally:
//! ```bash
//! cargo test --test autotagger_evaluation -- --ignored
//! ```

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use cons::autotagger::AutoTaggerBuilder;
use cons::ollama::{OllamaClientTrait, OllamaError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Test corpus entry structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CorpusEntry {
    content: String,
    expected_tags: Vec<String>,
    notes: String,
}

/// Loads the test corpus from the fixtures directory.
fn load_corpus() -> Result<Vec<CorpusEntry>, Box<dyn std::error::Error>> {
    // Get the path to the corpus file relative to the project root
    let corpus_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("auto_tagger_corpus.json");

    let content = fs::read_to_string(&corpus_path)?;
    let entries: Vec<CorpusEntry> = serde_json::from_str(&content)?;
    Ok(entries)
}

/// Mock Ollama client for testing.
struct MockOllamaClient {
    response: String,
}

#[async_trait]
impl OllamaClientTrait for MockOllamaClient {
    async fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
        Ok(self.response.clone())
    }
}

#[test]
fn test_corpus_file_parses_successfully() {
    let result = load_corpus();
    assert!(result.is_ok(), "Corpus file should parse successfully");
    
    let entries = result.unwrap();
    assert!(
        entries.len() >= 5 && entries.len() <= 8,
        "Corpus should contain 5-8 entries, got {}",
        entries.len()
    );

    // Verify each entry has required fields
    for (i, entry) in entries.iter().enumerate() {
        assert!(!entry.content.is_empty(), "Entry {}: content should not be empty", i);
        assert!(!entry.expected_tags.is_empty(), "Entry {}: expected_tags should not be empty", i);
        assert!(!entry.notes.is_empty(), "Entry {}: notes should not be empty", i);
    }
}

#[test]
fn test_corpus_includes_aboutness_vs_mention_cases() {
    let entries = load_corpus().unwrap();
    
    // Find the debugging/Python entry which tests "aboutness vs mention"
    let aboutness_entry = entries.iter().find(|e| {
        e.content.contains("Debugging") && e.content.contains("Python")
    });
    
    assert!(
        aboutness_entry.is_some(),
        "Corpus should include an entry testing 'aboutness vs mention' distinction"
    );

    let entry = aboutness_entry.unwrap();
    // The expected tags should prioritize "debugging" over "python"
    assert!(
        entry.expected_tags.contains(&"debugging".to_string()),
        "Aboutness test entry should include 'debugging' tag"
    );
}

#[tokio::test]
async fn test_tag_extraction_on_sample_with_mock() {
    // Load corpus
    let entries = load_corpus().unwrap();
    let sample = &entries[0]; // Use first entry

    // Create mock client that returns expected JSON format
    let mock_response = format!(
        r#"{{"rust": 0.95, "async": 0.9, "tokio": 0.85, "concurrency": 0.75}}"#
    );
    let mock = MockOllamaClient {
        response: mock_response,
    };

    // Create tagger with mock client
    let tagger = AutoTaggerBuilder::new()
        .client(Arc::new(mock))
        .build();

    // Generate tags
    let result = tagger.generate_tags("test-model", &sample.content).await;
    assert!(result.is_ok(), "Tag generation should succeed");

    let tags = result.unwrap();
    
    // Verify we got some tags
    assert!(!tags.is_empty(), "Should extract at least some tags");
    
    // Verify expected tags are present (normalized)
    for expected_tag in &sample.expected_tags {
        assert!(
            tags.contains_key(expected_tag),
            "Expected tag '{}' should be present in results",
            expected_tag
        );
    }
}

/// Integration test with real Ollama (ignored by default).
///
/// This test requires:
/// - Ollama running locally
/// - A model available (e.g., gemma3:4b or deepseek-r1:8b)
///
/// To run this test:
/// ```bash
/// cargo test --test autotagger_evaluation -- --ignored test_real_ollama_integration
/// ```
#[tokio::test]
#[ignore]
async fn test_real_ollama_integration() {
    use cons::ollama::OllamaClientBuilder;

    // Create real Ollama client
    let client = match OllamaClientBuilder::new().build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping test: Failed to create Ollama client: {}", e);
            return;
        }
    };

    // Create tagger
    let tagger = AutoTaggerBuilder::new()
        .client(Arc::new(client))
        .build();

    // Load corpus and test with first entry
    let entries = load_corpus().unwrap();
    let sample = &entries[0];

    // Try to generate tags (may fail if Ollama is not available)
    let result = tagger
        .generate_tags("gemma3:4b", &sample.content)
        .await;

    match result {
        Ok(tags) => {
            println!("Successfully generated {} tags", tags.len());
            for (tag, confidence) in &tags {
                println!("  {}: {:.2}", tag, confidence);
            }
            // Just verify we got some tags if the call succeeded
            assert!(!tags.is_empty(), "Should extract at least some tags");
        }
        Err(e) => {
            eprintln!("Ollama call failed (this is OK if Ollama is not running): {}", e);
            // Don't fail the test - this is expected if Ollama is not available
        }
    }
}

