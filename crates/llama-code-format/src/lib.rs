// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Llama-native prompt formatting and constrained decoding for Llama Code.
//!
//! This crate handles formatting prompts for Llama 3.x models with proper
//! special tokens, tool call formatting, and JSON repair for constrained decoding.

pub mod constrained;
pub mod generic;
pub mod llama3;
pub mod templates;

use serde::{Deserialize, Serialize};

/// A chat message with role and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// Message roles in the conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    /// Tool/ipython results
    Tool,
}

/// A tool definition for the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A parsed tool call from model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedToolCall {
    pub name: String,
    pub parameters: serde_json::Value,
}

/// The phase of the agent loop, affects system prompt selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Planning,
    Execution,
    Validation,
}

/// Trait for prompt formatters.
pub trait PromptFormatter: Send + Sync {
    /// Format a full conversation into a prompt string.
    fn format_prompt(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> String;

    /// Format a tool result to feed back to the model.
    fn format_tool_result(&self, result: &str) -> String;

    /// Parse tool calls from model output.
    fn parse_tool_calls(&self, output: &str) -> Vec<ParsedToolCall>;

    /// Get the stop tokens for this format.
    fn stop_tokens(&self) -> Vec<String>;

    /// Name of this formatter.
    fn name(&self) -> &str;
}
