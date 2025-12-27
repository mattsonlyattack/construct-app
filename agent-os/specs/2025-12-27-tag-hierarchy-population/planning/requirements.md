# Spec Requirements: Tag Hierarchy Population

## Initial Description
**Tag hierarchy population** - LLM suggests broader/narrower relationships between existing tags with confidence scores; user confirms via CLI; distinguish generic (is-a: "transformer" specializes "neural-network") from partitive (part-of: "attention" isPartOf "transformer") using XKOS semantics

This is for the cons project - a structure-last personal knowledge management CLI tool. Key context:
- Local-first with SQLite + Ollama (deepseek-r1:8b model)
- Graph schema foundation already exists with edges table containing: confidence (REAL), hierarchy_type ('generic'|'partitive'|NULL), valid_from/valid_until (TIMESTAMP nullable)
- Tag aliases system already implemented
- Core business logic is in NoteService layer
- CLI commands use clap

## Requirements Discussion

### First Round Questions

**Q1:** Suggestion trigger - Should the LLM analyze relationships when explicitly invoked via a CLI command (e.g., `cons hierarchy suggest`) rather than automatically running in the background, or should hierarchy suggestions also trigger automatically when certain thresholds are met?
**Answer:** CLI command (explicit invocation, e.g., `cons hierarchy suggest`)

**Q2:** Batch vs incremental - Should the suggestion workflow analyze ALL existing tags in one batch to find relationships, or also support incremental mode where adding a new tag triggers analysis of just that tag against existing tags?
**Answer:** Batch - analyze all existing tags at once

**Q3:** Suggestion scope - Should the LLM suggest relationships only between tags that already have notes (ensuring real usage), or also between tags that exist but might be orphaned?
**Answer:** Only tags that have notes (real usage, no orphaned tags)

**Q4:** User confirmation workflow - Should we use a two-phase approach (suggest then review), or surface suggestions inline during normal commands?
**Answer:** Confidence-based tiers with auto-accept for high confidence, manual review for mid-range.

**Q5:** Confidence threshold for display - Should rejected suggestions be stored to prevent re-suggestion, or simply discarded?
**Answer:** Tiered approach based on confidence scores.

**Q6:** Hierarchy type determination - Should the LLM determine hierarchy type (generic vs partitive), or should users specify it during confirmation?
**Answer:** LLM should determine the type (generic vs partitive)

**Q7:** Directional semantics - For the edges table, should source_tag_id represent the narrower/child concept and target_tag_id represent the broader/parent concept?
**Answer:** Yes, enforce convention: source = narrower/child, target = broader/parent. Edges point "up" the hierarchy.

**Q8:** What should we explicitly exclude from this feature?
**Answer:** User did not specify - reasonable exclusions proposed below.

### Existing Code to Reference

**Similar Features Identified:**
- Feature: Tag Alias CLI - Path: `/home/md/construct-app/src/main.rs` (TagAliasCommand, TagAliasCommands enum, execute_tag_alias_* functions)
- Feature: AutoTagger - Path: `/home/md/construct-app/src/autotagger.rs` (LLM prompt patterns, confidence handling)
- Feature: NoteService - Path: `/home/md/construct-app/src/service.rs` (business logic layer patterns, alias creation with provenance)
- Feature: Edges table schema - Path: `/home/md/construct-app/src/db/schema.rs` (edges table with hierarchy_type, confidence, source, model_version, verified)
- Feature: OllamaClient - Path: `/home/md/construct-app/src/ollama/` (LLM HTTP client patterns)

### Follow-up Questions

**Follow-up 1:** For suggestions in the 0.5-0.7 confidence range that require manual review, should the review be interactive (one-at-a-time) or batch (accept/reject by ID)?
**Answer:** No manual review at all. Updated confidence tiers:
- >= 0.7: Auto-accept (create edge automatically)
- < 0.7: Discard

This eliminates the need for a review command or pending/staging state.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

**Core Functionality:**
- Single CLI command `cons hierarchy suggest` (or similar naming) triggers the hierarchy population workflow
- LLM analyzes ALL existing tags that have at least one associated note (batch analysis)
- LLM identifies broader/narrower relationships between tag pairs
- LLM determines hierarchy type for each relationship:
  - `generic` (is-a): e.g., "transformer" specializes "neural-network"
  - `partitive` (part-of): e.g., "attention" isPartOf "transformer"
- LLM provides confidence score (0.0-1.0) for each suggested relationship

**Confidence-Based Auto-Processing:**
- Confidence >= 0.7: Automatically create edge in database (no user review)
- Confidence < 0.7: Discard suggestion (do not store or show to user)

**Edge Creation:**
- Store relationships in existing `edges` table
- Enforce directional convention: `source_tag_id` = narrower/child concept, `target_tag_id` = broader/parent concept
- Populate metadata fields:
  - `confidence`: The LLM's confidence score
  - `hierarchy_type`: 'generic' or 'partitive' as determined by LLM
  - `source`: 'llm'
  - `model_version`: The Ollama model used (from OLLAMA_MODEL env var)
  - `verified`: 0 (unverified, since auto-accepted)
  - `created_at`: Current timestamp
  - `valid_from`: NULL (always valid)
  - `valid_until`: NULL (no expiration)

**CLI Output:**
- Display summary of analysis results (total tags analyzed, relationships found)
- List auto-accepted relationships with their confidence scores and hierarchy types
- Report count of discarded low-confidence suggestions
- Follow fail-safe pattern: LLM failures should not crash the command

**Idempotency:**
- Running the command multiple times should not create duplicate edges
- Use INSERT OR IGNORE or check for existing edges before insertion

### Reusability Opportunities

**Components to Reference:**
- `AutoTaggerBuilder` / `AutoTagger` pattern for LLM interaction with configurable client
- `TagNormalizer::normalize_tag()` for consistent tag name handling
- `NoteService` methods for database operations (transaction patterns, error handling)
- `OllamaClientBuilder` for HTTP client construction
- Existing prompt engineering patterns in `autotagger.rs`

**Backend Patterns:**
- Fail-safe error handling (LLM failures logged but don't block execution)
- Provenance tracking (source, model_version, confidence, timestamps)
- Transaction patterns for atomicity

### Scope Boundaries

**In Scope:**
- CLI command to trigger hierarchy suggestion
- LLM prompt design for relationship inference with hierarchy type classification
- Batch analysis of all tags with note associations
- Confidence-based auto-acceptance (>= 0.7)
- Edge creation with full metadata (confidence, hierarchy_type, source, model_version, verified, timestamps)
- Directional edge convention enforcement (source = narrower, target = broader)
- Summary output showing created relationships
- Idempotent edge creation (no duplicates)

**Out of Scope:**
- Manual review workflow (no staging/pending state needed)
- Interactive confirmation prompts
- Automatic background triggering (only explicit CLI invocation)
- Incremental analysis (when new tags are added)
- Relationship suggestions between note content (only between tags)
- Visualization of the hierarchy graph
- Editing or deleting existing edges via this command
- Relationships involving orphaned tags (tags with no notes)
- Bidirectional edge storage (only "upward" edges stored)

### Technical Considerations

**Integration Points:**
- Ollama API via existing `OllamaClient` for LLM inference
- SQLite `edges` table (schema already exists)
- SQLite `tags` and `note_tags` tables for finding tags with notes
- `OLLAMA_MODEL` environment variable for model selection

**Existing System Constraints:**
- Local-first architecture (no cloud API calls)
- Fail-safe design: LLM failures never block core functionality
- Async only for HTTP (tokio for Ollama calls)
- NoteService layer for business logic (CLI is thin presentation layer)

**Technology Preferences:**
- Rust with clap for CLI
- rusqlite for database operations
- reqwest/tokio for async HTTP
- serde for JSON serialization

**Similar Code Patterns to Follow:**
- `auto_tag_note()` in main.rs for LLM integration with fail-safe error handling
- `execute_tag_alias_add()` for CLI command patterns
- `create_alias()` in service.rs for provenance tracking (source, confidence, model_version)
- Transaction patterns in `create_note()` for atomicity

**LLM Prompt Considerations:**
- Input: List of tag names that have associated notes
- Output: JSON array of relationship suggestions with:
  - source_tag (narrower/child)
  - target_tag (broader/parent)
  - hierarchy_type ('generic' or 'partitive')
  - confidence (0.0-1.0)
- Prompt should explain XKOS semantics distinction clearly
- Consider token limits when many tags exist (may need batching strategy for large tag sets)
