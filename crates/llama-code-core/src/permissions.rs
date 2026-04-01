// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Permission system for dangerous operations.
//!
//! Three tiers: Auto-Approve, Confirm Once, Always Confirm.

use llama_code_tools::{bash::BashTool, git::GitTool, ToolCall};
use std::collections::HashSet;
use std::sync::Mutex;

/// Permission decision for a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    /// Automatically approved, no user interaction needed.
    AutoApprove,
    /// Needs confirmation once per session, then remembered.
    ConfirmOnce,
    /// Always needs confirmation.
    AlwaysConfirm,
}

/// Permission manager that tracks session-level approvals.
pub struct PermissionManager {
    yolo_mode: bool,
    /// Tools/commands that have been approved this session.
    session_approvals: Mutex<HashSet<String>>,
}

impl PermissionManager {
    pub fn new(yolo_mode: bool) -> Self {
        Self {
            yolo_mode,
            session_approvals: Mutex::new(HashSet::new()),
        }
    }

    /// Classify the permission level for a tool call.
    pub fn classify(&self, call: &ToolCall) -> Permission {
        match call.name.as_str() {
            // Always auto-approve: reads, search, think
            "file_read" | "grep" | "ls" | "think" => Permission::AutoApprove,

            // Bash: depends on the command
            "bash" => {
                let command = call
                    .parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if BashTool::is_always_dangerous(command) {
                    Permission::AlwaysConfirm
                } else if BashTool::is_allowlisted(command) || self.yolo_mode {
                    Permission::AutoApprove
                } else {
                    Permission::ConfirmOnce
                }
            }

            // Git: depends on subcommand
            "git" => {
                let subcommand = call
                    .parameters
                    .get("subcommand")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if GitTool::always_requires_confirmation(subcommand) {
                    Permission::AlwaysConfirm
                } else if GitTool::is_read_only(subcommand) || self.yolo_mode {
                    Permission::AutoApprove
                } else {
                    Permission::ConfirmOnce
                }
            }

            // File writes/edits
            "file_write" | "file_edit" => {
                if self.yolo_mode {
                    Permission::AutoApprove
                } else {
                    Permission::ConfirmOnce
                }
            }

            // Unknown tools: always confirm
            _ => Permission::AlwaysConfirm,
        }
    }

    /// Check if a tool call is approved (considering session memory).
    pub fn is_approved(&self, call: &ToolCall) -> bool {
        match self.classify(call) {
            Permission::AutoApprove => true,
            Permission::ConfirmOnce => {
                let key = self.session_key(call);
                let approvals = self.session_approvals.lock().unwrap();
                approvals.contains(&key)
            }
            Permission::AlwaysConfirm => false,
        }
    }

    /// Record that a tool call type was approved for this session.
    pub fn approve_for_session(&self, call: &ToolCall) {
        let key = self.session_key(call);
        let mut approvals = self.session_approvals.lock().unwrap();
        approvals.insert(key);
    }

    /// Generate a session key for a tool call (for confirm-once tracking).
    fn session_key(&self, call: &ToolCall) -> String {
        match call.name.as_str() {
            "bash" => {
                // Group by command prefix
                let cmd = call
                    .parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let prefix = cmd.split_whitespace().next().unwrap_or("");
                format!("bash:{prefix}")
            }
            "git" => {
                let sub = call
                    .parameters
                    .get("subcommand")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let prefix = sub.split_whitespace().next().unwrap_or("");
                format!("git:{prefix}")
            }
            other => other.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_call(name: &str, params: serde_json::Value) -> ToolCall {
        ToolCall {
            name: name.to_string(),
            parameters: params,
        }
    }

    #[test]
    fn test_auto_approve_reads() {
        let pm = PermissionManager::new(false);
        let call = make_call("file_read", serde_json::json!({"path": "test.rs"}));
        assert_eq!(pm.classify(&call), Permission::AutoApprove);
        assert!(pm.is_approved(&call));
    }

    #[test]
    fn test_confirm_once_file_write() {
        let pm = PermissionManager::new(false);
        let call = make_call(
            "file_write",
            serde_json::json!({"path": "test.rs", "content": ""}),
        );
        assert_eq!(pm.classify(&call), Permission::ConfirmOnce);
        assert!(!pm.is_approved(&call));

        pm.approve_for_session(&call);
        assert!(pm.is_approved(&call));
    }

    #[test]
    fn test_yolo_mode_approves_writes() {
        let pm = PermissionManager::new(true);
        let call = make_call(
            "file_write",
            serde_json::json!({"path": "test.rs", "content": ""}),
        );
        assert_eq!(pm.classify(&call), Permission::AutoApprove);
    }

    #[test]
    fn test_always_confirm_git_push() {
        let pm = PermissionManager::new(true); // even in yolo
        let call = make_call("git", serde_json::json!({"subcommand": "push origin main"}));
        assert_eq!(pm.classify(&call), Permission::AlwaysConfirm);
    }

    #[test]
    fn test_dangerous_bash_always_confirms() {
        let pm = PermissionManager::new(true); // even in yolo
        let call = make_call("bash", serde_json::json!({"command": "rm -rf /"}));
        assert_eq!(pm.classify(&call), Permission::AlwaysConfirm);
    }

    #[test]
    fn test_allowlisted_bash_auto_approves() {
        let pm = PermissionManager::new(false);
        let call = make_call("bash", serde_json::json!({"command": "ls -la"}));
        assert_eq!(pm.classify(&call), Permission::AutoApprove);
    }
}
