// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Model ladder - routes tasks to different model sizes based on complexity.

use crate::config::ModelConfig;
use crate::context::estimate_tokens;

/// Model tier for the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Light,
    Default,
    Heavy,
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelTier::Light => write!(f, "light"),
            ModelTier::Default => write!(f, "default"),
            ModelTier::Heavy => write!(f, "heavy"),
        }
    }
}

/// Model router that selects the appropriate model for a task.
pub struct ModelRouter {
    config: ModelConfig,
    /// Available model tiers (based on what's configured).
    available_tiers: Vec<ModelTier>,
}

impl ModelRouter {
    pub fn new(config: &ModelConfig) -> Self {
        let mut available_tiers = vec![ModelTier::Default];
        if config.light.is_some() {
            available_tiers.push(ModelTier::Light);
        }
        if config.heavy.is_some() {
            available_tiers.push(ModelTier::Heavy);
        }

        Self {
            config: config.clone(),
            available_tiers,
        }
    }

    /// Select the appropriate model for a user input.
    pub fn select_model(&self, user_input: &str) -> (String, ModelTier) {
        let tier = self.estimate_tier(user_input);
        let model = self.model_for_tier(tier);
        (model, tier)
    }

    /// Get the model name for a specific tier.
    pub fn model_for_tier(&self, tier: ModelTier) -> String {
        match tier {
            ModelTier::Light => self
                .config
                .light
                .clone()
                .unwrap_or_else(|| self.config.default.clone()),
            ModelTier::Default => self.config.default.clone(),
            ModelTier::Heavy => self
                .config
                .heavy
                .clone()
                .unwrap_or_else(|| self.config.default.clone()),
        }
    }

    /// Escalate to the next tier (for retry on failure).
    pub fn escalate(&self, current_tier: ModelTier) -> Option<(String, ModelTier)> {
        let next_tier = match current_tier {
            ModelTier::Light => ModelTier::Default,
            ModelTier::Default => ModelTier::Heavy,
            ModelTier::Heavy => return None,
        };

        if self.available_tiers.contains(&next_tier) {
            Some((self.model_for_tier(next_tier), next_tier))
        } else {
            None
        }
    }

    /// Estimate task complexity based on the user input.
    fn estimate_tier(&self, user_input: &str) -> ModelTier {
        let input_lower = user_input.to_lowercase();
        let input_tokens = estimate_tokens(user_input);

        // Simple heuristics for routing

        // Heavy: complex multi-file tasks, refactoring, architecture
        let heavy_keywords = [
            "refactor", "redesign", "architect", "rewrite",
            "across all files", "entire codebase", "multi-file",
            "from scratch", "implement", "complex",
        ];
        if heavy_keywords.iter().any(|kw| input_lower.contains(kw)) {
            if self.available_tiers.contains(&ModelTier::Heavy) {
                return ModelTier::Heavy;
            }
        }

        // Light: simple reads, listings, quick lookups
        let light_keywords = [
            "show me", "read", "list", "what's in",
            "cat", "find", "grep", "search for",
            "status", "diff", "log",
        ];
        if light_keywords.iter().any(|kw| input_lower.contains(kw))
            && input_tokens < 50
        {
            if self.available_tiers.contains(&ModelTier::Light) {
                return ModelTier::Light;
            }
        }

        // Default for everything else
        ModelTier::Default
    }

    /// Get the default model name.
    pub fn default_model(&self) -> &str {
        &self.config.default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModelConfig;

    #[test]
    fn test_default_model_only() {
        let config = ModelConfig {
            default: "llama3.1:8b".to_string(),
            heavy: None,
            light: None,
            ..ModelConfig::default()
        };
        let router = ModelRouter::new(&config);

        let (model, tier) = router.select_model("fix the bug");
        assert_eq!(model, "llama3.1:8b");
        assert_eq!(tier, ModelTier::Default);
    }

    #[test]
    fn test_with_all_tiers() {
        let config = ModelConfig {
            default: "llama3.1:8b".to_string(),
            heavy: Some("llama3.1:70b".to_string()),
            light: Some("llama3.2:3b".to_string()),
            ..ModelConfig::default()
        };
        let router = ModelRouter::new(&config);

        // Heavy task
        let (model, tier) = router.select_model("refactor the entire authentication module");
        assert_eq!(tier, ModelTier::Heavy);
        assert_eq!(model, "llama3.1:70b");

        // Light task
        let (model, tier) = router.select_model("show me the file");
        assert_eq!(tier, ModelTier::Light);
        assert_eq!(model, "llama3.2:3b");

        // Default task
        let (model, tier) = router.select_model("add error handling to this function");
        assert_eq!(tier, ModelTier::Default);
        assert_eq!(model, "llama3.1:8b");
    }

    #[test]
    fn test_escalation() {
        let config = ModelConfig {
            default: "llama3.1:8b".to_string(),
            heavy: Some("llama3.1:70b".to_string()),
            light: Some("llama3.2:3b".to_string()),
            ..ModelConfig::default()
        };
        let router = ModelRouter::new(&config);

        let escalated = router.escalate(ModelTier::Default);
        assert!(escalated.is_some());
        let (model, tier) = escalated.unwrap();
        assert_eq!(tier, ModelTier::Heavy);
        assert_eq!(model, "llama3.1:70b");

        // Can't escalate beyond heavy
        assert!(router.escalate(ModelTier::Heavy).is_none());
    }

    #[test]
    fn test_fallback_when_tier_unavailable() {
        let config = ModelConfig {
            default: "llama3.1:8b".to_string(),
            heavy: None,
            light: None,
            ..ModelConfig::default()
        };
        let router = ModelRouter::new(&config);

        // Should fall back to default even for "heavy" requests
        let model = router.model_for_tier(ModelTier::Heavy);
        assert_eq!(model, "llama3.1:8b");
    }
}
