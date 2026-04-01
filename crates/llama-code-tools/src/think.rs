// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Extended thinking / scratchpad tool.
//!
//! The model uses this to work through complex problems before acting.
//! The thought content is NOT shown to the user (only in debug mode).

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;

pub struct ThinkTool;

#[async_trait]
impl Tool for ThinkTool {
    fn name(&self) -> &str {
        "think"
    }

    fn description(&self) -> &str {
        "Use this tool to think through complex problems step by step before acting. \
         Write out your reasoning, analysis, and plan. The output is used internally \
         and not shown to the user."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your reasoning and analysis"
                }
            },
            "required": ["thought"]
        })
    }

    async fn execute(&self, params: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        let thought = match params.get("thought").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ToolResult::error("Missing required parameter: thought"),
        };

        tracing::debug!(thought = thought, "Agent thinking");

        // Return the thought as content (for the model's context) but no display
        // The TUI will show a brief indicator like "🤔 Thinking..." instead
        ToolResult::success_with_display(thought.to_string(), "🤔 Thinking...".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_think() {
        let tool = ThinkTool;
        let ctx = ToolContext::new(std::env::current_dir().unwrap());

        let result = tool
            .execute(
                serde_json::json!({
                    "thought": "I need to read the file first, then understand the structure."
                }),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("read the file"));
        // Display should be minimal
        assert_eq!(result.display.as_deref(), Some("🤔 Thinking..."));
    }
}
