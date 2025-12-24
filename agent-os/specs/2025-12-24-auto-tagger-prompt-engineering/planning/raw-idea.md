# Raw Idea

## User Description

roadmap #8, one note i currently have gemma3:4b model available in ollama

## Roadmap Item #8

Auto-tagger prompt engineering -- Design and iterate on prompts for deepseek-r1:8b to extract relevant tags from note content with high accuracy

**Effort Estimate:** M (1 week)

**Dependencies:**
- Roadmap #7: Ollama HTTP client (completed)

**Enables:**
- Roadmap #9: CLI: --auto-tag flag
- Roadmap #10: Fail-safe error handling
- Roadmap #11: Tag normalization

## Context

The project currently has:
- Working Ollama HTTP client (async with retry logic)
- Note capture and storage system
- Manual tagging capability
- User has gemma3:4b model available in Ollama (in addition to deepseek-r1:8b mentioned in roadmap)

Goal: Design prompts that can extract relevant, high-quality tags from user notes to support automatic organization of thoughts in the personal knowledge management system.
