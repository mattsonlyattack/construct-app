# Spec Requirements: Note Text Enhancement

## Initial Description

AI expands fragmentary notes into complete thoughts with clarified intent, storing both original and enhanced versions with provenance metadata.

## Requirements Discussion

### First Round Questions

**Q1:** I assume enhancement should be optional and on-demand (e.g., `cons enhance 123` or `cons add "quick thought" --enhance`) rather than automatic on every capture. This preserves the zero-friction capture philosophy. Is that correct, or should enhancement be automatic with an opt-out flag?
**Answer:** Automatic on every capture - enhancement happens automatically, not on-demand.

**Q2:** I'm thinking enhancement should happen synchronously when requested, blocking until complete (similar to current tagging behavior). Should we instead queue enhancements for background processing, or is synchronous fine for an on-demand feature?
**Answer:** Synchronous for now - blocking until complete.

**Q3:** I assume we should store the enhanced version as a separate field on the same note record (e.g., `enhanced_content` alongside `content`) rather than creating a separate "versions" table. This keeps queries simple and follows the existing pattern. Is that correct, or do you need full version history with multiple enhancement iterations?
**Answer:** Yes, `content_enhanced` field - separate field on same note record.

**Q4:** For provenance metadata, I'm planning to store: `enhanced_at` (timestamp), `enhancement_model` (e.g., "deepseek-r1:8b"), and `enhancement_source` ("llm" to match tag provenance patterns). Should we also track: enhancement confidence/quality score, prompt version used, or LLM token counts?
**Answer:** Add confidence/quality score to provenance metadata.

**Q5:** I assume the CLI should default to showing the enhanced version when available, with a `--original` flag to view the raw capture (e.g., `cons show 123 --original`). Is that correct, or should we show original by default with `--enhanced` to see the expanded version?
**Answer:** Show both versions - not defaulting to one or the other.

**Q6:** When a user edits a note that has been enhanced, should editing: (a) modify the original and clear/regenerate the enhancement, (b) modify the enhanced version and preserve the original, or (c) prompt the user to choose which version to edit?
**Answer:** No editing to worry about yet - editing feature doesn't exist.

**Q7:** For the tagging workflow, I assume we should tag the **original** content (preserving user intent) rather than the enhanced version. This ensures tags reflect what the user actually captured. Is that correct, or should tagging consider the enhanced content for richer tag extraction?
**Answer:** Tag just the original - preserve user intent for tagging.

**Q8:** Is there anything that should explicitly be OUT of scope for this feature? For example: batch enhancement of existing notes, undo/rollback of enhancements, or side-by-side diff view?
**Answer:** Out of scope: batch enhancement of existing notes, undo/rollback, side-by-side diff view - stick to core capability.

### Existing Code to Reference

**Similar Features Identified:**
- Feature: Autotagger - Path: `/home/md/construct-app/src/autotagger.rs`
- Components to potentially reuse: LLM call patterns, OllamaClient integration
- Backend logic to reference: Provenance metadata patterns in schema (`source`, `confidence`, `model_version` fields in `note_tags` and `tag_aliases` tables)

### Follow-up Questions

**Follow-up 1:** Since enhancement happens automatically on every capture, and fail-safe AI is a core principle, I need to clarify: when the LLM is unavailable, should the note save with `content_enhanced = NULL` (enhancement simply missing), or should we retry enhancement later?
**Answer:** NULL for now - save immediately with `content_enhanced = NULL` when LLM unavailable, no retry. Keeps capture instant per fail-safe principle.

**Follow-up 2:** When you say "show both versions," what format do you envision for `cons list` and `cons show`?
**Answer:** Stacked format - show original first, then enhanced below with a separator.

**Follow-up 3:** For the confidence/quality score: should this be visible to users in the CLI output (e.g., "Enhanced (confidence: 0.85)"), or is it primarily for internal tracking and future filtering?
**Answer:** Show confidence score - visible to users in CLI output.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements
- Automatic text enhancement on every `cons add` command
- LLM expands fragmentary notes into complete thoughts with clarified intent
- Both original and enhanced versions stored and displayed
- Provenance metadata tracked for each enhancement
- Confidence score calculated and displayed to users

### Data Model Changes

**New fields on `notes` table:**
- `content_enhanced` (TEXT, nullable) - AI-expanded version of the note
- `enhanced_at` (INTEGER, nullable) - Unix timestamp when enhancement occurred
- `enhancement_model` (TEXT, nullable) - Model used (e.g., "deepseek-r1:8b")
- `enhancement_confidence` (REAL, nullable) - Quality/confidence score from LLM

### CLI Output Format

**Stacked display format for `cons list` and `cons show`:**
```
Original: [user's raw input]
---
Enhanced: [AI-expanded version]
(confidence: 0.85)
```

### Fail-Safe Behavior
- Note capture is never blocked by LLM failures
- When LLM is unavailable: save note immediately with `content_enhanced = NULL`
- No automatic retry of failed enhancements
- Original content always preserved regardless of enhancement status

### Reusability Opportunities
- Autotagger module patterns for LLM integration
- OllamaClient for HTTP calls to Ollama
- Existing provenance metadata schema patterns (`source`, `confidence`, `model_version`)
- Prompt engineering patterns from auto-tagger

### Scope Boundaries

**In Scope:**
- Automatic enhancement on every new note capture
- Storage of both original and enhanced content
- Provenance metadata (timestamp, model, confidence)
- Stacked CLI display showing both versions
- Confidence score visible in CLI output
- Fail-safe behavior when LLM unavailable

**Out of Scope:**
- Batch enhancement of existing notes
- Undo/rollback of enhancements
- Side-by-side diff view
- On-demand enhancement (e.g., `cons enhance <id>`)
- Note editing (feature doesn't exist yet)
- Background/async enhancement processing

### Technical Considerations
- Enhancement runs synchronously, blocking until complete
- Tagging operates on original content only (preserves user intent)
- Enhancement and tagging are separate LLM calls
- Schema changes required: new nullable columns on `notes` table
- Follows existing idempotent schema pattern (ALTER TABLE IF NOT EXISTS or similar)
- Must integrate with existing `cons add` workflow without breaking current functionality
