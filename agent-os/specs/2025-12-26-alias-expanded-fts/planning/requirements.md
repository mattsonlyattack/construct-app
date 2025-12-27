# Spec Requirements: Alias-expanded FTS

## Initial Description

From roadmap item 16:

> Alias-expanded FTS -- Integrate tag_aliases into search queries, expanding "ML" to "ML OR machine-learning OR machine learning" before FTS5 matching; automatic synonym bridging

Size: S (small, 2-3 days)

Dependencies (both completed):
- Item 12: Tag aliases table with tag_aliases mapping alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern)
- Item 15: Full-text search with FTS5 - `cons search "query"` command

## Requirements Discussion

### First Round Questions

**Q1:** Should alias expansion happen automatically for ALL search terms, or only terms that look like tags (e.g., short abbreviations like "ML")?
**Answer:** All search terms - alias expansion happens for ALL terms, not just tag-like ones.

**Q2:** Should expansion be bi-directional (alias->canonical AND canonical->aliases) or one-way?
**Answer:** Bi-directional expansion - both alias->canonical and canonical->aliases.

**Q3:** Should expanded terms be exact matches only (quoted) or include word variations (unquoted, allowing stemming)?
**Answer:** Include word variations - unquoted terms allowing stemming via porter tokenizer.

**Q4:** When a search term matches multiple alias groups, should we expand ALL matching relationships or prefer one mapping?
**Answer:** Expand ALL matching relationships - KNOWLEDGE.md confirms: "expand aliases always" (noise control is for broader/related concepts, not aliases).

**Q5:** Should we always expand aliases, or add a `--no-expand` flag to disable expansion?
**Answer:** Always expand - no flag needed, always expand aliases.

**Q6:** Do we need to change what's stored in the FTS index, or only how queries are processed?
**Answer:** Query processing only - no need to change what's stored in FTS index.

**Q7:** Should all aliases be included in expansion regardless of confidence, or should low-confidence LLM-suggested aliases be excluded?
**Answer:** Exclude low confidence - low-confidence LLM-suggested aliases should be excluded from expansion.

**Q8:** Is there anything specifically out of scope?
**Answer:** Only future roadmap items - broader concept expansion from item 21 is deferred.

### Existing Code to Reference

No similar existing features identified for reference.

Relevant existing code:
- `/home/md/construct-app/src/service.rs` - Contains `NoteService::search_notes()` method where query expansion will be implemented
- `/home/md/construct-app/src/service.rs` - Contains `resolve_alias()` and `list_aliases()` methods for alias lookups
- `/home/md/construct-app/src/db/schema.rs` - Contains `tag_aliases` table schema

### Follow-up Questions

**Follow-up 1:** What confidence threshold should be used for excluding low-confidence LLM-suggested aliases?
**Answer:** Option B with 0.8 threshold - Include all user-created aliases (`source = 'user'`) regardless of confidence, and LLM-suggested aliases only if confidence >= 0.8.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

1. **Query Expansion**: Before executing FTS5 search, expand each search term using the tag_aliases table
   - For each search term, find all related aliases and canonical forms
   - Bi-directional: if term is an alias, include canonical; if term is canonical, include all aliases

2. **Expansion Scope**: Apply expansion to ALL search terms (not just tag-like terms)

3. **Confidence Filtering**: Only include aliases meeting confidence criteria:
   - All user-created aliases (`source = 'user'`) - always included
   - LLM-suggested aliases (`source = 'llm'`) - only if confidence >= 0.8

4. **FTS5 Query Construction**: Build expanded query using FTS5 OR syntax with unquoted terms
   - Example: "ML" expands to `ML OR machine-learning OR machine learning`
   - Unquoted terms allow porter stemming to work

5. **Always Active**: Alias expansion is always enabled (no flag to disable)

6. **Index Unchanged**: Only query processing changes; FTS index content remains unchanged

### Reusability Opportunities

- Existing `resolve_alias()` method in NoteService can inform the lookup pattern
- Existing `list_aliases()` method shows how to query tag_aliases table with metadata
- The `tag_aliases` table already has all required columns (alias, canonical_tag_id, source, confidence)

### Scope Boundaries

**In Scope:**
- Query expansion using tag_aliases table
- Bi-directional alias/canonical expansion
- Confidence-based filtering (user always, LLM >= 0.8)
- FTS5 OR query construction with unquoted terms

**Out of Scope:**
- Broader concept expansion (roadmap item 21)
- Related concept expansion
- Changes to FTS index content or triggers
- User-configurable expansion settings or flags
- Displaying which expansions were applied in output

### Technical Considerations

- FTS5 already uses porter tokenizer for stemming
- Current search implementation in `NoteService::search_notes()` escapes and quotes terms
- Need to modify query construction to support unquoted OR expressions
- Tag_aliases table uses COLLATE NOCASE for case-insensitive matching
- Must handle case where a term matches both as alias AND as canonical tag name

### Algorithm Outline

```
For each search term in query:
  1. Normalize term (lowercase, trim)
  2. Check if term is an alias -> get canonical_tag_id (with confidence filter)
  3. Check if term matches a canonical tag name -> get all aliases (with confidence filter)
  4. Collect unique expansion terms (original + canonical + aliases)
  5. Join with OR for FTS5 query

Confidence filter:
  - source = 'user' -> always include
  - source = 'llm' AND confidence >= 0.8 -> include
  - source = 'llm' AND confidence < 0.8 -> exclude
```

### Reference from KNOWLEDGE.md

> "Alias expansion: 'ML' -> 'ML OR machine learning OR machine-learning' (from tag_aliases)"

> "Control expansion aggressively—uninhibited expansion produces noise. A practical heuristic: expand aliases always, broader concepts only for short queries (<3 terms), related concepts only on explicit user request."
