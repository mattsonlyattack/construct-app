//! Health check and maintenance utilities for cons.
//!
//! Provides the `doctor` command functionality:
//! - System health checks (database, migrations, Ollama)
//! - Note statistics and enrichment status
//! - Backfill capabilities for missing enrichments

use std::io::{self, Write};
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::autotagger::AutoTaggerBuilder;
use crate::enhancer::NoteEnhancerBuilder;
use crate::hierarchy::HierarchySuggesterBuilder;
use crate::ollama::OllamaClientBuilder;
use crate::{NoteId, NoteService, TagId, TagSource};

// ANSI color codes for terminal output
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

/// Health status for a component.
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// Component is healthy
    Ok,
    /// Component has a warning but is functional
    Warning(String),
    /// Component is not functional
    Error(String),
}

impl HealthStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, HealthStatus::Ok)
    }
}

/// Database health information.
#[derive(Debug)]
pub struct DatabaseHealth {
    pub status: HealthStatus,
    pub file_path: String,
}

/// Migration tracking information.
#[derive(Debug)]
pub struct MigrationInfo {
    pub version: u32,
    pub description: String,
    pub applied_at: i64,
}

/// Ollama connectivity information.
#[derive(Debug)]
pub struct OllamaHealth {
    pub status: HealthStatus,
    pub base_url: String,
    pub models: Vec<String>,
}

/// Note statistics for doctor output.
#[derive(Debug)]
pub struct NoteStats {
    pub total_notes: i64,
    pub notes_with_enhancement: i64,
    pub notes_without_enhancement: i64,
    pub notes_with_tags: i64,
    pub notes_without_tags: i64,
    pub total_tags: i64,
    pub total_edges: i64,
}

/// Backfill plan showing what will be processed.
#[derive(Debug)]
pub struct BackfillPlan {
    pub notes_needing_enhancement: Vec<(NoteId, String)>,
    pub notes_needing_tags: Vec<(NoteId, String)>,
    pub tags_needing_hierarchy: Vec<(TagId, String)>,
}

impl BackfillPlan {
    pub fn is_empty(&self) -> bool {
        self.notes_needing_enhancement.is_empty()
            && self.notes_needing_tags.is_empty()
            && self.tags_needing_hierarchy.is_empty()
    }

    pub fn total_items(&self) -> usize {
        self.notes_needing_enhancement.len()
            + self.notes_needing_tags.len()
            + self.tags_needing_hierarchy.len()
    }
}

/// Result of a backfill operation.
#[derive(Debug, Default)]
pub struct BackfillResult {
    pub enhanced_count: usize,
    pub tagged_count: usize,
    pub hierarchy_edges_created: usize,
    pub errors: Vec<String>,
}

// ============================================================================
// Health Check Functions
// ============================================================================

/// Performs all health checks and prints results.
pub fn run_health_checks(db_path: &str, service: &NoteService) -> Result<()> {
    let db_health = check_database_health(db_path, service);
    let migrations = get_applied_migrations(service)?;
    let ollama_health = check_ollama_health();
    let stats = get_note_stats(service)?;

    print_health_report(&db_health, &migrations, &ollama_health, &stats);

    Ok(())
}

fn check_database_health(db_path: &str, service: &NoteService) -> DatabaseHealth {
    let conn = service.database().connection();
    let status = match conn.query_row("SELECT 1", [], |_| Ok(())) {
        Ok(_) => HealthStatus::Ok,
        Err(e) => HealthStatus::Error(format!("Connection test failed: {}", e)),
    };

    DatabaseHealth {
        status,
        file_path: db_path.to_string(),
    }
}

fn get_applied_migrations(service: &NoteService) -> Result<Vec<MigrationInfo>> {
    let conn = service.database().connection();

    let mut stmt = conn.prepare(
        "SELECT version, applied_at, description FROM schema_migrations ORDER BY version",
    )?;

    let migrations = stmt.query_map([], |row| {
        Ok(MigrationInfo {
            version: row.get(0)?,
            applied_at: row.get(1)?,
            description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        })
    })?;

    migrations.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn check_ollama_health() -> OllamaHealth {
    let client = match OllamaClientBuilder::new().build() {
        Ok(c) => c,
        Err(e) => {
            return OllamaHealth {
                status: HealthStatus::Error(format!("Failed to build client: {}", e)),
                base_url: String::new(),
                models: Vec::new(),
            }
        }
    };

    let base_url = client.base_url().to_string();

    match client.list_models() {
        Ok(models) => OllamaHealth {
            status: if models.is_empty() {
                HealthStatus::Warning("No models installed".to_string())
            } else {
                HealthStatus::Ok
            },
            base_url,
            models,
        },
        Err(e) => OllamaHealth {
            status: HealthStatus::Error(format!("Connection failed: {}", e)),
            base_url,
            models: Vec::new(),
        },
    }
}

fn get_note_stats(service: &NoteService) -> Result<NoteStats> {
    let conn = service.database().connection();

    let total_notes: i64 =
        conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;

    let notes_with_enhancement: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notes WHERE content_enhanced IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    let notes_with_tags: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT note_id) FROM note_tags",
        [],
        |row| row.get(0),
    )?;

    let total_tags: i64 =
        conn.query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;

    let total_edges: i64 =
        conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;

    Ok(NoteStats {
        total_notes,
        notes_with_enhancement,
        notes_without_enhancement: total_notes - notes_with_enhancement,
        notes_with_tags,
        notes_without_tags: total_notes - notes_with_tags,
        total_tags,
        total_edges,
    })
}

// ============================================================================
// Pretty Printing
// ============================================================================

fn status_symbol(status: &HealthStatus) -> &'static str {
    match status {
        HealthStatus::Ok => "\u{2713}",
        HealthStatus::Warning(_) => "!",
        HealthStatus::Error(_) => "\u{2717}",
    }
}

fn status_color(status: &HealthStatus) -> &'static str {
    match status {
        HealthStatus::Ok => GREEN,
        HealthStatus::Warning(_) => YELLOW,
        HealthStatus::Error(_) => RED,
    }
}

fn print_health_report(
    db: &DatabaseHealth,
    migrations: &[MigrationInfo],
    ollama: &OllamaHealth,
    stats: &NoteStats,
) {
    println!("{}cons doctor{}", BOLD, RESET);
    println!();

    // Database section
    println!("{}Database{}", BOLD, RESET);
    println!(
        "  {}{}{} Connection: {}",
        status_color(&db.status),
        status_symbol(&db.status),
        RESET,
        if db.status.is_ok() { "OK" } else { "FAILED" }
    );
    println!("    {}Path: {}{}", DIM, db.file_path, RESET);
    println!();

    // Migrations section
    println!("{}Migrations{}", BOLD, RESET);
    if migrations.is_empty() {
        println!("  {}No migrations applied{}", YELLOW, RESET);
    } else {
        for m in migrations {
            let check = status_symbol(&HealthStatus::Ok);
            println!(
                "  {}{}{} v{}: {}",
                GREEN,
                check,
                RESET,
                m.version,
                m.description
            );
        }
    }
    println!();

    // Ollama section
    println!("{}Ollama{}", BOLD, RESET);
    let status_text = match &ollama.status {
        HealthStatus::Ok => "Connected".to_string(),
        HealthStatus::Warning(w) => w.clone(),
        HealthStatus::Error(e) => e.clone(),
    };
    println!(
        "  {}{}{} Status: {}",
        status_color(&ollama.status),
        status_symbol(&ollama.status),
        RESET,
        status_text
    );
    if !ollama.base_url.is_empty() {
        println!("    {}URL: {}{}", DIM, ollama.base_url, RESET);
    }
    if !ollama.models.is_empty() {
        let models_display = if ollama.models.len() > 3 {
            format!(
                "{}, ... ({} more)",
                ollama.models[..3].join(", "),
                ollama.models.len() - 3
            )
        } else {
            ollama.models.join(", ")
        };
        println!("    {}Models: {}{}", DIM, models_display, RESET);
    }
    println!();

    // Statistics section
    println!("{}Statistics{}", BOLD, RESET);
    println!("  Notes:      {:>6} total", stats.total_notes);
    if stats.total_notes > 0 {
        println!(
            "              {:>6} enhanced  {:>6} missing",
            stats.notes_with_enhancement, stats.notes_without_enhancement
        );
        println!(
            "              {:>6} tagged    {:>6} untagged",
            stats.notes_with_tags, stats.notes_without_tags
        );
    }
    println!("  Tags:       {:>6}", stats.total_tags);
    println!("  Edges:      {:>6}", stats.total_edges);
}

// ============================================================================
// Backfill Functions
// ============================================================================

/// Creates a backfill plan showing what will be processed.
pub fn create_backfill_plan(service: &NoteService) -> Result<BackfillPlan> {
    let conn = service.database().connection();

    // Notes missing enhancement (content_enhanced IS NULL)
    let mut stmt = conn.prepare(
        "SELECT id, SUBSTR(content, 1, 50) FROM notes WHERE content_enhanced IS NULL",
    )?;
    let notes_needing_enhancement: Vec<(NoteId, String)> = stmt
        .query_map([], |row| Ok((NoteId::new(row.get(0)?), row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Notes missing tags (no entry in note_tags)
    let mut stmt = conn.prepare(
        "SELECT n.id, SUBSTR(n.content, 1, 50)
         FROM notes n
         LEFT JOIN note_tags nt ON n.id = nt.note_id
         WHERE nt.note_id IS NULL",
    )?;
    let notes_needing_tags: Vec<(NoteId, String)> = stmt
        .query_map([], |row| Ok((NoteId::new(row.get(0)?), row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Tags without any edges (orphaned - could benefit from hierarchy)
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name FROM tags t
         WHERE NOT EXISTS (
             SELECT 1 FROM edges e
             WHERE e.source_tag_id = t.id OR e.target_tag_id = t.id
         )",
    )?;
    let tags_needing_hierarchy: Vec<(TagId, String)> = stmt
        .query_map([], |row| Ok((TagId::new(row.get(0)?), row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(BackfillPlan {
        notes_needing_enhancement,
        notes_needing_tags,
        tags_needing_hierarchy,
    })
}

/// Prints the backfill plan.
pub fn print_backfill_plan(plan: &BackfillPlan) {
    println!("{}Backfill Plan{}", BOLD, RESET);
    println!();

    if !plan.notes_needing_enhancement.is_empty() {
        println!(
            "Notes to enhance: {}{}{}",
            BOLD,
            plan.notes_needing_enhancement.len(),
            RESET
        );
        for (id, preview) in plan.notes_needing_enhancement.iter().take(5) {
            let preview = preview.replace('\n', " ");
            println!("  {}#{}{}: {}...", DIM, id, RESET, preview);
        }
        if plan.notes_needing_enhancement.len() > 5 {
            println!(
                "  {}... and {} more{}",
                DIM,
                plan.notes_needing_enhancement.len() - 5,
                RESET
            );
        }
        println!();
    }

    if !plan.notes_needing_tags.is_empty() {
        println!(
            "Notes to auto-tag: {}{}{}",
            BOLD,
            plan.notes_needing_tags.len(),
            RESET
        );
        for (id, preview) in plan.notes_needing_tags.iter().take(5) {
            let preview = preview.replace('\n', " ");
            println!("  {}#{}{}: {}...", DIM, id, RESET, preview);
        }
        if plan.notes_needing_tags.len() > 5 {
            println!(
                "  {}... and {} more{}",
                DIM,
                plan.notes_needing_tags.len() - 5,
                RESET
            );
        }
        println!();
    }

    if !plan.tags_needing_hierarchy.is_empty() {
        println!(
            "Orphan tags for hierarchy analysis: {}{}{}",
            BOLD,
            plan.tags_needing_hierarchy.len(),
            RESET
        );
        for (_id, name) in plan.tags_needing_hierarchy.iter().take(10) {
            println!("  {}- {}{}", DIM, name, RESET);
        }
        if plan.tags_needing_hierarchy.len() > 10 {
            println!(
                "  {}... and {} more{}",
                DIM,
                plan.tags_needing_hierarchy.len() - 10,
                RESET
            );
        }
    }
}

/// Prompts user for confirmation.
pub fn confirm_backfill() -> bool {
    print!("\nProceed with backfill? [y/N] ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Executes the backfill operations.
pub fn execute_backfill(service: &NoteService, plan: &BackfillPlan) -> Result<BackfillResult> {
    let mut result = BackfillResult::default();

    // Build Ollama client once
    let client = Arc::new(
        OllamaClientBuilder::new()
            .build()
            .context("Failed to build Ollama client")?,
    );

    // Auto-detect model
    let model = match std::env::var("OLLAMA_MODEL") {
        Ok(m) if !m.is_empty() => m,
        _ => {
            let models = client.list_models().context("Ollama not reachable")?;
            models
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No models installed in Ollama"))?
        }
    };

    println!("Using model: {}{}{}", BOLD, model, RESET);
    println!();

    // Phase 1: Enhance notes
    if !plan.notes_needing_enhancement.is_empty() {
        println!("{}Phase 1: Enhancing notes...{}", BOLD, RESET);
        let enhancer = NoteEnhancerBuilder::new().client(client.clone()).build();

        for (i, (note_id, _)) in plan.notes_needing_enhancement.iter().enumerate() {
            print!(
                "  [{}/{}] Note #{}... ",
                i + 1,
                plan.notes_needing_enhancement.len(),
                note_id
            );
            io::stdout().flush().ok();

            // Get full note content
            match service.get_note(*note_id) {
                Ok(Some(note)) => match enhancer.enhance_content(&model, note.content()) {
                    Ok(enhancement) => {
                        let now = time::OffsetDateTime::now_utc();
                        if let Err(e) = service.update_note_enhancement(
                            *note_id,
                            enhancement.enhanced_content(),
                            &model,
                            enhancement.confidence(),
                            now,
                        ) {
                            result.errors.push(format!("Note #{}: {}", note_id, e));
                            println!("{}FAILED{}", RED, RESET);
                        } else {
                            result.enhanced_count += 1;
                            println!(
                                "{}OK{} ({:.0}%)",
                                GREEN,
                                RESET,
                                enhancement.confidence() * 100.0
                            );
                        }
                    }
                    Err(e) => {
                        result.errors.push(format!("Note #{}: {}", note_id, e));
                        println!("{}FAILED{}", RED, RESET);
                    }
                },
                Ok(None) => {
                    result.errors.push(format!("Note #{}: not found", note_id));
                    println!("{}SKIPPED{}", YELLOW, RESET);
                }
                Err(e) => {
                    result.errors.push(format!("Note #{}: {}", note_id, e));
                    println!("{}FAILED{}", RED, RESET);
                }
            }
        }
        println!();
    }

    // Phase 2: Auto-tag notes
    if !plan.notes_needing_tags.is_empty() {
        println!("{}Phase 2: Auto-tagging notes...{}", BOLD, RESET);
        let tagger = AutoTaggerBuilder::new().client(client.clone()).build();

        for (i, (note_id, _)) in plan.notes_needing_tags.iter().enumerate() {
            print!(
                "  [{}/{}] Note #{}... ",
                i + 1,
                plan.notes_needing_tags.len(),
                note_id
            );
            io::stdout().flush().ok();

            match service.get_note(*note_id) {
                Ok(Some(note)) => match tagger.generate_tags(&model, note.content()) {
                    Ok(tags) if !tags.is_empty() => {
                        let mut tag_errors = false;
                        for (tag_name, confidence) in &tags {
                            let confidence_u8 = (*confidence * 100.0).round() as u8;
                            let source = TagSource::llm(model.clone(), confidence_u8);
                            if let Err(e) =
                                service.add_tags_to_note(*note_id, &[tag_name.as_str()], source)
                            {
                                result.errors.push(format!(
                                    "Note #{} tag '{}': {}",
                                    note_id, tag_name, e
                                ));
                                tag_errors = true;
                            }
                        }
                        if tag_errors {
                            println!("{}PARTIAL{} ({} tags)", YELLOW, RESET, tags.len());
                        } else {
                            result.tagged_count += 1;
                            println!("{}OK{} ({} tags)", GREEN, RESET, tags.len());
                        }
                    }
                    Ok(_) => {
                        println!("{}OK{} (no tags)", GREEN, RESET);
                    }
                    Err(e) => {
                        result.errors.push(format!("Note #{}: {}", note_id, e));
                        println!("{}FAILED{}", RED, RESET);
                    }
                },
                Ok(None) => {
                    result.errors.push(format!("Note #{}: not found", note_id));
                    println!("{}SKIPPED{}", YELLOW, RESET);
                }
                Err(e) => {
                    result.errors.push(format!("Note #{}: {}", note_id, e));
                    println!("{}FAILED{}", RED, RESET);
                }
            }
        }
        println!();
    }

    // Phase 3: Hierarchy suggestion (if enough orphan tags)
    if plan.tags_needing_hierarchy.len() >= 2 {
        println!("{}Phase 3: Suggesting hierarchy...{}", BOLD, RESET);
        let suggester = HierarchySuggesterBuilder::new().client(client).build();

        let tag_names: Vec<String> = plan
            .tags_needing_hierarchy
            .iter()
            .map(|(_, name)| name.clone())
            .collect();

        match suggester.suggest_relationships(&model, tag_names) {
            Ok(suggestions) if !suggestions.is_empty() => {
                // Create edges
                let mut edges = Vec::new();
                for suggestion in &suggestions {
                    if let (Ok(source_id), Ok(target_id)) = (
                        service.get_or_create_tag(&suggestion.source_tag),
                        service.get_or_create_tag(&suggestion.target_tag),
                    ) {
                        edges.push((
                            source_id,
                            target_id,
                            suggestion.confidence,
                            suggestion.hierarchy_type.as_str(),
                            Some(model.as_str()),
                        ));
                    }
                }

                if !edges.is_empty() {
                    match service.create_edges_batch(&edges) {
                        Ok(count) => {
                            result.hierarchy_edges_created = count;
                            println!("  {}Created {} edges{}", GREEN, count, RESET);
                        }
                        Err(e) => {
                            result.errors.push(format!("Hierarchy: {}", e));
                            println!("  {}FAILED: {}{}", RED, e, RESET);
                        }
                    }
                } else {
                    println!("  {}No valid edges to create{}", DIM, RESET);
                }
            }
            Ok(_) => {
                println!("  {}No relationships suggested{}", DIM, RESET);
            }
            Err(e) => {
                result.errors.push(format!("Hierarchy: {}", e));
                println!("  {}FAILED: {}{}", RED, e, RESET);
            }
        }
    } else if !plan.tags_needing_hierarchy.is_empty() {
        println!("{}Phase 3: Hierarchy suggestion{}", BOLD, RESET);
        println!(
            "  {}Skipped: need at least 2 orphan tags (have {}){}",
            DIM,
            plan.tags_needing_hierarchy.len(),
            RESET
        );
    }

    Ok(result)
}

/// Prints the backfill summary.
pub fn print_backfill_summary(result: &BackfillResult) {
    println!();
    println!("{}Backfill Complete{}", BOLD, RESET);
    println!("  Enhanced: {}", result.enhanced_count);
    println!("  Tagged:   {}", result.tagged_count);
    println!("  Edges:    {}", result.hierarchy_edges_created);

    if !result.errors.is_empty() {
        println!();
        println!(
            "{}Errors ({}){}:",
            YELLOW,
            result.errors.len(),
            RESET
        );
        for err in result.errors.iter().take(10) {
            println!("  - {}", err);
        }
        if result.errors.len() > 10 {
            println!("  ... and {} more", result.errors.len() - 10);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[test]
    fn test_health_status_is_ok() {
        assert!(HealthStatus::Ok.is_ok());
        assert!(!HealthStatus::Warning("test".into()).is_ok());
        assert!(!HealthStatus::Error("test".into()).is_ok());
    }

    #[test]
    fn test_backfill_plan_is_empty() {
        let plan = BackfillPlan {
            notes_needing_enhancement: vec![],
            notes_needing_tags: vec![],
            tags_needing_hierarchy: vec![],
        };
        assert!(plan.is_empty());
        assert_eq!(plan.total_items(), 0);
    }

    #[test]
    fn test_backfill_plan_not_empty() {
        let plan = BackfillPlan {
            notes_needing_enhancement: vec![(NoteId::new(1), "test".to_string())],
            notes_needing_tags: vec![],
            tags_needing_hierarchy: vec![],
        };
        assert!(!plan.is_empty());
        assert_eq!(plan.total_items(), 1);
    }

    #[test]
    fn test_get_note_stats_empty_database() {
        let db = Database::in_memory().unwrap();
        let service = NoteService::new(db);

        let stats = get_note_stats(&service).unwrap();

        assert_eq!(stats.total_notes, 0);
        assert_eq!(stats.notes_with_enhancement, 0);
        assert_eq!(stats.notes_without_enhancement, 0);
        assert_eq!(stats.total_tags, 0);
        assert_eq!(stats.total_edges, 0);
    }

    #[test]
    fn test_get_note_stats_with_data() {
        let db = Database::in_memory().unwrap();
        let service = NoteService::new(db);

        // Create notes with and without tags
        service.create_note("Note 1", Some(&["rust"])).unwrap();
        service.create_note("Note 2", None).unwrap();

        let stats = get_note_stats(&service).unwrap();

        assert_eq!(stats.total_notes, 2);
        assert_eq!(stats.notes_with_tags, 1);
        assert_eq!(stats.notes_without_tags, 1);
        assert_eq!(stats.total_tags, 1);
    }

    #[test]
    fn test_create_backfill_plan_identifies_missing_enrichments() {
        let db = Database::in_memory().unwrap();
        let service = NoteService::new(db);

        // Create notes - none have enhancement (fresh notes)
        service.create_note("Note 1", None).unwrap();
        service.create_note("Note 2", Some(&["test"])).unwrap();

        let plan = create_backfill_plan(&service).unwrap();

        assert_eq!(plan.notes_needing_enhancement.len(), 2);
        assert_eq!(plan.notes_needing_tags.len(), 1); // Note 1 has no tags
    }

    #[test]
    fn test_get_applied_migrations() {
        let db = Database::in_memory().unwrap();
        let service = NoteService::new(db);

        let migrations = get_applied_migrations(&service).unwrap();

        // Should have migrations from the schema initialization
        assert!(!migrations.is_empty());
        assert!(migrations.iter().any(|m| m.version == 1));
    }

    #[test]
    fn test_backfill_result_default() {
        let result = BackfillResult::default();
        assert_eq!(result.enhanced_count, 0);
        assert_eq!(result.tagged_count, 0);
        assert_eq!(result.hierarchy_edges_created, 0);
        assert!(result.errors.is_empty());
    }
}
