/// Ollama HTTP client module.
///
/// This module provides an async HTTP client for interacting with the Ollama API,
/// including error handling, retry logic, and timeout configuration.
mod client;

pub use client::{OllamaClient, OllamaClientBuilder, OllamaClientTrait, OllamaError};
