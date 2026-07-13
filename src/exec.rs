use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Runs a command, preferring Docker container if available, else host.
///
/// Returns `(output, was_docker)` — true if the command ran inside a container.
pub fn run_command(
    project_dir: &Path,
    cmd: &str,
    args: &[&str],
) -> Result<(std::process::Output, bool)> {
    // 1. Try Docker container first
    if let Some((output, _service)) = crate::docker::try_run_in_container(project_dir, cmd, args) {
        return Ok((output, true));
    }

    // 2. Fallback: try direct node_modules/.bin path (fastest on host)
    let bin_dir = project_dir.join("node_modules/.bin");
    if bin_dir.exists() {
        #[cfg(windows)]
        let cmd_path = bin_dir.join(format!("{}.cmd", cmd));
        #[cfg(not(windows))]
        let cmd_path = bin_dir.join(cmd);

        if cmd_path.exists() {
            let output = Command::new(&cmd_path)
                .args(args)
                .current_dir(project_dir)
                .output()?;
            return Ok((output, false));
        }
    }

    // 3. Fallback: npx
    #[cfg(windows)]
    let npx_cmd = "npx.cmd";
    #[cfg(not(windows))]
    let npx_cmd = "npx";

    let output = Command::new(npx_cmd)
        .arg(cmd)
        .args(args)
        .current_dir(project_dir)
        .output()?;

    Ok((output, false))
}
