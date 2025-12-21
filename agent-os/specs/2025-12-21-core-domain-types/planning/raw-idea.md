# Raw Idea

Define Note, Tag, and related structs with proper Rust idioms (derive macros, Display implementations, builder patterns where appropriate).

## Key Requirements from KNOWLEDGE.md

The domain types must support:

1. **SKOS-inspired vocabulary patterns** (without full RDF complexity):
   - Preferred and alternate labels for tags (solving synonymy: "ML" → "machine learning")
   - Broader/narrower relationships for natural hierarchies
   - Tag aliases mapping to canonical IDs

2. **AI-first metadata on all LLM-inferred data**:
   - Confidence scores (0.0-1.0)
   - Provenance tracking (`source: "user" | "llm"`)
   - Timestamps for all inferences
   - User verification flags

3. **Property graph model fundamentals**:
   - Nodes: notes, concepts/tags, structured records (contacts, events, bibliographic entries)
   - Edges: relationships with properties (type, confidence, provenance, timestamps)
   - Support for both structured and unstructured content

4. **Fail-safe design principle**:
   - Notes must be capturable even if AI tagging fails
   - All LLM inferences stored immediately, never blocking capture
   - Correction happens during retrieval, not during capture

5. **Folksonomy-first organization**:
   - User vocabulary is inherently correct
   - Emergent structure beats imposed taxonomy
   - Support for faceted classification (type, domain, status, source)
