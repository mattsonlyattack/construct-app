//! Evaluation helpers for auto-tagger prompt engineering.
//!
//! This module provides utilities for evaluating tag extraction quality,
//! comparing expected vs actual tags, and loading test corpora.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Test corpus entry structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusEntry {
    /// The note content to extract tags from.
    pub content: String,
    /// Expected tags (normalized).
    pub expected_tags: Vec<String>,
    /// Notes about this test case.
    pub notes: String,
}

/// Loads the test corpus from the fixtures directory.
///
/// # Arguments
///
/// * `corpus_path` - Optional path to corpus file. If None, uses default location.
///
/// # Returns
///
/// Returns a vector of corpus entries, or an error if the file cannot be read or parsed.
///
/// # Examples
///
/// ```no_run
/// use cons::autotagger::load_corpus;
///
/// let entries = load_corpus(None)?;
/// for entry in entries {
///     println!("Content: {}", entry.content);
///     println!("Expected tags: {:?}", entry.expected_tags);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn load_corpus(
    corpus_path: Option<PathBuf>,
) -> Result<Vec<CorpusEntry>, Box<dyn std::error::Error>> {
    let path = corpus_path.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("auto_tagger_corpus.json")
    });

    let content = fs::read_to_string(&path)?;
    let entries: Vec<CorpusEntry> = serde_json::from_str(&content)?;
    Ok(entries)
}

/// Calculates Jaccard similarity between two sets of tags.
///
/// Jaccard similarity is the size of the intersection divided by the size of the union.
/// Returns a value between 0.0 (no overlap) and 1.0 (identical sets).
///
/// # Arguments
///
/// * `expected` - Set of expected tags
/// * `actual` - Set of actual tags
///
/// # Returns
///
/// Jaccard similarity score between 0.0 and 1.0.
///
/// # Examples
///
/// ```
/// use cons::autotagger::jaccard_similarity;
/// use std::collections::HashSet;
///
/// let expected: HashSet<String> = ["rust", "async", "tokio"]
///     .iter()
///     .map(|s| s.to_string())
///     .collect();
/// let actual: HashSet<String> = ["rust", "async", "concurrency"]
///     .iter()
///     .map(|s| s.to_string())
///     .collect();
///
/// let similarity = jaccard_similarity(&expected, &actual);
/// // Intersection: {"rust", "async"} = 2
/// // Union: {"rust", "async", "tokio", "concurrency"} = 4
/// // Similarity: 2/4 = 0.5
/// assert_eq!(similarity, 0.5);
/// ```
pub fn jaccard_similarity(expected: &HashSet<String>, actual: &HashSet<String>) -> f64 {
    if expected.is_empty() && actual.is_empty() {
        return 1.0;
    }

    let intersection = expected.intersection(actual).count();
    let union = expected.union(actual).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Calculates precision and recall for tag extraction.
///
/// - Precision: How many of the extracted tags were correct (true positives / (true positives + false positives))
/// - Recall: How many of the expected tags were found (true positives / (true positives + false negatives))
///
/// # Arguments
///
/// * `expected` - Set of expected tags
/// * `actual` - Set of actual tags
///
/// # Returns
///
/// Tuple of (precision, recall) scores, both between 0.0 and 1.0.
///
/// # Examples
///
/// ```
/// use cons::autotagger::precision_recall;
/// use std::collections::HashSet;
///
/// let expected: HashSet<String> = ["rust", "async", "tokio"]
///     .iter()
///     .map(|s| s.to_string())
///     .collect();
/// let actual: HashSet<String> = ["rust", "async", "concurrency"]
///     .iter()
///     .map(|s| s.to_string())
///     .collect();
///
/// let (precision, recall) = precision_recall(&expected, &actual);
/// // True positives: {"rust", "async"} = 2
/// // False positives: {"concurrency"} = 1
/// // False negatives: {"tokio"} = 1
/// // Precision: 2 / (2 + 1) = 0.667
/// // Recall: 2 / (2 + 1) = 0.667
/// assert!((precision - 0.667).abs() < 0.01);
/// assert!((recall - 0.667).abs() < 0.01);
/// ```
pub fn precision_recall(expected: &HashSet<String>, actual: &HashSet<String>) -> (f64, f64) {
    let true_positives = expected.intersection(actual).count();
    let false_positives = actual.difference(expected).count();
    let false_negatives = expected.difference(actual).count();

    let precision = if actual.is_empty() {
        if expected.is_empty() { 1.0 } else { 0.0 }
    } else {
        true_positives as f64 / (true_positives + false_positives) as f64
    };

    let recall = if expected.is_empty() {
        if actual.is_empty() { 1.0 } else { 0.0 }
    } else {
        true_positives as f64 / (true_positives + false_negatives) as f64
    };

    (precision, recall)
}

/// Compares expected tags with actual tags and returns evaluation metrics.
///
/// # Arguments
///
/// * `expected` - Vector of expected tag names
/// * `actual` - HashMap of actual tags (keys are tag names, values are confidence scores)
///
/// # Returns
///
/// Tuple of (jaccard_similarity, precision, recall) scores.
pub fn compare_tags(expected: &[String], actual: &HashMap<String, f64>) -> (f64, f64, f64) {
    let expected_set: HashSet<String> = expected.iter().cloned().collect();
    let actual_set: HashSet<String> = actual.keys().cloned().collect();

    let jaccard = jaccard_similarity(&expected_set, &actual_set);
    let (precision, recall) = precision_recall(&expected_set, &actual_set);

    (jaccard, precision, recall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_jaccard_similarity_identical() {
        let set: HashSet<String> = ["rust", "async"].iter().map(|s| s.to_string()).collect();
        assert_eq!(jaccard_similarity(&set, &set), 1.0);
    }

    #[test]
    fn test_jaccard_similarity_no_overlap() {
        let expected: HashSet<String> = ["rust"].iter().map(|s| s.to_string()).collect();
        let actual: HashSet<String> = ["python"].iter().map(|s| s.to_string()).collect();
        assert_eq!(jaccard_similarity(&expected, &actual), 0.0);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        let expected: HashSet<String> = ["rust", "async", "tokio"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let actual: HashSet<String> = ["rust", "async", "concurrency"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Intersection: 2, Union: 4
        assert_eq!(jaccard_similarity(&expected, &actual), 0.5);
    }

    #[test]
    fn test_precision_recall_perfect() {
        let set: HashSet<String> = ["rust", "async"].iter().map(|s| s.to_string()).collect();
        let (precision, recall) = precision_recall(&set, &set);
        assert_eq!(precision, 1.0);
        assert_eq!(recall, 1.0);
    }

    #[test]
    fn test_precision_recall_partial() {
        let expected: HashSet<String> = ["rust", "async", "tokio"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let actual: HashSet<String> = ["rust", "async", "concurrency"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // TP: 2, FP: 1, FN: 1
        // Precision: 2/3 = 0.667, Recall: 2/3 = 0.667
        let (precision, recall) = precision_recall(&expected, &actual);
        assert!((precision - 0.667).abs() < 0.01);
        assert!((recall - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_compare_tags() {
        let expected = vec!["rust".to_string(), "async".to_string(), "tokio".to_string()];
        let mut actual = HashMap::new();
        actual.insert("rust".to_string(), 0.9);
        actual.insert("async".to_string(), 0.85);
        actual.insert("concurrency".to_string(), 0.75);

        let (jaccard, precision, recall) = compare_tags(&expected, &actual);
        assert!((jaccard - 0.5).abs() < 0.01); // 2 intersection, 4 union
        assert!((precision - 0.667).abs() < 0.01); // 2 TP, 1 FP
        assert!((recall - 0.667).abs() < 0.01); // 2 TP, 1 FN
    }
}
