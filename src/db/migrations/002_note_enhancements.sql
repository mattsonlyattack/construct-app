-- Add enhancement fields to notes table
-- These columns store LLM-enhanced versions of fragmentary notes with provenance metadata
-- Version: 002

ALTER TABLE notes ADD COLUMN content_enhanced TEXT;
ALTER TABLE notes ADD COLUMN enhanced_at INTEGER;
ALTER TABLE notes ADD COLUMN enhancement_model TEXT;
ALTER TABLE notes ADD COLUMN enhancement_confidence REAL;