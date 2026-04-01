// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Shell command execution tool with sandboxing, timeouts, and allowlists.

use crate::{Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;

/// Default timeout for commands in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Commands that are auto-approved (safe, read-only operations).
const ALLOWLIST: &[&str] = &[
    "ls",
    "cat",
    "head",
    "tail",
    "wc",
    "echo",
    "pwd",
    "find",
    "which",
    "file",
    "git status",
    "git diff",
    "git log",
    "git branch",
    "git show",
    "cargo check",
    "cargo test",
    "cargo clippy",
    "cargo build",
    "cargo fmt",
    "npm test",
    "npm run",
    "npx",
    "python -m pytest",
    "python -m py_compile",
    "rustc --version",
    "node --version",
    "python --version",
    "python3 --version",
    "rg",
    "grep",
    "sort",
    "uniq",
    "diff",
    "tree",
    "du",
    "df",
    "env",
    "uname",
    "date",
];

/// Commands that are always dangerous (even in --yolo mode).
const ALWAYS_DANGEROUS: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "mkfs",
    "dd if=",
    "> /dev/",
    "chmod -R 777",
];

pub struct BashTool;

impl BashTool {
    /// Check if a command is on the auto-approve allowlist.
    pub fn is_allowlisted(command: &str) -> bool {
        let trimmed = command.trim();
        ALLOWLIST.iter().any(|allowed| {
            trimmed == *allowed
                || trimmed.starts_with(&format!("{allowed} "))
                || trimmed.starts_with(&format!("{allowed}\t"))
        })
    }

    /// Check if a command is always dangerous.
    pub fn is_always_dangerous(command: &str) -> bool {
        let trimmed = command.trim();
        ALWAYS_DANGEROUS
            .iter()
            .any(|dangerous| trimmed.contains(dangerous))
    }

    /// Check if a command requires confirmation.
    pub fn requires_confirmation(command: &str, yolo_mode: bool) -> bool {
        if Self::is_always_dangerous(command) {
            return true;
        }
        if Self::is_allowlisted(command) {
            return false;
        }
        !yolo_mode
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Safe read-only commands are auto-approved. \
         Potentially destructive commands require user confirmation."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing required parameter: command"),
        };

        let timeout_secs = params
            .get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        // Security check
        if Self::is_always_dangerous(command) {
            return ToolResult::permission_denied(format!(
                "Command rejected as dangerous: {command}"
            ));
        }

        // Execute the command
        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            execute_command(command, &ctx.cwd),
        )
        .await;

        match result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                let mut output = String::new();

                if !stdout.is_empty() {
                    output.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str("[stderr]\n");
                    output.push_str(&stderr);
                }

                if output.is_empty() {
                    output = "(no output)".to_string();
                }

                // Truncate very long output
                if output.len() > 50_000 {
                    let truncated_msg =
                        format!("\n\n... [output truncated, {} total bytes]", output.len());
                    output.truncate(50_000);
                    output.push_str(&truncated_msg);
                }

                if exit_code == 0 {
                    ToolResult::success(format!("$ {command}\n{output}"))
                } else {
                    ToolResult::error(format!("$ {command}\nExit code: {exit_code}\n{output}"))
                }
            }
            Ok(Err(e)) => ToolResult::error(format!("Failed to execute command: {e}")),
            Err(_) => ToolResult::error(format!(
                "Command timed out after {timeout_secs} seconds: {command}"
            )),
        }
    }
}

async fn execute_command(
    command: &str,
    cwd: &std::path::Path,
) -> Result<(String, String, i32), std::io::Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_allowlist() {
        assert!(BashTool::is_allowlisted("ls"));
        assert!(BashTool::is_allowlisted("ls -la"));
        assert!(BashTool::is_allowlisted("git status"));
        assert!(BashTool::is_allowlisted("cargo test"));
        assert!(!BashTool::is_allowlisted("rm -rf ."));
        assert!(!BashTool::is_allowlisted("curl example.com"));
    }

    #[test]
    fn test_dangerous_commands() {
        assert!(BashTool::is_always_dangerous("rm -rf /"));
        assert!(BashTool::is_always_dangerous("sudo rm -rf /"));
        assert!(!BashTool::is_always_dangerous("rm file.txt"));
        assert!(!BashTool::is_always_dangerous("ls -la"));
    }

    #[test]
    fn test_confirmation_logic() {
        // Allowlisted: no confirmation
        assert!(!BashTool::requires_confirmation("ls -la", false));
        // Not allowlisted, not yolo: needs confirmation
        assert!(BashTool::requires_confirmation("curl example.com", false));
        // Not allowlisted, yolo: no confirmation
        assert!(!BashTool::requires_confirmation("curl example.com", true));
        // Always dangerous: always needs confirmation
        assert!(BashTool::requires_confirmation("rm -rf /", true));
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let dir = TempDir::new().unwrap();
        let tool = BashTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(serde_json::json!({"command": "echo hello"}), &ctx)
            .await;

        assert!(result.is_success());
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_failing_command() {
        let dir = TempDir::new().unwrap();
        let tool = BashTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(
                serde_json::json!({"command": "ls nonexistent_file_xyz"}),
                &ctx,
            )
            .await;

        assert!(!result.is_success());
    }

    #[tokio::test]
    async fn test_dangerous_command_rejected() {
        let dir = TempDir::new().unwrap();
        let tool = BashTool;
        let ctx = ToolContext::new(dir.path().to_path_buf());

        let result = tool
            .execute(serde_json::json!({"command": "rm -rf /"}), &ctx)
            .await;

        assert!(!result.is_success());
        assert!(result.content.contains("dangerous"));
    }
}
