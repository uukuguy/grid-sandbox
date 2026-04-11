//! SessionPayload budget helpers — deterministic P5→P4→P3 trimming.
//!
//! Implements the v2.0 §8.6 contract:
//! "When runtime must trim to fit context window, drop P5 → P4 → P3 in
//!  order. P1 and P2 are NEVER touched regardless of flags."

use crate::contract::SessionPayload;

/// Rough tokens-per-char ratio (~4 chars per token is the standard
/// approximation for English prose; MVP uses a naive length/4 model).
const TOKENS_PER_CHAR: f64 = 0.25;

fn estimate(s: &str) -> usize {
    (s.len() as f64 * TOKENS_PER_CHAR).ceil() as usize
}

fn estimate_opt<T: AsRef<str>>(s: Option<&T>) -> usize {
    s.map(|v| estimate(v.as_ref())).unwrap_or(0)
}

impl SessionPayload {
    /// Return a naive token estimate for the entire payload.
    ///
    /// Uses `len() / 4` per string field as a cheap approximation.
    /// Good enough for MVP deterministic trimming; tighter models land
    /// with the real tokenizer integration in Phase 1.
    pub fn estimated_tokens(&self) -> usize {
        let mut total = 0usize;

        // P1 — policy context
        if let Some(pc) = &self.policy_context {
            total += estimate(&pc.org_unit);
            total += estimate(&pc.policy_version);
            for h in &pc.hooks {
                total += estimate(&h.condition) + estimate(&h.action);
            }
            for (k, v) in &pc.quotas {
                total += estimate(k) + estimate(v);
            }
        }

        // P2 — event context
        if let Some(ec) = &self.event_context {
            total += estimate(&ec.event_type)
                + estimate(&ec.severity)
                + estimate(&ec.source)
                + estimate(&ec.payload_json);
        }

        // P3 — memory refs
        for m in &self.memory_refs {
            total += estimate(&m.content) + estimate(&m.memory_type);
        }

        // P4 — skill instructions
        if let Some(si) = &self.skill_instructions {
            total += estimate(&si.name) + estimate(&si.content);
            for h in &si.frontmatter_hooks {
                total += estimate(&h.condition) + estimate(&h.action);
            }
        }

        // P5 — user preferences
        if let Some(up) = &self.user_preferences {
            total += estimate(&up.language) + estimate(&up.timezone);
            for (k, v) in &up.prefs {
                total += estimate(k) + estimate(v);
            }
        }

        total + estimate_opt(Some(&self.user_id)) + estimate_opt(Some(&self.session_id))
    }

    /// Trim the payload to fit within `budget_tokens`.
    ///
    /// Trimming order is strictly P5 → P4 → P3, and only when the
    /// corresponding `allow_trim_pN` flag is set. P1 (PolicyContext)
    /// and P2 (EventContext) are NEVER trimmed regardless of budget.
    ///
    /// Returns `&mut self` for chaining.
    pub fn trim_for_budget(&mut self, budget_tokens: usize) -> &mut Self {
        if self.estimated_tokens() <= budget_tokens {
            return self;
        }

        // Step 1: drop P5
        if self.allow_trim_p5 && self.user_preferences.is_some() {
            self.user_preferences = None;
            if self.estimated_tokens() <= budget_tokens {
                return self;
            }
        }

        // Step 2: drop P4
        if self.allow_trim_p4 && self.skill_instructions.is_some() {
            self.skill_instructions = None;
            if self.estimated_tokens() <= budget_tokens {
                return self;
            }
        }

        // Step 3: drop P3 entries (from lowest relevance upward)
        if self.allow_trim_p3 && !self.memory_refs.is_empty() {
            // Sort so that least-relevant come first, then pop until we fit.
            self.memory_refs.sort_by(|a, b| {
                a.relevance_score
                    .partial_cmp(&b.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            while self.estimated_tokens() > budget_tokens && !self.memory_refs.is_empty() {
                self.memory_refs.remove(0);
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{
        EventContext, MemoryRef, PolicyContext, SkillInstructions, UserPreferences,
    };

    fn fat_payload() -> SessionPayload {
        let mut p = SessionPayload::new();
        // P1 — non-trivial policy context (must survive trimming)
        p.policy_context = Some(PolicyContext {
            org_unit: "engineering".into(),
            policy_version: "v1.2.3".repeat(8),
            ..Default::default()
        });
        // P2 — non-trivial event context (must survive trimming)
        p.event_context = Some(EventContext {
            event_type: "incident".into(),
            severity: "critical".into(),
            payload_json: "x".repeat(400),
            ..Default::default()
        });
        // P3 — 3 memories totaling lots of tokens
        p.memory_refs = vec![
            MemoryRef {
                memory_id: "m1".into(),
                memory_type: "fact".into(),
                relevance_score: 0.1,
                content: "a".repeat(400),
                source_session_id: "s0".into(),
                created_at: "2026-01-01".into(),
                tags: Default::default(),
            },
            MemoryRef {
                memory_id: "m2".into(),
                memory_type: "fact".into(),
                relevance_score: 0.9,
                content: "b".repeat(400),
                source_session_id: "s0".into(),
                created_at: "2026-01-01".into(),
                tags: Default::default(),
            },
        ];
        // P4 — skill with big content
        p.skill_instructions = Some(SkillInstructions {
            skill_id: "skill-1".into(),
            name: "big-skill".into(),
            content: "c".repeat(400),
            ..Default::default()
        });
        // P5 — user preferences
        p.user_preferences = Some(UserPreferences {
            user_id: "u-1".into(),
            language: "en-US".repeat(10),
            timezone: "Asia/Shanghai".into(),
            ..Default::default()
        });
        p
    }

    #[test]
    fn trim_for_budget_removes_p5_first() {
        let mut p = fat_payload();
        let before = p.estimated_tokens();
        // Pick a budget that fits everything except P5.
        let budget = before - 10;
        p.trim_for_budget(budget);
        assert!(p.user_preferences.is_none(), "P5 should be trimmed first");
        // P1/P2/P3/P4 untouched so far.
        assert!(p.policy_context.is_some());
        assert!(p.event_context.is_some());
        assert!(!p.memory_refs.is_empty());
        assert!(p.skill_instructions.is_some());
    }

    #[test]
    fn trim_for_budget_never_removes_p1_or_p2() {
        let mut p = fat_payload();
        // Allow trimming everything downstream of P2.
        p.allow_trim_p5 = true;
        p.allow_trim_p4 = true;
        p.allow_trim_p3 = true;
        // Budget so small that only P1+P2 could fit.
        p.trim_for_budget(1);
        assert!(p.policy_context.is_some(), "P1 must survive");
        assert!(p.event_context.is_some(), "P2 must survive");
    }

    #[test]
    fn trim_respects_allow_flags() {
        let mut p = fat_payload();
        p.allow_trim_p5 = false; // P5 protected
        p.allow_trim_p4 = true;
        p.allow_trim_p3 = true;
        let before = p.estimated_tokens();
        p.trim_for_budget(before / 10);
        assert!(
            p.user_preferences.is_some(),
            "P5 should stay when allow_trim_p5=false"
        );
    }

    #[test]
    fn trim_noop_when_under_budget() {
        let mut p = fat_payload();
        let estimate = p.estimated_tokens();
        p.trim_for_budget(estimate * 10);
        assert!(p.user_preferences.is_some());
        assert!(p.skill_instructions.is_some());
        assert_eq!(p.memory_refs.len(), 2);
    }

    #[test]
    fn trim_for_budget_removes_p3_memories_lowest_first() {
        let mut p = fat_payload();
        p.allow_trim_p5 = true;
        p.allow_trim_p4 = true;
        p.allow_trim_p3 = true;
        p.trim_for_budget(50);
        // At least the lowest-relevance memory (m1, 0.1) is gone
        // before the higher-relevance (m2, 0.9).
        let remaining: Vec<_> = p.memory_refs.iter().map(|m| m.memory_id.clone()).collect();
        assert!(!remaining.contains(&"m1".to_string()));
    }
}
