# Specification: Tag Aliases

## Goal

Implement SKOS-style tag alias resolution that maps alternate forms (e.g., "ml", "ML", "machine-learning") to canonical tags, enabling transparent synonym handling with LLM-suggested aliases auto-created following the "apply immediately, correct later" philosophy.

## User Stories

- As a user, I want my notes tagged with "ml" to be found when I search for "machine-learning" so that I don't have to remember exact tag names
- As a user, I want to manage tag aliases manually via CLI commands so that I can define domain-specific synonyms

## Specific Requirements

**Schema Redesign for tag_aliases Table**
- Replace existing simple schema with proper provenance-aware structure
- Include columns: alias (TEXT PRIMARY KEY COLLATE NOCASE), canonical_tag_id (INTEGER FK), source (TEXT: 'user' or 'llm'), confidence (REAL), created_at (INTEGER), model_version (TEXT nullable)
- Foreign key to tags table with ON DELETE CASCADE
- Index on canonical_tag_id for reverse lookups
- Use IF NOT EXISTS pattern consistent with existing schema design

**Alias Resolution in get_or_create_tag**
- After normalizing input via TagNormalizer, check tag_aliases table for matching alias
- If alias found, return the canonical tag_id instead of creating new tag
- Resolution is silent with no warnings or user prompts
- Ensures creating a tag that exists as an alias resolves to canonical form

**Alias Resolution in list_notes Tag Filtering**
- Before querying notes by tag in ListNotesOptions, resolve any alias to canonical tag name
- Use normalized alias lookup (COLLATE NOCASE) to match user input
- Multiple tag filters should each be resolved independently
- AND logic preserved after alias resolution

**LLM-Suggested Alias Auto-Creation**
- During auto_tag_note flow, when LLM suggests a tag that normalizes to match an existing tag's alias pattern, auto-create alias mapping
- Store LLM-suggested aliases with source='llm', confidence from tagger, model_version from OLLAMA_MODEL
- No confirmation gates per "apply immediately, correct later" philosophy
- Integrate with existing AutoTagger output processing

**CLI Command: cons tag-alias add**
- Command signature: `cons tag-alias add <alias> <canonical>`
- Normalize both alias and canonical before storage
- Verify canonical tag exists (or create it)
- Prevent alias-to-alias chains (canonical must not itself be an alias)
- Store with source='user', confidence=1.0, created_at=now

**CLI Command: cons tag-alias list**
- Display all aliases grouped by canonical tag name
- Show source (user/llm) and confidence for each alias
- Order by canonical tag name, then by alias name

**CLI Command: cons tag-alias remove**
- Command signature: `cons tag-alias remove <alias>`
- Delete alias mapping from tag_aliases table
- Idempotent: removing non-existent alias returns success

**Service Layer Methods**
- Add methods to NoteService: resolve_alias(name) -> Option<TagId>, create_alias(alias, canonical_tag_id, source), list_aliases() -> Vec<AliasInfo>, remove_alias(alias)
- Use existing transaction patterns from create_note for atomicity
- Follow rusqlite::OptionalExtension pattern for optional query results

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**NoteService (src/service.rs)**
- get_or_create_tag method is the primary integration point for alias resolution
- Transaction pattern (BEGIN/COMMIT/ROLLBACK) should be reused for alias operations
- list_notes method needs modification to resolve tag filter aliases before query

**TagNormalizer (src/autotagger/normalizer.rs)**
- normalize_tag function must be called before alias lookup to ensure consistent matching
- Normalization happens BEFORE alias resolution in the lookup chain

**Database Schema (src/db/schema.rs)**
- Existing tag_aliases table structure will be replaced with enhanced schema
- Follow IF NOT EXISTS pattern and COLLATE NOCASE for case-insensitive matching
- Index patterns (idx_tag_aliases_canonical) should be preserved

**CLI Command Structure (src/main.rs)**
- Use clap derive macros with #[derive(Parser)] and #[derive(Subcommand)]
- Follow existing pattern: Commands enum with struct per command
- Handler functions follow handle_X and execute_X separation pattern

**AutoTagger Integration (src/autotagger/tagger.rs)**
- generate_tags output is processed in auto_tag_note function in main.rs
- Alias creation should integrate after tag normalization, before add_tags_to_note

## Out of Scope

- User confirmation workflow CLI command for reviewing LLM-suggested aliases
- Hierarchical tag relationships (broader/narrower SKOS semantics)
- Tag merging workflows to consolidate duplicate tags
- Automatic alias detection/suggestion from existing tag patterns
- Interactive prompts during cons add for alias suggestions
- Alias chains (alias pointing to another alias rather than canonical tag)
- Bulk alias import/export functionality
- Alias conflict resolution UI when multiple canonicals could match
- Reverse lookup CLI command to find aliases for a given canonical tag
- Alias versioning or history tracking
