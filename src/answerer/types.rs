//! Types for natural language query results.

use crate::models::NoteId;

/// Types of natural language queries supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Question answering: "What did I write about X?"
    QuestionAnswering,
    /// Summarization: "Summarize my notes on Y"
    Summarization,
    /// Exploration: "What topics are related to Z?"
    Exploration,
}

impl QueryType {
    /// Parse from string representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "question_answering" | "questionanswering" => Some(Self::QuestionAnswering),
            "summarization" => Some(Self::Summarization),
            "exploration" => Some(Self::Exploration),
            _ => None,
        }
    }
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuestionAnswering => write!(f, "question_answering"),
            Self::Summarization => write!(f, "summarization"),
            Self::Exploration => write!(f, "exploration"),
        }
    }
}

/// A citation referencing a specific note used in the answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Citation {
    /// The note ID being cited
    note_id: NoteId,
    /// A relevant snippet from the note (up to ~100 chars)
    snippet: String,
    /// How relevant this note was to the answer (0.0-1.0)
    relevance: f64,
}

impl Citation {
    /// Creates a new citation.
    pub fn new(note_id: NoteId, snippet: String, relevance: f64) -> Self {
        Self {
            note_id,
            snippet,
            relevance: relevance.clamp(0.0, 1.0),
        }
    }

    /// Returns the note ID being cited.
    pub fn note_id(&self) -> NoteId {
        self.note_id
    }

    /// Returns the snippet from the note.
    pub fn snippet(&self) -> &str {
        &self.snippet
    }

    /// Returns the relevance score (0.0-1.0).
    pub fn relevance(&self) -> f64 {
        self.relevance
    }
}

/// Result of a natural language query over notes.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The generated answer text
    answer: String,
    /// Citations to source notes
    citations: Vec<Citation>,
    /// The original query
    query: String,
    /// Type of query that was processed
    query_type: QueryType,
    /// Model used to generate the answer
    model: String,
    /// True if the LLM determined no relevant notes exist for this query
    no_relevant_notes: bool,
    /// Optional explanation if no answer could be generated
    refusal_reason: Option<String>,
}

impl QueryResult {
    /// Creates a new successful query result with an answer.
    pub fn new(
        answer: String,
        citations: Vec<Citation>,
        query: String,
        query_type: QueryType,
        model: String,
    ) -> Self {
        Self {
            answer,
            citations,
            query,
            query_type,
            model,
            no_relevant_notes: false,
            refusal_reason: None,
        }
    }

    /// Creates a query result indicating no relevant notes were found.
    pub fn no_relevant_notes(query: String, model: String, reason: Option<String>) -> Self {
        Self {
            answer: String::new(),
            citations: Vec::new(),
            query,
            query_type: QueryType::QuestionAnswering,
            model,
            no_relevant_notes: true,
            refusal_reason: reason,
        }
    }

    /// Returns the answer text.
    pub fn answer(&self) -> &str {
        &self.answer
    }

    /// Returns the citations to source notes.
    pub fn citations(&self) -> &[Citation] {
        &self.citations
    }

    /// Returns the original query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns the type of query.
    pub fn query_type(&self) -> QueryType {
        self.query_type
    }

    /// Returns the model used.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns true if no relevant notes were found.
    pub fn is_no_relevant_notes(&self) -> bool {
        self.no_relevant_notes
    }

    /// Returns true if the result has a valid answer.
    pub fn has_answer(&self) -> bool {
        !self.no_relevant_notes && !self.answer.is_empty()
    }

    /// Returns the refusal reason if no answer was generated.
    pub fn refusal_reason(&self) -> Option<&str> {
        self.refusal_reason.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_type_parse() {
        assert_eq!(
            QueryType::parse("question_answering"),
            Some(QueryType::QuestionAnswering)
        );
        assert_eq!(
            QueryType::parse("summarization"),
            Some(QueryType::Summarization)
        );
        assert_eq!(
            QueryType::parse("exploration"),
            Some(QueryType::Exploration)
        );
        assert_eq!(QueryType::parse("unknown"), None);
    }

    #[test]
    fn query_type_display() {
        assert_eq!(QueryType::QuestionAnswering.to_string(), "question_answering");
        assert_eq!(QueryType::Summarization.to_string(), "summarization");
        assert_eq!(QueryType::Exploration.to_string(), "exploration");
    }

    #[test]
    fn citation_clamps_relevance() {
        let citation = Citation::new(NoteId::new(1), "test".to_string(), 1.5);
        assert_eq!(citation.relevance(), 1.0);

        let citation = Citation::new(NoteId::new(1), "test".to_string(), -0.5);
        assert_eq!(citation.relevance(), 0.0);
    }

    #[test]
    fn query_result_has_answer() {
        let result = QueryResult::new(
            "Answer".to_string(),
            vec![],
            "query".to_string(),
            QueryType::QuestionAnswering,
            "model".to_string(),
        );
        assert!(result.has_answer());

        let result = QueryResult::no_relevant_notes(
            "query".to_string(),
            "model".to_string(),
            Some("No relevant notes".to_string()),
        );
        assert!(!result.has_answer());
    }
}
