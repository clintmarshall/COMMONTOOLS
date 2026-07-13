use crate::history::History;
use anyhow::Result;
use std::path::Path;

/// Generate quality.html from the history
pub fn generate(history: &History, path: &Path) -> Result<()> {
    let template = include_str!("../templates/quality.html");

    // Serialize runs as JSON for the embedded JS
    let runs_json = serde_json::to_string(&history.runs)?;

    // Replace the placeholder with actual data
    let html = template.replace("__RUNS_DATA_PLACEHOLDER__", &runs_json);

    std::fs::write(path, html)?;
    Ok(())
}
