# Raw Idea: Tag Hierarchy Population

**Tag hierarchy population** - LLM suggests broader/narrower relationships between existing tags with confidence scores; user confirms via CLI; distinguish generic (is-a: "transformer" specializes "neural-network") from partitive (part-of: "attention" isPartOf "transformer") using XKOS semantics

## Context

This feature is part of the cons project - a structure-last personal knowledge management CLI tool. The project uses SQLite + Ollama for local-first, privacy-focused knowledge management.
