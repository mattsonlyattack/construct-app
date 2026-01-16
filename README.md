# cons

> **A second brain that actually remembers things without requiring you to organize them.**

`cons` is a structure-last personal knowledge management tool that lets you capture thoughts freely—with zero organizational overhead. AI handles all tagging, categorization, and relationship mapping automatically. Local-first, privacy-focused, built with Rust.

```bash
# Just capture. AI does the rest.
$ cons add "interesting pattern in async rust: using tokio::select with timeouts for graceful degradation"

✓ Note saved
  Tags (AI): rust, async, patterns, tokio
```

## The Problem

Every PKM tool imposes an "organization tax" on note capture. You must decide:
- Where to put this note
- What tags to apply
- How to link it to other notes

This friction causes three failure modes:

1. **Capture avoidance** — "Not worth organizing, I'll just remember" (you won't)
2. **Deferred organization** — Notes pile up in "Inbox" folders that never get processed
3. **Inconsistent structure** — Different organizational decisions on different days make retrieval unreliable

The result: Notes scattered across Notion, Apple Notes, random `.txt` files, and Slack DMs to yourself. Hours spent searching for information you *know* you captured but can't find.

## The Solution

Move all organizational decisions to AI. Your only job is capture. The system handles the rest—automatically, consistently, locally.

```bash
# Capture freely
$ cons add "need to research graph databases for relationship mapping"
✓ Note saved | Tags: databases, research, graphs, architecture

# Search naturally
$ cons search "database options"
Found 3 notes:
  [2 days ago] "need to research graph databases for relationship mapping"
  [1 week ago] "SQLite vs Postgres for local-first apps"
  [2 weeks ago] "property graphs vs RDF triples comparison"

# Filter by AI-generated tags
$ cons list --tags rust
Showing 12 notes tagged "rust"...
```

No menus. No decisions. No organizational overhead. Just capture and retrieve.

## Key Features

### Zero-Friction Capture
`cons add "thought"` saves immediately. No prompts, no menus, no decisions. AI tags run asynchronously—failures never block saving.

### True Zero-Effort Organization
Unlike tools where AI *suggests* organization that you must approve, `cons` organizes automatically. No confirmation gates, no decision fatigue.

### Local-First Privacy
Runs entirely on your machine. SQLite database + Ollama for local LLM inference. No cloud APIs, no subscriptions, no data leaving your computer.

### CLI-First Design
Integrates into existing terminal workflows. Capture from any terminal, anywhere. Scriptable integration with other tools.

### Fail-Safe Architecture
LLM failures never block note capture. Notes save successfully even if tagging fails. AI augments, never gatekeeps.

## Installation

### Homebrew (macOS)

```bash
brew install mattsonlyattack/tap/cons
```

### Prerequisites

[Ollama](https://ollama.ai/) with a model for AI features:

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull a model (deepseek-r1:8b recommended)
ollama pull deepseek-r1:8b
```

### Build from source

```bash
git clone https://github.com/mattsonlyattack/construct-app.git
cd construct-app
cargo build --release
cargo install --path .
```

Requires Rust 1.83+ ([install](https://rustup.rs/))

## Quick Start

```bash
# Add a note (AI tags automatically if Ollama is running)
cons add "just learned about Rust's Pin type for self-referential structs"

# Add with manual tags
cons add "meeting notes from standup" --tags work,meetings

# List recent notes
cons list

# Filter by tags
cons list --tags rust,learning

# Search content
cons search "self-referential"

# Limit results
cons list --limit 10
```

## Why cons?

### For Knowledge Workers
Drowning in scattered notes across multiple apps. You need consolidation without migration overhead or maintenance burden.

### For Context-Switchers
Developers, researchers, founders jumping between projects constantly. You lose context because organizing notes feels like more work.

### For Organization-Averse
You hate maintaining folder hierarchies and tag taxonomies. You have "I'll organize this later" collections that never get organized.

### For Privacy-Conscious
Your notes contain sensitive information. Cloud-based tools make you uncomfortable. You want complete control over your data.

## Architecture

Layered design separating concerns:

```
CLI (clap) ─────┐
                ├──> NoteService ──> SQLite
TUI (ratatui) ──┘         │
                          └──> OllamaClient ──> Ollama
```

- **NoteService**: Core business logic, UI-independent, reusable across interfaces
- **OllamaClient**: Isolated AI integration, mockable for tests
- **CLI/TUI**: Thin presentation layers calling NoteService

This architecture proves extensibility: same core logic powers CLI today, TUI tomorrow, GUI later.

See [ARCHITECTURE.md](ARCHITECTURE.md) for design decisions and [KNOWLEDGE.md](KNOWLEDGE.md) for information science principles.

## Roadmap

**Current Status:** Week 1 of MVP development

- [x] SQLite schema with idempotent initialization
- [ ] Core domain types (Note, Tag)
- [ ] NoteService implementation
- [ ] CLI commands (add, list, search)
- [ ] Ollama integration for auto-tagging
- [ ] Full-text search with SQLite FTS5
- [ ] TUI interface (stretch goal)

**Future:**
- Semantic search via vector embeddings
- Entity extraction (people, projects, concepts)
- Relationship mapping (discovers connections between notes)
- GUI desktop app

See [roadmap.md](agent-os/product/roadmap.md) for detailed timeline.

## Design Philosophy

### Structure-Last
Capture first, organize never. Structure emerges from AI analysis, not upfront categorization.

### Folksonomy-First
Your vocabulary is inherently correct. The tool adapts to how you think and search, not vice versa.

### Fail-Safe AI
AI enhances but never blocks. If LLM fails, your note still saves. Full-text search is always a fallback.

### Local-First
Privacy by architecture. Data lives on your machine. No accounts, no cloud sync, no vendor lock-in.

### Epistemic Humility
AI inferences carry confidence scores. Correction happens during retrieval when errors matter, not during capture.

## Contributing

This is a personal project and work sample. Issues and discussions welcome; pull requests accepted selectively.

See [CLAUDE.md](CLAUDE.md) for development guidance when working with Claude Code.

## License

MIT

## Philosophy

Personal knowledge management should augment human judgment, not replace it. But for a personal tool, augmentation happens during **retrieval and synthesis**, not during capture.

Traditional PKM tools make you organize as you capture—interrupting flow, creating friction, causing notes to never get written. `cons` inverts this: capture is instant and frictionless; organization happens automatically via AI; correction happens during retrieval when you actually notice errors that matter.

The result is a system you'll actually use. Because the best PKM system is the one that doesn't get in your way.

---

**Built with Rust.** Local-first. Privacy-focused. Structure-last.

*Dive into your construct.*
