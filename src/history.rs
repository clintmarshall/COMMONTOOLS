use crate::metrics::UnifiedMetrics;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Top-level qualifier.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub version: u32,
    pub project: String,
    pub runs: Vec<Run>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            version: 1,
            project: String::new(),
            runs: Vec::new(),
        }
    }
}

/// A single quality check run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub ts: String,
    pub note: String,
    pub commentary: String,
    /// SHA-256 hash of (git commit SHA + working tree diff).
    /// Identical hash = identical code state, even with uncommitted changes.
    pub workspace_hash: Option<String>,
    pub checkers: BTreeMap<String, CheckerResult>,
    pub metrics: UnifiedMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckerResult {
    pub version: Option<String>,
    pub elapsed_ms: u64,
}

/// Load history from file, or return default if file doesn't exist
pub fn load(path: &Path) -> Result<History> {
    if !path.exists() {
        return Ok(History::default());
    }

    let content = std::fs::read_to_string(path)?;
    let history: History = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))?;

    Ok(history)
}

/// Save history to file
pub fn save(history: &History, path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(history)?;
    std::fs::write(path, content)?;
    Ok(())
}
