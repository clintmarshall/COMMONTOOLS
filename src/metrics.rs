use std::collections::BTreeMap;

/// A metric value from a checker
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Float(f64),
    Int(i64),
    String(String),
}

impl MetricValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MetricValue::Float(v) => Some(*v),
            MetricValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            MetricValue::Int(v) => Some(*v),
            MetricValue::Float(v) => Some(*v as i64),
            _ => None,
        }
    }
}

/// Unified metrics struct — all checkers contribute to this
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UnifiedMetrics {
    // Fallow metrics
    pub loc: Option<i64>,
    pub dead_file_pct: Option<f64>,
    pub dead_file_count: Option<i64>,
    pub dead_export_pct: Option<f64>,
    pub dead_export_count: Option<i64>,
    pub avg_cyclomatic: Option<f64>,
    pub p90_cyclomatic: Option<f64>,
    pub mi: Option<f64>,
    pub max_crap_app: Option<f64>,
    pub dup_lines: Option<i64>,
    pub dup_pct: Option<f64>,
    pub clone_groups: Option<i64>,

    // Vitest metrics
    pub cov_stmt: Option<f64>,
    pub cov_branch: Option<f64>,
    pub cov_func: Option<f64>,
    pub cov_lines: Option<f64>,
    pub test_count: Option<i64>,
    pub test_files: Option<i64>,

    // Security metrics (future)
    // pub vuln_count: Option<i64>,
    // pub vuln_critical: Option<i64>,
    // pub vuln_high: Option<i64>,
}

/// Build unified metrics from a flat BTreeMap
pub fn build_unified(metrics: &BTreeMap<String, MetricValue>) -> UnifiedMetrics {
    UnifiedMetrics {
        loc: get_int(metrics, "loc"),
        dead_file_pct: get_float(metrics, "dead_file_pct"),
        dead_file_count: get_int(metrics, "dead_file_count"),
        dead_export_pct: get_float(metrics, "dead_export_pct"),
        dead_export_count: get_int(metrics, "dead_export_count"),
        avg_cyclomatic: get_float(metrics, "avg_cyclomatic"),
        p90_cyclomatic: get_float(metrics, "p90_cyclomatic"),
        mi: get_float(metrics, "mi"),
        max_crap_app: get_float(metrics, "max_crap_app"),
        dup_lines: get_int(metrics, "dup_lines"),
        dup_pct: get_float(metrics, "dup_pct"),
        clone_groups: get_int(metrics, "clone_groups"),
        cov_stmt: get_float(metrics, "cov_stmt"),
        cov_branch: get_float(metrics, "cov_branch"),
        cov_func: get_float(metrics, "cov_func"),
        cov_lines: get_float(metrics, "cov_lines"),
        test_count: get_int(metrics, "test_count"),
        test_files: get_int(metrics, "test_files"),
    }
}

fn get_float(metrics: &BTreeMap<String, MetricValue>, key: &str) -> Option<f64> {
    metrics.get(key).and_then(|v| v.as_f64())
}

fn get_int(metrics: &BTreeMap<String, MetricValue>, key: &str) -> Option<i64> {
    metrics.get(key).and_then(|v| v.as_i64())
}
