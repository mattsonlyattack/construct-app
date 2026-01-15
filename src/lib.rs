pub mod autotagger;
pub mod db;
pub mod doctor;
pub mod enhancer;
pub mod hierarchy;
pub mod models;
pub mod ollama;
pub mod service;
pub mod spreading_activation;
pub mod tui;
pub mod utils;

pub use autotagger::{AutoTagger, AutoTaggerBuilder, TagNormalizer};
pub use db::Database;
pub use enhancer::{EnhancementResult, NoteEnhancer, NoteEnhancerBuilder};
pub use hierarchy::{HierarchySuggester, HierarchySuggesterBuilder, RelationshipSuggestion};
pub use models::{AliasInfo, Note, NoteBuilder, NoteId, Tag, TagAssignment, TagId, TagSource};
pub use ollama::{OllamaClient, OllamaClientBuilder, OllamaClientTrait, OllamaError};
pub use service::{
    DualSearchConfig, DualSearchMetadata, DualSearchResult, ListNotesOptions, NoteService,
    QueryExpansionConfig, SearchResult, SortOrder,
};
pub use utils::{ensure_database_directory, get_database_path, get_tag_names};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_accessible_from_crate_root() {
        let db = Database::in_memory();
        assert!(db.is_ok());
    }

    #[test]
    fn types_accessible_from_crate_root() {
        use time::OffsetDateTime;

        let tag = Tag::new(TagId::new(1), "test");
        assert_eq!(tag.name(), "test");

        let source = TagSource::User;
        assert_eq!(format!("{}", source), "user");

        let now = OffsetDateTime::now_utc();
        let tag_assignment = TagAssignment::user(TagId::new(1), "test-tag", now);
        assert_eq!(tag_assignment.confidence(), 100);
        assert_eq!(tag_assignment.name(), "test-tag");

        let note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content("test")
            .build();
        assert_eq!(note.content(), "test");

        // Verify TagNormalizer is accessible from crate root
        let normalized = TagNormalizer::normalize_tag("Test Tag!");
        assert_eq!(normalized, "test-tag");
    }
}
