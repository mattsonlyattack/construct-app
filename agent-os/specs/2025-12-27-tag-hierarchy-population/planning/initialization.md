# Spec Initialization

## Raw Idea

**Tag hierarchy population** - LLM suggests broader/narrower relationships between existing tags with confidence scores; user confirms via CLI; distinguish generic (is-a: "transformer" specializes "neural-network") from partitive (part-of: "attention" isPartOf "transformer") using XKOS semantics

## Context

This is for the cons project - a structure-last personal knowledge management CLI tool. Key context:
- Local-first with SQLite + Ollama (deepseek-r1:8b model)
- Graph schema foundation already exists with edges table containing: confidence (REAL), hierarchy_type ('generic'|'partitive'|NULL), valid_from/valid_until (TIMESTAMP nullable)
- Tag aliases system already implemented
- Core business logic is in NoteService layer
- CLI commands use clap
