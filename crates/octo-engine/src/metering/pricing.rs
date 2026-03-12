//! Model pricing table for estimating LLM API costs.

/// Pricing information for a single model pattern.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub model_pattern: &'static str,
    pub input_per_million: f64,
    pub output_per_million: f64,
}

static PRICING_TABLE: &[ModelPricing] = &[
    ModelPricing { model_pattern: "claude-opus", input_per_million: 15.0, output_per_million: 75.0 },
    ModelPricing { model_pattern: "claude-sonnet", input_per_million: 3.0, output_per_million: 15.0 },
    ModelPricing { model_pattern: "claude-haiku", input_per_million: 0.25, output_per_million: 1.25 },
    ModelPricing { model_pattern: "gpt-4o", input_per_million: 2.5, output_per_million: 10.0 },
    ModelPricing { model_pattern: "gpt-4-turbo", input_per_million: 10.0, output_per_million: 30.0 },
    ModelPricing { model_pattern: "gpt-4", input_per_million: 30.0, output_per_million: 60.0 },
    ModelPricing { model_pattern: "gpt-3.5", input_per_million: 0.5, output_per_million: 1.5 },
    ModelPricing { model_pattern: "o1", input_per_million: 15.0, output_per_million: 60.0 },
    ModelPricing { model_pattern: "o3", input_per_million: 10.0, output_per_million: 40.0 },
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
        let o3 = ModelPricing::lookup("o3-mini");
        assert_eq!(o3.model_pattern, "o3");
    }

    #[test]
    fn test_pricing_lookup_unknown() {
        let unknown = ModelPricing::lookup("some-custom-model-v2");
        assert_eq!(unknown.model_pattern, "unknown");
        assert!((unknown.input_per_million - 3.0).abs() < f64::EPSILON);
        assert!((unknown.output_per_million - 15.0).abs() < f64::EPSILON);
    }
}
