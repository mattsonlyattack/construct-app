//! Auto-tagger evaluation tests.
//!
//! This test file provides evaluation tests for the auto-tagger prompt engineering.
//! Tests that require Ollama are automatically skipped in GitHub Actions CI.
//!
//! To run all tests locally (with Ollama running):
//! ```bash
//! cargo test --test autotagger_evaluation
//! ```

use std::sync::Arc;

use cons::autotagger::{AutoTaggerBuilder, compare_tags, load_corpus};
use cons::ollama::{OllamaClientTrait, OllamaError};

/// Skip test if running in GitHub Actions
fn skip_in_ci() -> bool {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        println!("Skipping test in GitHub Actions (no Ollama available)");
        return true;
    }
    false
}

/// Mock Ollama client for testing.
struct MockOllamaClient {
    response: String,
}

impl OllamaClientTrait for MockOllamaClient {
    fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
        Ok(self.response.clone())
    }
}

#[test]
fn test_corpus_file_parses_successfully() {
    let result = load_corpus(None);
    assert!(result.is_ok(), "Corpus file should parse successfully");

    let entries = result.unwrap();
    assert!(
        entries.len() >= 5 && entries.len() <= 8,
        "Corpus should contain 5-8 entries, got {}",
        entries.len()
    );

    // Verify each entry has required fields
    for (i, entry) in entries.iter().enumerate() {
        assert!(
            !entry.content.is_empty(),
            "Entry {}: content should not be empty",
            i
        );
        assert!(
            !entry.expected_tags.is_empty(),
            "Entry {}: expected_tags should not be empty",
            i
        );
        assert!(
            !entry.notes.is_empty(),
            "Entry {}: notes should not be empty",
            i
        );
    }
}

#[test]
fn test_corpus_includes_aboutness_vs_mention_cases() {
    let entries = load_corpus(None).unwrap();

    // Find the debugging/Python entry which tests "aboutness vs mention"
    let aboutness_entry = entries
        .iter()
        .find(|e| e.content.contains("Debugging") && e.content.contains("Python"));

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

#[test]
fn test_tag_extraction_on_sample_with_mock() {
    // Load corpus
    let entries = load_corpus(None).unwrap();
    let sample = &entries[0]; // Use first entry

    // Create mock client that returns expected JSON format
    let mock_response =
        r#"{"rust": 0.95, "async": 0.9, "tokio": 0.85, "concurrency": 0.75}"#.to_string();
    let mock = MockOllamaClient {
        response: mock_response,
    };

    // Create tagger with mock client
    let tagger = AutoTaggerBuilder::new().client(Arc::new(mock)).build();

    // Generate tags
    let result = tagger.generate_tags("test-model", &sample.content);
    assert!(result.is_ok(), "Tag generation should succeed");

    let tags = result.unwrap();

    // Verify we got some tags
    assert!(!tags.is_empty(), "Should extract at least some tags");

    // Use eval.rs metrics to verify quality (mock should be perfect)
    let (jaccard, precision, recall) = compare_tags(&sample.expected_tags, &tags);
    assert_eq!(
        jaccard, 1.0,
        "Mock should produce perfect Jaccard similarity"
    );
    assert_eq!(precision, 1.0, "Mock should produce perfect precision");
    assert_eq!(recall, 1.0, "Mock should produce perfect recall");
}

/// Integration test with real Ollama.
///
/// This test requires:
/// - Ollama running locally
/// - A model available (e.g., gemma3:4b or deepseek-r1:8b)
#[test]
fn test_real_ollama_integration() {
    if skip_in_ci() {
        return;
    }

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
    let tagger = AutoTaggerBuilder::new().client(Arc::new(client)).build();

    // Load corpus and test with first entry
    let entries = load_corpus(None).unwrap();
    let sample = &entries[0];

    // Get model from env or use default
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma3:4b".to_string());

    // Try to generate tags (may fail if Ollama is not available)
    let result = tagger.generate_tags(&model, &sample.content);

    match result {
        Ok(tags) => {
            println!("Successfully generated {} tags", tags.len());
            for (tag, confidence) in &tags {
                println!("  {}: {:.2}", tag, confidence);
            }

            // Use eval.rs to check quality - this is where regressions would be caught
            let (jaccard, precision, recall) = compare_tags(&sample.expected_tags, &tags);

            println!("Quality metrics:");
            println!("  Jaccard similarity: {:.3}", jaccard);
            println!("  Precision: {:.3}", precision);
            println!("  Recall: {:.3}", recall);

            // Quality thresholds - fail if quality drops too low (regression detection)
            // These thresholds are intentionally conservative - adjust based on baseline
            assert!(
                jaccard >= 0.4,
                "Jaccard similarity {} below threshold 0.4 - possible regression",
                jaccard
            );
            assert!(
                precision >= 0.5,
                "Precision {} below threshold 0.5 - possible regression",
                precision
            );
            assert!(
                recall >= 0.4,
                "Recall {} below threshold 0.4 - possible regression",
                recall
            );
        }
        Err(e) => {
            eprintln!(
                "Ollama call failed (this is OK if Ollama is not running): {}",
                e
            );
            // Don't fail the test - this is expected if Ollama is not available
        }
    }
}

/// Comprehensive evaluation test against full corpus.
///
/// Evaluates tag extraction quality across all corpus entries using eval.rs metrics.
/// This test can catch LLM regressions by comparing actual vs expected tags.
///
/// Requires:
/// - Ollama running locally
/// - Model available (set via OLLAMA_MODEL env var, defaults to gemma3:4b)
#[test]
fn test_evaluate_full_corpus() {
    if skip_in_ci() {
        return;
    }

    use cons::ollama::OllamaClientBuilder;

    // Create real Ollama client
    let client = match OllamaClientBuilder::new().build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping test: Failed to create Ollama client: {}", e);
            return;
        }
    };

    let tagger = AutoTaggerBuilder::new().client(Arc::new(client)).build();

    let entries = load_corpus(None).unwrap();
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma3:4b".to_string());

    println!(
        "Evaluating {} corpus entries with model: {}",
        entries.len(),
        model
    );

    let mut total_jaccard = 0.0;
    let mut total_precision = 0.0;
    let mut total_recall = 0.0;
    let mut successful = 0;

    for (i, entry) in entries.iter().enumerate() {
        match tagger.generate_tags(&model, &entry.content) {
            Ok(tags) => {
                let (jaccard, precision, recall) = compare_tags(&entry.expected_tags, &tags);

                total_jaccard += jaccard;
                total_precision += precision;
                total_recall += recall;
                successful += 1;

                println!(
                    "Entry {}: J={:.3} P={:.3} R={:.3} | Expected: {:?} | Got: {:?}",
                    i + 1,
                    jaccard,
                    precision,
                    recall,
                    entry.expected_tags,
                    tags.keys().collect::<Vec<_>>()
                );
            }
            Err(e) => {
                eprintln!("Entry {} failed: {}", i + 1, e);
            }
        }
    }

    if successful == 0 {
        eprintln!("No successful evaluations - Ollama may not be available");
        return;
    }

    let avg_jaccard = total_jaccard / successful as f64;
    let avg_precision = total_precision / successful as f64;
    let avg_recall = total_recall / successful as f64;

    println!("\nOverall metrics ({} successful evaluations):", successful);
    println!("  Average Jaccard similarity: {:.3}", avg_jaccard);
    println!("  Average Precision: {:.3}", avg_precision);
    println!("  Average Recall: {:.3}", avg_recall);

    // Overall quality thresholds - fail if average quality drops
    assert!(
        avg_jaccard >= 0.4,
        "Average Jaccard similarity {} below threshold 0.4 - possible regression",
        avg_jaccard
    );
    assert!(
        avg_precision >= 0.5,
        "Average precision {} below threshold 0.5 - possible regression",
        avg_precision
    );
    assert!(
        avg_recall >= 0.4,
        "Average recall {} below threshold 0.4 - possible regression",
        avg_recall
    );
}
