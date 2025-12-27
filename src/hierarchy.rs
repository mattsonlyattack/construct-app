//! Tag hierarchy suggestion functionality using LLMs to identify semantic relationships.
//!
//! This module provides components for analyzing existing tags and identifying
//! broader/narrower relationships using XKOS semantics (generic vs partitive hierarchies).
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use std::sync::Arc;
//! use cons::hierarchy::HierarchySuggesterBuilder;
//! use cons::ollama::OllamaClientBuilder;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Ollama client
//! let client = OllamaClientBuilder::new()
//!     .base_url("http://localhost:11434")
//!     .build()?;
//!
//! // Create HierarchySuggester using builder pattern
//! let suggester = HierarchySuggesterBuilder::new()
//!     .client(Arc::new(client))
//!     .build();
//!
//! // Analyze tags and suggest relationships
//! let tag_names = vec![
//!     "transformer".to_string(),
//!     "neural-network".to_string(),
//!     "attention".to_string(),
//! ];
//! let suggestions = suggester.suggest_relationships("deepseek-r1:8b", tag_names)?;
//!
//! // Process suggestions (only those with confidence >= 0.7 are returned)
//! for suggestion in &suggestions {
//!     println!(
//!         "{} -> {} ({}, {:.2} confidence)",
//!         suggestion.source_tag,
//!         suggestion.target_tag,
//!         suggestion.hierarchy_type,
//!         suggestion.confidence
//!     );
//! }
//! # Ok(())
//! # }
//! ```

mod suggester;

pub use suggester::{HierarchySuggester, HierarchySuggesterBuilder, RelationshipSuggestion};
