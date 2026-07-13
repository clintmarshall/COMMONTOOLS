use crate::metrics::UnifiedMetrics;
use anyhow::Result;
use chrono::Utc;
use std::path::Path;

/// Update an existing fallow-chart.html by appending a new row to the embedded JS data array.
pub fn update(chart_path: &Path, metrics: &UnifiedMetrics, note: &str) -> Result<()> {
    let content = std::fs::read_to_string(chart_path)?;

    let now = Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let mi = metrics.mi.unwrap_or(0.0);
    let dead = metrics.dead_file_pct.unwrap_or(0.0);
    let dup = metrics.dup_pct.unwrap_or(0.0);
    let stmt = metrics.cov_stmt.unwrap_or(0.0);
    let func = metrics.cov_func.unwrap_or(0.0);
    let crap = metrics.max_crap_app.unwrap_or(0.0);

    // Escape single quotes in note
    let escaped_note = note.replace('\'', "\\'");

    let new_row = format!(
        "    {{ date: '{}', mi: {}, dead: {}, dup: {}, stmt: {}, func: {}, crap: {}, note: '{}' }},",
        now, mi, dead, dup, stmt, func, crap, escaped_note
    );

    // Find the last row in the data array and insert after it
    let re = regex::Regex::new(r"(note: '[^']*' },\n)(\];)")?;
    let replaced: String = re.replace(&content, |caps: &regex::Captures| {
        format!("{}\n{}\n{}", &caps[1], new_row, &caps[2])
    }).to_string();

    if replaced == content {
        // Fallback: try inserting before the closing ];
        if let Some(pos) = content.rfind("];") {
            let mut new_content = content[..pos].to_string();
            new_content.push_str(&new_row);
            new_content.push('\n');
            new_content.push_str(&content[pos..]);
            std::fs::write(chart_path, new_content)?;
        } else {
            anyhow::bail!("Could not find data array in chart HTML");
        }
    } else {
        std::fs::write(chart_path, &replaced)?;
    }

    Ok(())
}
