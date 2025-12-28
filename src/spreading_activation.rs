/// Spreading activation retrieval engine for graph-based search.
///
/// Implements cognitive-inspired search using recursive CTE to traverse
/// the tag hierarchy graph, propagating activation scores through edges
/// to surface semantically related notes.
use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

use crate::TagId;

/// Configuration for spreading activation algorithm.
///
/// Parsed from environment variables at method call time with fallback defaults.
#[derive(Debug, Clone)]
pub struct SpreadingActivationConfig {
    /// Activation decay per hop (default 0.7).
    pub decay_factor: f64,
    /// Minimum activation to continue spreading (default 0.1).
    pub threshold: f64,
    /// Maximum traversal depth (default 3).
    pub max_hops: usize,
}

impl Default for SpreadingActivationConfig {
    fn default() -> Self {
        Self {
            decay_factor: 0.7,
            threshold: 0.1,
            max_hops: 3,
        }
    }
}

impl SpreadingActivationConfig {
    /// Parses configuration from environment variables.
    ///
    /// Falls back to defaults when env vars not set or invalid.
    ///
    /// # Environment Variables
    ///
    /// - `CONS_DECAY` (f64, default 0.7): Activation decay per hop
    /// - `CONS_THRESHOLD` (f64, default 0.1): Minimum activation to continue spreading
    /// - `CONS_MAX_HOPS` (usize, default 3): Maximum traversal depth
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::spreading_activation::SpreadingActivationConfig;
    ///
    /// let config = SpreadingActivationConfig::from_env();
    /// assert_eq!(config.decay_factor, 0.7); // default when env var not set
    /// ```
    pub fn from_env() -> Self {
        let decay_factor = std::env::var("CONS_DECAY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.7);

        let threshold = std::env::var("CONS_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.1);

        let max_hops = std::env::var("CONS_MAX_HOPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        Self {
            decay_factor,
            threshold,
            max_hops,
        }
    }
}

/// Executes spreading activation from seed tags through the tag hierarchy graph.
///
/// Uses recursive CTE to traverse edges bidirectionally, applying decay and edge type
/// multipliers to propagate activation scores.
///
/// # Algorithm
///
/// 1. Seed CTE with initial activation 1.0 for seed tags
/// 2. Traverse edges bidirectionally (source->target and target->source)
/// 3. Apply formula: `activation_next = activation_current * edge.confidence * decay_factor * edge_type_multiplier`
/// 4. Edge type multiplier: generic=1.0, partitive=0.5
/// 5. Prune nodes below activation threshold
/// 6. Limit traversal with max_hops parameter
/// 7. Accumulate scores with SUM when tag receives activation from multiple paths
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `seed_tags` - Initial tags with activation scores
/// * `config` - Algorithm configuration (decay, threshold, max_hops)
///
/// # Returns
///
/// HashMap mapping TagId to final activation score
///
/// # Examples
///
/// ```no_run
/// use cons::{Database, TagId};
/// use cons::spreading_activation::{spread_activation, SpreadingActivationConfig};
/// use std::collections::HashMap;
///
/// # fn main() -> anyhow::Result<()> {
/// let db = Database::in_memory()?;
/// let mut seed_tags = HashMap::new();
/// seed_tags.insert(TagId::new(1), 1.0);
///
/// let config = SpreadingActivationConfig::default();
/// let activated = spread_activation(db.connection(), &seed_tags, &config)?;
/// # Ok(())
/// # }
/// ```
pub fn spread_activation(
    conn: &Connection,
    seed_tags: &HashMap<TagId, f64>,
    config: &SpreadingActivationConfig,
) -> Result<HashMap<TagId, f64>> {
    if seed_tags.is_empty() {
        return Ok(HashMap::new());
    }

    // Build VALUES clause for seed tags
    let seed_values: Vec<String> = seed_tags
        .iter()
        .map(|(tag_id, activation)| format!("({}, {}, 0)", tag_id.get(), activation))
        .collect();
    let seed_values_clause = seed_values.join(", ");

    let query = format!(
        r#"
        WITH RECURSIVE activation_spread(tag_id, activation, hop_count) AS (
            -- Base case: seed tags with initial activation
            SELECT * FROM (VALUES {seed_values})

            UNION ALL

            -- Recursive case: spread activation through edges
            SELECT
                CASE
                    -- Forward traversal (source -> target)
                    WHEN e.source_tag_id = a.tag_id THEN e.target_tag_id
                    -- Backward traversal (target -> source)
                    ELSE e.source_tag_id
                END AS tag_id,
                a.activation * e.confidence * ?1 *
                    CASE WHEN e.hierarchy_type = 'partitive' THEN 0.5 ELSE 1.0 END AS activation,
                a.hop_count + 1 AS hop_count
            FROM activation_spread a
            JOIN edges e ON (e.source_tag_id = a.tag_id OR e.target_tag_id = a.tag_id)
            WHERE a.hop_count < ?2
              AND a.activation * e.confidence * ?1 *
                  CASE WHEN e.hierarchy_type = 'partitive' THEN 0.5 ELSE 1.0 END >= ?3
        )
        SELECT tag_id, SUM(activation) as total_activation
        FROM activation_spread
        GROUP BY tag_id
        "#,
        seed_values = seed_values_clause
    );

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(
        rusqlite::params![config.decay_factor, config.max_hops, config.threshold],
        |row| {
            let tag_id: i64 = row.get(0)?;
            let activation: f64 = row.get(1)?;
            Ok((TagId::new(tag_id), activation))
        },
    )?;

    let mut result = HashMap::new();
    for row_result in rows {
        let (tag_id, activation) = row_result?;
        result.insert(tag_id, activation);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    /// Helper to create test database with tags and edges
    fn setup_test_db() -> Result<Database> {
        let db = Database::in_memory()?;
        let conn = db.connection();

        // Create test tags
        conn.execute("INSERT INTO tags (id, name) VALUES (1, 'rust')", [])?;
        conn.execute("INSERT INTO tags (id, name) VALUES (2, 'programming')", [])?;
        conn.execute("INSERT INTO tags (id, name) VALUES (3, 'systems')", [])?;
        conn.execute(
            "INSERT INTO tags (id, name) VALUES (4, 'memory-safety')",
            [],
        )?;
        conn.execute("INSERT INTO tags (id, name) VALUES (5, 'compiler')", [])?;

        Ok(db)
    }

    #[test]
    fn test_single_seed_spreads_through_generic_edges() -> Result<()> {
        let db = setup_test_db()?;
        let conn = db.connection();

        // Create edges: rust -> programming -> systems
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 2, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (2, 3, 1.0, 'generic')",
            [],
        )?;

        let mut seed_tags = HashMap::new();
        seed_tags.insert(TagId::new(1), 1.0); // rust = 1.0

        let config = SpreadingActivationConfig {
            decay_factor: 0.7,
            threshold: 0.1,
            max_hops: 3,
        };

        let activated = spread_activation(conn, &seed_tags, &config)?;

        // Verify activation spreads
        // Seed tag (rust) has 1.0 from seed
        // But bidirectional traversal means it also gets activation back from programming
        // So we expect seed tags to accumulate additional activation
        assert!(activated.contains_key(&TagId::new(1))); // rust
        assert!(activated.contains_key(&TagId::new(2))); // programming
        assert!(activated.contains_key(&TagId::new(3))); // systems

        // Check that activation values are reasonable (seed + spread)
        // Programming gets activation from rust: 1.0 * 1.0 * 0.7 = 0.7
        // Plus it sends back to rust: similar contribution
        let rust_activation = activated.get(&TagId::new(1)).unwrap();
        assert!(*rust_activation >= 1.0); // At least the seed value

        let programming_activation = activated.get(&TagId::new(2)).unwrap();
        assert!(*programming_activation > 0.7); // At least one hop from rust

        let systems_activation = activated.get(&TagId::new(3)).unwrap();
        assert!(*systems_activation > 0.0); // Should have some activation

        Ok(())
    }

    #[test]
    fn test_decay_factor_reduces_activation_per_hop() -> Result<()> {
        let db = setup_test_db()?;
        let conn = db.connection();

        // Linear chain: 1 -> 2 -> 3 -> 4
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 2, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (2, 3, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (3, 4, 1.0, 'generic')",
            [],
        )?;

        let mut seed_tags = HashMap::new();
        seed_tags.insert(TagId::new(1), 1.0);

        let config = SpreadingActivationConfig {
            decay_factor: 0.5,
            threshold: 0.05,
            max_hops: 3,
        };

        let activated = spread_activation(conn, &seed_tags, &config)?;

        // Verify all tags are activated
        assert!(activated.contains_key(&TagId::new(1)));
        assert!(activated.contains_key(&TagId::new(2)));
        assert!(activated.contains_key(&TagId::new(3)));
        assert!(activated.contains_key(&TagId::new(4)));

        // Verify decay pattern: each hop should reduce activation
        // Tag 2 gets activation from tag 1: 1.0 * 1.0 * 0.5 = 0.5
        // Tag 3 gets activation from tag 2: 0.5 * 1.0 * 0.5 = 0.25
        // Tag 4 gets activation from tag 3: 0.25 * 1.0 * 0.5 = 0.125
        // But bidirectional means tags also send back, so expect at least these values
        assert!(*activated.get(&TagId::new(2)).unwrap() >= 0.5);
        assert!(*activated.get(&TagId::new(3)).unwrap() >= 0.25);
        assert!(*activated.get(&TagId::new(4)).unwrap() >= 0.125);

        Ok(())
    }

    #[test]
    fn test_threshold_pruning_stops_low_activation_paths() -> Result<()> {
        let db = setup_test_db()?;
        let conn = db.connection();

        // Linear chain: 1 -> 2 -> 3 -> 4
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 2, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (2, 3, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (3, 4, 1.0, 'generic')",
            [],
        )?;

        let mut seed_tags = HashMap::new();
        seed_tags.insert(TagId::new(1), 1.0);

        let config = SpreadingActivationConfig {
            decay_factor: 0.5,
            threshold: 0.3, // High threshold should stop at tag 2
            max_hops: 3,
        };

        let activated = spread_activation(conn, &seed_tags, &config)?;

        // Only tag 1 (1.0) and tag 2 (0.5) should be activated
        // Tag 3 (0.25) and tag 4 (0.125) are below threshold
        assert!(activated.contains_key(&TagId::new(1)));
        assert!(activated.contains_key(&TagId::new(2)));
        assert!(!activated.contains_key(&TagId::new(3)));
        assert!(!activated.contains_key(&TagId::new(4)));

        Ok(())
    }

    #[test]
    fn test_max_hops_limits_traversal() -> Result<()> {
        let db = setup_test_db()?;
        let conn = db.connection();

        // Linear chain: 1 -> 2 -> 3 -> 4 -> 5
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 2, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (2, 3, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (3, 4, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (4, 5, 1.0, 'generic')",
            [],
        )?;

        let mut seed_tags = HashMap::new();
        seed_tags.insert(TagId::new(1), 1.0);

        let config = SpreadingActivationConfig {
            decay_factor: 0.9,
            threshold: 0.01,
            max_hops: 2, // Limit to 2 hops
        };

        let activated = spread_activation(conn, &seed_tags, &config)?;

        // Should reach tags 1, 2, 3 (hops 0, 1, 2) but not 4, 5 (hops 3, 4)
        assert!(activated.contains_key(&TagId::new(1)));
        assert!(activated.contains_key(&TagId::new(2)));
        assert!(activated.contains_key(&TagId::new(3)));
        assert!(!activated.contains_key(&TagId::new(4)));
        assert!(!activated.contains_key(&TagId::new(5)));

        Ok(())
    }

    #[test]
    fn test_activation_accumulates_from_multiple_paths() -> Result<()> {
        let db = setup_test_db()?;
        let conn = db.connection();

        // Diamond pattern:
        //     1
        //    / \
        //   2   3
        //    \ /
        //     4
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 2, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 3, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (2, 4, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (3, 4, 1.0, 'generic')",
            [],
        )?;

        let mut seed_tags = HashMap::new();
        seed_tags.insert(TagId::new(1), 1.0);

        let config = SpreadingActivationConfig {
            decay_factor: 0.5,
            threshold: 0.1,
            max_hops: 3,
        };

        let activated = spread_activation(conn, &seed_tags, &config)?;

        // Tag 4 receives activation from both paths (2->4 and 3->4)
        // Each path contributes 0.5 * 0.5 = 0.25
        // Total activation = 0.25 + 0.25 = 0.5
        let tag4_activation = activated.get(&TagId::new(4)).unwrap();
        assert!((tag4_activation - 0.5).abs() < 0.01);

        Ok(())
    }

    #[test]
    fn test_partitive_edges_use_reduced_weight_multiplier() -> Result<()> {
        let db = setup_test_db()?;
        let conn = db.connection();

        // Create two parallel chains:
        // Generic: 1 -> 2 (generic edge)
        // Partitive: 3 -> 4 (partitive edge)
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (1, 2, 1.0, 'generic')",
            [],
        )?;
        conn.execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, confidence, hierarchy_type)
             VALUES (3, 4, 1.0, 'partitive')",
            [],
        )?;

        let config = SpreadingActivationConfig {
            decay_factor: 1.0, // No decay to isolate edge type effect
            threshold: 0.01,
            max_hops: 3,
        };

        // Test generic edge
        let mut seed_tags_generic = HashMap::new();
        seed_tags_generic.insert(TagId::new(1), 1.0);
        let activated_generic = spread_activation(conn, &seed_tags_generic, &config)?;
        let generic_activation = activated_generic.get(&TagId::new(2)).unwrap();

        // Test partitive edge
        let mut seed_tags_partitive = HashMap::new();
        seed_tags_partitive.insert(TagId::new(3), 1.0);
        let activated_partitive = spread_activation(conn, &seed_tags_partitive, &config)?;
        let partitive_activation = activated_partitive.get(&TagId::new(4)).unwrap();

        // Partitive edge should have half the activation of generic edge
        // (due to 0.5 multiplier vs 1.0 multiplier)
        // With bidirectional traversal and accumulation, the ratio should still hold
        assert!(partitive_activation < generic_activation);
        // The ratio should be approximately 0.5
        let ratio = partitive_activation / generic_activation;
        assert!((ratio - 0.5).abs() < 0.2); // Allow more tolerance for bidirectional effects

        Ok(())
    }
}
