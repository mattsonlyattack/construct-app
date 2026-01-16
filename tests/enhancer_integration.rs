/// Integration tests for NoteEnhancer with real Ollama.
///
/// These tests require a running Ollama instance. They are automatically
/// skipped in GitHub Actions CI where Ollama isn't available.
///
/// To run locally (with Ollama running):
/// ```bash
/// cargo test --test enhancer_integration
/// ```
use cons::{NoteEnhancerBuilder, OllamaClientBuilder};
use std::sync::Arc;

/// Load environment from .env file (same as main app)
fn load_env() {
    let _ = dotenvy::dotenv();
}

/// Skip test if running in GitHub Actions
fn skip_in_ci() -> bool {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        println!("Skipping test in GitHub Actions (no Ollama available)");
        return true;
    }
    false
}

/// Get model name from env or detect from Ollama
fn get_model(base_url: &str) -> String {
    if let Ok(model) = std::env::var("OLLAMA_MODEL") {
        return model;
    }

    // Detect available models
    let tags_url = format!("{}/api/tags", base_url);

    let response = reqwest::blocking::get(&tags_url)
        .unwrap_or_else(|e| panic!("Could not connect to Ollama at {}: {}", base_url, e));

    let json: serde_json::Value = response
        .json()
        .unwrap_or_else(|e| panic!("Could not parse Ollama response: {}", e));

    json.get("models")
        .and_then(|m| m.as_array())
        .and_then(|models| models.first())
        .and_then(|model| model.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| panic!("No models available in Ollama"))
}

/// Test that NoteEnhancer can enhance a fragmentary note with real Ollama.
#[test]
fn enhance_fragmentary_note_with_real_ollama() {
    load_env();
    if skip_in_ci() {
        return;
    }

    let client = OllamaClientBuilder::new()
        .build()
        .expect("Failed to create Ollama client");

    let model = get_model(client.base_url());
    println!("Using model: {}", model);

    let enhancer = NoteEnhancerBuilder::new().client(Arc::new(client)).build();

    // Test with a fragmentary note
    let result = enhancer.enhance_content(&model, "buy milk eggs bread");

    match result {
        Ok(enhancement) => {
            println!("Original: buy milk eggs bread");
            println!("Enhanced: {}", enhancement.enhanced_content());
            println!("Confidence: {:.0}%", enhancement.confidence() * 100.0);

            // Enhanced content should be non-empty
            assert!(
                !enhancement.enhanced_content().is_empty(),
                "Enhanced content should not be empty"
            );

            // Confidence should be in valid range
            assert!(
                (0.0..=1.0).contains(&enhancement.confidence()),
                "Confidence {} should be between 0.0 and 1.0",
                enhancement.confidence()
            );

            // Enhanced content should be longer or equal (expanded)
            assert!(
                enhancement.enhanced_content().len() >= "buy milk eggs bread".len(),
                "Enhanced content should be at least as long as original"
            );
        }
        Err(e) => {
            panic!("Enhancement failed: {}", e);
        }
    }
}

/// Test that NoteEnhancer handles already-complete notes appropriately.
#[test]
fn enhance_complete_note_with_real_ollama() {
    load_env();
    if skip_in_ci() {
        return;
    }

    let client = OllamaClientBuilder::new()
        .build()
        .expect("Failed to create Ollama client");

    let model = get_model(client.base_url());
    println!("Using model: {}", model);

    let enhancer = NoteEnhancerBuilder::new().client(Arc::new(client)).build();

    // Test with an already-complete note
    let complete_note = "Remember to buy milk, eggs, and bread from the grocery store tomorrow.";
    let result = enhancer.enhance_content(&model, complete_note);

    match result {
        Ok(enhancement) => {
            println!("Original: {}", complete_note);
            println!("Enhanced: {}", enhancement.enhanced_content());
            println!("Confidence: {:.0}%", enhancement.confidence() * 100.0);

            // Enhanced content should not be empty
            assert!(
                !enhancement.enhanced_content().is_empty(),
                "Enhanced content should not be empty"
            );

            // For complete notes, confidence should be relatively high
            // (model is confident because there's nothing ambiguous to expand)
            assert!(
                enhancement.confidence() >= 0.5,
                "Confidence for complete note should be >= 0.5, got {}",
                enhancement.confidence()
            );
        }
        Err(e) => {
            panic!("Enhancement failed: {}", e);
        }
    }
}
