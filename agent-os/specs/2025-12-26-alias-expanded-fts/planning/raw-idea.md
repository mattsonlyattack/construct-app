# Raw Idea: Alias-expanded FTS

**Roadmap Item**: 16
**Size**: Small (S)
**Date Created**: 2025-12-26

## Description

"Alias-expanded FTS -- Integrate tag_aliases into search queries, expanding 'ML' to 'ML OR machine-learning OR machine learning' before FTS5 matching; automatic synonym bridging"

## Dependencies

This feature builds on:

- **Item 12 (completed)**: Tag aliases - tag_aliases table mapping alternate forms to canonical tag IDs
- **Item 15 (completed)**: Full-text search with FTS5 - SQLite FTS5 virtual table for content search

## Context

Users should be able to search using any alias form and get results matching all related forms. For example, searching for "ML" should automatically find notes containing "machine-learning", "machine learning", or "ML" without requiring the user to manually specify all variants.

The feature integrates the existing tag_aliases system with the FTS5 full-text search to provide automatic synonym expansion during search queries.
