mod checker;
mod chart;
mod clippy;
mod compat_chart;
mod compat_md;
mod detect;
mod fallow;
mod history;
mod metrics;
mod summary;
mod vitest;

use anyhow::Result;
use clap::Parser;
use chrono::Utc;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Qualifier — Universal Project Quality Dashboard
///
/// Auto-detects project tooling, runs quality checks, and writes results.
#[derive(Parser, Debug)]
#[command(name = "qualifier", version, about, long_about = None)]
struct Args {
    /// One-line note for this run
    #[arg(short, long)]
    note: Option<String>,

    /// Path to qualifier.json (default: qualifier.json in project dir)
    #[arg(long, default_value = "qualifier.json")]
    json: String,

    /// Target project directory (default: current dir)
    #[arg(long, short = 'd')]
    project_dir: Option<PathBuf>,

    /// Run checks but don't write any files
    #[arg(long)]
    dry_run: bool,

    /// Show checker stdout/stderr
    #[arg(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let project_dir = args
        .project_dir
        .unwrap_or_else(|| PathBuf::from("."));

    // Resolve note early to avoid move issues
    let note = args.note.clone();

    println!("🔍 Scanning project: {}", project_dir.display());

    // 1. Detect available checkers
    let checkers = detect::detect_checkers(&project_dir);
    if checkers.is_empty() {
        eprintln!("⚠️  No quality tools detected in {}", project_dir.display());
        eprintln!("   Place .fallowrc.json or vitest.config.* to enable checks.");
        std::process::exit(1);
    }

    println!("📋 Detected checkers: {}", checkers.join(", "));

    // 2. Run checkers and collect metrics
    let mut all_metrics: BTreeMap<String, metrics::MetricValue> = BTreeMap::new();
    let mut checker_results: BTreeMap<String, history::CheckerResult> = BTreeMap::new();

    for name in &checkers {
        let start = std::time::Instant::now();

        let checker = checker::get_checker(name)?;
        if !checker.can_run(&project_dir) {
            println!("⏭️  Skipping {name} (prerequisites not met)");
            continue;
        }

        println!("⏳ Running {name}...");
        match checker.run(&project_dir, args.verbose) {
            Ok(result) => {
                let elapsed = start.elapsed().as_millis();
                println!("✅ {name} completed in {elapsed}ms");

                // Merge metrics
                all_metrics.extend(result.metrics);

                // Record checker info
                checker_results.insert(
                    name.clone(),
                    history::CheckerResult {
                        version: result.version,
                        elapsed_ms: elapsed as u64,
                    },
                );
            }
            Err(e) => {
                eprintln!("❌ {name} failed: {e}");
                checker_results.insert(
                    name.clone(),
                    history::CheckerResult {
                        version: None,
                        elapsed_ms: start.elapsed().as_millis() as u64,
                    },
                );
            }
        }
    }

    // 3. Build unified metrics
    let unified = metrics::build_unified(&all_metrics);

    // 4. Detect project name
    let project_name = detect_project_name(&project_dir);

    // 5. Print summary
    summary::print_summary(&unified, &checker_results, &project_name);

    if args.dry_run {
        println!("🔇 Dry run — no files written.");
        return Ok(());
    }

    // 6. Load existing history
    let json_path = project_dir.join(&args.json);
    let mut history = history::load(&json_path)?;

    // 7. Ensure project name is set
    if history.project.is_empty() {
        history.project = project_name.clone();
    }

    // 8. Append new run
    let run = history::Run {
        ts: Utc::now().to_rfc3339(),
        note: note.clone().unwrap_or_default(),
        commentary: String::new(),
        checkers: checker_results,
        metrics: unified.clone(),
    };
    history.runs.push(run);

    // 9. Save qualifier.json
    history::save(&history, &json_path)?;
    println!("💾 Saved {}/{}", project_dir.display(), args.json);

    // 10. Generate quality.html
    let html_path = project_dir.join("quality.html");
    chart::generate(&history, &html_path)?;
    println!("📊 Generated quality.html");

    // 11. Compat: update fallow-chart.html if it exists
    let chart_path = project_dir.join("fallow-chart.html");
    if chart_path.exists() {
        if let Err(e) = compat_chart::update(&chart_path, &unified, &note.clone().unwrap_or_default()) {
            eprintln!("⚠️  Failed to update fallow-chart.html: {e}");
        } else {
            println!("📈 Updated fallow-chart.html (compat)");
        }
    }

    // 12. Compat: update fallow-progress.md if it exists
    let md_path = project_dir.join("fallow-progress.md");
    if md_path.exists() {
        if let Err(e) = compat_md::update(&md_path, &unified, &note.clone().unwrap_or_default()) {
            eprintln!("⚠️  Failed to update fallow-progress.md: {e}");
        } else {
            println!("📝 Updated fallow-progress.md (compat)");
        }
    }

    println!("\nDone. Open quality.html in a browser to see the dashboard.");
    Ok(())
}

fn detect_project_name(project_dir: &Path) -> String {
    // Try package.json
    let package_json = project_dir.join("package.json");
    if package_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = value.get("name").and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
    }

    // Try Cargo.toml
    let cargo_toml = project_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            for line in content.lines() {
                if line.starts_with("name") {
                    if let Some(name) = line.split('=').nth(1) {
                        let name = name.trim().trim_matches('"');
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }

    // Fallback to directory name
    project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown-project".to_string())
}
