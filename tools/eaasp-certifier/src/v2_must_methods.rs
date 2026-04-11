//! v2.0 method classification — 12 MUST + 4 OPTIONAL + 1 PLACEHOLDER.
//!
//! Source of truth: `proto/eaasp/runtime/v2/runtime.proto` §8 and
//! `docs/design/EAASP/EAASP-Design-Specification-v2.0.docx` §8.
//!
//! Any conforming runtime **MUST** pass every test that targets a method
//! in `MUST_METHODS`. Failure on an `OPTIONAL_METHODS` entry is reported
//! as a WARN, not a FAIL, so that minimal runtimes can still be certified.
//!
//! The single placeholder method (`emit_event`, ADR-V2-001 pending) is
//! reported as metadata — its presence is informational, not required.

/// 12 certified core methods. Every v2-conforming runtime MUST implement
/// and pass tests for each of these.
pub const MUST_METHODS: &[&str] = &[
    "initialize",
    "send",
    "load_skill",
    "on_tool_call",
    "on_tool_result",
    "on_stop",
    "get_state",
    "connect_mcp",
    "emit_telemetry",
    "get_capabilities",
    "terminate",
    "restore_state",
];

/// 4 optional methods. Runtimes MAY implement these for feature parity.
/// Absent entries produce a WARN in the certifier report.
pub const OPTIONAL_METHODS: &[&str] = &[
    "health",
    "disconnect_mcp",
    "pause_session",
    "resume_session",
];

/// Placeholder method — ADR-V2-001 pending. Presence is reported as
/// metadata only; failures do not affect certification status.
pub const PLACEHOLDER_METHODS: &[&str] = &["emit_event"];

/// Canonical list of all 17 v2 methods (MUST + OPTIONAL + PLACEHOLDER).
pub fn all_methods() -> Vec<&'static str> {
    let mut v = Vec::with_capacity(
        MUST_METHODS.len() + OPTIONAL_METHODS.len() + PLACEHOLDER_METHODS.len(),
    );
    v.extend_from_slice(MUST_METHODS);
    v.extend_from_slice(OPTIONAL_METHODS);
    v.extend_from_slice(PLACEHOLDER_METHODS);
    v
}

/// Normalize a method name to snake_case for table lookup.
///
/// Accepts `"OnToolCall"`, `"on_tool_call"`, `"onToolCall"` — all map
/// to `"on_tool_call"`.
pub fn normalize(method: &str) -> String {
    let mut out = String::with_capacity(method.len() + 4);
    let mut prev_lower = false;
    for ch in method.chars() {
        if ch == '_' || ch == '-' {
            out.push('_');
            prev_lower = false;
            continue;
        }
        if ch.is_ascii_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower = false;
        } else {
            out.push(ch);
            prev_lower = ch.is_ascii_lowercase();
        }
    }
    out
}

/// Returns true if `method` is one of the 12 MUST methods.
pub fn is_must(method: &str) -> bool {
    let n = normalize(method);
    MUST_METHODS.iter().any(|m| *m == n)
}

/// Returns true if `method` is one of the 4 OPTIONAL methods.
pub fn is_optional(method: &str) -> bool {
    let n = normalize(method);
    OPTIONAL_METHODS.iter().any(|m| *m == n)
}

/// Returns true if `method` is one of the placeholder methods
/// (currently: `emit_event`).
pub fn is_placeholder(method: &str) -> bool {
    let n = normalize(method);
    PLACEHOLDER_METHODS.iter().any(|m| *m == n)
}

/// Classification of a v2 method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodClass {
    /// 12 MUST methods — failure is a certification FAIL.
    Must,
    /// 4 OPTIONAL methods — failure is a WARN.
    Optional,
    /// 1 PLACEHOLDER method — informational only.
    Placeholder,
    /// Not a known v2 method.
    Unknown,
}

impl MethodClass {
    pub fn of(method: &str) -> Self {
        if is_must(method) {
            Self::Must
        } else if is_optional(method) {
            Self::Optional
        } else if is_placeholder(method) {
            Self::Placeholder
        } else {
            Self::Unknown
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Must => "MUST",
            Self::Optional => "OPTIONAL",
            Self::Placeholder => "PLACEHOLDER",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn must_has_12_methods() {
        assert_eq!(MUST_METHODS.len(), 12);
    }

    #[test]
    fn optional_has_4_methods() {
        assert_eq!(OPTIONAL_METHODS.len(), 4);
    }

    #[test]
    fn placeholder_has_emit_event() {
        assert_eq!(PLACEHOLDER_METHODS.len(), 1);
        assert_eq!(PLACEHOLDER_METHODS[0], "emit_event");
    }

    #[test]
    fn all_methods_length_is_17() {
        assert_eq!(all_methods().len(), 17);
    }

    #[test]
    fn is_must_accepts_multiple_cases() {
        assert!(is_must("initialize"));
        assert!(is_must("Initialize"));
        assert!(is_must("OnToolCall"));
        assert!(is_must("on_tool_call"));
        assert!(is_must("LoadSkill"));
        assert!(is_must("RestoreState"));
    }

    #[test]
    fn is_optional_accepts_v2_methods() {
        assert!(is_optional("health"));
        assert!(is_optional("Health"));
        assert!(is_optional("DisconnectMcp"));
        assert!(is_optional("pause_session"));
        assert!(is_optional("ResumeSession"));
    }

    #[test]
    fn is_placeholder_detects_emit_event() {
        assert!(is_placeholder("EmitEvent"));
        assert!(is_placeholder("emit_event"));
    }

    #[test]
    fn must_and_optional_are_disjoint() {
        for m in MUST_METHODS {
            assert!(!is_optional(m), "{m} leaked from MUST into OPTIONAL");
        }
        for m in OPTIONAL_METHODS {
            assert!(!is_must(m), "{m} leaked from OPTIONAL into MUST");
        }
    }

    #[test]
    fn method_class_distinguishes_categories() {
        assert_eq!(MethodClass::of("initialize"), MethodClass::Must);
        assert_eq!(MethodClass::of("health"), MethodClass::Optional);
        assert_eq!(MethodClass::of("emit_event"), MethodClass::Placeholder);
        assert_eq!(MethodClass::of("totally_bogus"), MethodClass::Unknown);
    }

    #[test]
    fn method_class_labels() {
        assert_eq!(MethodClass::Must.label(), "MUST");
        assert_eq!(MethodClass::Optional.label(), "OPTIONAL");
        assert_eq!(MethodClass::Placeholder.label(), "PLACEHOLDER");
    }
}
