# Spec Requirements: Auto-tagger Prompt Engineering

## Initial Description

Roadmap #8: Auto-tagger prompt engineering -- Design and iterate on prompts for deepseek-r1:8b to extract relevant tags from note content with high accuracy.

**Effort Estimate:** M (1 week)

**Dependencies:**
- Roadmap #7: Ollama HTTP client (completed)

**Enables:**
- Roadmap #9: CLI: --auto-tag flag
- Roadmap #10: Fail-safe error handling
- Roadmap #11: Tag normalization (NOW INCLUDED IN THIS SPEC)

**Context:**
The project currently has:
- Working Ollama HTTP client (async with retry logic)
- Note capture and storage system
- Manual tagging capability
- User has gemma3:4b model available in Ollama (in addition to deepseek-r1:8b mentioned in roadmap)

Goal: Design prompts that can extract relevant, high-quality tags from user notes to support automatic organization of thoughts in the personal knowledge management system.

## Requirements Discussion

### First Round Questions

**Q1:** I assume the prompt should return tags as a simple comma-separated list (e.g., "rust, async, error-handling") that can be easily parsed. Is that correct, or would you prefer structured JSON output from the model?
**Answer:** Structured JSON because of confidence scoring requirement.

**Q2:** Based on KNOWLEDGE.md, I assume tags should be applied immediately with confidence metadata stored (source: "llm") but no confidence scores for MVP since parsing numeric confidence from small models is unreliable. Should we store `source: "llm"` only, or do you want to attempt confidence scoring?
**Answer:** Yes, include a confidence score in the output.

**Q3:** I assume we should target 3-7 tags per note as a reasonable range - enough to be useful for filtering, not so many as to cause tag explosion. Does that feel right, or do you have a different target range?
**Answer:** Yes, 3-7 tags per note is correct.

**Q4:** For tag normalization, I assume we should instruct the model to output lowercase, hyphenated tags (e.g., "machine-learning" not "Machine Learning") to reduce post-processing. Should tag normalization happen in the prompt instructions, or defer to roadmap item 11 (Tag normalization) as a separate post-processing step?
**Answer:** Implement roadmap #11 now - normalize the tags as part of this spec (not deferred).

**Q5:** Based on KNOWLEDGE.md's distinction between "aboutness" and "mention" - should the prompt explicitly instruct the model to focus on what the note is ABOUT (primary topics) rather than things merely mentioned? For example, a note mentioning "Python" while actually being about debugging strategies should be tagged "debugging" not "python".
**Answer:** Yes, prompt should focus on what the note is ABOUT, not things merely mentioned.

**Q6:** You mentioned having gemma3:4b available. I assume we should design prompts primarily for gemma3:4b with the understanding they should also work reasonably well with deepseek-r1:8b. Should we optimize specifically for gemma3:4b, or create model-agnostic prompts?
**Answer:** Model-agnostic prompts (should work across gemma3:4b, deepseek-r1:8b, and others).

**Q7:** For the prompt iteration process, I assume we need a way to evaluate prompt quality. Should this spec include creating a small test corpus of notes with expected tags, or defer evaluation methodology to a separate effort?
**Answer:** Set the foundation for prompt evaluations and include a few tests (likely ignored until GitHub Actions is figured out).

**Q8:** What should be explicitly OUT of scope for this prompt engineering work? For example: entity extraction, relationship inference, note enhancement, multi-language support?
**Answer:** Future roadmap items are out of scope EXCEPT #11 (tag normalization) which is now IN scope.

### Existing Code to Reference

No similar existing features identified for reference. The OllamaClient at `/home/md/construct-app/src/ollama/client.rs` provides the `generate()` method that will be used to call the prompts.

### Follow-up Questions

**Follow-up 1:** For the JSON structure, I'm thinking something like:
```json
{
  "tags": [
    {"tag": "rust", "confidence": 0.9},
    {"tag": "error-handling", "confidence": 0.75}
  ]
}
```
Is that the structure you have in mind, or would you prefer a simpler format like `{"rust": 0.9, "error-handling": 0.75}`?
**Answer:** Option B (simple key-value format): `{"rust": 0.9, "error-handling": 0.75}` - this is sufficient for now.

**Follow-up 2:** For normalization rules, I assume: lowercase, replace spaces with hyphens, remove special characters, deduplicate. Should we also handle common synonyms/aliases (e.g., "ML" -> "machine-learning", "JS" -> "javascript"), or keep normalization purely syntactic for MVP?
**Answer:** Purely syntactic normalization for now (lowercase, hyphens, remove special chars, dedupe) - NO synonym/alias handling for MVP.

**Follow-up 3:** Should normalization happen:
- (a) In the prompt itself (instruct the model to output normalized tags), OR
- (b) As a post-processing step in Rust code after parsing the JSON response, OR
- (c) Both (prompt tries to normalize, code ensures consistency)?
**Answer:** Both (c) - prompt instructs model to output normalized tags AND Rust code ensures consistency as post-processing.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

- **Prompt Design**: Create model-agnostic prompts that extract tags from note content
  - Must work across gemma3:4b, deepseek-r1:8b, and other Ollama-compatible models
  - Prompt instructs model to focus on "aboutness" (primary topics) not mere mentions
  - Prompt instructs model to output normalized tags (lowercase, hyphenated)

- **JSON Output Format**: Model returns structured JSON with confidence scores
  - Format: `{"tag-name": 0.9, "another-tag": 0.75}`
  - Simple key-value pairs where key is tag name, value is confidence (0.0-1.0)

- **Tag Quantity**: Target 3-7 tags per note
  - Enough for useful filtering
  - Not so many as to cause tag explosion

- **Tag Normalization** (Roadmap #11 - now in scope):
  - Lowercase all tags
  - Replace spaces with hyphens
  - Remove special characters
  - Deduplicate tags
  - Two-layer approach: prompt instructs model to normalize AND Rust post-processing ensures consistency
  - NO synonym/alias handling for MVP (purely syntactic normalization)

- **Confidence Scoring**:
  - Each tag includes a confidence score (0.0-1.0)
  - Enables future filtering by confidence threshold
  - Supports KNOWLEDGE.md principle of "tiered confidence with correction affordances"

- **Evaluation Foundation**:
  - Create small test corpus of notes with expected tags
  - Include a few prompt evaluation tests
  - Tests may be ignored initially until GitHub Actions is configured

### Reusability Opportunities

- OllamaClient at `/home/md/construct-app/src/ollama/client.rs` provides the `generate()` method
- OllamaClientTrait enables mocking for tests
- Existing retry logic handles transient failures

### Scope Boundaries

**In Scope:**
- Prompt design and iteration for tag extraction
- JSON output format with confidence scores
- Tag normalization (roadmap #11) - syntactic only
- Model-agnostic prompt design
- Foundation for prompt evaluation with initial tests
- Rust code for parsing JSON response and normalizing tags

**Out of Scope:**
- Entity extraction (roadmap #24)
- Relationship inference (roadmap #25)
- Note text enhancement (roadmap #12)
- Multi-language support
- Synonym/alias handling for tags (defer to future)
- CLI integration (roadmap #9 - separate spec)
- Fail-safe error handling (roadmap #10 - separate spec)

### Technical Considerations

- **Integration Point**: Uses existing `OllamaClient.generate(model, prompt)` method
- **Async Context**: Ollama calls are async (tokio runtime)
- **Model Flexibility**: Prompts must work across different Ollama models without modification
- **JSON Parsing**: Need robust parsing that handles model output variations
- **Post-processing**: Rust code normalizes tags even if model output is imperfect
- **Testing**: Prompt evaluation tests should use mockable OllamaClientTrait

### Key Design Principles (from KNOWLEDGE.md)

- **Folksonomy-first**: User's vocabulary is correct; tags should emerge from content
- **Aboutness vs Mention**: Distinguish primary topics from incidental references
- **AI-first personal tools**: Apply tags immediately, correct during retrieval not capture
- **Confidence metadata**: Store confidence scores to enable later filtering/correction
- **Error recovery through use**: Don't block capture for perfect tagging
