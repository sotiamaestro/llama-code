// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! File reading tool with smart truncation and binary detection.

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use std::fs;

/// Maximum lines to show without a range specified.
const MAX_LINES_DEFAULT: usize = 500;
/// Lines to show from the beginning when truncating.
const HEAD_LINES: usize = 100;
/// Lines to show from the end when truncating.
const TAIL_LINES: usize = 50;
/// Bytes to check for binary detection.
const BINARY_CHECK_SIZE: usize = 8192;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path. Returns line-numbered content. \
         For large files, use line_range to read specific sections."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative or absolute path to the file"
                },
                "line_range": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "Optional [start, end] line range (1-indexed, inclusive)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: path"),
        };

        let resolved = ctx.resolve_path(path);

        if !resolved.exists() {
            return ToolResult::error(format!("File not found: {path}"));
        }

        if !resolved.is_file() {
            return ToolResult::error(format!("Not a file: {path}"));
        }

        // Check for binary file
        if is_binary(&resolved) {
            return ToolResult::error(format!(
                "Binary file detected: {path}. Cannot display binary content."
            ));
        }

        let content = match fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Failed to read {path}: {e}")),
        };

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Parse line range
        let (start, end) = if let Some(range) = params.get("line_range").and_then(|v| v.as_array())
        {
            let start = range
                .first()
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(1)
                .max(1);
            let end = range
                .get(1)
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(total_lines)
                .min(total_lines);
            (start, end)
        } else {
            (1, total_lines)
        };

        // Format with line numbers
        if total_lines <= MAX_LINES_DEFAULT || params.get("line_range").is_some() {
            let numbered: Vec<String> = lines
                .iter()
                .enumerate()
                .filter(|(i, _)| *i + 1 >= start && *i + 1 <= end)
                .map(|(i, line)| format!("{:>4} | {}", i + 1, line))
                .collect();

            let header = format!("📄 {path} ({total_lines} lines)\n");
            ToolResult::success(format!("{header}{}", numbered.join("\n")))
        } else {
            // Smart truncation
            let head: Vec<String> = lines
                .iter()
                .enumerate()
                .take(HEAD_LINES)
                .map(|(i, line)| format!("{:>4} | {}", i + 1, line))
                .collect();

            let tail: Vec<String> = lines
                .iter()
                .enumerate()
                .skip(total_lines - TAIL_LINES)
                .map(|(i, line)| format!("{:>4} | {}", i + 1, line))
                .collect();

            let truncated = total_lines - HEAD_LINES - TAIL_LINES;
            let header = format!("📄 {path} ({total_lines} lines)\n");
            ToolResult::success(format!(
                "{header}{}\n     | ... [{truncated} lines truncated] ...\n{}",
                head.join("\n"),
                tail.join("\n")
            ))
        }
    }
}

/// Check if a file appears to be binary by looking for null bytes.
fn is_binary(path: &std::path::Path) -> bool {
    match fs::read(path) {
        Ok(bytes) => {
            let check_len = bytes.len().min(BINARY_CHECK_SIZE);
            bytes[..check_len].contains(&0)
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let tool = FileReadTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let result = tool
            .execute(serde_json::json!({"path": "test.txt"}), &ctx)
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("line 1"));
        assert!(result.content.contains("line 2"));
    }

    #[tokio::test]
    async fn test_read_with_range() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "a\nb\nc\nd\ne\n").unwrap();

        let tool = FileReadTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let result = tool
            .execute(
                serde_json::json!({"path": "test.txt", "line_range": [2, 4]}),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("b"));
        assert!(result.content.contains("d"));
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let dir = TempDir::new().unwrap();
        let tool = FileReadTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let result = tool
            .execute(serde_json::json!({"path": "missing.txt"}), &ctx)
            .await;

        assert!(!result.is_success());
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_read_binary() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("binary.bin");
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(&[0u8, 1, 2, 3, 0, 255]).unwrap();

        let tool = FileReadTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let result = tool
            .execute(serde_json::json!({"path": "binary.bin"}), &ctx)
            .await;

        assert!(!result.is_success());
        assert!(result.content.contains("Binary"));
    }
}
