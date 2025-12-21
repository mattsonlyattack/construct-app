# Product Mission

## Pitch

**cons** is a structure-last personal knowledge management tool that helps knowledge workers capture thoughts freely without organizational overhead by providing fully automated AI tagging, categorization, and relationship mapping.

The goal is zero-friction note capture with intelligent automated organization. Not AI-assisted organization where users still make structure decisions--cons makes structure the AI's job entirely.

## Vision

A second brain that actually remembers things without requiring you to organize them.

Existing tools like Obsidian, Logseq, and Notion require upfront structure decisions. Even advanced tools like Mem and Reflect still make users think about organization. cons eliminates that cognitive burden completely.

## Users

### Primary Customers

- **Knowledge workers**: Drowning in scattered notes across multiple apps, need consolidation without migration overhead
- **Context-switchers**: Developers, researchers, and founders who jump between projects constantly and lose context
- **Organization-averse**: People who hate maintaining folder hierarchies, tag taxonomies, and linking systems
- **Perpetual procrastinators**: Anyone with "I'll organize this later" note collections that never get organized

### User Persona

**The Overwhelmed Knowledge Worker** (25-45)
- **Role:** Developer, researcher, founder, or technical professional
- **Context:** Works across multiple projects, contexts, and information streams simultaneously
- **Pain Points:**
  - Notes scattered across Notion, Apple Notes, random .txt files, Slack DMs to self
  - Spends more time organizing than capturing; often doesn't capture because organization feels like work
  - Can't find notes when needed; search fails because past-self used different terminology
  - Has tried multiple PKM tools but abandoned each when organization debt accumulated
- **Goals:**
  - Capture thoughts instantly with zero friction
  - Find relevant notes when needed without remembering exact words or structure
  - Never think about folders, tags, or links again
  - Actually use a notes system long-term

### Dogfooding

Primary user: the developer. Personal memory issues make reliable note retrieval essential. Strong aversion to manual organization means the tool must prove its value through actual daily use.

## The Problem

### The Organization Tax

Every existing PKM tool imposes an "organization tax" on note capture. Users must decide where to put notes, what to tag them, how to link them. This friction causes three failure modes:

1. **Capture avoidance**: Thought not worth the organization overhead, so it's lost
2. **Deferred organization**: "I'll organize later" creates ever-growing backlogs that never get processed
3. **Inconsistent structure**: Different organizational decisions on different days make retrieval unreliable

**Quantifiable impact**: Users with scattered notes across 3+ apps. Hours spent searching for information that was captured but unfindable. Hundreds of notes in "Inbox" or "Unsorted" folders.

**Our Solution**: Move all organizational decisions to AI. The user's only job is capture. The system handles tagging, categorization, and relationship mapping automatically and consistently.

## Differentiators

### True Zero-Effort Organization

Unlike Mem or Reflect where AI suggests organization that users must approve, cons performs organization automatically with no user decision points. Capture a note, done. The AI tags it, categorizes it, and maps relationships without asking.

This results in: Capture friction reduced to absolute minimum. Users capture more because there's no organizational overhead.

### Local-First Privacy

Unlike cloud-based PKM tools, cons runs entirely locally. SQLite database on your machine, Ollama for local LLM inference, no data leaves your computer.

This results in: Complete privacy, offline capability, no subscription costs, no vendor lock-in.

### CLI-First Design

Unlike GUI-first tools that require context-switching to a separate app, cons integrates into existing terminal workflows. `cons add "thought"` from any terminal, anywhere.

This results in: Faster capture for terminal users, scriptable integration with other tools, lower barrier to capture.

### Layered Architecture for Extensibility

Unlike monolithic applications, cons separates core logic (NoteService) from presentation (CLI/TUI/GUI). The same business logic powers all interfaces.

This results in: Consistent behavior across interfaces, easier testing, clear path from CLI to TUI to GUI.

## Key Features

### Core Features

- **Instant capture**: `cons add "thought"` saves note immediately, no menus or decisions
- **Automatic tagging**: AI analyzes content and applies relevant tags without user input
- **Full-text search**: Find notes by content, not just exact title matches
- **Tag-based filtering**: Browse notes by AI-generated tags when exploration is needed

### Retrieval Features

- **Content-based search**: `cons search "rust async"` finds relevant notes regardless of exact wording
- **Tag filtering**: `cons list --tags rust` shows all notes the AI tagged with "rust"
- **Chronological browsing**: `cons list` shows recent notes for review

### Future Features

- **Semantic search**: Vector embeddings for meaning-based retrieval beyond keyword matching
- **Entity extraction**: Automatic identification of people, projects, and concepts
- **Relationship mapping**: AI-discovered connections between notes
- **TUI interface**: Interactive terminal browsing with same core functionality
- **GUI interface**: Desktop app for users who prefer graphical interaction

## Success Criteria

### Usage Metrics

- Replace scattered note apps with cons as primary capture tool
- Capture 20+ notes per day with zero organizational friction
- Daily active use (dogfooding proves value)

### Quality Metrics

- Auto-tagging 70%+ accurate without manual correction
- Search retrieval finds relevant notes on first query 80%+ of time
- Zero capture failures due to AI processing delays (fail-safe design)

### Technical Metrics

- Codebase demonstrates production-quality Rust: clean architecture, proper error handling, comprehensive tests
- Architecture proves reusability: same NoteService works for CLI and TUI
- Project serves as centerpiece work sample for Oxide Product Engineer application

### Timeline

- 3-week MVP demonstrating core capture, auto-tagging, and search
- Fully open source as work sample
- Demonstrates systems thinking, architectural judgment, and shipping ability
