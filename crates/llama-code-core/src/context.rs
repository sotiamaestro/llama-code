// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Context window management with Llama-optimized packing.

use crate::config::ModelParameters;

/// Approximate tokens from character count.
/// Rule of thumb: 1 token ~= 4 characters for English code.
/// We overestimate slightly to leave headroom.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Context budget calculator.
#[derive(Debug, Clone)]
pub struct ContextBudget {
    /// Total context window size in tokens.
    pub total: usize,
    /// Reserved for system prompt.
    pub system_prompt: usize,
    /// Reserved for tool definitions.
    pub tool_definitions: usize,
    /// Reserved for model response.
    pub response_reserve: usize,
    /// Safety buffer.
    pub safety_buffer: usize,
}

impl ContextBudget {
    pub fn new(params: &ModelParameters) -> Self {
        Self {
            total: params.num_ctx,
            system_prompt: 2000,
            tool_definitions: 1500,
            response_reserve: params.num_predict,
            safety_buffer: 500,
        }
    }

    /// Available tokens for conversation history and file context.
    pub fn available(&self) -> usize {
        self.total
            .saturating_sub(self.system_prompt)
            .saturating_sub(self.tool_definitions)
            .saturating_sub(self.response_reserve)
            .saturating_sub(self.safety_buffer)
    }

    /// Calculate current usage as a fraction (0.0 to 1.0+).
    pub fn usage_fraction(&self, used_tokens: usize) -> f64 {
        used_tokens as f64 / self.total as f64
    }

    /// Check if compaction should be triggered (>80% full).
    pub fn should_compact(&self, used_tokens: usize) -> bool {
        self.usage_fraction(used_tokens) > 0.8
    }
}

/// Manages the context window, tracking what's included.
#[derive(Debug)]
pub struct ContextManager {
    budget: ContextBudget,
    /// Total estimated tokens currently in context.
    current_tokens: usize,
}

impl ContextManager {
    pub fn new(params: &ModelParameters) -> Self {
        Self {
            budget: ContextBudget::new(params),
            current_tokens: 0,
        }
    }

    /// Get the context budget.
    pub fn budget(&self) -> &ContextBudget {
        &self.budget
    }

    /// Get current token usage.
    pub fn current_tokens(&self) -> usize {
        self.current_tokens
    }

    /// Update the token count after building a prompt.
    pub fn update_usage(&mut self, prompt: &str) {
        self.current_tokens = estimate_tokens(prompt);
    }

    /// Check if we should trigger compaction.
    pub fn should_compact(&self) -> bool {
        self.budget.should_compact(self.current_tokens)
    }

    /// Get available tokens for new content.
    pub fn available_tokens(&self) -> usize {
        self.budget.available()
    }

    /// Get a display string for context usage (e.g. "12.4k/32k").
    pub fn usage_display(&self) -> String {
        let used_k = self.current_tokens as f64 / 1000.0;
        let total_k = self.budget.total as f64 / 1000.0;
        format!("{used_k:.1}k/{total_k:.0}k")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars / 4 ≈ 3
                                                       // Code tends to have more special chars
        assert!(estimate_tokens("fn main() { println!(\"hello\"); }") > 5);
    }

    #[test]
    fn test_context_budget() {
        let params = ModelParameters {
            num_ctx: 32768,
            num_predict: 4096,
            ..ModelParameters::default()
        };
        let budget = ContextBudget::new(&params);
        assert_eq!(budget.total, 32768);
        // Available should be total minus all reserves
        let available = budget.available();
        assert!(available > 20000);
        assert!(available < 30000);
    }

    #[test]
    fn test_should_compact() {
        let params = ModelParameters::default();
        let budget = ContextBudget::new(&params);
        // Below 80% - no compaction
        assert!(!budget.should_compact(10000));
        // Above 80% - should compact
        assert!(budget.should_compact(28000));
    }

    #[test]
    fn test_usage_display() {
        let params = ModelParameters::default();
        let mut ctx = ContextManager::new(&params);
        ctx.current_tokens = 12400;
        let display = ctx.usage_display();
        assert!(display.contains("12.4k"));
    }
}
