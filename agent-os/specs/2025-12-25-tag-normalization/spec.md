# Specification: Tag Normalization

## Goal

Apply consistent tag formatting across all entry points (CLI, TUI, future GUI) by integrating the existing `TagNormalizer` into the `NoteService` layer, ensuring that both manual and AI-generated tags follow identical normalization rules.

## User Stories

- As a user, I want my tags normalized automatically so that "Rust", "rust", and "RUST" all resolve to the same tag without manual effort
- As a developer, I want normalization applied at the service layer so that all interfaces benefit without code duplication

## Specific Requirements

**Normalize tags in `NoteService::get_or_create_tag()`**
- Import `TagNormalizer` from `crate::autotagger`
- Call `TagNormalizer::normalize_tag(name)` before database lookup and insertion
- Use the normalized form for both the `SELECT` query and the `INSERT` statement
- This single change point ensures all tag creation flows through normalization

**Update deduplication in `NoteService::create_note()`**
- Currently uses `tag_name.to_lowercase()` for deduplication (line 108 in service.rs)
- Replace with `TagNormalizer::normalize_tag(tag_name)` to use full normalization
- Ensures "Machine Learning" and "machine-learning" deduplicate correctly
- The normalized form is already used by `get_or_create_tag`, ensuring consistency

**Normalize tags in `NoteService::add_tags_to_note()`**
- Tags passed to this method should be normalized before calling `get_or_create_tag`
- Alternatively, rely on `get_or_create_tag` to normalize (already handled by first requirement)
- No duplicate normalization needed since `get_or_create_tag` handles it

**Preserve existing `TagNormalizer` behavior**
- Lowercase conversion
- Spaces replaced with hyphens
- Only alphanumeric characters and hyphens retained
- Consecutive hyphens collapsed to single hyphen
- Leading/trailing whitespace and hyphens trimmed

**Preserve existing database behavior**
- Keep `COLLATE NOCASE` constraint on tags table as defense-in-depth
- Do not migrate existing tags; normalization applies to new tags only
- No schema changes required

## Visual Design

Not applicable for this backend-focused feature.

## Existing Code to Leverage

**`TagNormalizer` in `src/autotagger/normalizer.rs`**
- Already exported at crate root via `pub use autotagger::TagNormalizer`
- Contains `normalize_tag()` for single tag and `normalize_tags()` for collections
- Has comprehensive test coverage for edge cases (special chars, whitespace, case)
- Used by `AutoTagger::generate_tags()` to normalize LLM output

**`NoteService::get_or_create_tag()` in `src/service.rs` (line 309)**
- Currently stores tags without normalization (preserves original form)
- Single point where tags are created/retrieved from database
- Modifying this method applies normalization to all callers automatically

**Deduplication logic in `NoteService::create_note()` (line 108)**
- Currently uses simple `to_lowercase()` for deduplication
- Should use `TagNormalizer::normalize_tag()` for consistency with insertion

**CLI `parse_tags()` in `src/main.rs` (line 327)**
- Currently only trims whitespace, no normalization
- No changes needed since `NoteService` will handle normalization

## Out of Scope

- Migration of existing database tags to normalized form
- Tag aliases feature (e.g., "c++" -> "cpp")
- Changes to `COLLATE NOCASE` database constraint
- CLI output formatting changes
- UI/display changes
- Changes to `TagNormalizer` implementation
- Changes to the LLM prompt or auto-tagger behavior
- Adding normalization at CLI layer (handled by service layer)
- Backward compatibility for existing non-normalized tags in queries
- Performance optimization for normalization
