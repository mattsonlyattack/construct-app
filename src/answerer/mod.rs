//! Natural language query answering using LLMs.
//!
//! This module provides the `QueryAnswerer` struct which uses an Ollama-compatible
//! LLM to answer questions about notes with strict citation requirements.

mod query_answerer;
mod types;

pub use query_answerer::{QueryAnswerer, QueryAnswererBuilder};
pub use types::{Citation, QueryResult, QueryType};
