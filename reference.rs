use std::process::Command;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Launching project health telemetry...");

    // 1. Run Fallow concurrently / sequentially
    let fallow_output = Command::new("npx")
        .args(&["fallow", "health", "--format=json"])
        .output()?;
    let fallow_json: Value = serde_json::from_slice(&fallow_output.stdout)?;
    let fallow_score = fallow_json["health_score"].as_f64().unwrap_or(0.0);

    // 2. Run your test coverage (Example: Vitest JSON output)
    let coverage_output = Command::new("npx")
        .args(&["vitest", "run", "--coverage", "--reporter=json"])
        .output()?;
    let coverage_json: Value = serde_json::from_slice(&coverage_output.stdout)?;
    let coverage_score = coverage_json["total"]["statements"]["pct"].as_f64().unwrap_or(0.0);

    // 3. Run Security Scan (Example: Skylos for vulnerabilities)
    let skylos_output = Command::new("skylos")
        .args(&[".", "--danger", "--format=json"])
        .output()?;
    let skylos_json: Value = serde_json::from_slice(&skylos_output.stdout)?;
    let vulns_count = skylos_json["vulnerabilities"].as_array().map_or(0, |v| v.len());

    // 4. Save metrics securely to your trajectory file
    let timestamp = Utc::now().format("%Y-%m-%d").to_string();
    let log_line = format!("{},{},{},{}\n", timestamp, fallow_score, coverage_score, vulns_count);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("docs/quality-history.csv")?;
    file.write_all(log_line.as_bytes())?;

    println!("✅ Quality trajectory logged successfully!");
    Ok(())
}
