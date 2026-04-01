// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Event bus for communication between components.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Events emitted during the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Agent state changed.
    StateChanged(String),

    /// New token streamed from the model.
    TokenReceived(String),

    /// Model finished generating.
    GenerationComplete,

    /// Tool call detected in model output.
    ToolCallDetected {
        tool_name: String,
        parameters: serde_json::Value,
    },

    /// Tool execution started.
    ToolExecutionStarted { tool_name: String },

    /// Tool execution completed.
    ToolExecutionCompleted {
        tool_name: String,
        success: bool,
        content: String,
    },

    /// Context compaction triggered.
    ContextCompacted {
        old_tokens: usize,
        new_tokens: usize,
    },

    /// Model switched (escalation or manual).
    ModelSwitched {
        from: String,
        to: String,
        reason: String,
    },

    /// Error occurred.
    Error(String),

    /// Agent turn completed.
    TurnComplete,
}

/// Event bus using broadcast channels.
pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }

    /// Send an event.
    pub fn emit(&self, event: AgentEvent) {
        // Ignore errors (no receivers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
