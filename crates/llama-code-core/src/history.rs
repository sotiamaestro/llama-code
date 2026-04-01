// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Conversation history and compaction.

use crate::context::estimate_tokens;
use crate::model::OllamaMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single exchange in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub timestamp: DateTime<Utc>,
    pub user_input: String,
    pub assistant_response: String,
    pub tool_calls: Vec<ToolCallRecord>,
}

/// Record of a tool call and its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: String,
    pub success: bool,
}

/// Conversation history manager.
#[derive(Debug)]
pub struct History {
    exchanges: Vec<Exchange>,
    /// Compacted summary of older exchanges.
    compacted_summary: Option<String>,
}

impl History {
    pub fn new() -> Self {
        Self {
            exchanges: Vec::new(),
            compacted_summary: None,
        }
    }

    /// Add a completed exchange.
    pub fn push(&mut self, exchange: Exchange) {
        self.exchanges.push(exchange);
    }

    /// Get all exchanges.
    pub fn exchanges(&self) -> &[Exchange] {
        &self.exchanges
    }

    /// Get the most recent N exchanges.
    pub fn recent(&self, n: usize) -> &[Exchange] {
        let start = self.exchanges.len().saturating_sub(n);
        &self.exchanges[start..]
    }

    /// Get the compacted summary if available.
    pub fn compacted_summary(&self) -> Option<&str> {
        self.compacted_summary.as_deref()
    }

    /// Estimate total tokens used by history.
    pub fn estimate_tokens(&self) -> usize {
        let mut total = 0;

        if let Some(summary) = &self.compacted_summary {
            total += estimate_tokens(summary);
        }

        for exchange in &self.exchanges {
            total += estimate_tokens(&exchange.user_input);
            total += estimate_tokens(&exchange.assistant_response);
            for tc in &exchange.tool_calls {
                total += estimate_tokens(&tc.result);
            }
        }

        total
    }

    /// Convert history to Ollama messages.
    pub fn to_messages(&self) -> Vec<OllamaMessage> {
        let mut messages = Vec::new();

        for exchange in &self.exchanges {
            messages.push(OllamaMessage {
                role: "user".to_string(),
                content: exchange.user_input.clone(),
            });

            // Include tool call results as context
            let mut assistant_content = String::new();
            for tc in &exchange.tool_calls {
                assistant_content.push_str(&format!(
                    "[Tool: {} → {}]\n",
                    tc.tool_name,
                    if tc.success { "success" } else { "error" }
                ));
            }
            assistant_content.push_str(&exchange.assistant_response);

            messages.push(OllamaMessage {
                role: "assistant".to_string(),
                content: assistant_content,
            });
        }

        messages
    }

    /// Compact older history into a summary, keeping recent exchanges.
    ///
    /// This is a simple version - a more sophisticated implementation
    /// would use the LLM to generate summaries.
    pub fn compact(&mut self, keep_recent: usize) {
        if self.exchanges.len() <= keep_recent {
            return;
        }

        let split_at = self.exchanges.len() - keep_recent;
        let old_exchanges: Vec<Exchange> = self.exchanges.drain(..split_at).collect();

        // Build summary from old exchanges
        let mut summary = String::new();
        if let Some(existing) = &self.compacted_summary {
            summary.push_str(existing);
            summary.push('\n');
        }

        for exchange in &old_exchanges {
            summary.push_str(&format!("- User asked: {}\n", truncate(&exchange.user_input, 100)));
            for tc in &exchange.tool_calls {
                summary.push_str(&format!(
                    "  - Used {} ({})\n",
                    tc.tool_name,
                    if tc.success { "success" } else { "error" }
                ));
            }
            summary.push_str(&format!(
                "  - Result: {}\n",
                truncate(&exchange.assistant_response, 200)
            ));
        }

        self.compacted_summary = Some(summary);
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.exchanges.clear();
        self.compacted_summary = None;
    }

    /// Number of exchanges.
    pub fn len(&self) -> usize {
        self.exchanges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exchanges.is_empty()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_exchange(msg: &str) -> Exchange {
        Exchange {
            timestamp: Utc::now(),
            user_input: msg.to_string(),
            assistant_response: format!("Response to: {msg}"),
            tool_calls: vec![],
        }
    }

    #[test]
    fn test_push_and_recent() {
        let mut history = History::new();
        history.push(sample_exchange("first"));
        history.push(sample_exchange("second"));
        history.push(sample_exchange("third"));

        assert_eq!(history.len(), 3);
        assert_eq!(history.recent(2).len(), 2);
        assert_eq!(history.recent(2)[0].user_input, "second");
    }

    #[test]
    fn test_compact() {
        let mut history = History::new();
        for i in 0..5 {
            history.push(sample_exchange(&format!("message {i}")));
        }

        history.compact(2);
        assert_eq!(history.len(), 2);
        assert!(history.compacted_summary().is_some());
        assert!(history.compacted_summary().unwrap().contains("message 0"));
    }

    #[test]
    fn test_to_messages() {
        let mut history = History::new();
        history.push(sample_exchange("hello"));
        let messages = history.to_messages();
        assert_eq!(messages.len(), 2); // user + assistant
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[test]
    fn test_clear() {
        let mut history = History::new();
        history.push(sample_exchange("test"));
        history.clear();
        assert!(history.is_empty());
    }
}
