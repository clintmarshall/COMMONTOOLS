use crate::checker::{Checker, CheckerOutput};
use crate::metrics::MetricValue;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub struct ClippyChecker;

impl Checker for ClippyChecker {
    fn name(&self) -> &str {
        "clippy"
    }

    fn can_run(&self, project_dir: &Path) -> bool {
        project_dir.join("Cargo.toml").exists()
    }

    fn run(&self, project_dir: &Path, verbose: bool, _last_workspace_hash: Option<&str>) -> Result<CheckerOutput> {
        // Run clippy with JSON output
        let output = Command::new("cargo")
            .args(&["clippy", "--message-format", "json", "--"])
            .current_dir(project_dir)
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse JSON messages from stderr (clippy outputs JSON to stderr)
        let mut warnings: usize = 0;
        let mut errors: usize = 0;
        let mut allowed: usize = 0;
        let mut files_checked: usize = 0;
        let mut complexity_issues: usize = 0;
        let mut correctness_issues: usize = 0;
        let mut perf_issues: usize = 0;

        for line in stderr.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Try to parse as JSON diagnostic
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                if msg.get("reason")
                    .and_then(|r| r.as_str())
                    .map(|r| r.starts_with("compiler-message"))
                    .unwrap_or(false)
                {
                    // This is a compiler message wrapper, skip
                    continue;
                }

                if msg.get("reason")
                    .and_then(|r| r.as_str())
                    .map(|r| r.starts_with("compiler-artifact"))
                    .unwrap_or(false)
                {
                    // Count artifacts as files checked
                    files_checked += 1;
                    continue;
                }

                // Parse diagnostic messages
                if let Some(diag) = msg.get("message") {
                    let level = diag
                        .get("level")
                        .and_then(|l| l.as_str())
                        .unwrap_or("");

                    match level {
                        "error" => errors += 1,
                        "warning" => warnings += 1,
                        "allowed" => allowed += 1,
                        _ => {}
                    }

                    // Categorize by clippy category
                    if let Some(spans) = diag.get("spans").and_then(|s| s.as_array()) {
                        for span in spans {
                            let category = span
                                .get("category")
                                .and_then(|c| c.as_str())
                                .unwrap_or("");

                            match category {
                                c if c.starts_with("clippy::complexity") => complexity_issues += 1,
                                c if c.starts_with("clippy::correctness") => correctness_issues += 1,
                                c if c.starts_with("clippy::perf") => perf_issues += 1,
                                _ => {}
                            }
                        }
                    }

                    // Also check explanation for category
                    if let Some(explanation) = diag.get("children") {
                        if let Some(children) = explanation.as_array() {
                            for child in children {
                                let message = child
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("");

                                // Check for category tags in the message
                                if message.contains("clippy::complexity") {
                                    complexity_issues += 1;
                                } else if message.contains("clippy::correctness") {
                                    correctness_issues += 1;
                                } else if message.contains("clippy::perf") {
                                    perf_issues += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        if verbose {
            println!(
                "  clippy: {} warnings, {} errors, {} allowed",
                warnings, errors, allowed
            );
        }

        let mut metrics = BTreeMap::new();
        metrics.insert("clippy_warnings".to_string(), MetricValue::Int(warnings as i64));
        metrics.insert("clippy_errors".to_string(), MetricValue::Int(errors as i64));
        metrics.insert("clippy_allowed".to_string(), MetricValue::Int(allowed as i64));
        metrics.insert("clippy_files".to_string(), MetricValue::Int(files_checked as i64));
        metrics.insert(
            "clippy_complexity".to_string(),
            MetricValue::Int(complexity_issues as i64),
        );
        metrics.insert(
            "clippy_correctness".to_string(),
            MetricValue::Int(correctness_issues as i64),
        );
        metrics.insert("clippy_perf".to_string(), MetricValue::Int(perf_issues as i64));

        Ok(CheckerOutput {
            version: None,
            metrics,
        })
    }
}
