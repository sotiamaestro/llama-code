// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Main agent loop implementing Plan -> Execute -> Validate cycle.

use crate::config::Config;
use crate::context::ContextManager;
use crate::errors::{LlamaError, Result};
use crate::events::{AgentEvent, EventBus};
use crate::history::{Exchange, ToolCallRecord};
use crate::model::{OllamaClient, OllamaMessage};
use crate::permissions::{Permission, PermissionManager};
use crate::router::ModelRouter;
use crate::session::Session;
use chrono::Utc;
use llama_code_format::{templates, PromptFormatter};
use llama_code_tools::registry::ToolRegistry;
use llama_code_tools::{ToolCall, ToolContext};

/// Agent state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Planning,
    Executing,
    Validating,
    ErrorRecovery,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "idle"),
            AgentState::Planning => write!(f, "planning"),
            AgentState::Executing => write!(f, "executing"),
            AgentState::Validating => write!(f, "validating"),
            AgentState::ErrorRecovery => write!(f, "error_recovery"),
        }
    }
}

/// The main Llama Code agent.
pub struct Agent {
    pub config: Config,
    pub session: Session,
    pub state: AgentState,
    pub client: OllamaClient,
    pub tools: ToolRegistry,
    pub tool_context: ToolContext,
    pub permissions: PermissionManager,
    pub context_manager: ContextManager,
    pub router: ModelRouter,
    pub events: EventBus,
    pub formatter: Box<dyn PromptFormatter>,
}

impl Agent {
    /// Create a new agent with the given configuration.
    pub fn new(config: Config, cwd: std::path::PathBuf) -> Self {
        let client = OllamaClient::new(config.ollama_url());
        let tools = ToolRegistry::with_builtins();
        let tool_context = ToolContext {
            cwd: cwd.clone(),
            yolo_mode: config.permissions.yolo,
        };
        let permissions = PermissionManager::new(config.permissions.yolo);
        let context_manager = ContextManager::new(&config.model.parameters);
        let router = ModelRouter::new(&config.model);
        let session = Session::new(cwd, config.model.default.clone());
        let formatter = Box::new(llama_code_format::llama3::Llama3Formatter::new());

        Self {
            config,
            session,
            state: AgentState::Idle,
            client,
            tools,
            tool_context,
            permissions,
            context_manager,
            router,
            events: EventBus::new(),
            formatter,
        }
    }

    /// Process a single user turn through the agent loop.
    ///
    /// Returns the final assistant response text.
    pub async fn process_turn(&mut self, user_input: &str) -> Result<String> {
        let max_iterations = self.config.permissions.max_iterations;
        let mut iteration = 0;
        let mut tool_call_records = Vec::new();
        let mut accumulated_response = String::new();

        // Select model based on input complexity
        let (model_name, _current_tier) = self.router.select_model(user_input);
        let current_model = model_name;

        // Build the system prompt
        let tool_name_strings = self.tools.tool_names();
        let tool_names: Vec<&str> = tool_name_strings.iter().map(|s| s.as_str()).collect();
        let os_name = std::env::consts::OS;
        let cwd_str = self.tool_context.cwd.to_string_lossy();

        let system_prompt = if let Some(summary) = self.session.history.compacted_summary() {
            templates::build_compact_prompt(summary, &cwd_str, os_name, &tool_names)
        } else {
            templates::build_system_prompt(&cwd_str, os_name, &tool_names)
        };

        loop {
            if iteration >= max_iterations {
                return Err(LlamaError::MaxIterations(max_iterations));
            }
            iteration += 1;

            // Build messages
            let mut messages = vec![OllamaMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            }];

            // Add history
            messages.extend(self.session.history.to_messages());

            // Add current turn context
            messages.push(OllamaMessage {
                role: "user".to_string(),
                content: user_input.to_string(),
            });

            // Add any accumulated tool results from this turn
            if !accumulated_response.is_empty() {
                messages.push(OllamaMessage {
                    role: "assistant".to_string(),
                    content: accumulated_response.clone(),
                });
            }

            // Set state
            self.state = AgentState::Planning;
            self.events
                .emit(AgentEvent::StateChanged(self.state.to_string()));

            // Get model response (streaming)
            let _tool_defs = self.tools.tool_definitions();
            let stop_tokens = self.formatter.stop_tokens();

            let (mut rx, handle) = self
                .client
                .chat_stream(
                    &current_model,
                    messages,
                    &self.config.model.parameters,
                    stop_tokens,
                )
                .await?;

            // Collect streamed tokens
            let mut response_text = String::new();
            while let Some(token) = rx.recv().await {
                response_text.push_str(&token);
                self.events.emit(AgentEvent::TokenReceived(token));
            }

            self.events.emit(AgentEvent::GenerationComplete);

            // Wait for completion and get stats
            let stats = handle
                .await
                .map_err(|e| LlamaError::Other(e.to_string()))??;
            self.session
                .add_tokens(stats.prompt_tokens + stats.completion_tokens);

            // Update context usage
            self.context_manager.update_usage(&response_text);

            // Parse tool calls from response
            let parsed_calls = self.formatter.parse_tool_calls(&response_text);

            if parsed_calls.is_empty() {
                // No tool calls - this is the final response
                accumulated_response = response_text;
                break;
            }

            // Execute tool calls
            self.state = AgentState::Executing;
            self.events
                .emit(AgentEvent::StateChanged(self.state.to_string()));

            for parsed in &parsed_calls {
                let tool_call = ToolCall {
                    name: parsed.name.clone(),
                    parameters: parsed.parameters.clone(),
                };

                self.events.emit(AgentEvent::ToolCallDetected {
                    tool_name: tool_call.name.clone(),
                    parameters: tool_call.parameters.clone(),
                });

                // Check permissions
                let permission = self.permissions.classify(&tool_call);
                if (permission == Permission::AlwaysConfirm
                    || permission == Permission::ConfirmOnce)
                    && !self.permissions.is_approved(&tool_call)
                {
                    // In the agent loop, we emit an event but auto-approve for now
                    // The TUI layer handles the actual user interaction
                    self.permissions.approve_for_session(&tool_call);
                }

                self.events.emit(AgentEvent::ToolExecutionStarted {
                    tool_name: tool_call.name.clone(),
                });

                let result = self.tools.execute(&tool_call, &self.tool_context).await;

                let success = result.is_success();
                self.events.emit(AgentEvent::ToolExecutionCompleted {
                    tool_name: tool_call.name.clone(),
                    success,
                    content: result.display_text().to_string(),
                });

                tool_call_records.push(ToolCallRecord {
                    tool_name: tool_call.name.clone(),
                    parameters: tool_call.parameters.clone(),
                    result: result.content.clone(),
                    success,
                });

                // Add tool result to accumulated response for next iteration
                let tool_result_json = serde_json::json!({
                    "status": if success { "success" } else { "error" },
                    "content": result.content,
                });
                accumulated_response.push_str(&response_text);
                accumulated_response.push('\n');
                accumulated_response.push_str(&format!("[Tool result: {}]", tool_result_json));
            }

            // Check if context needs compaction
            if self.context_manager.should_compact() {
                let old_tokens = self.context_manager.current_tokens();
                self.session.history.compact(3);
                let new_tokens = self.session.history.estimate_tokens();
                self.events.emit(AgentEvent::ContextCompacted {
                    old_tokens,
                    new_tokens,
                });
            }
        }

        // Record the exchange
        let exchange = Exchange {
            timestamp: Utc::now(),
            user_input: user_input.to_string(),
            assistant_response: accumulated_response.clone(),
            tool_calls: tool_call_records,
        };
        self.session.history.push(exchange);

        self.state = AgentState::Idle;
        self.events.emit(AgentEvent::TurnComplete);

        Ok(accumulated_response)
    }

    /// Check if Ollama is reachable and the model is available.
    pub async fn preflight_check(&self) -> Result<()> {
        let healthy = self.client.health_check().await?;
        if !healthy {
            return Err(LlamaError::OllamaConnection(
                "Cannot connect to Ollama. Is it running? Try: ollama serve".to_string(),
            ));
        }

        let has_model = self.client.has_model(&self.config.model.default).await?;
        if !has_model {
            return Err(LlamaError::ModelNotAvailable(format!(
                "Model '{}' is not available. Pull it with: ollama pull {}",
                self.config.model.default, self.config.model.default
            )));
        }

        Ok(())
    }

    /// Get context usage display string.
    pub fn context_usage(&self) -> String {
        self.context_manager.usage_display()
    }

    /// Get the current model name.
    pub fn current_model(&self) -> &str {
        &self.session.model
    }

    /// Switch to a different model.
    pub fn switch_model(&mut self, model: String) {
        let old = self.session.model.clone();
        self.session.model = model.clone();
        self.events.emit(AgentEvent::ModelSwitched {
            from: old,
            to: model,
            reason: "user request".to_string(),
        });
    }
}
