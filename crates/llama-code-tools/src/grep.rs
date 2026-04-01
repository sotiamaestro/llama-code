// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Ripgrep-powered codebase search tool.

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use tokio::process::Command;

/// Default maximum results to return.
const DEFAULT_MAX_RESULTS: usize = 50;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a pattern in files using ripgrep. Respects .gitignore. \
         Returns matching lines with file paths and line numbers."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: current directory)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs', '*.py')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 50)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let pattern = match params.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: pattern"),
        };

        let search_path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);

        let include = params.get("include").and_then(|v| v.as_str());

        let resolved_path = ctx.resolve_path(search_path);

        // Try ripgrep first, fall back to grep
        let result = if is_rg_available().await {
            run_rg(pattern, &resolved_path, include, max_results).await
        } else {
            run_grep_fallback(pattern, &resolved_path, include, max_results).await
        };

        match result {
            Ok(output) => {
                if output.is_empty() {
                    ToolResult::success(format!("No matches found for pattern: {pattern}"))
                } else {
                    let lines: Vec<&str> = output.lines().collect();
                    let total = lines.len();
                    let display_lines: Vec<&str> = lines.into_iter().take(max_results).collect();
                    let mut content = display_lines.join("\n");
                    if total > max_results {
                        content.push_str(&format!(
                            "\n\n... [{} more results truncated]",
                            total - max_results
                        ));
                    }
                    ToolResult::success(format!(
                        "🔍 Found {total} matches for '{pattern}':\n{content}"
                    ))
                }
            }
            Err(e) => ToolResult::error(format!("Search failed: {e}")),
        }
    }
}

async fn is_rg_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn run_rg(
    pattern: &str,
    path: &std::path::Path,
    include: Option<&str>,
    max_results: usize,
) -> Result<String, String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--no-heading")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count")
        .arg(max_results.to_string());

    if let Some(glob) = include {
        cmd.arg("--glob").arg(glob);
    }

    cmd.arg(pattern).arg(path);

    let output = cmd.output().await.map_err(|e| e.to_string())?;

    // rg exit code 1 means no matches (not an error)
    if output.status.code() == Some(2) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ripgrep error: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn run_grep_fallback(
    pattern: &str,
    path: &std::path::Path,
    include: Option<&str>,
    max_results: usize,
) -> Result<String, String> {
    let mut cmd = Command::new("grep");
    cmd.arg("-rn").arg("--color=never");

    if let Some(glob) = include {
        cmd.arg("--include").arg(glob);
    }

    cmd.arg(pattern).arg(path);

    let output = cmd.output().await.map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().take(max_results).collect();
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_grep_finds_pattern() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.rs"),
            "fn main() {\n    // TODO: fix this\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let tool = GrepTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(serde_json::json!({"pattern": "TODO"}), &ctx)
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("TODO"));
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

        let tool = GrepTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({"pattern": "NONEXISTENT_PATTERN_XYZ"}),
                &ctx,
            )
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("No matches"));
    }
}
