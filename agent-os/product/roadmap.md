# Product Roadmap

Always consider how the roadmap should support @KNOWLEDGE.md

1. [x] SQLite schema design -- Create notes, tags, and note_tags junction tables with proper indexes for efficient querying and full-text search preparation `S`

2. [x] Core domain types -- Define Note, Tag, and related structs with proper Rust idioms (derive macros, Display implementations, builder patterns where appropriate) `S`

3. [x] NoteService implementation -- Build the core business logic layer independent of any UI, handling note CRUD operations and tag management `M`

4. [x] CLI: add command -- Implement `cons add "thought"` for instant note capture with optional manual tags via `--tags` flag `S`

5. [x] CLI: list command -- Implement `cons list` showing recent notes with `--tags` filtering and `--limit` pagination `S`

6. [x] Architecture validation -- Verify layered architecture by confirming NoteService can be used without CLI dependencies, proving reusability for future TUI/GUI `XS`

7. [x] Ollama HTTP client -- Build async client for Ollama API using reqwest and tokio, with proper timeout and retry handling `S`

8. [x] Auto-tagger prompt engineering -- Design and iterate on prompts for deepseek-r1:8b to extract relevant tags from note content with high accuracy `M`

9. [x] CLI: Integrate auto-tagging into `cons add` command, calling Ollama asynchronously and updating note tags `S`

10. [ ] Fail-safe error handling -- Ensure LLM failures never block note capture; notes save successfully even if tagging fails, with graceful degradation `S`

11. [x] Tag normalization -- Implement consistent tag formatting (lowercase, hyphenation, deduplication) across manual and AI-generated tags `XS`

12. [x] Tag aliases -- Implement tag_aliases table mapping alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern) to solve synonymy problems (car/auto/automobile), with LLM-suggested aliases and user confirmation workflows `M`

13. [ ] Structured logging -- Replace eprintln!/println! with tracing crate for structured logs with context (note IDs, model names, operation types), supporting RUST_LOG environment variable for log levels `S`

14. [x] Note text enhancement -- AI expands fragmentary notes into complete thoughts with clarified intent, storing both original and enhanced versions with provenance metadata `M`

15. [x] Full-text search with FTS5 -- Implement SQLite FTS5 virtual table for content search, with `cons search "query"` command; foundation for dual-channel retrieval `M`

16. [ ] Alias-expanded FTS -- Integrate tag_aliases into search queries, expanding "ML" to "ML OR machine-learning OR machine learning" before FTS5 matching; automatic synonym bridging `S`

17. [ ] Graph schema foundation -- Create edges table with confidence (REAL), hierarchy_type ('generic'|'partitive'|NULL), valid_from/valid_until (TIMESTAMP nullable); enables spreading activation and temporal queries `M`

18. [ ] Tag hierarchy population -- LLM suggests broader/narrower relationships between existing tags with confidence scores; user confirms via CLI; distinguish generic (is-a: "transformer" specializes "neural-network") from partitive (part-of: "attention" isPartOf "transformer") using XKOS semantics `M`

19. [ ] Spreading activation retrieval -- Implement recursive CTE spreading activation from query tags through edges with decay=0.7, threshold=0.1, max_hops=3; accumulate scores to surface hub notes connecting multiple query concepts; cognitive psychology foundation per KNOWLEDGE.md `M`

20. [ ] Dual-channel search -- Combine FTS5 results with spreading activation using intersection boost (1.5x multiplier for notes found by both channels); graceful degradation to FTS-only when graph density below threshold (cold-start handling) `M`

21. [ ] Query expansion -- Before FTS, expand query using aliases (always), broader concepts (for short queries <3 terms); aggressive noise control to prevent over-expansion; configurable expansion depth `S`

22. [ ] Degree centrality -- Precompute connection count per tag/concept, update incrementally on edge changes; use for "most connected" queries, visualization node sizing, and importance signals in retrieval ranking `S`

23. [ ] Integration tests -- Build comprehensive test suite covering happy paths for add, list, search, and auto-tagging workflows `M`

24. [ ] Metrics collection -- Add metrics crate for LLM call metrics (latency, success rate, retry counts), tag generation metrics (tags per note, confidence distribution), and database operation metrics (query duration, operation counts) with optional file-based export `M`

25. [ ] Error message polish -- Ensure all user-facing errors are clear and actionable, following error handling standards `S`

26. [ ] OpenTelemetry integration -- Add OpenTelemetry support for distributed tracing and metrics export, enabling integration with observability backends (Jaeger, Prometheus, etc.) while maintaining local-first privacy `M`

27. [ ] README documentation -- Write usage examples, installation instructions, and architecture overview for open source release `S`

28. [ ] ARCHITECTURE.md -- Document system design decisions, layered architecture, and future extensibility for work sample context `S`

29. [ ] GitHub Actions CI -- Set up automated testing, linting (clippy), and formatting checks on pull requests `S`

30. [ ] Ratatui TUI foundation -- Build terminal UI scaffold using ratatui with basic layout (note list, detail view, search input) `M`

31. [ ] TUI note browsing -- Implement scrollable note list with keyboard navigation, displaying note content and tags `M`

32. [ ] TUI search and filtering -- Add interactive search and tag filtering within TUI, reusing NoteService for all operations `S`

33. [ ] Architecture proof -- Demonstrate that TUI and CLI share identical NoteService with zero code duplication in business logic `XS`

34. [ ] Entity mention extraction -- LLM identifies people, projects, concepts mentioned in notes; store as note_entities junction with confidence (REAL) and mention_type ('about'|'mentions'); aboutness vs. mention distinction per KNOWLEDGE.md `L`

35. [ ] Entity resolution -- Merge duplicate entities using alias detection (fuzzy matching + LLM suggestions) and user confirmation; link resolved entities to canonical concept IDs; prevents entity proliferation `M`

36. [ ] Relationship suggestion UI -- LLM infers semantic relationships (supports, contradicts, extends) between notes; surface as suggestions in TUI/CLI, never auto-assert; user confirms/rejects; only confirmed relationships affect retrieval `M`

37. [ ] PageRank computation -- Periodic background calculation of PageRank scores for concepts; store as precomputed column updated on startup or significant graph changes; use for "authoritative notes" ranking `M`

38. [ ] Temporal retrieval -- Implement recency-weighted activation (boost = 1.0 + 0.5×e^(-days/30)), enforce validity windows on edges (valid_from/valid_until), support historical queries ("what did I believe about X in 2023") `M`

39. [ ] Concept schemes -- Namespace concepts into work/personal/project schemes; scheme_id on concepts table; scheme membership provides retrieval boost (1.5x) or hard filter; cross-scheme links remain possible `M`

40. [ ] Betweenness centrality -- Offline computation identifying concepts that bridge different topic clusters; surface as "cross-domain insights" or "connector notes"; expensive, compute in background `M`

41. [ ] Vector embeddings (conditional) -- Only implement if retrieval metrics show FTS+graph fails for >20% of user queries; use local embedding model (e.g., all-MiniLM-L6-v2) for privacy; store in separate table with note_id foreign key `L`

42. [ ] GUI desktop app -- Tauri-based graphical interface reusing same NoteService layer `XL`

43. [ ] Note editing -- Add `cons edit` command for modifying existing notes with re-tagging `M`

44. [ ] Import from other apps -- Bulk import from common formats (Markdown files, Notion export, Apple Notes) `L`

> Notes
> - Order reflects technical dependencies and KNOWLEDGE.md phasing: FTS → graph schema → spreading activation → dual-channel → entity extraction → relationship inference
> - Graph retrieval (items 19-22) can be built on existing tag infrastructure before entity extraction
> - Relationship inference (item 36) surfaces suggestions only; never auto-asserts per KNOWLEDGE.md risk analysis
> - Vector embeddings (item 41) are conditional—only implement if FTS+graph retrieval proves insufficient
> - Each item represents end-to-end testable functionality
> - Effort estimates: XS (1 day), S (2-3 days), M (1 week), L (2 weeks), XL (3+ weeks)
