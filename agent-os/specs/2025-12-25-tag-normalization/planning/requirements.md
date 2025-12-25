# Spec Requirements: Tag Normalization

## Initial Description

Tag normalization -- Implement consistent tag formatting (lowercase, hyphenation, deduplication) across manual and AI-generated tags.

## Requirements Discussion

### First Round Questions

**Q1:** I assume the existing `TagNormalizer` rules (lowercase, spaces-to-hyphens, alphanumeric-only, collapse consecutive hyphens) should be the canonical normalization applied everywhere. Is that correct, or do you want different/additional formatting rules?
**Answer:** Correct - existing TagNormalizer rules are canonical.

**Q2:** I'm thinking we should apply normalization at the `NoteService` layer (in `get_or_create_tag()` and `add_tags_to_note()`) so that all entry points (CLI, TUI, future GUI) automatically get normalized tags. Should we normalize there, or at each individual entry point (CLI parsing, auto-tagger output, etc.)?
**Answer:** "Deep abstractions" - meaning NoteService layer so all entry points get it automatically.

**Q3:** The database already has `COLLATE NOCASE` on the tags table, which means "Rust" and "rust" resolve to the same tag. Should we preserve this database-level fallback, or is it now redundant if we normalize before insertion?
**Answer:** Keep it - seems redundant but doesn't hurt anything.

**Q4:** For existing tags in the database that may not be normalized (e.g., "Machine Learning" from older entries), should this feature include a one-time migration to normalize existing tags, or just apply normalization to new tags going forward?
**Answer:** New tags going forward only, no migration needed.

**Q5:** Currently, special characters like `++` in "C++" become just "c" after normalization. The tag aliases table exists (`tag_aliases`) but isn't being used. Should this feature include setting up common aliases (e.g., "c++" -> "cpp", "c#" -> "csharp"), or is that out of scope?
**Answer:** Skip for now, will add to roadmap separately.

**Q6:** Is there anything specific you want to EXCLUDE from this feature? For example: migration scripts, CLI output formatting changes, alias table population, or any particular edge cases?
**Answer:** None.

### Existing Code to Reference

**Similar Features Identified:**
- Feature: TagNormalizer - Path: `src/autotagger/normalizer.rs`
  - Contains canonical `normalize_tag()` and `normalize_tags()` methods
  - Already applied to AI-generated tags in `AutoTagger::generate_tags()`
  - Has comprehensive test coverage for edge cases

- Feature: NoteService tag handling - Path: `src/service.rs`
  - `get_or_create_tag()` method at line 309 - where normalization should be added
  - `add_tags_to_note()` method at line 360 - processes tags before insertion
  - `create_note()` method at line 81 - handles initial tag creation with deduplication

- Feature: CLI tag parsing - Path: `src/main.rs`
  - `parse_tags()` function at line 327 - currently only trims whitespace

### Follow-up Questions

No follow-up questions needed - answers were comprehensive.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
Not applicable for this backend-focused feature.

## Requirements Summary

### Functional Requirements

- Apply consistent tag normalization using existing `TagNormalizer::normalize_tag()` rules:
  - Convert to lowercase
  - Replace spaces with hyphens
  - Remove all characters except alphanumeric and hyphens
  - Collapse consecutive hyphens into single hyphen
  - Trim leading/trailing whitespace and hyphens
- Normalize tags at the NoteService layer so all entry points benefit automatically
- Deduplicate tags case-insensitively (already partially implemented, needs to use normalized form)
- Both manual tags (from CLI `--tags` flag) and AI-generated tags should be normalized identically

### Reusability Opportunities

- `TagNormalizer` struct already exists in `src/autotagger/normalizer.rs` with full implementation
- Can be re-exported or moved to a shared location for use by `NoteService`
- Existing test suite in normalizer.rs covers edge cases thoroughly

### Scope Boundaries

**In Scope:**
- Integrate `TagNormalizer::normalize_tag()` into `NoteService::get_or_create_tag()`
- Ensure deduplication in `create_note()` uses normalized tags for comparison
- Verify normalization applies to both user-provided and LLM-generated tags
- Update any unit tests that may be affected by normalization changes

**Out of Scope:**
- Migration of existing database tags to normalized form
- Tag aliases feature (e.g., "c++" -> "cpp") - deferred to separate roadmap item
- Changes to `COLLATE NOCASE` database constraint (keep as-is)
- CLI output formatting changes
- Any UI/display changes

### Technical Considerations

- `TagNormalizer` is currently in `src/autotagger/` module but needs to be accessible from `src/service.rs`
- May need to re-export `TagNormalizer` at crate root or move to a shared module
- Existing `COLLATE NOCASE` in database schema provides defense-in-depth for case variations
- Deduplication logic in `create_note()` already normalizes to lowercase for comparison; should use full `TagNormalizer::normalize_tag()` instead
- No async considerations - all affected code is synchronous
