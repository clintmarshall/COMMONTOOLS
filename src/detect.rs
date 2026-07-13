use std::path::Path;

/// Detect which checkers are available in the project
/// Returns a list of checker names
pub fn detect_checkers(project_dir: &Path) -> Vec<String> {
    let mut checkers = Vec::new();

    // Fallow: look for .fallowrc.json or fallow.toml
    if project_dir.join(".fallowrc.json").exists()
        || project_dir.join("fallow.toml").exists()
    {
        checkers.push("fallow".to_string());
    }

    // Vitest: look for vitest.config.*
    let pattern = format!("{}/vitest.config.*", project_dir.to_string_lossy());
    for entry in glob::glob(&pattern).unwrap_or_else(|_| {
        // Return an empty iterator on error
        glob::glob("/dev/null/nonexistent").unwrap_or_else(|_| glob::glob("___x___").unwrap())
    }) {
        if entry.is_ok() {
            checkers.push("vitest".to_string());
            break;
        }
    }

    // Clippy: look for Cargo.toml
    if project_dir.join("Cargo.toml").exists() {
        checkers.push("clippy".to_string());
    }

    checkers
}
