use crate::metrics::MetricValue;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

/// Result of running a single checker
pub struct CheckerOutput {
    pub version: Option<String>,
    pub metrics: BTreeMap<String, MetricValue>,
}

/// Trait for quality checkers
pub trait Checker: Send + Sync {
    fn name(&self) -> &str;

    /// Check if this checker can run in the given project
    fn can_run(&self, project_dir: &Path) -> bool;

    /// Run the checker and return metrics
    fn run(
        &self,
        project_dir: &Path,
        verbose: bool,
    ) -> Result<CheckerOutput, anyhow::Error>;
}

/// Get a checker by name
pub fn get_checker(name: &str) -> Result<Box<dyn Checker>> {
    match name {
        "fallow" => Ok(Box::new(crate::fallow::FallowChecker)),
        "vitest" => Ok(Box::new(crate::vitest::VitestChecker)),
        "clippy" => Ok(Box::new(crate::clippy::ClippyChecker)),
        unknown => anyhow::bail!("Unknown checker: {unknown}"),
    }
}
