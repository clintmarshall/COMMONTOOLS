use crate::checker::{Checker, CheckerOutput};
use crate::metrics::MetricValue;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

pub struct FallowChecker;

impl Checker for FallowChecker {
    fn name(&self) -> &str {
        "fallow"
    }

    fn can_run(&self, project_dir: &Path) -> bool {
        project_dir.join(".fallowrc.json").exists()
            || project_dir.join("fallow.toml").exists()
    }

    fn run(&self, project_dir: &Path, verbose: bool) -> Result<CheckerOutput> {
        let (output, was_docker) =
            crate::exec::run_command(project_dir, "fallow", &["--format", "json"])?;

        if was_docker && verbose {
            println!("  → fallow ran inside Docker container");
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if verbose {
                eprintln!("fallow stderr: {stderr}");
            }
            anyhow::bail!("fallow exited with non-zero status");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse fallow JSON: {e}"))?;

        let mut metrics = BTreeMap::new();

        // Extract vital signs
        if let Some(vs) = json.get("health").and_then(|h| h.get("vital_signs")) {
            if let Some(loc) = vs.get("total_loc").and_then(|v| v.as_i64()) {
                metrics.insert("loc".to_string(), MetricValue::Int(loc));
            }
            if let Some(pct) = vs.get("dead_file_pct").and_then(|v| v.as_f64()) {
                metrics.insert("dead_file_pct".to_string(), MetricValue::Float(pct));
            }
            if let Some(pct) = vs.get("dead_export_pct").and_then(|v| v.as_f64()) {
                metrics.insert("dead_export_pct".to_string(), MetricValue::Float(pct));
            }
            if let Some(avg) = vs.get("avg_cyclomatic").and_then(|v| v.as_f64()) {
                metrics.insert("avg_cyclomatic".to_string(), MetricValue::Float(avg));
            }
            if let Some(p90) = vs.get("p90_cyclomatic").and_then(|v| v.as_f64()) {
                metrics.insert("p90_cyclomatic".to_string(), MetricValue::Float(p90));
            }
            if let Some(mi) = vs.get("maintainability_avg").and_then(|v| v.as_f64()) {
                metrics.insert("mi".to_string(), MetricValue::Float(mi));
            }
        }

        // Extract dead file/export counts from check summary
        if let Some(summary) = json.get("check").and_then(|c| c.get("summary")) {
            if let Some(count) = summary.get("unused_files").and_then(|v| v.as_i64()) {
                metrics.insert("dead_file_count".to_string(), MetricValue::Int(count));
            }
            if let Some(count) = summary.get("unused_exports").and_then(|v| v.as_i64()) {
                metrics.insert("dead_export_count".to_string(), MetricValue::Int(count));
            }
        }

        // Extract duplication stats
        if let Some(ds) = json.get("dupes").and_then(|d| d.get("stats")) {
            if let Some(lines) = ds.get("duplicated_lines").and_then(|v| v.as_i64()) {
                metrics.insert("dup_lines".to_string(), MetricValue::Int(lines));
            }
            if let Some(pct) = ds.get("duplication_percentage").and_then(|v| v.as_f64()) {
                metrics.insert("dup_pct".to_string(), MetricValue::Float(pct));
            }
            if let Some(groups) = ds.get("clone_groups").and_then(|v| v.as_i64()) {
                metrics.insert("clone_groups".to_string(), MetricValue::Int(groups));
            }
        }

        // Extract max CRAP from file scores (exclude scripts/tests)
        if let Some(scores) = json
            .get("health")
            .and_then(|h| h.get("file_scores"))
            .and_then(|a| a.as_array())
        {
            let mut max_val = 0.0_f64;
            for score in scores {
                let path = score.get("path").and_then(|p| p.as_str()).unwrap_or("");
                // Skip scripts, tests, and node_modules
                if path.starts_with("scripts/")
                    || path.contains(".test.")
                    || path.contains(".spec.")
                    || path.contains("__tests__")
                    || path.contains("node_modules")
                {
                    continue;
                }
                if let Some(crap) = score.get("crap_max").and_then(|v| v.as_f64()) {
                    if crap > max_val {
                        max_val = crap;
                    }
                }
            }
            if max_val > 0.0 {
                metrics.insert("max_crap_app".to_string(), MetricValue::Float(max_val));
            }
        } else {
            metrics.insert("max_crap_app".to_string(), MetricValue::Float(0.0));
        }

        Ok(CheckerOutput {
            version: None, // fallow doesn't expose version in JSON output
            metrics,
        })
    }
}
