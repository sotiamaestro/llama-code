// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Generic ChatML format as a fallback for non-Llama models.
//!
//! This formatter uses the standard ChatML format:
//! ```text
//! <|im_start|>system
//! {content}<|im_end|>
//! <|im_start|>user
//! {content}<|im_end|>
//! ```

use crate::{ChatMessage, ParsedToolCall, PromptFormatter, Role, ToolDefinition};

const IM_START: &str = "<|im_start|>";
const IM_END: &str = "<|im_end|>";

/// Generic ChatML prompt formatter for non-Llama models.
pub struct GenericFormatter;

impl GenericFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenericFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptFormatter for GenericFormatter {
    fn format_prompt(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> String {
        let mut prompt = String::with_capacity(8192);

        for msg in messages {
            let role_str = match msg.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };

            prompt.push_str(IM_START);
            prompt.push_str(role_str);
            prompt.push('\n');

            // Inject tool definitions in system message
            if msg.role == Role::System && !tools.is_empty() {
                prompt.push_str("You have access to the following tools:\n\n");
                for tool in tools {
                    prompt.push_str(&format!(
                        "## {}\n{}\nParameters: {}\n\n",
                        tool.name,
                        tool.description,
                        serde_json::to_string_pretty(&tool.parameters).unwrap_or_default()
                    ));
                }
                prompt.push_str(
                    "To call a tool, respond with a JSON object: \
                     {\"name\": \"tool_name\", \"parameters\": {...}}\n\n",
                );
            }

            prompt.push_str(&msg.content);
            prompt.push_str(IM_END);
            prompt.push('\n');
        }

        // Prompt for assistant generation
        prompt.push_str(IM_START);
        prompt.push_str("assistant\n");

        prompt
    }

    fn format_tool_result(&self, result: &str) -> String {
        format!("{IM_START}tool\n{result}{IM_END}\n")
    }

    fn parse_tool_calls(&self, output: &str) -> Vec<ParsedToolCall> {
        // Same JSON extraction logic as Llama3 - look for JSON objects with name/parameters
        let mut calls = Vec::new();
        let trimmed = output.trim();

        // Try direct parse first
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(call) = extract_tool_call(&value) {
                calls.push(call);
                return calls;
            }
        }

        // Search for JSON objects in the text
        let mut depth = 0i32;
        let mut start = None;
        for (i, ch) in trimmed.char_indices() {
            match ch {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(s) = start {
                            let json_str = &trimmed[s..=i];
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str)
                            {
                                if let Some(call) = extract_tool_call(&value) {
                                    calls.push(call);
                                }
                            }
                        }
                        start = None;
                    }
                }
                _ => {}
            }
        }

        calls
    }

    fn stop_tokens(&self) -> Vec<String> {
        vec![IM_END.to_string()]
    }

    fn name(&self) -> &str {
        "generic-chatml"
    }
}

fn extract_tool_call(value: &serde_json::Value) -> Option<ParsedToolCall> {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;

    let parameters = value
        .get("parameters")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    Some(ParsedToolCall { name, parameters })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic() {
        let fmt = GenericFormatter::new();
        let messages = vec![
            ChatMessage {
                role: Role::System,
                content: "You help with code.".to_string(),
            },
            ChatMessage {
                role: Role::User,
                content: "Fix the bug.".to_string(),
            },
        ];

        let prompt = fmt.format_prompt(&messages, &[]);
        assert!(prompt.contains("<|im_start|>system"));
        assert!(prompt.contains("<|im_start|>user"));
        assert!(prompt.contains("Fix the bug."));
    }

    #[test]
    fn test_parse_chatml_tool_call() {
        let fmt = GenericFormatter::new();
        let output = r#"{"name": "bash", "parameters": {"command": "ls -la"}}"#;
        let calls = fmt.parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
    }
}
