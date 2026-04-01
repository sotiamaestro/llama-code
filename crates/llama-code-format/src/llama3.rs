// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Llama 3.x native chat template and tool calling format.
//!
//! Uses the official Llama 3.1+ format with `<|python_tag|>` for tool calls
//! and `ipython` role for tool results.

use crate::{ChatMessage, ParsedToolCall, PromptFormatter, Role, ToolDefinition};

/// Special tokens for Llama 3.x format.
pub const BEGIN_OF_TEXT: &str = "<|begin_of_text|>";
pub const END_OF_TEXT: &str = "<|end_of_text|>";
pub const START_HEADER: &str = "<|start_header_id|>";
pub const END_HEADER: &str = "<|end_header_id|>";
pub const EOT: &str = "<|eot_id|>";
pub const PYTHON_TAG: &str = "<|python_tag|>";

/// Llama 3.x prompt formatter.
pub struct Llama3Formatter;

impl Llama3Formatter {
    pub fn new() -> Self {
        Self
    }

    /// Format tool definitions for the system prompt.
    fn format_tool_definitions(tools: &[ToolDefinition]) -> String {
        let tool_defs: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();

        serde_json::to_string_pretty(&tool_defs).unwrap_or_default()
    }

    /// Format a single message with Llama 3.x header tokens.
    fn format_message(role: &str, content: &str) -> String {
        format!("{START_HEADER}{role}{END_HEADER}\n\n{content}{EOT}")
    }
}

impl Default for Llama3Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptFormatter for Llama3Formatter {
    fn format_prompt(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> String {
        let mut prompt = String::with_capacity(8192);
        prompt.push_str(BEGIN_OF_TEXT);

        for msg in messages {
            match msg.role {
                Role::System => {
                    prompt.push_str(START_HEADER);
                    prompt.push_str("system");
                    prompt.push_str(END_HEADER);
                    prompt.push_str("\n\n");

                    // Add environment and tool information to system message
                    if !tools.is_empty() {
                        prompt.push_str("Environment: ipython\n");
                        prompt.push_str("Tools: ");
                        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
                        prompt.push_str(&tool_names.join(", "));
                        prompt.push_str("\n\n");
                        prompt.push_str("Available tool definitions:\n");
                        prompt.push_str(&Self::format_tool_definitions(tools));
                        prompt.push_str("\n\n");
                    }

                    prompt.push_str(&msg.content);
                    prompt.push_str(EOT);
                }
                Role::User => {
                    prompt.push_str(&Self::format_message("user", &msg.content));
                }
                Role::Assistant => {
                    prompt.push_str(&Self::format_message("assistant", &msg.content));
                }
                Role::Tool => {
                    prompt.push_str(&Self::format_message("ipython", &msg.content));
                }
            }
        }

        // Add the assistant header to prompt the model to generate
        prompt.push_str(START_HEADER);
        prompt.push_str("assistant");
        prompt.push_str(END_HEADER);
        prompt.push_str("\n\n");

        prompt
    }

    fn format_tool_result(&self, result: &str) -> String {
        Self::format_message("ipython", result)
    }

    fn parse_tool_calls(&self, output: &str) -> Vec<ParsedToolCall> {
        let mut calls = Vec::new();

        // Look for <|python_tag|> followed by JSON
        // Also try to parse JSON objects directly (some models skip the tag)
        let search_text = if let Some(idx) = output.find(PYTHON_TAG) {
            &output[idx + PYTHON_TAG.len()..]
        } else {
            output
        };

        // Try to find JSON objects in the output
        let trimmed = search_text.trim();

        // Try single tool call first
        if let Some(call) = try_parse_tool_call(trimmed) {
            calls.push(call);
            return calls;
        }

        // Try to find JSON in the text by looking for { ... } patterns
        let mut depth = 0;
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
                            if let Some(call) = try_parse_tool_call(json_str) {
                                calls.push(call);
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
        vec![EOT.to_string(), END_OF_TEXT.to_string()]
    }

    fn name(&self) -> &str {
        "llama3"
    }
}

/// Try to parse a JSON string as a tool call.
fn try_parse_tool_call(json_str: &str) -> Option<ParsedToolCall> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

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
    fn test_format_basic_conversation() {
        let formatter = Llama3Formatter::new();
        let messages = vec![
            ChatMessage {
                role: Role::System,
                content: "You are a helpful assistant.".to_string(),
            },
            ChatMessage {
                role: Role::User,
                content: "Hello!".to_string(),
            },
        ];

        let prompt = formatter.format_prompt(&messages, &[]);

        assert!(prompt.starts_with(BEGIN_OF_TEXT));
        assert!(prompt.contains("system"));
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("user"));
        assert!(prompt.contains("Hello!"));
        assert!(prompt.ends_with("\n\n")); // Ready for assistant to generate
    }

    #[test]
    fn test_format_with_tools() {
        let formatter = Llama3Formatter::new();
        let messages = vec![ChatMessage {
            role: Role::System,
            content: "You are Llama Code.".to_string(),
        }];
        let tools = vec![ToolDefinition {
            name: "file_read".to_string(),
            description: "Read a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        }];

        let prompt = formatter.format_prompt(&messages, &tools);

        assert!(prompt.contains("Environment: ipython"));
        assert!(prompt.contains("Tools: file_read"));
        assert!(prompt.contains("file_read"));
    }

    #[test]
    fn test_parse_tool_call_with_python_tag() {
        let formatter = Llama3Formatter::new();
        let output =
            r#"<|python_tag|>{"name": "file_read", "parameters": {"path": "src/main.rs"}}"#;

        let calls = formatter.parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "file_read");
        assert_eq!(calls[0].parameters["path"], "src/main.rs");
    }

    #[test]
    fn test_parse_tool_call_without_tag() {
        let formatter = Llama3Formatter::new();
        let output = r#"{"name": "file_read", "parameters": {"path": "src/main.rs"}}"#;

        let calls = formatter.parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "file_read");
    }

    #[test]
    fn test_parse_tool_call_with_surrounding_text() {
        let formatter = Llama3Formatter::new();
        let output = r#"I'll read the file now.
{"name": "file_read", "parameters": {"path": "src/main.rs"}}
"#;

        let calls = formatter.parse_tool_calls(output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "file_read");
    }

    #[test]
    fn test_parse_no_tool_calls() {
        let formatter = Llama3Formatter::new();
        let output = "I'll just explain the code to you.";

        let calls = formatter.parse_tool_calls(output);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_tool_result_format() {
        let formatter = Llama3Formatter::new();
        let result = formatter.format_tool_result(r#"{"status": "success", "content": "hello"}"#);
        assert!(result.contains("ipython"));
        assert!(result.contains("success"));
    }
}
