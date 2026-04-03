//! Model pricing table for estimating LLM API costs.

/// Pricing information for a single model pattern.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub model_pattern: &'static str,
    pub input_per_million: f64,
    pub output_per_million: f64,
}

static PRICING_TABLE: &[ModelPricing] = &[
    // ── Anthropic (specific before generic) ──
    ModelPricing { model_pattern: "claude-4-opus", input_per_million: 15.0, output_per_million: 75.0 },
    ModelPricing { model_pattern: "claude-4-sonnet", input_per_million: 3.0, output_per_million: 15.0 },
    ModelPricing { model_pattern: "claude-opus", input_per_million: 15.0, output_per_million: 75.0 },
    ModelPricing { model_pattern: "claude-sonnet", input_per_million: 3.0, output_per_million: 15.0 },
    ModelPricing { model_pattern: "claude-haiku", input_per_million: 0.25, output_per_million: 1.25 },
    // ── OpenAI GPT (specific before generic) ──
    ModelPricing { model_pattern: "gpt-4.1-nano", input_per_million: 0.10, output_per_million: 0.40 },
    ModelPricing { model_pattern: "gpt-4.1-mini", input_per_million: 0.40, output_per_million: 1.60 },
    ModelPricing { model_pattern: "gpt-4.1", input_per_million: 2.0, output_per_million: 8.0 },
    ModelPricing { model_pattern: "gpt-4o-mini", input_per_million: 0.15, output_per_million: 0.60 },
    ModelPricing { model_pattern: "gpt-4o", input_per_million: 2.5, output_per_million: 10.0 },
    ModelPricing { model_pattern: "gpt-4-turbo", input_per_million: 10.0, output_per_million: 30.0 },
    ModelPricing { model_pattern: "gpt-4", input_per_million: 30.0, output_per_million: 60.0 },
    ModelPricing { model_pattern: "gpt-3.5", input_per_million: 0.5, output_per_million: 1.5 },
    // ── OpenAI o-series (specific before generic) ──
    ModelPricing { model_pattern: "o4-mini", input_per_million: 1.10, output_per_million: 4.40 },
    ModelPricing { model_pattern: "o3-mini", input_per_million: 1.10, output_per_million: 4.40 },
    ModelPricing { model_pattern: "o1-mini", input_per_million: 3.0, output_per_million: 12.0 },
    ModelPricing { model_pattern: "o1", input_per_million: 15.0, output_per_million: 60.0 },
    ModelPricing { model_pattern: "o3", input_per_million: 10.0, output_per_million: 40.0 },
    // ── Google Gemini (specific before generic) ──
    ModelPricing { model_pattern: "gemini-2.5-pro", input_per_million: 1.25, output_per_million: 10.0 },
    ModelPricing { model_pattern: "gemini-2.5-flash", input_per_million: 0.15, output_per_million: 0.60 },
    ModelPricing { model_pattern: "gemini-2.0-flash", input_per_million: 0.10, output_per_million: 0.40 },
    ModelPricing { model_pattern: "gemini-1.5-pro", input_per_million: 1.25, output_per_million: 5.0 },
    ModelPricing { model_pattern: "gemini-1.5-flash", input_per_million: 0.075, output_per_million: 0.30 },
    // ── Mistral ──
    ModelPricing { model_pattern: "mistral-large", input_per_million: 2.0, output_per_million: 6.0 },
    ModelPricing { model_pattern: "mistral-small", input_per_million: 0.2, output_per_million: 0.6 },
    ModelPricing { model_pattern: "codestral", input_per_million: 0.3, output_per_million: 0.9 },
    // ── Meta Llama ──
    ModelPricing { model_pattern: "llama-3.1-405b", input_per_million: 3.0, output_per_million: 3.0 },
    ModelPricing { model_pattern: "llama-3.1-70b", input_per_million: 0.88, output_per_million: 0.88 },
    ModelPricing { model_pattern: "llama-3.1-8b", input_per_million: 0.18, output_per_million: 0.18 },
    // ── DeepSeek ──
    ModelPricing { model_pattern: "deepseek-v3", input_per_million: 0.27, output_per_million: 1.10 },
    ModelPricing { model_pattern: "deepseek-r1", input_per_million: 0.55, output_per_million: 2.19 },
    // ── Cohere ──
    ModelPricing { model_pattern: "command-r-plus", input_per_million: 2.50, output_per_million: 10.0 },
    ModelPricing { model_pattern: "command-r", input_per_million: 0.15, output_per_million: 0.60 },
    // ── Alibaba Qwen ──
    ModelPricing { model_pattern: "qwen", input_per_million: 0.50, output_per_million: 1.50 },
];

fn default_pricing() -> ModelPricing {
    ModelPricing { model_pattern: "unknown", input_per_million: 3.0, output_per_million: 15.0 }
}

impl ModelPricing {
    pub fn lookup(model: &str) -> Self {
        let model_lower = model.to_lowercase();
        for entry in PRICING_TABLE {
            if model_lower.contains(entry.model_pattern) { return *entry; }
        }
        default_pricing()
    }

    pub fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 / 1_000_000.0) * self.input_per_million
            + (output_tokens as f64 / 1_000_000.0) * self.output_per_million
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_lookup_anthropic() {
        let opus = ModelPricing::lookup("claude-opus-4-20250514");
        assert_eq!(opus.model_pattern, "claude-opus");
        assert!((opus.input_per_million - 15.0).abs() < f64::EPSILON);
        assert!((opus.output_per_million - 75.0).abs() < f64::EPSILON);
        let sonnet = ModelPricing::lookup("claude-sonnet-4-20250514");
        assert_eq!(sonnet.model_pattern, "claude-sonnet");
        let haiku = ModelPricing::lookup("claude-haiku-3-20240307");
        assert_eq!(haiku.model_pattern, "claude-haiku");
    }

    #[test]
    fn test_pricing_lookup_anthropic_v4() {
        let opus4 = ModelPricing::lookup("claude-4-opus-20260101");
        assert_eq!(opus4.model_pattern, "claude-4-opus");
        assert!((opus4.input_per_million - 15.0).abs() < f64::EPSILON);
        let sonnet4 = ModelPricing::lookup("claude-4-sonnet-20260101");
        assert_eq!(sonnet4.model_pattern, "claude-4-sonnet");
    }

    #[test]
    fn test_pricing_lookup_openai() {
        let gpt4o = ModelPricing::lookup("gpt-4o-2024-05-13");
        assert_eq!(gpt4o.model_pattern, "gpt-4o");
        let gpt4_turbo = ModelPricing::lookup("gpt-4-turbo-preview");
        assert_eq!(gpt4_turbo.model_pattern, "gpt-4-turbo");
        let gpt4 = ModelPricing::lookup("gpt-4-0613");
        assert_eq!(gpt4.model_pattern, "gpt-4");
        let gpt35 = ModelPricing::lookup("gpt-3.5-turbo");
        assert_eq!(gpt35.model_pattern, "gpt-3.5");
        let o1 = ModelPricing::lookup("o1-preview");
        assert_eq!(o1.model_pattern, "o1");
        let o3 = ModelPricing::lookup("o3-2025-04-16");
        assert_eq!(o3.model_pattern, "o3");
    }

    #[test]
    fn test_pricing_lookup_unknown() {
        let unknown = ModelPricing::lookup("some-custom-model-v2");
        assert_eq!(unknown.model_pattern, "unknown");
        assert!((unknown.input_per_million - 3.0).abs() < f64::EPSILON);
        assert!((unknown.output_per_million - 15.0).abs() < f64::EPSILON);
    }

    /// Specific patterns MUST match before their less-specific parents.
    #[test]
    fn test_pricing_ordering_specificity() {
        // gpt-4o-mini must NOT fall through to gpt-4o
        let mini = ModelPricing::lookup("gpt-4o-mini-2024-07-18");
        assert_eq!(mini.model_pattern, "gpt-4o-mini");
        assert!((mini.input_per_million - 0.15).abs() < f64::EPSILON);

        // o1-mini must NOT fall through to o1
        let o1m = ModelPricing::lookup("o1-mini-2024-09-12");
        assert_eq!(o1m.model_pattern, "o1-mini");
        assert!((o1m.input_per_million - 3.0).abs() < f64::EPSILON);

        // o3-mini must NOT fall through to o3
        let o3m = ModelPricing::lookup("o3-mini-2025-01-31");
        assert_eq!(o3m.model_pattern, "o3-mini");
        assert!((o3m.input_per_million - 1.10).abs() < f64::EPSILON);

        // o4-mini
        let o4m = ModelPricing::lookup("o4-mini-2025-04-16");
        assert_eq!(o4m.model_pattern, "o4-mini");

        // gpt-4.1-mini must NOT fall through to gpt-4.1
        let g41m = ModelPricing::lookup("gpt-4.1-mini-2025-04-14");
        assert_eq!(g41m.model_pattern, "gpt-4.1-mini");

        // gpt-4.1-nano must NOT fall through to gpt-4.1
        let g41n = ModelPricing::lookup("gpt-4.1-nano-2025-04-14");
        assert_eq!(g41n.model_pattern, "gpt-4.1-nano");

        // command-r-plus must NOT fall through to command-r
        let crp = ModelPricing::lookup("command-r-plus-08-2024");
        assert_eq!(crp.model_pattern, "command-r-plus");
    }

    #[test]
    fn test_pricing_lookup_gemini() {
        let g25p = ModelPricing::lookup("gemini-2.5-pro-preview-05-06");
        assert_eq!(g25p.model_pattern, "gemini-2.5-pro");
        let g25f = ModelPricing::lookup("gemini-2.5-flash-preview-04-17");
        assert_eq!(g25f.model_pattern, "gemini-2.5-flash");
        let g20f = ModelPricing::lookup("gemini-2.0-flash");
        assert_eq!(g20f.model_pattern, "gemini-2.0-flash");
        let g15p = ModelPricing::lookup("gemini-1.5-pro-002");
        assert_eq!(g15p.model_pattern, "gemini-1.5-pro");
        let g15f = ModelPricing::lookup("gemini-1.5-flash-002");
        assert_eq!(g15f.model_pattern, "gemini-1.5-flash");
    }

    #[test]
    fn test_pricing_lookup_mistral() {
        let large = ModelPricing::lookup("mistral-large-latest");
        assert_eq!(large.model_pattern, "mistral-large");
        let small = ModelPricing::lookup("mistral-small-latest");
        assert_eq!(small.model_pattern, "mistral-small");
        let code = ModelPricing::lookup("codestral-2501");
        assert_eq!(code.model_pattern, "codestral");
    }

    #[test]
    fn test_pricing_lookup_llama() {
        let l405 = ModelPricing::lookup("llama-3.1-405b-instruct");
        assert_eq!(l405.model_pattern, "llama-3.1-405b");
        let l70 = ModelPricing::lookup("llama-3.1-70b-instruct");
        assert_eq!(l70.model_pattern, "llama-3.1-70b");
        let l8 = ModelPricing::lookup("llama-3.1-8b-instruct");
        assert_eq!(l8.model_pattern, "llama-3.1-8b");
    }

    #[test]
    fn test_pricing_lookup_deepseek() {
        let v3 = ModelPricing::lookup("deepseek-v3-0324");
        assert_eq!(v3.model_pattern, "deepseek-v3");
        let r1 = ModelPricing::lookup("deepseek-r1-0528");
        assert_eq!(r1.model_pattern, "deepseek-r1");
    }

    #[test]
    fn test_pricing_lookup_cohere() {
        let crp = ModelPricing::lookup("command-r-plus");
        assert_eq!(crp.model_pattern, "command-r-plus");
        let cr = ModelPricing::lookup("command-r");
        assert_eq!(cr.model_pattern, "command-r");
    }

    #[test]
    fn test_pricing_lookup_qwen() {
        let q = ModelPricing::lookup("qwen-2.5-72b");
        assert_eq!(q.model_pattern, "qwen");
    }

    #[test]
    fn test_pricing_table_entry_count() {
        // 9 original + 25 new = 34 total entries
        assert_eq!(PRICING_TABLE.len(), 34);
    }
}
