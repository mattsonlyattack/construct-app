/// Integration tests for Ollama HTTP client.
///
/// These tests require a running Ollama instance. They are automatically
/// skipped in GitHub Actions CI where Ollama isn't available.
///
/// To run locally (with Ollama running):
/// ```bash
/// cargo test --test ollama_integration
/// ```
use cons::{OllamaClientBuilder, OllamaClientTrait};

/// Skip test if running in GitHub Actions
fn skip_in_ci() -> bool {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        println!("Skipping test in GitHub Actions (no Ollama available)");
        return true;
    }
    false
}

/// Test that the Ollama client can successfully call a real Ollama instance.
///
/// This test requires:
/// - Ollama running locally (default: http://172.17.64.1:11434 or OLLAMA_HOST env var)
/// - A model available (specify via OLLAMA_MODEL env var, or auto-detects)
#[test]
fn generate_with_real_ollama_instance() {
    if skip_in_ci() {
        return;
    }

    // Build client using default configuration
    let client = OllamaClientBuilder::new()
        .build()
        .expect("Failed to create Ollama client");

    // Get model name from environment, or try to detect available models
    let model = if let Ok(env_model) = std::env::var("OLLAMA_MODEL") {
        env_model
    } else {
        // Try to detect available models by checking Ollama's /api/tags endpoint
        let base_url = client.base_url();
        let tags_url = format!("{}/api/tags", base_url);

        let response = reqwest::blocking::get(&tags_url).unwrap_or_else(|e| {
            panic!(
                "OLLAMA_MODEL not set and could not connect to Ollama at {} to detect models: {}",
                base_url, e
            );
        });

        let json: serde_json::Value = response.json().unwrap_or_else(|e| {
            panic!(
                "OLLAMA_MODEL not set and could not parse Ollama /api/tags response: {}",
                e
            );
        });

        let model_name = json
            .get("models")
            .and_then(|m| m.as_array())
            .and_then(|models| models.first())
            .and_then(|model| model.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_else(|| {
                panic!("OLLAMA_MODEL not set and could not detect model name from Ollama response");
            });

        println!("Detected available model: {}", model_name);
        model_name.to_string()
    };

    println!("Testing generation with model: {}", model);

    // Actually test generation - this should succeed
    let response = client
        .generate(&model, "Say hello in one word.")
        .unwrap_or_else(|_| {
            panic!(
                "Failed to generate text with model '{}'. Ensure Ollama is running and the model is available.",
                model
            );
        });

    // Verify we got a non-empty response
    assert!(
        !response.is_empty(),
        "Generated response should not be empty"
    );
    println!("Successfully generated: {}", response);
}

/// Test that the client handles connection errors gracefully when Ollama is not running.
///
/// This test verifies that the client properly handles the case where Ollama
/// is not available, which is important for fail-safe behavior.
#[test]
fn generate_handles_missing_ollama_gracefully() {
    if skip_in_ci() {
        return;
    }

    // Use a non-existent host to simulate Ollama not being available
    // Using a valid URL format but a host that won't respond
    let client = OllamaClientBuilder::new()
        .base_url("http://127.0.0.1:65535") // Valid URL but port unlikely to be in use
        .build()
        .expect("Failed to create Ollama client");

    let result = client.generate("test-model", "test prompt");

    // Should return an error, not panic
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = format!("{}", error);

    // Should be a network error (connection refused) or timeout
    assert!(
        error_msg.contains("Network error") || error_msg.contains("Request timed out"),
        "Expected network/timeout error, got: {}",
        error_msg
    );
}
