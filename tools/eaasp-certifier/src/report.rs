//! Report generation utilities for certifier output.

use crate::verifier::VerificationReport;

/// Generate a markdown-formatted report.
pub fn to_markdown(report: &VerificationReport) -> String {
    let mut md = String::new();
    md.push_str("# EAASP v2.0 Contract Verification Report\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Endpoint | `{}` |\n", report.endpoint));
    md.push_str(&format!(
        "| Runtime | {} ({}) |\n",
        report.runtime_name, report.runtime_id
    ));
    md.push_str(&format!("| Tier | {} |\n", report.tier));
    md.push_str(&format!(
        "| MUST methods | {}/{} PASS |\n",
        report.must_passed, report.must_total
    ));
    md.push_str(&format!(
        "| OPTIONAL methods | {}/{} present |\n",
        report.optional_present, report.optional_total
    ));
    md.push_str(&format!(
        "| EmitEvent placeholder | {} |\n",
        if report.placeholder_present {
            "present (ADR-V2-001 pending)"
        } else {
            "absent"
        }
    ));
    md.push_str(&format!(
        "| Status | {} |\n\n",
        if report.passed { "PASS" } else { "FAIL" }
    ));

    md.push_str("## Method Results\n\n");
    md.push_str("| Method | Class | Status | Duration | Notes |\n");
    md.push_str("|--------|-------|--------|----------|-------|\n");

    for r in &report.results {
        let status = if r.passed { "PASS" } else { "FAIL" };
        let notes = r
            .error
            .as_deref()
            .or(r.notes.as_deref())
            .unwrap_or("-");
        md.push_str(&format!(
            "| {} | {} | {} | {}ms | {} |\n",
            r.method, r.class, status, r.duration_ms, notes
        ));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::{MethodResult, VerificationReport};

    #[test]
    fn markdown_report_includes_must_split() {
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
            must_total: 1,
            must_passed: 1,
            optional_total: 1,
            optional_present: 1,
            placeholder_present: true,
            results: vec![
                MethodResult {
                    method: "initialize".into(),
                    class: "MUST".into(),
                    passed: true,
                    duration_ms: 5,
                    error: None,
                    notes: None,
                },
                MethodResult {
                    method: "health".into(),
                    class: "OPTIONAL".into(),
                    passed: true,
                    duration_ms: 2,
                    error: None,
                    notes: None,
                },
            ],
            timestamp: "2026-04-11T12:00:00Z".into(),
        };

        let md = to_markdown(&report);
        assert!(md.contains("PASS"));
        assert!(md.contains("Grid"));
        assert!(md.contains("MUST methods"));
        assert!(md.contains("OPTIONAL methods"));
        assert!(md.contains("EmitEvent placeholder"));
        assert!(md.contains("1/1 PASS"));
        assert!(md.contains("ADR-V2-001"));
    }
}
