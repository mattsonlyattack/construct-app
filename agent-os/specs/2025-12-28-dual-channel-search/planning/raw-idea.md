# Raw Idea

**Dual-channel search** -- Combine FTS5 results with spreading activation using intersection boost (1.5x multiplier for notes found by both channels); graceful degradation to FTS-only when graph density below threshold (cold-start handling)

## Context

- Roadmap item #20 from "cons" personal knowledge management CLI tool
- Tool uses SQLite + Ollama for local-first, privacy-focused note management
- This feature builds on existing FTS5 and spreading activation capabilities
