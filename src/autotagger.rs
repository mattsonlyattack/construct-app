//! Auto-tagging functionality for extracting and normalizing tags from note content.
//!
//! This module provides components for LLM-based tag extraction with confidence scores
//! and robust post-processing for consistent tag formatting.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use std::sync::Arc;
//! use cons::autotagger::AutoTaggerBuilder;
//! use cons::ollama::OllamaClientBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Ollama client
//! let client = OllamaClientBuilder::new()
//!     .base_url("http://localhost:11434")
//!     .build()?;
//!
//! // Create AutoTagger using builder pattern
//! let tagger = AutoTaggerBuilder::new()
//!     .client(Arc::new(client))
//!     .build();
//!
//! // Generate tags for note content
//! let note_content = "Learning async Rust programming with tokio runtime";
//! let tags = tagger.generate_tags("deepseek-r1:8b", note_content).await?;
//!
//! // Tags are returned as HashMap<String, f64> where:
//! // - Key: normalized tag name (e.g., "rust", "async-programming")
//! // - Value: confidence score (0.0-1.0)
//! for (tag, confidence) in &tags {
//!     println!("{}: {:.2}", tag, confidence);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Integration with NoteService
//!
//! ```no_run
//! use std::sync::Arc;
//! use cons::autotagger::AutoTaggerBuilder;
//! use cons::models::{TagSource, TagId};
//! use cons::ollama::OllamaClientBuilder;
//! use cons::service::NoteService;
//! use time::OffsetDateTime;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Setup
//! let db = cons::db::Database::in_memory()?;
//! let service = NoteService::new(db);
//!
//! let client = OllamaClientBuilder::new().build()?;
//! let tagger = AutoTaggerBuilder::new()
//!     .client(Arc::new(client))
//!     .build();
//!
//! // Create a note
//! let note = service.create_note("Learning Rust ownership patterns")?;
//!
//! // Generate tags
//! let tags = tagger.generate_tags("deepseek-r1:8b", note.content()).await?;
//!
//! // Add tags to note with LLM source
//! for (tag_name, confidence) in tags {
//!     let tag_id = service.create_tag_if_not_exists(&tag_name)?;
//!     let tag_source = TagSource::llm(
//!         "deepseek-r1:8b".to_string(),
//!         (confidence * 100.0) as u8,
//!     );
//!     service.add_tags_to_note(note.id(), &[tag_id], tag_source)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Prompt Evaluation
//!
//! This module includes evaluation tools for iterating on prompt design:
//!
//! ### Test Corpus
//!
//! A test corpus is available at `tests/fixtures/auto_tagger_corpus.json` with sample notes
//! and expected tags. This corpus includes:
//! - Mix of short and longer notes
//! - "Aboutness vs mention" test cases
//! - Technical, personal, and mixed content
//!
//! ### Running Evaluation Tests
//!
//! Evaluation tests are in `tests/autotagger_evaluation.rs`. To run them:
//!
//! ```bash
//! # Run all evaluation tests (non-ignored)
//! cargo test --test autotagger_evaluation
//!
//! # Run ignored tests (requires Ollama running locally)
//! cargo test --test autotagger_evaluation -- --ignored
//! ```
//!
//! ### Adding New Test Cases
//!
//! To add new test cases to the corpus, edit `tests/fixtures/auto_tagger_corpus.json`:
//!
//! ```json
//! {
//!   "content": "Your note content here",
//!   "expected_tags": ["tag1", "tag2", "tag3"],
//!   "notes": "Description of what this test case validates"
//! }
//! ```
//!
//! ### Evaluation Metrics
//!
//! The `eval` module provides helper functions for comparing expected vs actual tags:
//!
//! - `jaccard_similarity()` - Measures set overlap (intersection / union)
//! - `precision_recall()` - Measures extraction accuracy
//! - `compare_tags()` - Convenience function combining both metrics
//!
//! ```no_run
//! use cons::autotagger::{load_corpus, compare_tags};
//! use std::collections::HashMap;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load corpus
//! let entries = load_corpus(None)?;
//!
//! // Generate tags (example)
//! let actual_tags: HashMap<String, f64> = HashMap::new(); // ... from AutoTagger
//!
//! // Compare with expected
//! let entry = &entries[0];
//! let (jaccard, precision, recall) = compare_tags(&entry.expected_tags, &actual_tags);
//!
//! println!("Jaccard: {:.2}, Precision: {:.2}, Recall: {:.2}", jaccard, precision, recall);
//! # Ok(())
//! # }
//! ```
//!
//! ### Iterating on Prompts
//!
//! This evaluation foundation enables systematic prompt iteration:
//!
//! 1. Modify the prompt template in `tagger.rs`
//! 2. Run evaluation tests to measure impact
//! 3. Compare metrics across different models
//! 4. Add new test cases to corpus as edge cases are discovered
//!
//! The goal is to improve tag extraction quality while maintaining model-agnostic compatibility.

mod eval;
mod normalizer;
mod tagger;

pub use eval::{compare_tags, load_corpus, precision_recall, jaccard_similarity, CorpusEntry};
pub use normalizer::TagNormalizer;
pub use tagger::{AutoTagger, AutoTaggerBuilder};

