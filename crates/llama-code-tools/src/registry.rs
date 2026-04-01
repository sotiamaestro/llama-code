// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tool registry - manages available tools and dispatches calls.

use crate::{Tool, ToolCall, ToolContext, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with all built-in tools.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(crate::file_read::FileReadTool));
        registry.register(Arc::new(crate::file_write::FileWriteTool));
        registry.register(Arc::new(crate::file_edit::FileEditTool));
        registry.register(Arc::new(crate::bash::BashTool));
        registry.register(Arc::new(crate::grep::GrepTool));
        registry.register(Arc::new(crate::ls::LsTool));
        registry.register(Arc::new(crate::git::GitTool));
        registry.register(Arc::new(crate::think::ThinkTool));
        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Execute a tool call.
    pub async fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> ToolResult {
        match self.tools.get(&call.name) {
            Some(tool) => tool.execute(call.parameters.clone(), ctx).await,
            None => ToolResult::error(format!(
                "Unknown tool '{}'. Available tools: {}",
                call.name,
                self.tool_names().join(", ")
            )),
        }
    }

    /// Get all tool names.
    pub fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get tool definitions for prompt formatting.
    pub fn tool_definitions(&self) -> Vec<llama_code_format::ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .map(|t| llama_code_format::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_builtins() {
        let registry = ToolRegistry::with_builtins();
        assert!(registry.get("file_read").is_some());
        assert!(registry.get("file_write").is_some());
        assert!(registry.get("file_edit").is_some());
        assert!(registry.get("bash").is_some());
        assert!(registry.get("grep").is_some());
        assert!(registry.get("ls").is_some());
        assert!(registry.get("git").is_some());
        assert!(registry.get("think").is_some());
        assert_eq!(registry.len(), 8);
    }

    #[test]
    fn test_unknown_tool() {
        let registry = ToolRegistry::with_builtins();
        let ctx = ToolContext::new(std::env::current_dir().unwrap());
        let call = ToolCall {
            name: "nonexistent".to_string(),
            parameters: serde_json::Value::Null,
        };
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(registry.execute(&call, &ctx));
        assert!(!result.is_success());
        assert!(result.content.contains("Unknown tool"));
    }

    #[test]
    fn test_tool_definitions() {
        let registry = ToolRegistry::with_builtins();
        let defs = registry.tool_definitions();
        assert_eq!(defs.len(), 8);
        // Should be sorted alphabetically
        assert!(defs[0].name <= defs[1].name);
    }
}
