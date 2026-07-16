mod checker;
mod chart;
mod clippy;
mod compat_chart;
mod compat_md;
mod detect;
mod docker;
mod exec;
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
use std::process::Command;

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

    /// Output directory for generated files (default: auto-detect, e.g. public/ for Next.js)
    #[arg(long)]
    output_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let project_dir = args
        .project_dir
        .unwrap_or_else(|| PathBuf::from("."));

    // Resolve note early to avoid move issues
    let note = args.note.clone();

    // Resolve output directory (auto-detect Next.js/Vite → public/)
    let output_dir = resolve_output_dir(&project_dir, args.output_dir.as_ref());

    println!("🔍 Scanning project: {}", project_dir.display());

    // 1. Detect available checkers
    let checkers = detect::detect_checkers(&project_dir);
    if checkers.is_empty() {
        eprintln!("⚠️  No quality tools detected in {}", project_dir.display());
        eprintln!("   Place .fallowrc.json or vitest.config.* to enable checks.");
        std::process::exit(1);
    }

    println!("📋 Detected checkers: {}", checkers.join(", "));

    // Compute workspace hash (commit SHA + diff hash)
    let workspace_hash = get_workspace_hash(&project_dir);
    if let Some(hash) = &workspace_hash {
        println!("🔖 Workspace: {}", &hash[..12]);
    }

    // Load existing history early to check last run's workspace hash (for skip logic)
    let json_path = project_dir.join(&args.json);
    let mut history = history::load(&json_path).unwrap_or_default();
    let last_workspace_hash = history.runs.last().and_then(|r| r.workspace_hash.clone());

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
        match checker.run(&project_dir, args.verbose, last_workspace_hash.as_deref()) {
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

    // 6. Ensure project name is set
    if history.project.is_empty() {
        history.project = project_name.clone();
    }

    // 8. Append new run
    let run = history::Run {
        ts: Utc::now().to_rfc3339(),
        note: note.clone().unwrap_or_default(),
        commentary: String::new(),
        workspace_hash: workspace_hash.clone(),
        checkers: checker_results,
        metrics: unified.clone(),
    };
    history.runs.push(run);

    // 9. Save qualifier.json (project root — data, not served)
    history::save(&history, &json_path)?;
    println!("💾 Saved {}/{}", project_dir.display(), args.json);

    // 10. Generate quality.html (output_dir — auto-served for Next.js/Vite)
    std::fs::create_dir_all(&output_dir)?;
    let html_path = output_dir.join("quality.html");
    chart::generate(&history, &html_path)?;
    println!("📊 Generated quality.html");

    // 11. Compat: update fallow-chart.html if it exists (check both root and output_dir)
    let chart_path = output_dir.join("fallow-chart.html");
    let chart_path_root = project_dir.join("fallow-chart.html");
    if chart_path.exists() {
        if let Err(e) = compat_chart::update(&chart_path, &unified, &note.clone().unwrap_or_default()) {
            eprintln!("⚠️  Failed to update fallow-chart.html: {e}");
        } else {
            println!("📈 Updated fallow-chart.html (compat)");
        }
    } else if chart_path_root.exists() {
        if let Err(e) = compat_chart::update(&chart_path_root, &unified, &note.clone().unwrap_or_default()) {
            eprintln!("⚠️  Failed to update fallow-chart.html: {e}");
        } else {
            println!("📈 Updated fallow-chart.html (compat)");
        }
    }

    // 12. Compat: update fallow-progress.md if it exists (check both root and output_dir)
    let md_path = output_dir.join("fallow-progress.md");
    let md_path_root = project_dir.join("fallow-progress.md");
    if md_path.exists() {
        if let Err(e) = compat_md::update(&md_path, &unified, &note.clone().unwrap_or_default()) {
            eprintln!("⚠️  Failed to update fallow-progress.md: {e}");
        } else {
            println!("📝 Updated fallow-progress.md (compat)");
        }
    } else if md_path_root.exists() {
        if let Err(e) = compat_md::update(&md_path_root, &unified, &note.clone().unwrap_or_default()) {
            eprintln!("⚠️  Failed to update fallow-progress.md: {e}");
        } else {
            println!("📝 Updated fallow-progress.md (compat)");
        }
    }

    println!("\nDone. Open quality.html in a browser to see the dashboard.");
    Ok(())
}

/// Compute a SHA-256 hash of the workspace state (commit SHA + working tree diff).
/// Identical hash = identical code state, even with uncommitted changes.
/// Returns None if not a git repo or git isn't available.
fn get_workspace_hash(project_dir: &Path) -> Option<String> {
    // Get commit SHA
    let commit_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })?;

    // Get working tree diff, excluding qualifier's own output files
    // so that qualifier runs don't invalidate the hash themselves
    let diff = Command::new("git")
        .args(["diff", "--", ":!qualifier.json", ":!public/", ":!coverage/"])
        .current_dir(project_dir)
        .output()
        .ok()
        .and_then(|out| Some(String::from_utf8_lossy(&out.stdout).to_string()))
        .unwrap_or_default();

    // Hash: "commit_sha:sha256(diff)"
    use sha2::{Digest, Sha256};
    let diff_hash = hex::encode(Sha256::digest(&diff));
    Some(format!("{commit_sha}:{diff_hash}"))
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

/// Determine where output files should be written.
///
/// - Explicit `--output-dir` flag takes priority
/// - Next.js projects → `public/` (auto-served by Next.js static file handler)
/// - Vite projects → `public/` (served as-is)
/// - Everything else → project root
fn resolve_output_dir(
    project_dir: &Path,
    explicit: Option<&PathBuf>,
) -> PathBuf {
    // Explicit flag wins
    if let Some(path) = explicit {
        return path.clone();
    }

    // Detect Next.js (next.config.* or next in package.json dependencies)
    let has_next_config = project_dir.join("next.config.js").exists()
        || project_dir.join("next.config.mjs").exists()
        || project_dir.join("next.config.ts").exists();

    let has_next_dep = std::fs::read_to_string(project_dir.join("package.json"))
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .map(|v| {
            v.get("dependencies")
                .or_else(|| v.get("devDependencies"))
                .and_then(|d| d.get("next"))
                .is_some()
        })
        .unwrap_or(false);

    if has_next_config || has_next_dep {
        let public_dir = project_dir.join("public");
        if public_dir.exists() {
            return public_dir;
        }
    }

    // Detect Vite (vite.config.*)
    let has_vite = project_dir.join("vite.config.js").exists()
        || project_dir.join("vite.config.ts").exists();
    if has_vite {
        let public_dir = project_dir.join("public");
        if public_dir.exists() {
            return public_dir;
        }
    }

    // Default: project root
    project_dir.to_path_buf()
}
