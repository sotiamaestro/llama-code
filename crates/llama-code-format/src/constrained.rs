// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Constrained decoding and JSON repair for tool calls.
//!
//! When smaller models produce malformed JSON tool calls, this module
//! attempts to repair the output before giving up.

use crate::ParsedToolCall;
use regex::Regex;
use std::sync::LazyLock;

/// Attempt to repair malformed JSON from model output.
///
/// Common issues with smaller models:
/// - Trailing commas in objects/arrays
/// - Single quotes instead of double quotes
/// - Unquoted keys
/// - Missing closing braces
/// - Extra text before/after the JSON
pub fn repair_json(input: &str) -> Option<String> {
    let trimmed = input.trim();

    // If it parses as-is, return it
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }

    let mut repaired = trimmed.to_string();

    // Replace single quotes with double quotes (but not within strings)
    repaired = fix_single_quotes(&repaired);

    // Remove trailing commas before } or ]
    repaired = remove_trailing_commas(&repaired);

    // Try to close unclosed braces
    repaired = close_braces(&repaired);

    // Validate the result
    if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
        Some(repaired)
    } else {
        None
    }
}

/// Try to parse a potentially malformed tool call, with repair attempts.
pub fn parse_tool_call_with_repair(input: &str) -> Option<ParsedToolCall> {
    let trimmed = input.trim();

    // Direct parse attempt
    if let Some(call) = try_parse(trimmed) {
        return Some(call);
    }

    // Try repair
    if let Some(repaired) = repair_json(trimmed) {
        if let Some(call) = try_parse(&repaired) {
            return Some(call);
        }
    }

    // Try extracting JSON from surrounding text
    if let Some(json_str) = extract_json_object(trimmed) {
        if let Some(call) = try_parse(&json_str) {
            return Some(call);
        }
        if let Some(repaired) = repair_json(&json_str) {
            if let Some(call) = try_parse(&repaired) {
                return Some(call);
            }
        }
    }

    None
}

/// Build an error message for a malformed tool call to send back to the model.
pub fn tool_call_error_message(raw_output: &str, error: &str) -> String {
    format!(
        "Your previous tool call was malformed. Here is the error:\n\n\
         Error: {error}\n\n\
         Your output was:\n```\n{raw_output}\n```\n\n\
         Please try again with valid JSON in this exact format:\n\
         {{\"name\": \"tool_name\", \"parameters\": {{...}}}}"
    )
}

fn try_parse(json_str: &str) -> Option<ParsedToolCall> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let name = value.get("name")?.as_str()?.to_string();
    let parameters = value
        .get("parameters")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    Some(ParsedToolCall { name, parameters })
}

fn fix_single_quotes(input: &str) -> String {
    // Simple replacement - works for most cases
    // A more robust implementation would track string context
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'([^']*)'(\s*[,:\}\]])").unwrap());
    RE.replace_all(input, r#""$1"$2"#).to_string()
}

fn remove_trailing_commas(input: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r",\s*([}\]])").unwrap());
    RE.replace_all(input, "$1").to_string()
}

fn close_braces(input: &str) -> String {
    let mut open_braces = 0i32;
    let mut open_brackets = 0i32;

    for ch in input.chars() {
        match ch {
            '{' => open_braces += 1,
            '}' => open_braces -= 1,
            '[' => open_brackets += 1,
            ']' => open_brackets -= 1,
            _ => {}
        }
    }

    let mut result = input.to_string();
    for _ in 0..open_brackets.max(0) {
        result.push(']');
    }
    for _ in 0..open_braces.max(0) {
        result.push('}');
    }
    result
}

fn extract_json_object(input: &str) -> Option<String> {
    let start = input.find('{')?;
    let mut depth = 0;
    let mut end = None;

    for (i, ch) in input[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let slice = match end {
        Some(end_idx) => &input[start..=end_idx],
        None => &input[start..],
    };
    Some(slice.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_trailing_comma() {
        let input = r#"{"name": "file_read", "parameters": {"path": "test.rs",}}"#;
        let repaired = repair_json(input).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&repaired).is_ok());
    }

    #[test]
    fn test_repair_unclosed_brace() {
        let input = r#"{"name": "file_read", "parameters": {"path": "test.rs"}"#;
        let repaired = repair_json(input).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&repaired).is_ok());
    }

    #[test]
    fn test_parse_with_repair() {
        let input = r#"{"name": "bash", "parameters": {"command": "ls -la",}}"#;
        let call = parse_tool_call_with_repair(input).unwrap();
        assert_eq!(call.name, "bash");
        assert_eq!(call.parameters["command"], "ls -la");
    }

    #[test]
    fn test_extract_json_from_text() {
        let input = r#"Let me read that file: {"name": "file_read", "parameters": {"path": "main.rs"}} done."#;
        let call = parse_tool_call_with_repair(input).unwrap();
        assert_eq!(call.name, "file_read");
    }

    #[test]
    fn test_valid_json_passthrough() {
        let input = r#"{"name": "grep", "parameters": {"pattern": "TODO"}}"#;
        let repaired = repair_json(input).unwrap();
        assert_eq!(repaired, input);
    }

    #[test]
    fn test_completely_invalid() {
        let input = "this is not json at all";
        assert!(repair_json(input).is_none());
    }

    #[test]
    fn test_error_message() {
        let msg = tool_call_error_message("{bad json", "unexpected character");
        assert!(msg.contains("malformed"));
        assert!(msg.contains("{bad json"));
    }

    #[test]
    fn extract_json_object_handles_trailing_multibyte_char() {
        let input = r#"{"name": "bash", "parameters": {"command": "ls"}}🎉"#;
        let json = extract_json_object(input).expect("json object");
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
    }
}
