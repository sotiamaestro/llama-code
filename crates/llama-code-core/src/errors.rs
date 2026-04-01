// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Error types for Llama Code core.

use thiserror::Error;

/// Core error types.
#[derive(Error, Debug)]
pub enum LlamaError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("Ollama connection failed: {0}")]
    OllamaConnection(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool call parse error: {0}")]
    ToolCallParse(String),

    #[error("Context overflow: used {used} tokens, limit is {limit}")]
    ContextOverflow { used: usize, limit: usize },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Max iterations reached ({0})")]
    MaxIterations(usize),

    #[error("Session error: {0}")]
    Session(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("User cancelled")]
    UserCancelled,

    #[error("{0}")]
    Other(String),
}

/// Result type alias for Llama Code.
pub type Result<T> = std::result::Result<T, LlamaError>;
