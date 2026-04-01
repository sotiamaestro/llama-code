// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Git operations wrapper tool.

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use tokio::process::Command;

/// Git subcommands that are read-only and auto-approved.
const READ_SUBCOMMANDS: &[&str] = &[
    "status",
    "diff",
    "log",
    "branch",
    "show",
    "remote",
    "tag",
    "stash list",
    "describe",
    "rev-parse",
    "config",
];

/// Git subcommands that always require confirmation (even in --yolo).
const ALWAYS_CONFIRM: &[&str] = &["push", "force-push", "push --force"];

pub struct GitTool;

impl GitTool {
    /// Check if a git subcommand is read-only.
    pub fn is_read_only(subcommand: &str) -> bool {
        READ_SUBCOMMANDS
            .iter()
            .any(|cmd| subcommand.starts_with(cmd))
    }

    /// Check if a git subcommand always requires confirmation.
    pub fn always_requires_confirmation(subcommand: &str) -> bool {
        ALWAYS_CONFIRM.iter().any(|cmd| subcommand.starts_with(cmd))
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Execute git operations. Read operations (status, diff, log) are auto-approved. \
         Write operations (commit, push) require confirmation."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "subcommand": {
                    "type": "string",
                    "description": "The git subcommand to run (e.g. 'status', 'diff', 'log --oneline -10')"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Additional arguments (optional, can also be part of subcommand string)"
                }
            },
            "required": ["subcommand"]
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let subcommand = match params.get("subcommand").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return ToolResult::error("Missing required parameter: subcommand"),
        };

        let extra_args: Vec<String> = params
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Build the full command
        let mut full_command = format!("git {subcommand}");
        for arg in &extra_args {
            full_command.push(' ');
            full_command.push_str(arg);
        }

        // Execute
        let output = Command::new("sh")
            .arg("-c")
            .arg(&full_command)
            .current_dir(&ctx.cwd)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                let mut result_text = String::new();
                result_text.push_str(&format!("$ {full_command}\n"));

                if !stdout.is_empty() {
                    result_text.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !stdout.is_empty() {
                        result_text.push('\n');
                    }
                    result_text.push_str(&stderr);
                }

                if stdout.is_empty() && stderr.is_empty() {
                    result_text.push_str("(no output)");
                }

                if exit_code == 0 {
                    ToolResult::success(result_text)
                } else {
                    ToolResult::error(format!("{result_text}\nExit code: {exit_code}"))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to execute git: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_detection() {
        assert!(GitTool::is_read_only("status"));
        assert!(GitTool::is_read_only("diff --cached"));
        assert!(GitTool::is_read_only("log --oneline -10"));
        assert!(!GitTool::is_read_only("commit -m 'test'"));
        assert!(!GitTool::is_read_only("push origin main"));
    }

    #[test]
    fn test_always_confirm() {
        assert!(GitTool::always_requires_confirmation("push"));
        assert!(GitTool::always_requires_confirmation("push origin main"));
        assert!(!GitTool::always_requires_confirmation("commit"));
        assert!(!GitTool::always_requires_confirmation("add ."));
    }

    #[tokio::test]
    async fn test_git_in_non_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        let tool = GitTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(serde_json::json!({"subcommand": "status"}), &ctx)
            .await;

        // Should fail because it's not a git repo
        assert!(!result.is_success());
    }
}
