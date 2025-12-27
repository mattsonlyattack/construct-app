# Alias-expanded FTS

## Initial Idea

From roadmap item 16:

> Alias-expanded FTS -- Integrate tag_aliases into search queries, expanding "ML" to "ML OR machine-learning OR machine learning" before FTS5 matching; automatic synonym bridging

## Context

- Size: S (small, 2-3 days)
- Dependencies:
  - Item 12 (completed): Tag aliases table with tag_aliases mapping alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern)
  - Item 15 (completed): Full-text search with FTS5 - `cons search "query"` command

## Technical Background

The `tag_aliases` table already exists with this schema:
- `alias` TEXT PRIMARY KEY (case-insensitive)
- `canonical_tag_id` INTEGER (foreign key to tags.id)
- `source` TEXT (user/llm)
- `confidence` REAL
- `created_at` INTEGER
- `model_version` TEXT (nullable)

The FTS5 search is implemented in `NoteService::search_notes()` which:
1. Validates query is not empty
2. Escapes and quotes search terms
3. Queries `notes_fts` virtual table with BM25 ranking
4. Returns `SearchResult` with note and relevance score

The FTS5 table indexes: `content`, `content_enhanced`, and `tags` columns.
