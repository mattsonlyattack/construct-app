# Specification: Auto-tagger Prompt Engineering

## Goal

Design model-agnostic prompts for local LLMs (gemma3:4b, deepseek-r1:8b, etc.) to extract relevant tags from note content with confidence scores, and implement a two-layer normalization strategy that ensures consistent tag formatting through both prompt instructions and Rust post-processing.

## User Stories

- As a user capturing notes, I want tags automatically extracted from my content so that I can organize my knowledge without manual tagging effort
- As a user retrieving notes, I want confidence scores on LLM-inferred tags so that I can filter by reliability and correct low-confidence assignments during use

## Specific Requirements

**Model-agnostic prompt design**
- Prompts must produce valid JSON output from gemma3:4b, deepseek-r1:8b, and other Ollama-compatible models
- Use clear, explicit instructions that small models can follow reliably
- Include few-shot examples in the prompt to demonstrate expected output format
- Avoid complex reasoning chains that smaller models may struggle with
- Test prompts against multiple models to verify consistent behavior

**JSON output schema with confidence scores**
- Output format: `{"tag-name": 0.9, "another-tag": 0.75}` (simple key-value pairs)
- Keys are normalized tag names, values are confidence scores (0.0-1.0)
- Prompt must explicitly instruct model to output ONLY valid JSON with no additional text
- Handle model preamble/postamble text that may wrap the JSON response

**Aboutness vs mention distinction**
- Prompt instructs model to focus on what the note is ABOUT (primary topics)
- Explicitly tell model to ignore things merely mentioned in passing
- Example in prompt: a note mentioning "Python" while being about debugging should be tagged "debugging" not "python"

**Tag quantity targeting**
- Target 3-7 tags per note as specified in requirements
- Prompt includes explicit instruction for this range
- Fewer tags for short notes, more for longer/complex notes

**Tag normalization - prompt layer**
- Instruct model to output lowercase tags
- Instruct model to use hyphens instead of spaces (e.g., "machine-learning" not "machine learning")
- Instruct model to avoid special characters
- Include normalized examples in few-shot prompts

**Tag normalization - Rust post-processing layer**
- Create `TagNormalizer` module to ensure consistency regardless of model output
- Convert to lowercase (handles models that ignore instructions)
- Replace spaces with hyphens
- Remove special characters (keep alphanumeric and hyphens only)
- Deduplicate tags (case-insensitive)
- Trim leading/trailing whitespace and hyphens

**JSON parsing with robustness**
- Parse JSON output using serde_json
- Handle common model output issues: leading/trailing whitespace, markdown code blocks wrapping JSON
- Extract JSON from model response even if surrounded by explanatory text
- Return empty tag set if JSON parsing fails (fail-safe)

**Integration with OllamaClient**
- Create `AutoTagger` struct that takes `OllamaClientTrait` for testability
- Use existing `generate()` method with constructed prompt
- Accept model name as parameter (not hardcoded)
- Return `HashMap<String, f64>` of normalized tags to confidence scores

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**`/home/md/construct-app/src/ollama/client.rs` - OllamaClient**
- Use `OllamaClientTrait` for dependency injection and mocking
- Leverage existing `generate(model, prompt)` method for LLM calls
- Existing retry logic handles transient network failures automatically
- Error types (OllamaError) already defined for consistent error handling

**`/home/md/construct-app/src/models/tag_source.rs` - TagSource enum**
- Use `TagSource::llm(model, confidence)` when storing inferred tags
- Confidence stored as u8 (0-100), so multiply f64 by 100 when converting
- Model version tracking already built into TagSource::Llm variant

**`/home/md/construct-app/src/service.rs` - NoteService**
- Use `add_tags_to_note()` method to persist auto-generated tags
- Method already handles TagSource::Llm with confidence and model metadata
- Tags deduplicated case-insensitively at storage layer

**`/home/md/construct-app/KNOWLEDGE.md` - Information science principles**
- "Aboutness vs mention" distinction guides prompt design
- "Apply tags immediately with confidence metadata" pattern
- "Error recovery through use, not capture" philosophy

## Out of Scope

- CLI integration with --auto-tag flag (roadmap #9 - separate spec)
- Fail-safe error handling wrapper for note capture (roadmap #10 - separate spec)
- Entity extraction beyond simple tags (roadmap #24)
- Relationship inference between notes (roadmap #25)
- Note text enhancement or summarization (roadmap #12)
- Multi-language support for non-English notes
- Synonym/alias handling for tags (deferred to future - purely syntactic normalization only)
- Tag hierarchy or broader/narrower relationships
- Confirmation workflows or user approval before applying tags
- Streaming response handling from Ollama (use non-streaming mode)
