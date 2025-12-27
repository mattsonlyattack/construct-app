# Specification: Alias-expanded FTS

## Goal

Integrate tag_aliases into FTS5 search queries, automatically expanding search terms to include all related aliases and canonical forms, enabling users to find notes using any synonym form without manual query construction.

## User Stories

- As a user, I want to search for "ML" and automatically find notes containing "machine-learning" or "machine learning" so that I do not need to remember or manually specify all synonym variants.
- As a user, I want my search results to leverage my existing alias relationships so that the synonym mappings I have created improve my search experience.

## Specific Requirements

**Query Term Expansion**
- Before executing FTS5 search, expand each search term using the tag_aliases table
- Bi-directional expansion: if term is an alias, include the canonical tag name; if term matches a canonical tag, include all its aliases
- Collect unique expansion terms to avoid duplicates in the expanded query
- Original search term is always included in expansion, even if no aliases exist

**Confidence-based Alias Filtering**
- User-created aliases (source = 'user') are always included regardless of confidence score
- LLM-suggested aliases (source = 'llm') are only included if confidence >= 0.8
- Query the tag_aliases table with source and confidence columns to filter appropriately

**FTS5 Query Construction**
- Build expanded query using FTS5 OR syntax: `term1 OR term2 OR term3`
- Terms should be unquoted to allow porter stemming to work (e.g., "learning" matches "learn")
- Multi-word aliases (e.g., "machine learning") should be handled as phrase matches
- Example: "ML" expands to `ML OR machine-learning OR "machine learning"`

**Lookup Implementation**
- Create new method `expand_search_term(&self, term: &str) -> Result<Vec<String>>` in NoteService
- Normalize input term before lookup using existing TagNormalizer
- Query tag_aliases to find if term is an alias (returns canonical_tag_id)
- Query tags table to find if term matches a canonical tag name
- Query tag_aliases to get all aliases for matched canonical tag (with confidence filtering)

**Search Integration**
- Modify `NoteService::search_notes()` to call expansion logic before FTS5 query execution
- Apply expansion to each whitespace-separated search term independently
- Join expanded terms with proper FTS5 syntax for AND between original terms, OR within expansions

**Always Active**
- Alias expansion is always enabled with no flag to disable
- Expansion adds no overhead when no aliases exist (query falls through unchanged)

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**NoteService::resolve_alias() in /home/md/construct-app/src/service.rs**
- Resolves a single alias to its canonical TagId
- Uses TagNormalizer for input normalization
- Queries tag_aliases with COLLATE NOCASE matching
- Reuse this pattern for checking if a term is an alias

**NoteService::list_aliases() in /home/md/construct-app/src/service.rs**
- Queries tag_aliases table joining with tags to get canonical names
- Includes source, confidence, and model_version columns
- Reuse query structure for getting all aliases of a canonical tag with confidence filtering

**NoteService::search_notes() in /home/md/construct-app/src/service.rs**
- Current implementation escapes and quotes each search term
- Joins terms with spaces for AND logic
- Modify to call expansion before constructing FTS query
- Preserve BM25 scoring and SearchResult structure

**tag_aliases table schema in /home/md/construct-app/src/db/schema.rs**
- Columns: alias (PRIMARY KEY), canonical_tag_id, source, confidence, created_at, model_version
- COLLATE NOCASE on alias column for case-insensitive matching
- Foreign key to tags table for canonical tag lookup

**TagNormalizer::normalize_tag() in autotagger module**
- Standard normalization (lowercase, hyphenation) already used throughout codebase
- Apply to search terms before alias lookup for consistent matching

## Out of Scope

- Broader concept expansion using tag hierarchies (roadmap item 21)
- Related concept expansion through graph relationships
- Changes to FTS index content or triggers
- User-configurable expansion settings or command-line flags
- Displaying which expansions were applied in search output
- Expanding query terms that are not tags/aliases (only matches in tag_aliases or tags tables are expanded)
- Adding new database tables or schema changes
- Caching of expansion lookups
- Performance optimization for large alias sets
