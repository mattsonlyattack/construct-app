//! Query answering implementation using LLMs.

use std::collections::HashSet;
use std::sync::Arc;

use crate::models::NoteId;
use crate::ollama::{OllamaClientTrait, OllamaError};
use crate::service::DualSearchResult;

use super::types::{Citation, QueryResult, QueryType};

/// Prompt template for answering queries with strict citation.
const PROMPT_TEMPLATE: &str = r#"You are a knowledge retrieval assistant. Answer the user's question using ONLY the notes provided below. You MUST cite specific notes by their ID.

CRITICAL RULES:
1. ONLY use information from the provided notes - do not add external knowledge
2. Every claim must reference at least one note by its ID
3. If no notes are relevant to the question, respond with NO_RELEVANT_NOTES in the answer field
4. Include actual text snippets from notes in your citations
5. If you're uncertain, say so rather than guess

USER QUERY:
{query}

AVAILABLE NOTES:
{notes_context}

Respond in JSON format:
{
  "answer": "Your answer referencing notes by ID like [note:42]...",
  "citations": [
    {"note_id": 42, "snippet": "relevant text from note", "relevance": 0.9},
    {"note_id": 15, "snippet": "another relevant excerpt", "relevance": 0.7}
  ],
  "query_type": "question_answering|summarization|exploration",
  "no_relevant_notes": false
}

If no relevant notes exist:
{
  "answer": "",
  "citations": [],
  "query_type": "question_answering",
  "no_relevant_notes": true,
  "refusal_reason": "explanation of why no notes are relevant"
}

JSON OUTPUT:"#;

/// Builder for constructing `QueryAnswerer` instances.
#[derive(Default)]
pub struct QueryAnswererBuilder {
    client: Option<Arc<dyn OllamaClientTrait>>,
}

impl QueryAnswererBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Ollama client to use.
    pub fn client(mut self, client: Arc<dyn OllamaClientTrait>) -> Self {
        self.client = Some(client);
        self
    }

    /// Builds the `QueryAnswerer`.
    ///
    /// # Panics
    ///
    /// Panics if `client()` was not called.
    #[must_use]
    pub fn build(self) -> QueryAnswerer {
        QueryAnswerer {
            client: self.client.expect("client must be set via client() method"),
        }
    }
}

/// Answers natural language queries over notes using LLM.
pub struct QueryAnswerer {
    client: Arc<dyn OllamaClientTrait>,
}

impl QueryAnswerer {
    /// Creates a new `QueryAnswerer` with the specified client.
    #[must_use]
    pub fn new(client: Arc<dyn OllamaClientTrait>) -> Self {
        Self { client }
    }

    /// Answers a query using the provided notes as context.
    ///
    /// # Arguments
    ///
    /// * `model` - Ollama model to use
    /// * `query` - The natural language query
    /// * `notes` - Retrieved notes to use as context (from dual_search)
    ///
    /// # Returns
    ///
    /// `QueryResult` with answer and citations, or refusal if no relevant notes.
    pub fn answer_query(
        &self,
        model: &str,
        query: &str,
        notes: &[DualSearchResult],
    ) -> Result<QueryResult, OllamaError> {
        // Build notes context for the prompt
        let notes_context = format_notes_context(notes);

        // Construct prompt
        let prompt = PROMPT_TEMPLATE
            .replace("{query}", query)
            .replace("{notes_context}", &notes_context);

        // Call LLM
        let response = self.client.generate(model, &prompt)?;

        // Extract and parse JSON response
        let json_str = extract_json(&response).ok_or_else(|| OllamaError::Api {
            message: "Failed to extract JSON from LLM response".to_string(),
        })?;

        // Parse the response
        let mut result = parse_query_result(&json_str, query, model)?;

        // Validate citations - reject any hallucinated note IDs
        let valid_ids: HashSet<i64> = notes.iter().map(|r| r.note.id().get()).collect();
        result = validate_citations(result, &valid_ids);

        Ok(result)
    }
}

/// Formats notes into context for the prompt.
fn format_notes_context(notes: &[DualSearchResult]) -> String {
    notes
        .iter()
        .map(|result| {
            let note = &result.note;
            let content = note.content_enhanced().unwrap_or_else(|| note.content());

            // Truncate very long notes
            let content = if content.len() > 1000 {
                format!("{}...", &content[..1000])
            } else {
                content.to_string()
            };

            let tags: Vec<&str> = note.tags().iter().map(|t| t.name()).collect();
            let tags_str = if tags.is_empty() {
                String::new()
            } else {
                format!("\nTags: {}", tags.join(", "))
            };

            format!(
                "[NOTE ID={}]\nCreated: {}\nContent: {}{}\nRelevance: {:.2}\n---",
                note.id().get(),
                note.created_at()
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "unknown".to_string()),
                content,
                tags_str,
                result.final_score
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Extracts JSON from model response.
fn extract_json(response: &str) -> Option<String> {
    let trimmed = response.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;

    if start <= end {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

/// Parses JSON into QueryResult.
fn parse_query_result(json_str: &str, query: &str, model: &str) -> Result<QueryResult, OllamaError> {
    let json_value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| OllamaError::Api {
            message: format!("Failed to parse JSON: {}", e),
        })?;

    let obj = json_value.as_object().ok_or_else(|| OllamaError::Api {
        message: "Expected JSON object".to_string(),
    })?;

    // Check if no relevant notes
    let no_relevant = obj
        .get("no_relevant_notes")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if no_relevant {
        let refusal_reason = obj
            .get("refusal_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        return Ok(QueryResult::no_relevant_notes(
            query.to_string(),
            model.to_string(),
            refusal_reason,
        ));
    }

    // Extract answer
    let answer = obj
        .get("answer")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Check for NO_RELEVANT_NOTES marker in answer
    if answer.contains("NO_RELEVANT_NOTES") || answer.is_empty() {
        return Ok(QueryResult::no_relevant_notes(
            query.to_string(),
            model.to_string(),
            Some("No relevant notes found for this query".to_string()),
        ));
    }

    // Extract query type
    let query_type = obj
        .get("query_type")
        .and_then(|v| v.as_str())
        .and_then(QueryType::parse)
        .unwrap_or(QueryType::QuestionAnswering);

    // Extract citations
    let citations = obj
        .get("citations")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let note_id = c.get("note_id").and_then(|v| v.as_i64())?;
                    let snippet = c
                        .get("snippet")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let relevance = c.get("relevance").and_then(|v| v.as_f64()).unwrap_or(0.5);

                    Some(Citation::new(NoteId::new(note_id), snippet, relevance))
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(QueryResult::new(
        answer,
        citations,
        query.to_string(),
        query_type,
        model.to_string(),
    ))
}

/// Validates citations and removes any with hallucinated note IDs.
fn validate_citations(result: QueryResult, valid_ids: &HashSet<i64>) -> QueryResult {
    // If result has citations, filter out invalid ones
    if !result.citations().is_empty() {
        let valid_citations: Vec<Citation> = result
            .citations()
            .iter()
            .filter(|c| valid_ids.contains(&c.note_id().get()))
            .cloned()
            .collect();

        // If all citations were invalid, treat as no relevant notes
        if valid_citations.is_empty() && !result.is_no_relevant_notes() {
            return QueryResult::no_relevant_notes(
                result.query().to_string(),
                result.model().to_string(),
                Some("LLM response contained no valid citations".to_string()),
            );
        }

        // Reconstruct with valid citations only
        if valid_citations.len() != result.citations().len() {
            return QueryResult::new(
                result.answer().to_string(),
                valid_citations,
                result.query().to_string(),
                result.query_type(),
                result.model().to_string(),
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoteBuilder;

    struct MockOllamaClient {
        response: String,
    }

    impl OllamaClientTrait for MockOllamaClient {
        fn generate(&self, _model: &str, _prompt: &str) -> Result<String, OllamaError> {
            Ok(self.response.clone())
        }
    }

    fn make_dual_search_result(id: i64, content: &str) -> DualSearchResult {
        let note = NoteBuilder::new()
            .id(NoteId::new(id))
            .content(content)
            .build();

        DualSearchResult {
            note,
            final_score: 0.8,
            fts_score: Some(0.7),
            graph_score: Some(0.5),
            found_by_both: true,
        }
    }

    #[test]
    fn test_query_answerer_builder() {
        let mock = MockOllamaClient {
            response: r#"{"answer": "Test", "citations": [], "query_type": "question_answering", "no_relevant_notes": false}"#.to_string(),
        };

        let answerer = QueryAnswererBuilder::new()
            .client(Arc::new(mock))
            .build();

        let notes = vec![make_dual_search_result(1, "Test note")];
        let result = answerer.answer_query("test-model", "test query", &notes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_answer_query_with_citations() {
        let mock = MockOllamaClient {
            response: r#"{
                "answer": "Based on your notes, you wrote about Rust [note:42].",
                "citations": [
                    {"note_id": 42, "snippet": "Learning Rust today", "relevance": 0.9}
                ],
                "query_type": "question_answering",
                "no_relevant_notes": false
            }"#
            .to_string(),
        };

        let answerer = QueryAnswerer::new(Arc::new(mock));
        let notes = vec![make_dual_search_result(42, "Learning Rust today")];

        let result = answerer
            .answer_query("test-model", "What did I write about Rust?", &notes)
            .unwrap();

        assert!(result.has_answer());
        assert_eq!(result.citations().len(), 1);
        assert_eq!(result.citations()[0].note_id().get(), 42);
    }

    #[test]
    fn test_no_relevant_notes_response() {
        let mock = MockOllamaClient {
            response: r#"{
                "answer": "",
                "citations": [],
                "query_type": "question_answering",
                "no_relevant_notes": true,
                "refusal_reason": "No notes discuss quantum physics"
            }"#
            .to_string(),
        };

        let answerer = QueryAnswerer::new(Arc::new(mock));
        let notes = vec![make_dual_search_result(1, "Test note")];

        let result = answerer
            .answer_query("test-model", "What about quantum physics?", &notes)
            .unwrap();

        assert!(!result.has_answer());
        assert!(result.is_no_relevant_notes());
        assert_eq!(
            result.refusal_reason(),
            Some("No notes discuss quantum physics")
        );
    }

    #[test]
    fn test_hallucinated_citation_removed() {
        let mock = MockOllamaClient {
            response: r#"{
                "answer": "Based on note [note:999]",
                "citations": [
                    {"note_id": 999, "snippet": "hallucinated", "relevance": 0.9}
                ],
                "query_type": "question_answering",
                "no_relevant_notes": false
            }"#
            .to_string(),
        };

        let answerer = QueryAnswerer::new(Arc::new(mock));
        // Note ID 42 exists, but citation references 999
        let notes = vec![make_dual_search_result(42, "Real note")];

        let result = answerer
            .answer_query("test-model", "test query", &notes)
            .unwrap();

        // Should be treated as no relevant notes since citation was invalid
        assert!(result.is_no_relevant_notes());
    }

    #[test]
    fn test_extract_json_handles_markdown() {
        let response = r#"Here's my answer:

```json
{"answer": "Test", "citations": [], "no_relevant_notes": false}
```

Hope this helps!"#;

        let json = extract_json(response);
        assert!(json.is_some());
        assert!(json.unwrap().contains("answer"));
    }

    #[test]
    fn test_format_notes_context() {
        let notes = vec![
            make_dual_search_result(1, "First note"),
            make_dual_search_result(2, "Second note"),
        ];

        let context = format_notes_context(&notes);

        assert!(context.contains("[NOTE ID=1]"));
        assert!(context.contains("[NOTE ID=2]"));
        assert!(context.contains("First note"));
        assert!(context.contains("Second note"));
    }

    #[test]
    fn test_query_type_parsing() {
        let mock = MockOllamaClient {
            response: r#"{
                "answer": "Summary of notes",
                "citations": [{"note_id": 1, "snippet": "test", "relevance": 0.8}],
                "query_type": "summarization",
                "no_relevant_notes": false
            }"#
            .to_string(),
        };

        let answerer = QueryAnswerer::new(Arc::new(mock));
        let notes = vec![make_dual_search_result(1, "Test note")];

        let result = answerer
            .answer_query("test-model", "Summarize my notes", &notes)
            .unwrap();

        assert_eq!(result.query_type(), QueryType::Summarization);
    }
}
