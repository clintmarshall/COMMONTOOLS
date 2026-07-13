use crate::checker::{Checker, CheckerOutput};
use crate::metrics::MetricValue;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub struct VitestChecker;

/// Run a command via npx, handling Windows (npx.cmd) and direct node_modules paths.
fn run_npx(project_dir: &Path, package: &str, args: &[&str]) -> Result<std::process::Output> {
    // Try direct node_modules/.bin path first (most reliable cross-platform)
    let bin_dir = project_dir.join("node_modules/.bin");
    if bin_dir.exists() {
        #[cfg(windows)]
        let cmd_path = bin_dir.join(format!("{}.cmd", package));
        #[cfg(not(windows))]
        let cmd_path = bin_dir.join(package);

        if cmd_path.exists() {
            return Ok(Command::new(&cmd_path).args(args).current_dir(project_dir).output()?);
        }
    }

    // Fallback to npx
    #[cfg(windows)]
    let npx_cmd = "npx.cmd";
    #[cfg(not(windows))]
    let npx_cmd = "npx";

    Ok(Command::new(npx_cmd)
        .arg(package)
        .args(args)
        .current_dir(project_dir)
        .output()?)
}

impl Checker for VitestChecker {
    fn name(&self) -> &str {
        "vitest"
    }

    fn can_run(&self, project_dir: &Path) -> bool {
        let pattern = format!("{}/vitest.config.*", project_dir.to_string_lossy());
        glob::glob(&pattern)
            .map(|mut iter| iter.next().is_some())
            .unwrap_or(false)
    }

    fn run(&self, project_dir: &Path, verbose: bool) -> Result<CheckerOutput> {
        // Run vitest with coverage
        let output = run_npx(project_dir, "vitest", &["run", "--coverage"])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if verbose {
                eprintln!("vitest stderr: {stderr}");
            }
            // Don't fail — coverage files may still be generated
        }

        let mut metrics = BTreeMap::new();

        // Try to read coverage-summary.json first (vitest v3+)
        let summary_path = project_dir.join("coverage/coverage-summary.json");
        if summary_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&summary_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(total) = json.get("total") {
                        if let Some(branches) = total.get("branches") {
                            if let Some(pct) = branches.get("pct").and_then(|v| v.as_f64()) {
                                metrics.insert("cov_branch".to_string(), MetricValue::Float(pct));
                            }
                        }
                        if let Some(funcs) = total.get("functions") {
                            if let Some(pct) = funcs.get("pct").and_then(|v| v.as_f64()) {
                                metrics.insert("cov_func".to_string(), MetricValue::Float(pct));
                            }
                        }
                        if let Some(lines) = total.get("lines") {
                            if let Some(pct) = lines.get("pct").and_then(|v| v.as_f64()) {
                                metrics.insert("cov_lines".to_string(), MetricValue::Float(pct));
                            }
                        }
                        if let Some(stmts) = total.get("statements") {
                            if let Some(pct) = stmts.get("pct").and_then(|v| v.as_f64()) {
                                metrics.insert("cov_stmt".to_string(), MetricValue::Float(pct));
                            }
                        }
                    }
                }
            }
        }

        // Fallback: parse coverage-final.json (Istanbul format)
        let final_path = project_dir.join("coverage/coverage-final.json");
        if final_path.exists() && metrics.get("cov_stmt").is_none() {
            if let Ok(content) = std::fs::read_to_string(&final_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(map) = json.as_object() {
                        let coverage = parse_istanbul_coverage(map);
                        if let Some(stmt) = coverage.stmt {
                            metrics.insert("cov_stmt".to_string(), MetricValue::Float(stmt));
                        }
                        if let Some(branch) = coverage.branch {
                            metrics.insert("cov_branch".to_string(), MetricValue::Float(branch));
                        }
                        if let Some(func) = coverage.func {
                            metrics.insert("cov_func".to_string(), MetricValue::Float(func));
                        }
                        if let Some(lines) = coverage.lines {
                            metrics.insert("cov_lines".to_string(), MetricValue::Float(lines));
                        }
                    }
                }
            }
        }

        // Try to get test count from coverage-summary.json
        if metrics.get("test_files").is_none() {
            let summary_path = project_dir.join("coverage/coverage-summary.json");
            if summary_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&summary_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Count files (excluding "total" key)
                        if let Some(map) = json.as_object() {
                            let file_count = map.len().saturating_sub(1); // subtract "total"
                            if file_count > 0 {
                                metrics.insert(
                                    "test_files".to_string(),
                                    MetricValue::Int(file_count as i64),
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(CheckerOutput {
            version: None,
            metrics,
        })
    }
}

struct IstanbulCoverage {
    stmt: Option<f64>,
    branch: Option<f64>,
    func: Option<f64>,
    lines: Option<f64>,
}

/// Parse Istanbul-format coverage-final.json
/// Format: { "path": { statementMap: { id: { start, end } }, s: { id: hitCount }, ... } }
fn parse_istanbul_coverage(map: &serde_json::Map<String, serde_json::Value>) -> IstanbulCoverage {
    let mut total_stmt: usize = 0;
    let covered_stmt: usize = 0;
    let mut total_branch: usize = 0;
    let mut covered_branch: usize = 0;
    let mut total_func: usize = 0;
    let mut covered_func: usize = 0;
    let mut total_lines: usize = 0;
    let mut covered_lines: usize = 0;

    for (_, file_data) in map {
        // Statements
        if let Some(stmt_map) = file_data.get("statementMap").and_then(|m| m.as_object()) {
            total_stmt += stmt_map.len();
        }
        if let Some(s) = file_data.get("s").and_then(|m| m.as_object()) {
            for (_, count) in s {
                if let Some(n) = count.as_i64() {
                    total_lines += 1;
                    if n > 0 {
                        covered_lines += 1;
                    }
                }
            }
        }

        // Branches
        if let Some(branch_map) = file_data.get("branchMap").and_then(|m| m.as_object()) {
            for (_, branch) in branch_map {
                if let Some(locs) = branch.get("locations").and_then(|a| a.as_array()) {
                    total_branch += locs.len();
                }
            }
        }
        if let Some(b) = file_data.get("b").and_then(|m| m.as_object()) {
            for (_, branch_hits) in b {
                if let Some(arr) = branch_hits.as_array() {
                    for hit in arr {
                        if let Some(n) = hit.as_i64() {
                            if n > 0 {
                                covered_branch += 1;
                            }
                        }
                    }
                }
            }
        }

        // Functions
        if let Some(func_map) = file_data.get("fnMap").and_then(|m| m.as_object()) {
            total_func += func_map.len();
        }
        if let Some(f) = file_data.get("f").and_then(|m| m.as_object()) {
            for (_, count) in f {
                if let Some(n) = count.as_i64() {
                    if n > 0 {
                        covered_func += 1;
                    }
                }
            }
        }
    }

    let pct = |covered: usize, total: usize| -> Option<f64> {
        if total == 0 {
            Some(100.0)
        } else {
            Some((covered as f64 / total as f64) * 100.0)
        }
    };

    IstanbulCoverage {
        stmt: pct(covered_stmt, total_stmt),
        branch: pct(covered_branch, total_branch),
        func: pct(covered_func, total_func),
        lines: pct(covered_lines, total_lines),
    }
}
