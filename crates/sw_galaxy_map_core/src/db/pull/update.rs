use crate::db::pull::diff::PullDiffReport;

/// Read-only execution plan for a future local pull update.
#[derive(Debug, Clone)]
pub struct PullUpdatePlan {
    pub would_insert: usize,
    pub would_mark_stale: usize,
    pub expected_fid_remaps: usize,
    pub blocking_fid_mismatches: usize,
    pub grid_unit_mismatches: usize,
    pub can_apply: bool,
    pub blocking_reasons: Vec<String>,
}

impl PullUpdatePlan {
    /// Builds a pull update plan from a local/remote diff report.
    pub fn from_diff(report: &PullDiffReport) -> Self {
        let blocking_fid_mismatches = report.suspicious_positive_fid_mismatches();

        let mut blocking_reasons = Vec::new();

        if blocking_fid_mismatches > 0 {
            blocking_reasons.push(format!(
                "{blocking_fid_mismatches} suspicious positive FID mismatches require review"
            ));
        }

        if !report.grid_unit_mismatches.is_empty() {
            blocking_reasons.push(format!(
                "{} grid_unit mismatches require coordinate-unit alignment",
                report.grid_unit_mismatches.len()
            ));
        }

        let can_apply = blocking_reasons.is_empty();

        Self {
            would_insert: report.missing_local_planets.len(),
            would_mark_stale: report.stale_local_planets.len(),
            expected_fid_remaps: report.expected_synthetic_fid_mismatches(),
            blocking_fid_mismatches,
            grid_unit_mismatches: report.grid_unit_mismatches.len(),
            can_apply,
            blocking_reasons,
        }
    }
}
