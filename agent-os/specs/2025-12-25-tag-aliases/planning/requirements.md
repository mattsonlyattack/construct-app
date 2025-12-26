# Spec Requirements: Tag Aliases

## Initial Description

Implement tag_aliases table mapping alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern) to solve synonymy problems (car/auto/automobile), with LLM-suggested aliases and user confirmation workflows.

## Requirements Discussion

### First Round Questions

**Q1:** The `tag_aliases` table already exists in the schema with a simple structure (alias TEXT -> canonical_tag_id). I assume we're now implementing the service layer logic and CLI commands to use this table. Is that correct, or do you also want schema modifications (e.g., adding provenance/confidence for LLM-suggested aliases)?
**Answer:** Re-design the schema the proper way (not just use the existing simple table)

**Q2:** For LLM-suggested aliases, should aliases be auto-created based on LLM suggestions, or should they require user confirmation? The KNOWLEDGE.md philosophy suggests "apply immediately, correct later" for low-stakes inferences.
**Answer:** Auto-created (aliases from LLM suggestions should be auto-created, following "apply immediately, correct later" philosophy)

**Q3:** For the user confirmation workflow mentioned in the roadmap, I assume this would be a CLI command like `cons aliases suggest` that surfaces unconfirmed alias candidates for review. Is that the right approach, or did you envision something different?
**Answer:** Skip the user confirmation workflow via CLI command - not needed

**Q4:** I assume alias resolution should be transparent to users - when they filter by `--tags ml`, it should automatically resolve to the canonical tag and find all notes tagged with "machine-learning". Is that correct?
**Answer:** Correct - alias resolution should be transparent to users

**Q5:** For manual alias management, I'm thinking commands like `cons alias add ml machine-learning`, `cons alias list`, `cons alias remove ml`. Does this match your expectations, or should alias management be part of a broader command group?
**Answer:** Use `tag-alias` instead of just `alias` for the command group (e.g., `cons tag-alias add ml machine-learning`)

**Q6:** The TagNormalizer currently converts "ML" to "ml" and "machine learning" to "machine-learning". Should alias lookup happen BEFORE or AFTER normalization?
**Answer:** After - alias lookup happens after normalization

**Q7:** What should happen if a user manually creates a tag that already exists as an alias for another tag?
**Answer:** Option A: Silently resolve to canonical tag

**Q8:** Are there any scenarios or edge cases you want to explicitly exclude from this implementation?
**Answer:** No exclusions specified

### Existing Code to Reference

**Similar Features Identified:**
- Feature: CLI commands - Path: `/home/md/construct-app/src/cli.rs` (for command structure patterns)
- Feature: NoteService - Path: `/home/md/construct-app/src/service.rs` (for database query patterns and service layer design)
- Feature: TagNormalizer - Path: `/home/md/construct-app/src/autotagger/normalizer.rs` (for normalization logic integration)
- Feature: Database schema - Path: `/home/md/construct-app/src/db/schema.rs` (for schema design patterns)
- Feature: Service tests - Path: `/home/md/construct-app/src/service/tests.rs` (for test patterns)

### Follow-up Questions

No follow-up questions needed - user answers were comprehensive.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

**Schema Design:**
- Redesign the `tag_aliases` table with proper structure following SKOS prefLabel/altLabel pattern
- Include provenance metadata (source: user vs LLM, confidence, timestamp, model version)
- Support mapping multiple alternate forms to a single canonical tag ID
- Ensure case-insensitive alias matching via COLLATE NOCASE

**Alias Resolution:**
- Transparent resolution: when user specifies a tag (via `--tags` or manual entry), automatically resolve aliases to canonical form
- Resolution happens AFTER normalization (e.g., "ML" -> "ml" -> "machine-learning")
- Silent resolution: no warnings or prompts when alias is resolved to canonical tag
- Works in all contexts: `cons add --tags`, `cons list --tags`, search filters

**LLM Integration:**
- Auto-create aliases when LLM suggests tags that match existing canonical forms
- Follow "apply immediately, correct later" philosophy - no confirmation gates
- Store LLM-suggested aliases with appropriate provenance metadata (model, confidence, timestamp)
- Integrate with existing AutoTagger flow in `cons add` command

**CLI Commands (tag-alias command group):**
- `cons tag-alias add <alias> <canonical>` - Create a manual alias mapping
- `cons tag-alias list` - Show all aliases grouped by canonical tag
- `cons tag-alias remove <alias>` - Delete an alias mapping

**Tag Creation Behavior:**
- When creating a tag that is an existing alias, silently resolve to canonical tag
- Preserve the user's intent while maintaining vocabulary consistency
- No duplicate tags created when alias already points to canonical form

### Reusability Opportunities

- NoteService patterns for database operations with transactions
- TagNormalizer for consistent tag formatting before alias lookup
- Existing CLI command structure (clap derive macros) for new `tag-alias` subcommand
- Test patterns from service tests for alias resolution testing
- Schema design patterns (IF NOT EXISTS, indexes, foreign keys with CASCADE)

### Scope Boundaries

**In Scope:**
- Schema redesign for tag_aliases table with proper metadata
- Service layer methods for alias CRUD and resolution
- Integration with existing tag creation flow (get_or_create_tag)
- CLI commands for manual alias management (add, list, remove)
- Transparent alias resolution in tag filtering (`--tags` flag)
- LLM-suggested alias auto-creation during auto-tagging
- Normalization-then-resolution lookup order

**Out of Scope:**
- User confirmation workflow CLI command (explicitly skipped)
- Hierarchical tag relationships (broader/narrower) - separate feature
- Tag merging workflows - separate feature
- Automatic alias detection from existing tags - not requested
- Interactive prompts during `cons add` - not needed

### Technical Considerations

**Integration Points:**
- NoteService.get_or_create_tag() - needs to resolve aliases before creating/finding tags
- NoteService.list_notes() with tags filter - needs to resolve aliases in query
- AutoTagger.generate_tags() output processing - needs to check for existing canonical forms
- CLI add command - needs to resolve aliases when processing `--tags` flag
- CLI list command - needs to resolve aliases when processing `--tags` filter

**Schema Design Considerations:**
- Primary key on normalized alias text (case-insensitive)
- Foreign key to tags table with CASCADE delete
- Index on canonical_tag_id for reverse lookups
- Provenance columns: source (user/llm), confidence, created_at, model_version
- Consider whether verified flag is needed (may skip per "apply immediately" philosophy)

**Existing System Constraints:**
- SQLite with idempotent schema (IF NOT EXISTS pattern)
- Synchronous database operations (no async for SQLite)
- TagNormalizer applies kebab-case, lowercase normalization
- Tags table uses COLLATE NOCASE for case-insensitive matching
- NoteService owns Database instance, provides all business logic

**Technology Preferences:**
- Rust with anyhow for error handling
- rusqlite for database operations
- clap derive macros for CLI
- Standard test patterns with in-memory database

**Similar Code Patterns to Follow:**
- Transaction pattern from NoteService.create_note()
- Optional query result handling with rusqlite::OptionalExtension
- Builder pattern for complex types (NoteBuilder, AutoTaggerBuilder)
- Comprehensive doc comments with examples
