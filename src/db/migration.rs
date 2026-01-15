/// Individual migration with version metadata.
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub up: &'static str,
}

impl Migration {
    /// Creates a new migration.
    pub const fn new(version: u32, description: &'static str, up: &'static str) -> Self {
        Self {
            version,
            description,
            up,
        }
    }

    /// Checks if this migration has been applied to the database.
    pub fn is_applied(&self, conn: &rusqlite::Connection) -> anyhow::Result<bool> {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [self.version],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Applies this migration to the database.
    /// Records the migration in schema_migrations table.
    pub fn apply(&self, conn: &mut rusqlite::Connection) -> anyhow::Result<()> {
        let tx = conn.transaction()?;

        // Execute migration SQL
        tx.execute_batch(self.up)?;

        // Record migration
        let applied_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
            rusqlite::params![self.version, applied_at as i64, self.description],
        )?;

        tx.commit()?;
        Ok(())
    }
}

/// Registry of all migrations in version order.
pub const MIGRATIONS: &[Migration] = &[
    // Initial schema - creates base tables and indexes
    Migration::new(
        1,
        "Initial schema: create notes, tags, note_tags, tag_aliases, edges tables",
        include_str!("migrations/001_initial_schema.sql"),
    ),
    // Note enhancement fields for LLM integration
    Migration::new(
        2,
        "Add enhancement fields to notes table (content_enhanced, enhanced_at, enhancement_model, enhancement_confidence)",
        include_str!("migrations/002_note_enhancements.sql"),
    ),
    // Tag degree centrality for graph analytics
    Migration::new(
        3,
        "Add degree_centrality column to tags table for graph analytics",
        include_str!("migrations/003_tag_degree_centrality.sql"),
    ),
];

/// Applies all pending migrations to the database.
/// Migrations are applied in version order and are additive-only.
pub fn apply_pending_migrations(conn: &mut rusqlite::Connection) -> anyhow::Result<()> {
    // Ensure schema_migrations table exists first
    ensure_migration_table_exists(conn)?;

    for migration in MIGRATIONS {
        if !migration.is_applied(conn)? {
            migration.apply(conn)?;
            eprintln!(
                "Applied migration {}: {}",
                migration.version, migration.description
            );
        }
    }

    Ok(())
}

/// Creates the schema_migrations table if it doesn't exist.
/// This is idempotent and safe to call multiple times.
fn ensure_migration_table_exists(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL,
            description TEXT
        );
        "#,
    )?;
    Ok(())
}
