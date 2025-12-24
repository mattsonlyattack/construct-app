# Task Breakdown: Auto-tagger Prompt Engineering

## Overview

This spec implements model-agnostic prompts for local LLMs to extract tags from note content with confidence scores, plus a two-layer normalization strategy (prompt instructions + Rust post-processing) for consistent tag formatting.

**Total Task Groups:** 4
**Estimated Effort:** M (1 week)

## Task List

### Foundation Layer

#### Task Group 1: TagNormalizer Module
**Dependencies:** None

Creates the Rust post-processing layer for tag normalization, ensuring consistent formatting regardless of LLM output quality.

- [x] 1.0 Complete TagNormalizer module
  - [x] 1.1 Write 4-6 focused tests for TagNormalizer functionality
    - Test lowercase conversion (e.g., "RUST" -> "rust")
    - Test space-to-hyphen replacement (e.g., "machine learning" -> "machine-learning")
    - Test special character removal (e.g., "c++" -> "c", "rust!" -> "rust")
    - Test deduplication case-insensitive (e.g., ["Rust", "rust", "RUST"] -> ["rust"])
    - Test trimming of whitespace and leading/trailing hyphens
    - Test combined normalization (multiple transformations in sequence)
  - [x] 1.2 Create `src/autotagger.rs` module file
    - Add `mod normalizer;` declaration
    - Re-export `TagNormalizer` for public use
  - [x] 1.3 Create `src/autotagger/normalizer.rs` with TagNormalizer struct
    - Implement `normalize_tag(tag: &str) -> String` method
      - Convert to lowercase
      - Replace spaces with hyphens
      - Remove all characters except alphanumeric and hyphens
      - Trim leading/trailing whitespace and hyphens
    - Implement `normalize_tags(tags: Vec<String>) -> Vec<String>` method
      - Apply normalize_tag to each tag
      - Deduplicate case-insensitively (keep first occurrence)
      - Filter out empty strings
  - [x] 1.4 Update `src/lib.rs` to export autotagger module
    - Add `pub mod autotagger;`
    - Re-export key types for convenience
  - [x] 1.5 Ensure TagNormalizer tests pass
    - Run ONLY the tests written in 1.1
    - Verify all normalization rules work correctly

**Acceptance Criteria:**
- All 4-6 TagNormalizer tests pass
- `normalize_tag("Machine Learning!")` returns `"machine-learning"`
- `normalize_tags(vec!["Rust", "rust", "RUST"])` returns `vec!["rust"]`
- Empty strings and whitespace-only inputs are filtered out

---

### Core Implementation Layer

#### Task Group 2: AutoTagger Struct and Prompt Design
**Dependencies:** Task Group 1

Implements the AutoTagger struct with model-agnostic prompt template and JSON parsing logic.

- [x] 2.0 Complete AutoTagger implementation
  - [x] 2.1 Write 4-6 focused tests for AutoTagger functionality
    - Test prompt construction includes note content
    - Test JSON parsing of valid model output: `{"rust": 0.9, "async": 0.75}`
    - Test JSON extraction from markdown code blocks: ` ```json\n{...}\n``` `
    - Test JSON extraction from text with preamble/postamble
    - Test fail-safe behavior: return empty HashMap on parse failure
    - Test confidence score clamping to 0.0-1.0 range
  - [x] 2.2 Create `src/autotagger/tagger.rs` with AutoTagger struct
    - Struct fields:
      - `client: Arc<dyn OllamaClientTrait>` (for async safety)
    - Constructor: `new(client: Arc<dyn OllamaClientTrait>) -> Self`
  - [x] 2.3 Implement prompt template as const
    - Include system instruction for JSON output only
    - Include "aboutness vs mention" guidance
    - Include tag quantity target (3-7 tags)
    - Include normalization instructions (lowercase, hyphens)
    - Include 2-3 few-shot examples demonstrating expected output format
    - Design for model-agnostic compatibility (clear, explicit instructions)
  - [x] 2.4 Implement `extract_json(response: &str) -> Option<String>` helper
    - Handle clean JSON response (no wrapping)
    - Handle markdown code block wrapping (```json ... ```)
    - Handle explanatory text before/after JSON
    - Use regex or simple parsing to find JSON object boundaries
  - [x] 2.5 Implement `parse_tags(json_str: &str) -> HashMap<String, f64>` helper
    - Parse JSON using serde_json
    - Validate values are f64 in range 0.0-1.0
    - Clamp out-of-range values
    - Apply TagNormalizer to all keys
    - Return empty HashMap on any parse error (fail-safe)
  - [x] 2.6 Implement async `generate_tags(model: &str, content: &str) -> Result<HashMap<String, f64>, OllamaError>` method
    - Construct prompt with template + note content
    - Call `client.generate(model, prompt).await`
    - Extract JSON from response
    - Parse and normalize tags
    - Return HashMap of normalized tags to confidence scores
  - [x] 2.7 Update `src/autotagger.rs` to export AutoTagger
    - Add `mod tagger;`
    - Re-export `AutoTagger`
  - [x] 2.8 Ensure AutoTagger tests pass
    - Run ONLY the tests written in 2.1
    - Use MockOllamaClient for testing (no actual LLM calls)

**Acceptance Criteria:**
- All 4-6 AutoTagger tests pass ✓
- Prompt template includes all required elements (JSON format, aboutness, quantity, normalization) ✓
- JSON extracted correctly from various model output formats ✓
- Parse failures return empty HashMap (never panic or error) ✓
- Tags are normalized before being returned ✓

---

### Integration Layer

#### Task Group 3: OllamaClient Integration and Error Handling
**Dependencies:** Task Group 2

Integrates AutoTagger with the existing OllamaClient and ensures proper async handling.

- [x] 3.0 Complete OllamaClient integration
  - [x] 3.1 Write 3-4 focused tests for integration
    - Test AutoTagger works with mock OllamaClient returning valid JSON
    - Test AutoTagger handles OllamaError gracefully
    - Test full workflow: content -> prompt -> mock response -> normalized tags
    - Test model name is passed correctly to client
  - [x] 3.2 Verify AutoTagger uses `OllamaClientTrait` for dependency injection
    - Ensure trait bounds are correct (`Send + Sync`)
    - Verify Arc wrapping works for async contexts
  - [x] 3.3 Create `AutoTaggerBuilder` for ergonomic construction
    - Builder pattern matching `OllamaClientBuilder` style
    - Method: `client(Arc<dyn OllamaClientTrait>) -> Self`
    - Method: `build() -> AutoTagger`
  - [x] 3.4 Add integration example in module documentation
    - Show how to construct AutoTagger with OllamaClient
    - Show how to call generate_tags
    - Document expected return format
  - [x] 3.5 Ensure integration tests pass
    - Run ONLY the tests written in 3.1
    - Verify mock client integration works correctly

**Acceptance Criteria:**
- All 3-4 integration tests pass
- AutoTagger can be constructed with real OllamaClient
- AutoTagger handles network errors gracefully
- Documentation shows correct usage patterns

---

### Evaluation Foundation

#### Task Group 4: Test Corpus and Prompt Evaluation
**Dependencies:** Task Group 3

Creates foundation for prompt evaluation with sample notes and expected tags.

- [x] 4.0 Complete evaluation foundation
  - [x] 4.1 Create test corpus file `tests/fixtures/auto_tagger_corpus.json`
    - Include 5-8 sample notes with expected tags
    - Mix of short notes (1-2 sentences) and longer notes (paragraph)
    - Include notes that test "aboutness vs mention" distinction
    - Include notes with varying complexity (technical, personal, mixed)
    - Format: `[{"content": "...", "expected_tags": ["tag1", "tag2"], "notes": "..."}, ...]`
  - [x] 4.2 Write 2-4 evaluation tests (may be `#[ignore]` for CI)
    - Test that parses corpus file successfully
    - Test that demonstrates tag extraction on one sample (with mock)
    - Optional: Integration test with real Ollama (ignored by default, requires running Ollama)
    - Document how to run ignored tests locally
  - [x] 4.3 Add evaluation helpers in `src/autotagger/eval.rs` (optional)
    - Function to load test corpus from file
    - Function to compare expected vs actual tags (Jaccard similarity or precision/recall)
    - Keep simple for MVP
  - [x] 4.4 Update module documentation with evaluation guidance
    - Document how to run evaluation tests
    - Document how to add new test cases to corpus
    - Note that this is foundation for future prompt iteration
  - [x] 4.5 Ensure evaluation tests pass
    - Run ONLY non-ignored tests written in 4.2
    - Verify corpus file parses correctly

**Acceptance Criteria:**
- Test corpus file exists with 5-8 sample notes
- Corpus includes "aboutness vs mention" test cases
- At least 2 evaluation tests exist (may be ignored for CI)
- Documentation explains how to iterate on prompts

---

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: TagNormalizer Module** - Foundation with no dependencies
2. **Task Group 2: AutoTagger Struct and Prompt Design** - Core logic depending on normalizer
3. **Task Group 3: OllamaClient Integration** - Integration with existing infrastructure
4. **Task Group 4: Test Corpus and Evaluation** - Foundation for future prompt iteration

## Files Created/Modified

### New Files
- `src/autotagger.rs` - Module root
- `src/autotagger/normalizer.rs` - TagNormalizer implementation
- `src/autotagger/tagger.rs` - AutoTagger implementation
- `src/autotagger/eval.rs` - Evaluation helpers (optional)
- `tests/fixtures/auto_tagger_corpus.json` - Test corpus

### Modified Files
- `src/lib.rs` - Add autotagger module export

## Key Design Decisions

1. **Two-layer normalization**: Prompt instructs LLM to normalize + Rust code ensures consistency
2. **Fail-safe parsing**: JSON parse failures return empty HashMap, never block note capture
3. **Model-agnostic prompts**: Clear, explicit instructions with few-shot examples for small model compatibility
4. **Trait-based DI**: Use `OllamaClientTrait` for testability with mock clients
5. **Simple JSON format**: `{"tag": confidence}` pairs for easy parsing
6. **Async only for HTTP**: Match existing OllamaClient pattern (tokio for network, sync for local)

## Out of Scope (Handled by Future Specs)

- CLI integration with `--auto-tag` flag (Roadmap #9)
- Fail-safe error handling wrapper for note capture (Roadmap #10)
- Entity extraction (Roadmap #24)
- Relationship inference (Roadmap #25)
- Note text enhancement (Roadmap #12)
- Multi-language support
- Synonym/alias handling for tags
- Tag hierarchy (broader/narrower relationships)
