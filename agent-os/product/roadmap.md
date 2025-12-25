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

11. [ ] Tag normalization -- Implement consistent tag formatting (lowercase, hyphenation, deduplication) across manual and AI-generated tags `XS`

12. [ ] Tag aliases -- Implement tag_aliases table mapping alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern) to solve synonymy problems (car/auto/automobile), with LLM-suggested aliases and user confirmation workflows `M`

13. [ ] Structured logging -- Replace eprintln!/println! with tracing crate for structured logs with context (note IDs, model names, operation types), supporting RUST_LOG environment variable for log levels `S`

14. [ ] Note text enhancement -- AI expands fragmentary notes into complete thoughts with clarified intent, storing both original and enhanced versions with provenance metadata `M`

15. [ ] Full-text search with FTS5 -- Implement SQLite FTS5 virtual table for content search, with `cons search "query"` command `M`

16. [ ] Integration tests -- Build comprehensive test suite covering happy paths for add, list, search, and auto-tagging workflows `M`

17. [ ] Metrics collection -- Add metrics crate for LLM call metrics (latency, success rate, retry counts), tag generation metrics (tags per note, confidence distribution), and database operation metrics (query duration, operation counts) with optional file-based export `M`

18. [ ] Error message polish -- Ensure all user-facing errors are clear and actionable, following error handling standards `S`

19. [ ] OpenTelemetry integration -- Add OpenTelemetry support for distributed tracing and metrics export, enabling integration with observability backends (Jaeger, Prometheus, etc.) while maintaining local-first privacy `M`

20. [ ] README documentation -- Write usage examples, installation instructions, and architecture overview for open source release `S`

21. [ ] ARCHITECTURE.md -- Document system design decisions, layered architecture, and future extensibility for work sample context `S`

22. [ ] GitHub Actions CI -- Set up automated testing, linting (clippy), and formatting checks on pull requests `S`

23. [ ] Ratatui TUI foundation -- Build terminal UI scaffold using ratatui with basic layout (note list, detail view, search input) `M`

24. [ ] TUI note browsing -- Implement scrollable note list with keyboard navigation, displaying note content and tags `M`

25. [ ] TUI search and filtering -- Add interactive search and tag filtering within TUI, reusing NoteService for all operations `S`

26. [ ] Architecture proof -- Demonstrate that TUI and CLI share identical NoteService with zero code duplication in business logic `XS`

27. [ ] Semantic search -- Add vector embeddings (local model) for meaning-based retrieval beyond keyword matching `L`

28. [ ] Entity extraction -- Automatically identify and index people, projects, dates, and concepts mentioned in notes `L`

29. [ ] Relationship mapping -- AI-discovered connections between notes based on shared entities and semantic similarity `L`

30. [ ] GUI desktop app -- Tauri-based graphical interface reusing same NoteService layer `XL`

31. [ ] Note editing -- Add `cons edit` command for modifying existing notes with re-tagging `M`

32. [ ] Import from other apps -- Bulk import from common formats (Markdown files, Notion export, Apple Notes) `L`

> Notes
> - Order reflects technical dependencies: schema before service, service before CLI, CLI before AI integration
> - Each item represents end-to-end testable functionality
> - Effort estimates: XS (1 day), S (2-3 days), M (1 week), L (2 weeks), XL (3+ weeks)
