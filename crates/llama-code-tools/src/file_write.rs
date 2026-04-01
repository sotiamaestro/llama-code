// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! File writing tool with diff preview and directory creation.

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use similar::TextDiff;
use std::fs;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file at the given path. Parent directories are created \
         automatically. If the file exists, a diff is shown."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The full content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: path"),
        };

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing required parameter: content"),
        };

        // Security: check path is within CWD
        if !ctx.is_within_cwd(path) {
            return ToolResult::permission_denied(format!(
                "Cannot write to {path}: path is outside the current working directory"
            ));
        }

        let resolved = ctx.resolve_path(path);

        // Create parent directories
        if let Some(parent) = resolved.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::error(format!("Failed to create directories for {path}: {e}"));
            }
        }

        // Generate diff if file exists
        let diff_display = if resolved.exists() {
            match fs::read_to_string(&resolved) {
                Ok(old_content) => Some(generate_diff(&old_content, content, path)),
                Err(_) => None,
            }
        } else {
            None
        };

        // Write the file
        match fs::write(&resolved, content) {
            Ok(()) => {
                let msg = if diff_display.is_some() {
                    format!("✏️  Updated {path}")
                } else {
                    format!("✏️  Created {path}")
                };
                let display = diff_display.map(|d| format!("{msg}\n{d}"));
                if let Some(d) = display {
                    ToolResult::success_with_display(msg, d)
                } else {
                    let line_count = content.lines().count();
                    ToolResult::success(format!("{msg} ({line_count} lines)"))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to write {path}: {e}")),
        }
    }
}

/// Generate a unified diff between old and new content.
pub fn generate_diff(old: &str, new: &str, path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    output.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{hunk}"));
    }

    if output.ends_with('\n') {
        output.pop();
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_new_file() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({"path": "new.txt", "content": "hello world\n"}),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("Created"));
        assert_eq!(
            fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "hello world\n"
        );
    }

    #[tokio::test]
    async fn test_write_creates_directories() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({"path": "deep/nested/file.txt", "content": "test\n"}),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        assert!(dir.path().join("deep/nested/file.txt").exists());
    }

    #[tokio::test]
    async fn test_write_overwrite_shows_diff() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.txt"), "old content\n").unwrap();

        let tool = FileWriteTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({"path": "existing.txt", "content": "new content\n"}),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("Updated"));
    }

    #[test]
    fn test_generate_diff() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nmodified line 2\nline 3\n";
        let diff = generate_diff(old, new, "test.txt");
        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+modified line 2"));
    }
}
