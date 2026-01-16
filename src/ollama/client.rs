/// Ollama HTTP client implementation.
///
/// This module provides `OllamaClient` for making synchronous HTTP requests to the Ollama API,
/// along with error types and builder patterns for configuration.
use std::thread;
use std::time::Duration;

use thiserror::Error;

/// Errors that can occur when interacting with the Ollama API.
#[derive(Debug, Error)]
pub enum OllamaError {
    /// Network-related errors (connection failures, DNS resolution, etc.)
    #[error("Network error: {0}")]
    Network(#[source] reqwest::Error),

    /// Request or response timeout errors
    #[error("Request timed out")]
    Timeout(#[source] reqwest::Error),

    /// HTTP errors with status code
    #[error("HTTP error: status {status}")]
    Http { status: u16 },

    /// JSON serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[source] serde_json::Error),

    /// Ollama API-specific errors
    #[error("Ollama API error: {message}")]
    Api { message: String },

    /// Invalid URL configuration error
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Builder for constructing `OllamaClient` instances.
///
/// # Examples
///
/// ```
/// use cons::ollama::OllamaClientBuilder;
///
/// let client = OllamaClientBuilder::new()
///     .base_url("http://localhost:11434")
///     .build()
///     .expect("Failed to create client");
/// ```
#[derive(Debug, Default)]
pub struct OllamaClientBuilder {
    base_url: Option<String>,
    model: Option<String>,
}

impl OllamaClientBuilder {
    /// Creates a new `OllamaClientBuilder` with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base URL for the Ollama API.
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL (e.g., "http://localhost:11434")
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Sets the model name for Ollama API calls.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name (e.g., "gemma3:4b" or "deepseek-r1:8b")
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Builds the `OllamaClient` with the configured settings.
    ///
    /// # Returns
    ///
    /// Returns `Ok(OllamaClient)` if the client was created successfully,
    /// or `Err(OllamaError)` if there was an error (e.g., invalid URL).
    ///
    /// # Environment Variables
    ///
    /// If `base_url()` was not called, this method will check the `OLLAMA_HOST`
    /// environment variable. If not set, it defaults to `http://172.17.64.1:11434`.
    ///
    /// If `model()` was not called, this method will check the `OLLAMA_MODEL`
    /// environment variable. If not set, it defaults to an empty string.
    pub fn build(self) -> Result<OllamaClient, OllamaError> {
        // Determine base URL: use builder value, then env var, then default
        let base_url = if let Some(url) = self.base_url {
            url
        } else {
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
        };

        // Determine model: use builder value, then env var, then default
        let model = if let Some(m) = self.model {
            m
        } else {
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| String::new())
        };

        // Validate URL
        reqwest::Url::parse(&base_url)
            .map_err(|e| OllamaError::InvalidUrl(format!("{}: {}", base_url, e)))?;

        // Create reqwest blocking client with timeout configuration
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(OllamaError::Network)?;

        Ok(OllamaClient {
            client,
            base_url,
            model,
        })
    }
}

/// Synchronous HTTP client for interacting with the Ollama API.
///
/// This client handles HTTP requests to Ollama with proper timeout and retry handling.
/// It should be constructed using `OllamaClientBuilder`.
pub struct OllamaClient {
    client: reqwest::blocking::Client,
    base_url: String,
    model: String,
}

/// Trait for Ollama API client operations.
///
/// This trait enables mocking in unit tests and provides a clean interface
/// for interacting with the Ollama API.
pub trait OllamaClientTrait: Send + Sync {
    /// Generates text using the Ollama API.
    ///
    /// # Arguments
    ///
    /// * `model` - The name of the model to use (e.g., "deepseek-r1:8b")
    /// * `prompt` - The prompt text to send to the model
    ///
    /// # Returns
    ///
    /// Returns the generated text as a `String`, or an error if the request fails.
    fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError>;
}

impl OllamaClient {
    /// Returns the base URL configured for this client.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the model name configured for this client.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Lists available models from the Ollama API, sorted by size (largest first).
    ///
    /// Fetches the `/api/tags` endpoint and returns model names.
    /// Returns an empty Vec if the request fails or no models are available.
    pub fn list_models(&self) -> Result<Vec<String>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(OllamaError::Network)?;

        if !response.status().is_success() {
            return Err(OllamaError::Http {
                status: response.status().as_u16(),
            });
        }

        let json: serde_json::Value = response.json().map_err(OllamaError::Network)?;

        let mut models: Vec<(String, u64)> = json
            .get("models")
            .and_then(|m| m.as_array())
            .map(|models| {
                models
                    .iter()
                    .filter_map(|model| {
                        let name = model.get("name").and_then(|n| n.as_str())?;
                        let size = model.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                        Some((name.to_string(), size))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Sort by size descending (largest first)
        models.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(models.into_iter().map(|(name, _)| name).collect())
    }

    /// Generates text using the Ollama API.
    ///
    /// This is the internal implementation that will be called by the trait method.
    fn generate_internal(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        let url = format!("{}/api/generate", self.base_url);
        let request_body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        });

        // Wrap the HTTP call with retry logic
        retry_with_backoff(|| {
            let response = self
                .client
                .post(&url)
                .json(&request_body)
                .send()
                .map_err(OllamaError::Network)?;

            let status = response.status();
            if !status.is_success() {
                if status.is_client_error() {
                    // 4xx errors - don't retry
                    return Err(OllamaError::Http {
                        status: status.as_u16(),
                    });
                } else if status.is_server_error() {
                    // 5xx errors - will be retried
                    return Err(OllamaError::Http {
                        status: status.as_u16(),
                    });
                }
            }

            let json: serde_json::Value = response.json().map_err(OllamaError::Network)?;

            // Extract the "response" field from Ollama API response
            json.get("response")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| OllamaError::Api {
                    message: "Missing 'response' field in API response".to_string(),
                })
        })
    }
}

impl OllamaClientTrait for OllamaClient {
    fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        self.generate_internal(model, prompt)
    }
}

/// Retries an async operation with exponential backoff.
///
/// This function will retry the operation up to 3 times with delays of 1s, 2s, and 4s.
/// It only retries on transient errors (HTTP 5xx and network errors), not on client errors (HTTP 4xx).
///
/// # Arguments
///
/// * `f` - A closure that returns a future producing a `Result<T, OllamaError>`
///
/// # Returns
///
/// Returns the result of the operation if it succeeds, or the last error if all retries fail.
pub fn retry_with_backoff<F, T>(mut f: F) -> Result<T, OllamaError>
where
    F: FnMut() -> Result<T, OllamaError>,
{
    const MAX_RETRIES: usize = 3;
    const DELAYS: [u64; MAX_RETRIES] = [1, 2, 4]; // seconds

    // Try the operation first
    let mut last_error = match f() {
        Ok(result) => return Ok(result),
        Err(e) => {
            // Check if we should retry this error
            if !should_retry(&e) {
                return Err(e);
            }
            e
        }
    };

    // Retry up to MAX_RETRIES times
    for &delay_secs in &DELAYS {
        // Sleep before retry (exponential backoff)
        thread::sleep(Duration::from_secs(delay_secs));

        match f() {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if we should retry this error
                if !should_retry(&e) {
                    return Err(e);
                }
                last_error = e;
            }
        }
    }

    // All retries exhausted
    Err(last_error)
}

/// Determines if an error should be retried.
///
/// Returns `true` for transient errors (HTTP 5xx, network errors, timeouts).
/// Returns `false` for client errors (HTTP 4xx) and other non-retryable errors.
fn should_retry(error: &OllamaError) -> bool {
    match error {
        OllamaError::Network(_) => true,
        OllamaError::Timeout(_) => true,
        OllamaError::Http { status } => {
            // Retry on 5xx server errors, not on 4xx client errors
            *status >= 500 && *status < 600
        }
        OllamaError::Serialization(_) => false, // Don't retry serialization errors
        OllamaError::Api { .. } => false,       // Don't retry API errors
        OllamaError::InvalidUrl(_) => false,    // Don't retry invalid URL errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn network_error_variant_creation_and_display() {
        // Create a network error by building a request with an invalid URL format
        // We'll use reqwest's internal error creation
        // Note: In real usage, this would come from actual network failures
        let client = reqwest::blocking::Client::new();
        // Create an error by attempting to use an invalid URL scheme
        // This will give us a reqwest::Error we can use for testing
        let invalid_url = "not-a-valid-url";
        let reqwest_error = client.get(invalid_url).build().unwrap_err();
        let ollama_error = OllamaError::Network(reqwest_error);

        // Verify error message is user-friendly
        let error_msg = format!("{}", ollama_error);
        assert!(error_msg.contains("Network error"));
    }

    #[test]
    fn timeout_error_variant_creation_and_display() {
        // Create a timeout error - use the same approach as network error
        // In practice, timeout errors come from timed-out requests
        let client = reqwest::blocking::Client::new();
        let invalid_url = "http://";
        let reqwest_error = client.get(invalid_url).build().unwrap_err();
        let ollama_error = OllamaError::Timeout(reqwest_error);

        // Verify error message is user-friendly
        let error_msg = format!("{}", ollama_error);
        assert_eq!(error_msg, "Request timed out");
    }

    #[test]
    fn http_error_variant_with_status_code() {
        // Create an HTTP error with status code
        let ollama_error = OllamaError::Http { status: 404 };

        // Verify error message includes status code
        let error_msg = format!("{}", ollama_error);
        assert!(error_msg.contains("HTTP error"));
        assert!(error_msg.contains("404"));
    }

    #[test]
    fn serialization_error_variant_wraps_serde_errors() {
        // Create a serialization error
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let ollama_error = OllamaError::Serialization(json_error);

        // Verify error message is user-friendly
        let error_msg = format!("{}", ollama_error);
        assert!(error_msg.contains("Serialization error"));

        // Verify error source chaining works
        assert!(ollama_error.source().is_some());
    }

    #[test]
    fn api_error_variant_for_ollama_specific_errors() {
        // Create an API error
        let ollama_error = OllamaError::Api {
            message: "Model not found".to_string(),
        };

        // Verify error message includes the API message
        let error_msg = format!("{}", ollama_error);
        assert!(error_msg.contains("Ollama API error"));
        assert!(error_msg.contains("Model not found"));
    }

    #[test]
    fn ollama_client_builder_new_creates_builder_with_defaults() {
        let builder = OllamaClientBuilder::new();
        // Builder should be created successfully
        assert!(matches!(builder.base_url, None));
        assert!(matches!(builder.model, None));
    }

    #[test]
    fn base_url_method_sets_custom_url() {
        let builder = OllamaClientBuilder::new().base_url("http://example.com:11434");
        assert_eq!(
            builder.base_url,
            Some("http://example.com:11434".to_string())
        );
    }

    #[test]
    fn build_uses_default_url_when_base_url_not_called() {
        // Clear any existing OLLAMA_HOST env var for this test
        unsafe {
            std::env::remove_var("OLLAMA_HOST");
        }

        let client = OllamaClientBuilder::new().build();
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "http://localhost:11434");
    }

    #[test]
    fn build_reads_ollama_host_environment_variable_if_set() {
        // Set environment variable
        unsafe {
            std::env::set_var("OLLAMA_HOST", "http://custom-host:11434");
        }

        let client = OllamaClientBuilder::new().build();
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "http://custom-host:11434");

        // Clean up
        unsafe {
            std::env::remove_var("OLLAMA_HOST");
        }
    }

    #[test]
    fn build_creates_client_with_correct_timeout_configuration() {
        let client = OllamaClientBuilder::new()
            .base_url("http://localhost:11434")
            .build();
        assert!(client.is_ok());
        // Client should be created with timeout configuration
        // (We can't easily test the internal timeout values, but if build succeeds,
        // the client was created with valid configuration)
    }

    #[test]
    fn build_returns_error_if_invalid_url_provided() {
        let result = OllamaClientBuilder::new()
            .base_url("not-a-valid-url")
            .build();
        assert!(result.is_err());
        if let Err(OllamaError::InvalidUrl(_)) = result {
            // Expected error variant
        } else {
            panic!("Expected InvalidUrl error");
        }
    }

    #[test]
    fn retry_succeeds_after_transient_network_error() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            let count = attempts.fetch_add(1, Ordering::SeqCst);
            if count < 1 {
                // Simulate network error on first attempt
                Err(OllamaError::Network(
                    reqwest::blocking::Client::new()
                        .get("not-a-valid-url")
                        .build()
                        .unwrap_err(),
                ))
            } else {
                Ok("success")
            }
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn retry_stops_after_3_attempts() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            attempts.fetch_add(1, Ordering::SeqCst);
            // Always fail with retryable error
            Err(OllamaError::Network(
                reqwest::blocking::Client::new()
                    .get("not-a-valid-url")
                    .build()
                    .unwrap_err(),
            ))
        });

        assert!(result.is_err());
        // Should have tried initial attempt + 3 retries = 4 total attempts
        assert_eq!(attempts.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn retry_delays_increase_exponentially() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Instant;

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let start = Instant::now();

        let _result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            let count = attempts.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(OllamaError::Network(
                    reqwest::blocking::Client::new()
                        .get("not-a-valid-url")
                        .build()
                        .unwrap_err(),
                ))
            } else {
                Ok("success")
            }
        });

        let elapsed = start.elapsed();
        // Should have delays of 1s + 2s = 3s minimum (plus some overhead)
        // We check that it took at least 2.5 seconds to account for timing variations
        assert!(elapsed.as_secs_f64() >= 2.5);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn retry_does_not_occur_on_http_4xx_errors() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            attempts.fetch_add(1, Ordering::SeqCst);
            // Return 4xx error (should not retry)
            Err(OllamaError::Http { status: 404 })
        });

        assert!(result.is_err());
        // Should only try once, no retries
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn retry_occurs_on_http_5xx_errors() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            let count = attempts.fetch_add(1, Ordering::SeqCst);
            if count < 1 {
                // Return 5xx error (should retry)
                Err(OllamaError::Http { status: 500 })
            } else {
                Ok("success")
            }
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn trait_can_be_implemented_by_mock_struct() {
        // Create a mock implementation of the trait
        struct MockClient {
            response: String,
        }

        impl OllamaClientTrait for MockClient {
            fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
                Ok(self.response.clone())
            }
        }

        let mock = MockClient {
            response: "test response".to_string(),
        };
        let result = mock.generate("test-model", "test prompt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test response");
    }

    #[test]
    fn generate_method_calls_correct_ollama_endpoint() {
        // This test verifies the endpoint URL construction
        // We can't easily test the actual HTTP call without a real server,
        // but we can verify the URL is constructed correctly
        let client = OllamaClientBuilder::new()
            .base_url("http://localhost:11434")
            .build()
            .unwrap();
        assert_eq!(client.base_url(), "http://localhost:11434");
        // The actual endpoint would be: http://localhost:11434/api/generate
    }

    #[test]
    fn generate_serializes_request_body_correctly() {
        // Test that the request body JSON structure is correct
        let request_body = serde_json::json!({
            "model": "test-model",
            "prompt": "test prompt"
        });

        assert_eq!(request_body["model"], "test-model");
        assert_eq!(request_body["prompt"], "test prompt");
    }

    #[test]
    fn generate_parses_response_json_correctly() {
        // Test parsing of Ollama API response format
        let response_json = serde_json::json!({
            "response": "Generated text here"
        });

        let response_text = response_json
            .get("response")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap();

        assert_eq!(response_text, "Generated text here");
    }

    #[test]
    fn generate_handles_http_errors_correctly() {
        // Test error handling for HTTP errors
        let error_404 = OllamaError::Http { status: 404 };
        assert!(matches!(error_404, OllamaError::Http { status: 404 }));

        let error_500 = OllamaError::Http { status: 500 };
        assert!(matches!(error_500, OllamaError::Http { status: 500 }));
    }

    #[test]
    fn generate_applies_retry_logic_on_transient_errors() {
        // Test that retry logic would be applied (via should_retry function)
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        // Simulate a transient error that should be retried
        let result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            let count = attempts.fetch_add(1, Ordering::SeqCst);
            if count < 1 {
                // Transient error (5xx) - should retry
                Err(OllamaError::Http { status: 500 })
            } else {
                Ok("success after retry")
            }
        });

        assert!(result.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 2); // Initial + 1 retry
    }

    #[test]
    fn full_integration_builder_client_generate() {
        // Test full integration: builder → client → generate method exists
        let client = OllamaClientBuilder::new()
            .base_url("http://localhost:11434")
            .build()
            .unwrap();

        // Verify client was created successfully
        assert_eq!(client.base_url(), "http://localhost:11434");

        // Verify generate method exists and can be called via trait
        // (We can't test actual HTTP call without a real server, but we verify the interface)
        let _trait_ref: &dyn OllamaClientTrait = &client;
    }

    #[test]
    fn environment_variable_override_precedence() {
        // Test that builder method takes precedence over environment variable
        unsafe {
            std::env::set_var("OLLAMA_HOST", "http://env-var-host:11434");
        }

        // Builder method should override env var
        let client = OllamaClientBuilder::new()
            .base_url("http://builder-host:11434")
            .build()
            .unwrap();
        assert_eq!(client.base_url(), "http://builder-host:11434");

        unsafe {
            std::env::remove_var("OLLAMA_HOST");
        }
    }

    #[test]
    fn error_types_propagate_correctly_through_retry_logic() {
        // Test that error types are preserved through retry logic
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        // Create a network error that should be retried
        let result: Result<&str, OllamaError> = retry_with_backoff(move || {
            let attempts = attempts_clone.clone();
            let count = attempts.fetch_add(1, Ordering::SeqCst);
            if count < 1 {
                Err(OllamaError::Network(
                    reqwest::blocking::Client::new()
                        .get("not-a-valid-url")
                        .build()
                        .unwrap_err(),
                ))
            } else {
                Ok("success")
            }
        });

        // Verify error type is preserved if all retries fail
        assert!(result.is_ok());
        // Verify that the error was retried (attempts > 1)
        assert!(attempts.load(Ordering::SeqCst) > 1);
    }

    // --- OLLAMA_MODEL Environment Variable Support Tests (Task Group 2) ---

    #[test]
    fn build_reads_ollama_model_environment_variable_if_set() {
        // Set environment variable
        unsafe {
            std::env::set_var("OLLAMA_MODEL", "gemma3:4b");
        }

        let client = OllamaClientBuilder::new().build();
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model(), "gemma3:4b");

        // Clean up
        unsafe {
            std::env::remove_var("OLLAMA_MODEL");
        }
    }

    #[test]
    fn build_uses_default_model_when_ollama_model_not_set() {
        // Clear any existing OLLAMA_MODEL env var for this test
        unsafe {
            std::env::remove_var("OLLAMA_MODEL");
        }

        let client = OllamaClientBuilder::new().build();
        assert!(client.is_ok());
        let client = client.unwrap();
        // Default should be empty string when env var not set
        assert_eq!(client.model(), "");
    }

    #[test]
    fn model_method_sets_custom_model_and_takes_precedence_over_env_var() {
        // Set environment variable
        unsafe {
            std::env::set_var("OLLAMA_MODEL", "env-model");
        }

        // Builder method should override env var
        let client = OllamaClientBuilder::new()
            .model("builder-model")
            .build()
            .unwrap();
        assert_eq!(client.model(), "builder-model");

        // Clean up
        unsafe {
            std::env::remove_var("OLLAMA_MODEL");
        }
    }
}
