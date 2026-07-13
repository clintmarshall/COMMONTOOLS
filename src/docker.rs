use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Check if the project has a docker-compose file.
pub fn has_compose(project_dir: &Path) -> bool {
    project_dir.join("docker-compose.yml").exists()
        || project_dir.join("docker-compose.yaml").exists()
}

/// Check if a specific service is currently running in docker compose.
pub fn is_service_running(project_dir: &Path, service: &str) -> bool {
    if !has_compose(project_dir) {
        return false;
    }

    let output = Command::new("docker")
        .args([
            "compose",
            "ps",
            "--services",
            "--filter",
            "status=running",
        ])
        .current_dir(project_dir)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let services = String::from_utf8_lossy(&out.stdout);
            services
                .lines()
                .any(|line| line.trim() == service)
        }
        _ => false,
    }
}

/// Try to find the application service name from docker-compose.
/// Strategy: try common names ("app", "web"), check which is running.
pub fn find_app_service(project_dir: &Path) -> Option<String> {
    let candidates = ["app", "web", "frontend", "backend", "server"];

    for name in &candidates {
        if is_service_running(project_dir, name) {
            return Some(name.to_string());
        }
    }

    // Fallback: if any service is running, return the first one
    let output = Command::new("docker")
        .args(["compose", "ps", "--services", "--filter", "status=running"])
        .current_dir(project_dir)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let services = String::from_utf8_lossy(&out.stdout);
            if let Some(first) = services.lines().find(|l| !l.trim().is_empty()) {
                return Some(first.trim().to_string());
            }
        }
    }

    None
}

/// Check if a binary exists inside the container's node_modules/.bin.
fn binary_in_container(project_dir: &Path, service: &str, binary: &str) -> bool {
    let output = Command::new("docker")
        .args([
            "compose",
            "exec",
            "-T",
            service,
            "sh",
            "-c",
            &format!("test -f node_modules/.bin/{} && echo yes", binary),
        ])
        .current_dir(project_dir)
        .output();

    matches!(output, Ok(out) if out.status.success() && String::from_utf8_lossy(&out.stdout).contains("yes"))
}

/// Execute a command inside a docker compose container.
/// Uses `docker compose exec -T <service> node_modules/.bin/<cmd> <args...>`
pub fn exec_in_container(
    project_dir: &Path,
    service: &str,
    cmd: &str,
    args: &[&str],
) -> Result<std::process::Output> {
    // Use node_modules/.bin/<cmd> for direct execution (no npx overhead)
    let bin_path = format!("node_modules/.bin/{}", cmd);
    let mut docker_args: Vec<String> =
        vec!["compose".into(), "exec".into(), "-T".into(), service.into(), bin_path];
    docker_args.extend(args.iter().map(|s| s.to_string()));

    Ok(Command::new("docker")
        .args(&docker_args)
        .current_dir(project_dir)
        .output()?)
}

/// Run a command inside Docker container if the binary is installed there.
/// Returns Some((output, service)) if run in container, None to fallback to host.
pub fn try_run_in_container(
    project_dir: &Path,
    cmd: &str,
    args: &[&str],
) -> Option<(std::process::Output, String)> {
    if !has_compose(project_dir) {
        return None;
    }

    let service = find_app_service(project_dir)?;

    // Only use container if the binary is installed there
    if !binary_in_container(project_dir, &service, cmd) {
        return None;
    }

    match exec_in_container(project_dir, &service, cmd, args) {
        Ok(output) => Some((output, service)),
        Err(_) => None,
    }
}
