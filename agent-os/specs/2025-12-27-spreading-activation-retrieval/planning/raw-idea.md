# Raw Idea

**Spreading activation retrieval** - Implement recursive CTE spreading activation from query tags through edges with decay=0.7, threshold=0.1, max_hops=3; accumulate scores to surface hub notes connecting multiple query concepts; cognitive psychology foundation per KNOWLEDGE.md

## Context

This is from the cons project - a structure-last personal knowledge management CLI tool. The project uses SQLite + Ollama for local-first, privacy-focused knowledge management.

Key context:
- The edges table already exists with confidence scores and hierarchy types (generic/partitive)
- Tag hierarchy population (item 18) has just been implemented
- This enables graph-based retrieval alongside the existing FTS5 search
- Foundation for dual-channel search (item 20) which combines FTS + graph retrieval
