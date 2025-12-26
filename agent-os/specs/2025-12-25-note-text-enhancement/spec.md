# Specification: Note Text Enhancement

## Goal

Automatically expand fragmentary notes into complete, clarified thoughts using LLM inference while preserving the original capture, with provenance metadata and confidence scores to maintain transparency and trust in AI-augmented content.

## User Stories

- As a user capturing quick thoughts, I want my fragmentary notes automatically expanded into complete sentences so that I can retrieve coherent information later without remembering my shorthand.
- As a user reviewing my notes, I want to see both the original and enhanced versions with a confidence score so that I can judge the quality of the enhancement and fall back to my original intent when needed.

## Specific Requirements

**Automatic Enhancement on Capture**
- Enhancement runs synchronously during `cons add`, blocking until complete (matches current tagging behavior)
- Enhancement executes AFTER note is saved but BEFORE tagging (original content preserved first, then enhanced, then original tagged)
- LLM failures never block note capture; note saves with `content_enhanced = NULL`
- No automatic retry of failed enhancements to preserve capture-first philosophy

**Enhancement Quality and Prompt Design**
- LLM prompt must instruct the model to: expand abbreviations, complete sentence fragments, clarify implicit context, and preserve original intent
- Prompt must include explicit instruction to return a confidence score (0.0-1.0) assessing enhancement quality
- Prompt must instruct model NOT to add information not implied by the original text
- For notes that are already complete thoughts, enhancement should return the original with high confidence
- Handle edge cases: very short notes (1-3 words), notes with code blocks, notes with URLs

**LLM Response Format**
- LLM returns JSON with two fields: `enhanced_content` (string) and `confidence` (float 0.0-1.0)
- Confidence score reflects model's assessment of: how fragmentary the original was, how much interpretation was required, and certainty in the expansion
- Low confidence (< 0.5) indicates significant guesswork or ambiguity
- High confidence (> 0.8) indicates straightforward expansion with clear intent

**Provenance Metadata Storage**
- Store enhancement metadata alongside the enhanced content in the `notes` table
- Track: `enhanced_at` (Unix timestamp), `enhancement_model` (e.g., "deepseek-r1:8b"), `enhancement_confidence` (0.0-1.0)
- Metadata enables future filtering/sorting by enhancement quality

**Schema Changes**
- Add four nullable columns to `notes` table using idempotent `ALTER TABLE` pattern
- Columns: `content_enhanced` (TEXT), `enhanced_at` (INTEGER), `enhancement_model` (TEXT), `enhancement_confidence` (REAL)
- Schema changes must be additive only; no modifications to existing columns
- Follow existing IF NOT EXISTS pattern for safe re-execution

**CLI Display Format (Stacked)**
- `cons list` and `cons show` display both versions when enhancement exists
- Format: Original content first, then visual separator, then enhanced content with confidence
- Separator line: `---` (three dashes)
- Confidence displayed as percentage: `(enhanced: 85% confidence)`
- When no enhancement exists, display only original content without separator

**NoteEnhancer Module**
- Create new `src/enhancer.rs` module following AutoTagger patterns
- Implement `NoteEnhancerBuilder` for ergonomic construction with OllamaClient
- `enhance_content()` method returns `EnhancementResult { enhanced_content: String, confidence: f64 }`
- Reuse OllamaClient and error handling patterns from autotagger

**Note Model Updates**
- Add optional fields to `Note` struct: `content_enhanced`, `enhanced_at`, `enhancement_model`, `enhancement_confidence`
- Extend `NoteBuilder` with methods for setting enhancement fields
- Add accessor methods: `content_enhanced()`, `enhanced_at()`, `enhancement_model()`, `enhancement_confidence()`

## Existing Code to Leverage

**AutoTagger Module (`src/autotagger/tagger.rs`)**
- Reuse PROMPT_TEMPLATE pattern for enhancement prompt design
- Copy `extract_json()` helper for parsing LLM JSON responses
- Follow `AutoTaggerBuilder` pattern for `NoteEnhancerBuilder`
- Reuse confidence score normalization (clamping to 0.0-1.0 range)

**OllamaClient (`src/ollama/client.rs`)**
- Reuse `OllamaClientTrait` for mockable LLM calls
- Leverage existing retry logic with exponential backoff
- Use same timeout configuration (60s) for enhancement requests

**NoteService (`src/service.rs`)**
- Follow `add_tags_to_note` pattern for adding enhancement data to existing notes
- Extend `create_note` transaction to include enhancement call
- Reuse error handling pattern: catch enhancement errors, log, continue

**CLI Main (`src/main.rs`)**
- Extend `execute_add` to call enhancement after note creation (parallel to auto_tag_note)
- Follow fail-safe error handling pattern: log enhancement errors without failing command

**Schema Pattern (`src/db/schema.rs`)**
- Use same idempotent pattern for adding new columns

## Out of Scope

- Batch enhancement of existing notes (no migration of historical notes)
- Undo/rollback of enhancements (enhancement is one-time, not versioned)
- Side-by-side diff view comparing original and enhanced
- On-demand enhancement command (e.g., `cons enhance <id>`)
- Note editing (feature doesn't exist yet)
- Background/async enhancement processing (synchronous only)
- Enhancement of notes during `cons list` or `cons show` (enhancement only on capture)
- User configuration of enhancement behavior (always automatic)
- Multiple enhancement models or A/B testing
- Enhancement quality feedback loop or learning from corrections
