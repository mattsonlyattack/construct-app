# Specification: Tag Hierarchy Population

## Goal
Implement a CLI command that uses LLM to analyze existing tags and automatically populate the edges table with broader/narrower relationships, distinguishing between generic (is-a) and partitive (part-of) hierarchy types using XKOS semantics.

## User Stories
- As a cons user, I want to run `cons hierarchy suggest` so that the system automatically discovers and creates meaningful relationships between my tags based on semantic analysis
- As a knowledge manager, I want the system to distinguish between is-a relationships (e.g., "transformer" specializes "neural-network") and part-of relationships (e.g., "attention" isPartOf "transformer") so that my tag hierarchy accurately represents different types of conceptual relationships

## Specific Requirements

**CLI Command Structure**
- Add new `hierarchy` subcommand group to the existing clap CLI structure
- Implement `cons hierarchy suggest` as the initial subcommand following existing patterns (TagAliasCommand, TagAliasCommands enum)
- Command takes no arguments; analyzes all tags in a single batch operation
- Follow the same database path resolution pattern used in handle_add, handle_list, etc.

**Tag Collection Query**
- Query only tags that have at least one associated note using JOIN with note_tags table
- Collect tag names and IDs for passing to LLM and subsequent edge creation
- Use existing NoteService database access patterns for query construction
- Handle empty tag sets gracefully (display message, no LLM call)

**LLM Prompt Design**
- Create a new prompt template in a dedicated module (e.g., `src/hierarchy.rs` or `src/hierarchy/suggester.rs`)
- Input: JSON array of tag names that have associated notes
- Output: JSON array of relationship objects with fields: source_tag (narrower), target_tag (broader), hierarchy_type ('generic' or 'partitive'), confidence (0.0-1.0)
- Include clear explanation of XKOS generic vs partitive distinction in prompt
- Follow PROMPT_TEMPLATE pattern from autotagger/tagger.rs with few-shot examples demonstrating both hierarchy types

**LLM Integration**
- Use existing OllamaClientBuilder and OllamaClientTrait patterns for client construction
- Read model from OLLAMA_MODEL environment variable (same as auto_tag_note)
- Build a HierarchySuggesterBuilder following AutoTaggerBuilder pattern with Arc-wrapped client
- Implement extract_json and parse patterns from autotagger for response parsing

**Confidence-Based Processing**
- Auto-accept relationships with confidence >= 0.7: create edge immediately
- Discard relationships with confidence < 0.7: do not store or display
- No manual review workflow or staging state required

**Edge Creation**
- Enforce directional convention: source_tag_id = narrower/child concept, target_tag_id = broader/parent concept
- Resolve tag names to tag IDs using existing get_or_create_tag or direct query pattern
- Populate all edge metadata fields: confidence, hierarchy_type, source='llm', model_version from OLLAMA_MODEL, verified=0, created_at=now, valid_from=NULL, valid_until=NULL
- Use INSERT OR IGNORE to prevent duplicate edges (idempotent operation)
- Wrap insertions in transaction for atomicity following create_note pattern

**CLI Output**
- Display summary: "Analyzed X tags, found Y relationships"
- List auto-accepted relationships in format: "transformer -> neural-network (generic, 0.85 confidence)"
- Report count of discarded low-confidence suggestions
- On LLM failure: display error message but exit cleanly (fail-safe, exit code 0)

**Error Handling**
- Follow fail-safe pattern from auto_tag_note: LLM failures logged but don't crash command
- Return empty results on JSON parsing failures (same as autotagger)
- Handle missing OLLAMA_MODEL environment variable with clear error message
- Validate tag resolution before edge creation

## Visual Design
No visual assets provided.

## Existing Code to Leverage

**AutoTagger and Prompt Pattern (`src/autotagger/tagger.rs`)**
- PROMPT_TEMPLATE constant with model-agnostic instructions and few-shot examples
- AutoTaggerBuilder pattern with Arc-wrapped OllamaClientTrait for testability
- extract_json function for handling markdown-wrapped or text-surrounded JSON responses
- parse_tags pattern for JSON parsing with fail-safe empty result on parse failure
- Confidence score clamping to 0.0-1.0 range

**CLI Command Structure (`src/main.rs`)**
- TagAliasCommand and TagAliasCommands enum pattern for nested subcommands
- handle_tag_alias dispatch pattern to execute_tag_alias_* functions
- get_database_path and ensure_database_directory for database access
- Separation of handle_* (path resolution) from execute_* (business logic with Database parameter) for testability

**NoteService Database Patterns (`src/service.rs`)**
- Transaction pattern: BEGIN TRANSACTION, execute operations, COMMIT or ROLLBACK
- INSERT OR IGNORE for idempotent inserts (see add_tags_to_note)
- Provenance tracking with source, confidence, model_version, created_at fields
- Query patterns with rusqlite::params! and rusqlite::params_from_iter

**OllamaClient (`src/ollama/client.rs`)**
- OllamaClientBuilder with environment variable fallbacks (OLLAMA_HOST, OLLAMA_MODEL)
- OllamaClientTrait for mockable interface
- Error types: OllamaError with Network, Timeout, Http, Api variants
- retry_with_backoff for transient error handling

**Edges Table Schema (`src/db/schema.rs`)**
- Existing edges table with all required columns: source_tag_id, target_tag_id, confidence, hierarchy_type (CHECK constraint for 'generic'|'partitive'), source, model_version, verified, created_at, valid_from, valid_until
- Indexes already exist on source_tag_id, target_tag_id, hierarchy_type, created_at

## Out of Scope
- Manual review workflow or interactive confirmation prompts
- Pending/staging state for low-confidence suggestions
- Automatic background triggering of hierarchy analysis
- Incremental analysis when new tags are added
- Relationship suggestions between note content (only between tags)
- Visualization of the hierarchy graph
- Editing or deleting existing edges via this command
- Relationships involving orphaned tags (tags with no notes)
- Bidirectional edge storage (only source->target "upward" edges)
- TUI interface for hierarchy management
