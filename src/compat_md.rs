use crate::metrics::UnifiedMetrics;
use anyhow::Result;
use chrono::Utc;
use std::path::Path;

/// Format an integer with comma separators (e.g. 12,345)
fn format_comma(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let mut count = 0;
    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            out.push(',');
        }
        out.push(c);
        count += 1;
    }
    out.chars().rev().collect()
}

/// Update an existing fallow-progress.md by inserting a new table row after the header separator.
/// Creates a .bak backup before writing.
pub fn update(md_path: &Path, metrics: &UnifiedMetrics, note: &str) -> Result<()> {
    let content = std::fs::read_to_string(md_path)?;

    // Backup before modifying
    let backup_path = md_path.with_extension("md.bak");
    std::fs::write(&backup_path, &content)?;

    let now = Utc::now().format("%Y-%m-%dT%H:%M").to_string();
    let loc = metrics.loc.unwrap_or(0);
    let dead_pct = metrics.dead_file_pct.unwrap_or(0.0);
    let dead_count = metrics.dead_file_count.unwrap_or(0);
    let dead_export_pct = metrics.dead_export_pct.unwrap_or(0.0);
    let dead_export_count = metrics.dead_export_count.unwrap_or(0);
    let avg_cycl = metrics.avg_cyclomatic.unwrap_or(0.0);
    let p90_cycl = metrics.p90_cyclomatic.unwrap_or(0.0);
    let mi = metrics.mi.unwrap_or(0.0);
    let max_crap = metrics.max_crap_app.unwrap_or(0.0);
    let dup_lines = metrics.dup_lines.unwrap_or(0);
    let dup_pct = metrics.dup_pct.unwrap_or(0.0);
    let clone_groups = metrics.clone_groups.unwrap_or(0);
    let cov_stmt = metrics.cov_stmt.unwrap_or(0.0);
    let cov_branch = metrics.cov_branch.unwrap_or(0.0);
    let cov_func = metrics.cov_func.unwrap_or(0.0);
  
    let crap_display = if max_crap >= 1482.0 {
        format!(">{max_crap:.0} (scripts/sync-from-wordpress)")
    } else {
        format!("{max_crap:.0}")
    };

    let md_row = format!(
        "| {} | {} | {} ({}) | {} ({}) | {} | {} | {} | {} | {} | {:.1}% | {} | {:.2}% | {:.2}% | {:.2}% | {} |",
        now,
        format_comma(loc),
        dead_count,
        dead_pct,
        dead_export_count,
        dead_export_pct,
        avg_cycl,
        p90_cycl,
        mi,
        crap_display,
        format_comma(dup_lines),
        dup_pct,
        clone_groups,
        cov_stmt,
        cov_branch,
        cov_func,
        note,
    );

    // Find the table separator line and insert the new row after it
    let re = regex::Regex::new(r"(?m)(^\|-+\|.*:\|.*\|\n)(^\|)")?;
    let replaced: String = re.replace(&content, |caps: &regex::Captures| {
        format!("{}{}{}", &caps[1], md_row, &caps[2])
    }).to_string();

    if replaced == content {
        anyhow::bail!("Could not find table separator in markdown");
    }

    std::fs::write(md_path, &replaced)?;
    Ok(())
}
