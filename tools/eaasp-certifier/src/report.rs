//! Report generation utilities for certifier output.

use crate::verifier::VerificationReport;

/// Generate a markdown-formatted report.
pub fn to_markdown(report: &VerificationReport) -> String {
    let mut md = String::new();
    md.push_str("# EAASP Contract Verification Report\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Endpoint | `{}` |\n", report.endpoint));
    md.push_str(&format!(
        "| Runtime | {} ({}) |\n",
        report.runtime_name, report.runtime_id
    ));
    md.push_str(&format!("| Tier | {} |\n", report.tier));
    md.push_str(&format!(
        "| Result | {}/{} passed |\n",
        report.passed_count, report.total
    ));
    md.push_str(&format!(
        "| Status | {} |\n\n",
        if report.passed { "PASS" } else { "FAIL" }
    ));

    md.push_str("## Method Results\n\n");
    md.push_str("| Method | Status | Duration | Notes |\n");
    md.push_str("|--------|--------|----------|-------|\n");

    for r in &report.results {
        let status = if r.passed { "PASS" } else { "FAIL" };
        let notes = r
            .error
            .as_deref()
            .or(r.notes.as_deref())
            .unwrap_or("-");
        md.push_str(&format!(
            "| {} | {} | {}ms | {} |\n",
            r.method, status, r.duration_ms, notes
        ));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::{MethodResult, VerificationReport};

    #[test]
    fn markdown_report_format() {
        let report = VerificationReport {
            endpoint: "http://localhost:50051".into(),
            runtime_id: "grid-harness".into(),
            runtime_name: "Grid".into(),
            tier: "harness".into(),
            deployment_mode: "shared".into(),
            passed: true,
            total: 2,
            passed_count: 2,
            failed_count: 0,
            results: vec![
                MethodResult {
                    method: "Health".into(),
                    passed: true,
                    duration_ms: 5,
                    error: None,
                    notes: None,
                },
                MethodResult {
                    method: "Initialize".into(),
                    passed: true,
                    duration_ms: 12,
                    error: None,
                    notes: Some("session-123".into()),
                },
            ],
            timestamp: "2026-04-06T12:00:00Z".into(),
        };

        let md = to_markdown(&report);
        assert!(md.contains("PASS"));
        assert!(md.contains("Grid"));
        assert!(md.contains("Health"));
        assert!(md.contains("2/2"));
    }
}
