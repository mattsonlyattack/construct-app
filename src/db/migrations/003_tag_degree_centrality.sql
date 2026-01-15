-- Add degree centrality tracking to tags table
-- Stores the total number of edges connected to this tag (incoming + outgoing)
-- Version: 003

ALTER TABLE tags ADD COLUMN degree_centrality INTEGER DEFAULT 0;