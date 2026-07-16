use crate::checker::{Checker, CheckerOutput};
use crate::metrics::MetricValue;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

pub struct VitestChecker;

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

    fn run(
        &self,
        project_dir: &Path,
        verbose: bool,
        last_workspace_hash: Option<&str>,
    ) -> Result<CheckerOutput> {
        // Check if we can skip: same code as last run + coverage files exist
        if Self::can_skip(project_dir, last_workspace_hash, verbose) {
            return Self::parse_existing_coverage(project_dir);
        }

        // Run vitest with coverage (auto-detects Docker container)
        let (output, was_docker) =
            crate::exec::run_command(project_dir, "vitest", &["run", "--coverage"])?;

        if was_docker && verbose {
            println!("  → vitest ran inside Docker container");
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if verbose {
                eprintln!("vitest stderr: {stderr}");
            }
            // Don't fail — coverage files may still be generated
        }

        let mut metrics = BTreeMap::new();

        // 1. Try coverage-summary.json (Istanbul provider)
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
                          if let Some(stmts) = total.get("statements") {
                            if let Some(pct) = stmts.get("pct").and_then(|v| v.as_f64()) {
                                metrics.insert("cov_stmt".to_string(), MetricValue::Float(pct));
                            }
                        }
                    }
                }
            }
        }

        // 2. Try clover.xml (produced by both v8 and istanbul providers)
        if metrics.get("cov_stmt").is_none() {
            let clover_path = project_dir.join("coverage/clover.xml");
            if let Ok(content) = std::fs::read_to_string(&clover_path) {
                if let Ok(clover) = parse_clover_xml(&content) {
                    if let Some(v) = clover.stmt {
                        metrics.insert("cov_stmt".to_string(), MetricValue::Float(v));
                    }
                    if let Some(v) = clover.branch {
                        metrics.insert("cov_branch".to_string(), MetricValue::Float(v));
                    }
                    if let Some(v) = clover.func {
                        metrics.insert("cov_func".to_string(), MetricValue::Float(v));
                    }
                 }
            }
        }

        // 3. Fallback: parse coverage-final.json (Istanbul format)
        if metrics.get("cov_stmt").is_none() {
            let final_path = project_dir.join("coverage/coverage-final.json");
            if final_path.exists() {
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
                         }
                    }
                }
            }
        }

        // Try to get test file count from coverage-final.json
        if metrics.get("test_files").is_none() {
            let final_path = project_dir.join("coverage/coverage-final.json");
            if final_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&final_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(map) = json.as_object() {
                            let file_count = map.len();
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

// ── VitestChecker helpers ──

impl VitestChecker {
    /// Check if vitest can be skipped: same workspace hash as last run and coverage files exist.
    /// The workspace hash (commit SHA + diff hash) is computed in main.rs, so we just compare.
    fn can_skip(
        project_dir: &Path,
        last_workspace_hash: Option<&str>,
        verbose: bool,
    ) -> bool {
        // Must have a previous hash to compare against
        let Some(last_hash) = last_workspace_hash else {
            return false;
        };

        // Compute current workspace hash (same logic as main.rs)
        let Some(current_hash) = crate::get_workspace_hash(project_dir) else {
            return false;
        };

        // Code state has changed — must run
        if current_hash != last_hash {
            return false;
        }

        // Check if any coverage files exist
        let has_coverage = project_dir.join("coverage/coverage-summary.json").exists()
            || project_dir.join("coverage/clover.xml").exists()
            || project_dir.join("coverage/coverage-final.json").exists();

        if has_coverage && verbose {
            println!("  → vitest skipped — code unchanged since last run, reusing coverage");
        }

        has_coverage
    }

    /// Parse coverage from existing files without running vitest.
    fn parse_existing_coverage(project_dir: &Path) -> Result<CheckerOutput> {
        let mut metrics = BTreeMap::new();

        // 1. Try coverage-summary.json (Istanbul provider)
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
                        if let Some(stmts) = total.get("statements") {
                            if let Some(pct) = stmts.get("pct").and_then(|v| v.as_f64()) {
                                metrics.insert("cov_stmt".to_string(), MetricValue::Float(pct));
                            }
                        }
                    }
                }
            }
        }

        // 2. Try clover.xml
        if metrics.get("cov_stmt").is_none() {
            let clover_path = project_dir.join("coverage/clover.xml");
            if let Ok(content) = std::fs::read_to_string(&clover_path) {
                if let Ok(clover) = parse_clover_xml(&content) {
                    if let Some(v) = clover.stmt {
                        metrics.insert("cov_stmt".to_string(), MetricValue::Float(v));
                    }
                    if let Some(v) = clover.branch {
                        metrics.insert("cov_branch".to_string(), MetricValue::Float(v));
                    }
                    if let Some(v) = clover.func {
                        metrics.insert("cov_func".to_string(), MetricValue::Float(v));
                    }
                }
            }
        }

        // 3. Fallback: coverage-final.json
        if metrics.get("cov_stmt").is_none() {
            let final_path = project_dir.join("coverage/coverage-final.json");
            if final_path.exists() {
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
                        }
                    }
                }
            }
        }

        // Test file count from coverage-final.json
        if metrics.get("test_files").is_none() {
            let final_path = project_dir.join("coverage/coverage-final.json");
            if final_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&final_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(map) = json.as_object() {
                            let file_count = map.len();
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

// ── Clover XML parsing ──

struct CloverCoverage {
    stmt: Option<f64>,
    branch: Option<f64>,
    func: Option<f64>,
}

/// Parse Clover XML coverage report.
/// Vitest's v8 provider writes metrics in the nested `<metrics>` element:
///   `<metrics statements="689" coveredstatements="536" conditionals="533"
///             coveredconditionals="353" methods="201" coveredmethods="175" .../>`
fn parse_clover_xml(content: &str) -> Result<CloverCoverage> {
    // Find the <metrics ...> tag
    let start = content
        .find("<metrics ")
        .ok_or_else(|| anyhow::anyhow!("No <metrics> tag in clover.xml"))?;
    let end = content[start..]
        .find('>')
        .ok_or_else(|| anyhow::anyhow!("Malformed <metrics> tag"))?;
    let tag = &content[start..start + end];

    let stmt = parse_int_pair(tag, "statements", "coveredstatements");
    let branch = parse_int_pair(tag, "conditionals", "coveredconditionals");
    let func = parse_int_pair(tag, "methods", "coveredmethods");

    Ok(CloverCoverage {
        stmt,
        branch,
        func,
    })
}

/// Parse a total+covered attribute pair and return percentage.
/// e.g. `statements="689" coveredstatements="536"` → 77.8
fn parse_int_pair(tag: &str, total_attr: &str, covered_attr: &str) -> Option<f64> {
    let total = parse_int_attr(tag, total_attr)?;
    let covered = parse_int_attr(tag, covered_attr)?;
    if total == 0 {
        Some(100.0)
    } else {
        Some((covered as f64 / total as f64) * 100.0)
    }
}

/// Parse an integer attribute from an XML tag.
fn parse_int_attr(tag: &str, attr: &str) -> Option<usize> {
    let pattern = format!("{}=\"", attr);
    let start = tag.find(&pattern)? + pattern.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

// ── Istanbul JSON parsing ──

struct IstanbulCoverage {
    stmt: Option<f64>,
    branch: Option<f64>,
    func: Option<f64>,
}

/// Parse Istanbul-format coverage-final.json
/// Format: { "path": { statementMap: { id: { start, end } }, s: { id: hitCount }, ... } }
fn parse_istanbul_coverage(map: &serde_json::Map<String, serde_json::Value>) -> IstanbulCoverage {
    let mut total_stmt: usize = 0;
    let mut covered_stmt: usize = 0;
    let mut total_branch: usize = 0;
    let mut covered_branch: usize = 0;
    let mut total_func: usize = 0;
    let mut covered_func: usize = 0;
    for (_, file_data) in map {
        // Statements: statementMap defines them, `s` tracks hits
        if let Some(stmt_map) = file_data.get("statementMap").and_then(|m| m.as_object()) {
            total_stmt += stmt_map.len();
        }
        if let Some(s) = file_data.get("s").and_then(|m| m.as_object()) {
            for (_, count) in s {
                if let Some(n) = count.as_i64() {
                    if n > 0 {
                        covered_stmt += 1;
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
    }
}
