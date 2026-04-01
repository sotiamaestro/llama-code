// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Surgical file editing tool using string replacement (str_replace style).

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use std::fs;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Make a surgical edit to a file by replacing an exact string match. \
         The old_text must match exactly one location in the file. \
         Use this for targeted changes instead of rewriting entire files."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "The exact text to find and replace (must match exactly once)"
                },
                "new_text": {
                    "type": "string",
                    "description": "The replacement text"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: path"),
        };

        let old_text = match params.get("old_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ToolResult::error("Missing required parameter: old_text"),
        };

        let new_text = match params.get("new_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ToolResult::error("Missing required parameter: new_text"),
        };

        if !ctx.is_within_cwd(path) {
            return ToolResult::permission_denied(format!(
                "Cannot edit {path}: path is outside the current working directory"
            ));
        }

        let resolved = ctx.resolve_path(path);

        if !resolved.exists() {
            return ToolResult::error(format!("File not found: {path}"));
        }

        let content = match fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Failed to read {path}: {e}")),
        };

        // Count occurrences
        let count = content.matches(old_text).count();

        match count {
            0 => ToolResult::error(format!(
                "Text not found in {path}. The old_text does not match any content in the file. \
                 Make sure the text matches exactly, including whitespace and indentation."
            )),
            1 => {
                let new_content = content.replacen(old_text, new_text, 1);

                // Generate a diff for display
                let diff = crate::file_write::generate_diff(&content, &new_content, path);

                match fs::write(&resolved, &new_content) {
                    Ok(()) => ToolResult::success_with_display(
                        format!("✏️  Edited {path}"),
                        format!("✏️  Edited {path}\n{diff}"),
                    ),
                    Err(e) => ToolResult::error(format!("Failed to write {path}: {e}")),
                }
            }
            n => ToolResult::error(format!(
                "Ambiguous match in {path}: found {n} occurrences of the search text. \
                 Provide more surrounding context in old_text to uniquely identify the location."
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_edit_single_match() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let tool = FileEditTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test.rs",
                    "old_text": "println!(\"hello\")",
                    "new_text": "println!(\"world\")"
                }),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        let content = fs::read_to_string(dir.path().join("test.rs")).unwrap();
        assert!(content.contains("world"));
        assert!(!content.contains("hello"));
    }

    #[tokio::test]
    async fn test_edit_no_match() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

        let tool = FileEditTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test.rs",
                    "old_text": "nonexistent text",
                    "new_text": "replacement"
                }),
                &ctx,
            )
            .await;

        assert!(!result.is_success());
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_edit_ambiguous_match() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.rs"),
            "let x = 1;\nlet y = 1;\nlet z = 1;\n",
        )
        .unwrap();

        let tool = FileEditTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test.rs",
                    "old_text": " = 1;",
                    "new_text": " = 2;"
                }),
                &ctx,
            )
            .await;

        assert!(!result.is_success());
        assert!(result.content.contains("Ambiguous"));
        assert!(result.content.contains("3 occurrences"));
    }

    #[tokio::test]
    async fn test_edit_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let tool = FileEditTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "missing.rs",
                    "old_text": "old",
                    "new_text": "new"
                }),
                &ctx,
            )
            .await;

        assert!(!result.is_success());
        assert!(result.content.contains("not found"));
    }
}
