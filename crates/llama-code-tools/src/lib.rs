// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Built-in tool implementations for Llama Code.
//!
//! Each tool follows the `Tool` trait interface, providing a name, description,
//! JSON schema for parameters, and an async execute method.

pub mod bash;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod git;
pub mod grep;
pub mod ls;
pub mod registry;
pub mod think;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub status: ToolStatus,
    pub content: String,
    /// Pretty-printed version for the TUI (optional, falls back to content).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// Status of a tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStatus {
    Success,
    Error(String),
    PermissionDenied(String),
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            status: ToolStatus::Success,
            content: content.into(),
            display: None,
        }
    }

    pub fn success_with_display(content: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            status: ToolStatus::Success,
            content: content.into(),
            display: Some(display.into()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            status: ToolStatus::Error(msg.clone()),
            content: msg,
            display: None,
        }
    }

    pub fn permission_denied(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            status: ToolStatus::PermissionDenied(msg.clone()),
            content: msg,
            display: None,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, ToolStatus::Success)
    }

    /// Get the display string (falls back to content).
    pub fn display_text(&self) -> &str {
        self.display.as_deref().unwrap_or(&self.content)
    }
}

/// Context provided to tool executions.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current working directory.
    pub cwd: PathBuf,
    /// Whether --yolo mode is enabled (skip confirmations).
    pub yolo_mode: bool,
}

impl ToolContext {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            yolo_mode: false,
        }
    }

    /// Resolve a path relative to the CWD.
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.cwd.join(p)
        }
    }

    /// Check if a path is within the CWD.
    pub fn is_within_cwd(&self, path: &str) -> bool {
        let resolved = self.resolve_path(path);
        match (resolved.canonicalize(), self.cwd.canonicalize()) {
            (Ok(resolved), Ok(cwd)) => resolved.starts_with(&cwd),
            // If paths don't exist yet, check prefix
            _ => {
                let resolved = self.resolve_path(path);
                // Check for path traversal attempts
                !path.contains("..") || resolved.starts_with(&self.cwd)
            }
        }
    }
}

/// A tool call parsed from model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub parameters: serde_json::Value,
}

/// Trait that all tools implement.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The tool's name (used in tool calls).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given parameters.
    async fn execute(&self, params: serde_json::Value, ctx: &ToolContext) -> ToolResult;
}
