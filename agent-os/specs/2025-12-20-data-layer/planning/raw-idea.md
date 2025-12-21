# Data Layer - Raw Idea

Phase 1: Foundation (Week 1)
Milestone 1.1: Data Layer (Days 1-2)
Goal: Working SQLite database with migrations.

Dependencies:
- rusqlite = { version = "0.32", features = ["bundled"] }
- anyhow = "1.0"
- thiserror = "1.0"

The feature includes:
- src/db/schema.rs with INITIAL_SCHEMA constant containing CREATE TABLE statements for notes, tags, and note_tags tables with appropriate indexes
- src/db/mod.rs with Database struct that wraps rusqlite::Connection, open() and in_memory() methods, and basic tests

Deliverable: Can create database, schema exists, basic test passes.
