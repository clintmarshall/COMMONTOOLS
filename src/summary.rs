use crate::metrics::UnifiedMetrics;
use std::collections::BTreeMap;

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

/// Print a compact summary table to stdout
pub fn print_summary(
    metrics: &UnifiedMetrics,
    checker_results: &BTreeMap<String, crate::history::CheckerResult>,
    project_name: &str,
) {
    let total_ms: u64 = checker_results.values().map(|c| c.elapsed_ms).sum();
    let total_s = total_ms as f64 / 1000.0;
    let run_num = checker_results.len();

    println!();
    println!("═══════════════════════════════════════");
    println!("  Qualifier — {project_name}");
    println!("═══════════════════════════════════════");

    // LOC
    if let Some(loc) = metrics.loc {
        println!(
            "  LOC:           {}",
            if loc >= 1000 { format_comma(loc) } else { loc.to_string() }
        );
    }

    // Quality row
    let mi_str = metrics
        .mi
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "-".to_string());
    let dead_str = metrics
        .dead_file_pct
        .map(|v| format!("{v:.1}%"))
        .unwrap_or_else(|| "-".to_string());
    let dup_str = metrics
        .dup_pct
        .map(|v| format!("{v:.1}%"))
        .unwrap_or_else(|| "-".to_string());
    println!(
        "  MI:            {mi_str:<10} Dead: {dead_str:<10} Dup: {dup_str}"
    );

    // Coverage row
    let stmt_str = metrics
        .cov_stmt
        .map(|v| format!("{v:.1}%"))
        .unwrap_or_else(|| "-".to_string());
    let func_str = metrics
        .cov_func
        .map(|v| format!("{v:.1}%"))
        .unwrap_or_else(|| "-".to_string());
    println!("  Coverage:      {stmt_str} stmt  /  {func_str} func");

    // CRAP + Tests
    let crap_str = metrics
        .max_crap_app
        .map(|v| format!("{v:.0}"))
        .unwrap_or_else(|| "-".to_string());

    let test_str = if let (Some(count), Some(files)) = (metrics.test_count, metrics.test_files) {
        format!("{count} / {files} files")
    } else if let Some(files) = metrics.test_files {
        format!("{files} files")
    } else {
        "-".to_string()
    };
    println!("  Max CRAP:      {crap_str:<10} Tests: {test_str}");

    // Clippy row (only if clippy ran)
    if let Some(warnings) = metrics.clippy_warnings {
        let warn_str = warnings.to_string();
        let err_str = metrics
            .clippy_errors
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let files_str = metrics
            .clippy_files
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("  Clippy:        {warn_str} warnings  /  {err_str} errors  /  {files_str} files");
    }

    // Footer
    println!("═══════════════════════════════════════");
    println!("  Run #{run_num} in {total_s:.1}s");
}
