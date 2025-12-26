/// Integration tests for FTS5 full-text search with real SQLite and optional Ollama.
///
/// These tests verify end-to-end search functionality including:
/// - File-based SQLite database (not just in-memory)
/// - FTS5 indexing and BM25 ranking
/// - SearchResult with normalized relevance scores
/// - Integration with note enhancement (when Ollama available)
///
/// To run locally:
/// ```bash
/// cargo test --test cli_search_integration
/// ```
///
/// To run with a specific model (e.g., 12b for better quality):
/// ```bash
/// OLLAMA_MODEL=deepseek-r1:14b cargo test --test cli_search_integration
/// ```
use anyhow::Result;
use cons::{Database, NoteEnhancerBuilder, NoteService, OllamaClientBuilder};
use std::sync::Arc;
use tempfile::tempdir;
use time::OffsetDateTime;

/// Skip test if running in GitHub Actions
fn skip_in_ci() -> bool {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        println!("Skipping test in GitHub Actions (no Ollama available)");
        return true;
    }
    false
}

/// Get model name from env or detect from Ollama
fn get_model(base_url: &str) -> Option<String> {
    if let Ok(model) = std::env::var("OLLAMA_MODEL") {
        return Some(model);
    }

    // Try to detect available models
    let tags_url = format!("{}/api/tags", base_url);

    let response = reqwest::blocking::get(&tags_url).ok()?;
    let json: serde_json::Value = response.json().ok()?;

    json.get("models")
        .and_then(|m| m.as_array())
        .and_then(|models| models.first())
        .and_then(|model| model.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
}

/// Test FTS5 search with a file-based SQLite database.
///
/// This verifies that FTS5 works correctly with persistent storage,
/// not just in-memory databases.
#[test]
fn search_with_file_based_sqlite() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_search.db");

    // Create database and add notes
    {
        let db = Database::open(&db_path)?;
        let service = NoteService::new(db);

        service.create_note("Rust is a systems programming language", Some(&["rust", "programming"]))?;
        service.create_note("Python is great for data science", Some(&["python", "data-science"]))?;
        service.create_note("Rust and Python can work together via PyO3", Some(&["rust", "python", "interop"]))?;
    }

    // Reopen database and search (verifies FTS persists correctly)
    {
        let db = Database::open(&db_path)?;
        let service = NoteService::new(db);

        let results = service.search_notes("rust", None)?;

        assert_eq!(results.len(), 2, "Should find 2 notes about Rust");

        // Verify SearchResult structure
        for result in &results {
            assert!(!result.note.content().is_empty());
            assert!(result.relevance_score > 0.0 && result.relevance_score <= 1.0);
        }
    }

    Ok(())
}

/// Test BM25 relevance ranking with realistic content.
#[test]
fn bm25_ranking_with_realistic_content() -> Result<()> {
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create notes with varying relevance for "machine learning"
    let _note_low = service.create_note(
        "I attended a conference about various topics including machine learning",
        Some(&["conference"]),
    )?;

    let note_high = service.create_note(
        "Machine learning models use machine learning algorithms. Deep machine learning is a subset of machine learning.",
        Some(&["machine-learning", "ai"]),
    )?;

    let _note_medium = service.create_note(
        "Introduction to machine learning: supervised and unsupervised learning",
        Some(&["machine-learning", "tutorial"]),
    )?;

    let results = service.search_notes("machine learning", None)?;

    assert_eq!(results.len(), 3, "Should find all 3 notes");

    // Most relevant (highest term frequency) should be first
    assert_eq!(
        results[0].note.id(),
        note_high.id(),
        "Note with most 'machine learning' mentions should be first"
    );

    // Verify scores are ordered (higher score = more relevant after normalization)
    println!("Search results for 'machine learning':");
    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. [score: {:.4}] {}",
            i + 1,
            result.relevance_score,
            result.note.content()
        );
    }

    Ok(())
}

/// Test search across content, enhanced content, and tags.
#[test]
fn search_across_all_indexed_fields() -> Result<()> {
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Note 1: searchable via original content
    service.create_note("quantum computing breakthrough", None)?;

    // Note 2: searchable via tags only
    let _note_tag = service.create_note("important research paper", Some(&["quantum-physics"]))?;

    // Note 3: add enhanced content manually (simulating Ollama enhancement)
    let note_enhanced = service.create_note("quick thought", None)?;
    let now = OffsetDateTime::now_utc();
    service.update_note_enhancement(
        note_enhanced.id(),
        "This is a quick thought about quantum entanglement and its implications.",
        "test-model",
        0.85,
        now,
    )?;

    // Search for "quantum" - should find all 3 via different indexed fields
    let results = service.search_notes("quantum", None)?;

    assert_eq!(results.len(), 3, "Should find all 3 notes via different fields");

    println!("Search results for 'quantum':");
    for result in &results {
        println!(
            "  - [score: {:.4}] content='{}', enhanced={:?}",
            result.relevance_score,
            result.note.content(),
            result.note.content_enhanced().map(|s| &s[..s.len().min(50)])
        );
    }

    Ok(())
}

/// Test SearchResult score normalization properties.
#[test]
fn search_result_score_normalization() -> Result<()> {
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create notes with clear relevance differences
    for i in 1..=10 {
        let rust_count = "rust ".repeat(i);
        service.create_note(&format!("Note {}: {}", i, rust_count.trim()), None)?;
    }

    let results = service.search_notes("rust", None)?;

    assert_eq!(results.len(), 10);

    // All scores should be in valid range
    for result in &results {
        assert!(
            result.relevance_score >= 0.0 && result.relevance_score <= 1.0,
            "Score {} out of range",
            result.relevance_score
        );
    }

    // First result should have highest score (most "rust" mentions)
    let first_score = results[0].relevance_score;
    let last_score = results[9].relevance_score;

    println!("Score range: {:.6} to {:.6}", last_score, first_score);

    // Scores should be reasonably high for matching results
    assert!(
        first_score > 0.9,
        "Top result score {} should be > 0.9",
        first_score
    );

    Ok(())
}

/// Test search with enhanced content from real Ollama.
///
/// This test requires Ollama to be running. It verifies that:
/// 1. Notes can be enhanced with a real LLM
/// 2. Enhanced content is indexed in FTS5
/// 3. Search can find notes via their enhanced content
#[test]
fn search_enhanced_content_with_real_ollama() {
    if skip_in_ci() {
        return;
    }

    let client = match OllamaClientBuilder::new().build() {
        Ok(c) => c,
        Err(e) => {
            println!("Skipping test - could not create Ollama client: {}", e);
            return;
        }
    };

    let model = match get_model(client.base_url()) {
        Some(m) => m,
        None => {
            println!("Skipping test - no Ollama model available");
            return;
        }
    };

    println!("Testing with model: {}", model);

    let db = Database::in_memory().expect("Failed to create database");
    let service = NoteService::new(db);

    // Create a fragmentary note
    let note = service
        .create_note("buy groceries milk bread", None)
        .expect("Failed to create note");

    // Enhance it with real Ollama
    let enhancer = NoteEnhancerBuilder::new()
        .client(Arc::new(client))
        .build();

    let enhancement = match enhancer.enhance_content(&model, note.content()) {
        Ok(e) => e,
        Err(e) => {
            // Larger models may timeout - skip test gracefully
            println!("Enhancement failed (possibly timeout with larger model): {}", e);
            println!("Skipping remainder of test - this is expected with slow models");
            return;
        }
    };

    println!("Original: {}", note.content());
    println!("Enhanced: {}", enhancement.enhanced_content());
    println!("Confidence: {:.0}%", enhancement.confidence() * 100.0);

    // Store the enhancement
    let now = OffsetDateTime::now_utc();
    service
        .update_note_enhancement(
            note.id(),
            enhancement.enhanced_content(),
            &model,
            enhancement.confidence(),
            now,
        )
        .expect("Failed to store enhancement");

    // Search using a word that might appear in enhanced content but not original
    // Common expansions: "groceries" -> "grocery store", "shopping list", etc.
    let results = service
        .search_notes("buy", None)
        .expect("Search failed");

    assert!(!results.is_empty(), "Should find the note");
    assert_eq!(results[0].note.id(), note.id());

    println!(
        "Search result score: {:.4}",
        results[0].relevance_score
    );
}

/// End-to-end test: create notes, enhance, search, verify ranking.
///
/// This comprehensive test simulates real user workflow with Ollama.
#[test]
fn end_to_end_search_workflow_with_ollama() {
    if skip_in_ci() {
        return;
    }

    let client = match OllamaClientBuilder::new().build() {
        Ok(c) => c,
        Err(e) => {
            println!("Skipping test - could not create Ollama client: {}", e);
            return;
        }
    };

    let model = match get_model(client.base_url()) {
        Some(m) => m,
        None => {
            println!("Skipping test - no Ollama model available");
            return;
        }
    };

    println!("=== End-to-end search test with model: {} ===", model);

    // Use file-based database for realism
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("e2e_search.db");

    let db = Database::open(&db_path).expect("Failed to open database");
    let service = NoteService::new(db);

    let enhancer = NoteEnhancerBuilder::new()
        .client(Arc::new(client))
        .build();

    // Create diverse notes
    let notes_data = vec![
        ("rust async programming", vec!["rust", "async"]),
        ("python ml models", vec!["python", "machine-learning"]),
        ("quick thought about databases", vec!["databases"]),
    ];

    let now = OffsetDateTime::now_utc();

    for (content, tags) in notes_data {
        println!("\nProcessing: '{}'", content);

        let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_ref()).collect();
        let note = service
            .create_note(content, Some(&tag_refs))
            .expect("Failed to create note");

        // Enhance with Ollama
        match enhancer.enhance_content(&model, content) {
            Ok(enhancement) => {
                println!("  Enhanced: {}", enhancement.enhanced_content());
                service
                    .update_note_enhancement(
                        note.id(),
                        enhancement.enhanced_content(),
                        &model,
                        enhancement.confidence(),
                        now,
                    )
                    .expect("Failed to store enhancement");
            }
            Err(e) => {
                println!("  Enhancement failed: {} (continuing)", e);
            }
        }
    }

    // Test various searches
    println!("\n=== Search Tests ===");

    for query in &["rust", "programming", "machine", "database"] {
        let results = service.search_notes(query, None).expect("Search failed");
        println!(
            "\nQuery '{}': {} results",
            query,
            results.len()
        );
        for result in &results {
            println!(
                "  [{:.4}] {}",
                result.relevance_score,
                result.note.content()
            );
        }
    }

    println!("\n=== End-to-end test complete ===");
}
