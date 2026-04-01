// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Model backend abstraction - Ollama HTTP API client.

use crate::config::ModelParameters;
use crate::errors::{LlamaError, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Ollama API client.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
}

/// A chat message for the Ollama API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: ChatOptions,
}

/// Model options for generation.
#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f64,
    top_p: f64,
    num_ctx: usize,
    num_predict: usize,
    repeat_penalty: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
}

/// Streaming response chunk from Ollama.
#[derive(Debug, Deserialize)]
pub struct ChatResponseChunk {
    pub message: Option<OllamaMessage>,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u64>,
}

/// Non-streaming response from Ollama.
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: OllamaMessage,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u64>,
}

/// Model info from Ollama.
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: Option<u64>,
    pub digest: Option<String>,
    pub modified_at: Option<String>,
}

/// List models response.
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Generation statistics.
#[derive(Debug, Clone, Default)]
pub struct GenerationStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_duration_ms: u64,
}

impl OllamaClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Check if Ollama is running and reachable.
    pub async fn health_check(&self) -> Result<bool> {
        match self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
        {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| LlamaError::OllamaConnection(e.to_string()))?;

        let models: ModelsResponse = resp
            .json()
            .await
            .map_err(|e| LlamaError::Model(format!("Failed to parse model list: {e}")))?;

        Ok(models.models)
    }

    /// Check if a specific model is available.
    pub async fn has_model(&self, model_name: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model_name || m.name.starts_with(&format!("{model_name}:"))))
    }

    /// Send a chat completion request with streaming.
    /// Returns a channel that receives tokens as they arrive.
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
        params: &ModelParameters,
        stop_tokens: Vec<String>,
    ) -> Result<(mpsc::Receiver<String>, tokio::task::JoinHandle<Result<GenerationStats>>)> {
        let (tx, rx) = mpsc::channel(256);

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            options: ChatOptions {
                temperature: params.temperature,
                top_p: params.top_p,
                num_ctx: params.num_ctx,
                num_predict: params.num_predict,
                repeat_penalty: params.repeat_penalty,
                stop: stop_tokens,
            },
        };

        let client = self.client.clone();
        let url = format!("{}/api/chat", self.base_url);

        let handle = tokio::spawn(async move {
            let resp = client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e| LlamaError::OllamaConnection(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(LlamaError::Model(format!(
                    "Ollama returned {status}: {body}"
                )));
            }

            let mut stream = resp.bytes_stream();
            let mut stats = GenerationStats::default();
            let mut buffer = Vec::new();

            while let Some(chunk) = stream.next().await {
                let bytes = chunk.map_err(|e| LlamaError::Model(e.to_string()))?;
                buffer.extend_from_slice(&bytes);

                // Process complete JSON lines from buffer
                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = buffer.drain(..=newline_pos).collect();
                    let line = String::from_utf8_lossy(&line);
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(chunk) = serde_json::from_str::<ChatResponseChunk>(trimmed) {
                        if let Some(msg) = &chunk.message {
                            if !msg.content.is_empty() {
                                let _ = tx.send(msg.content.clone()).await;
                            }
                        }

                        if chunk.done {
                            if let Some(eval) = chunk.eval_count {
                                stats.completion_tokens = eval as usize;
                            }
                            if let Some(prompt_eval) = chunk.prompt_eval_count {
                                stats.prompt_tokens = prompt_eval as usize;
                            }
                            if let Some(duration) = chunk.total_duration {
                                stats.total_duration_ms = duration / 1_000_000;
                            }
                        }
                    }
                }
            }

            Ok(stats)
        });

        Ok((rx, handle))
    }

    /// Send a non-streaming chat completion.
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
        params: &ModelParameters,
        stop_tokens: Vec<String>,
    ) -> Result<(String, GenerationStats)> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            options: ChatOptions {
                temperature: params.temperature,
                top_p: params.top_p,
                num_ctx: params.num_ctx,
                num_predict: params.num_predict,
                repeat_penalty: params.repeat_penalty,
                stop: stop_tokens,
            },
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| LlamaError::OllamaConnection(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlamaError::Model(format!(
                "Ollama returned {status}: {body}"
            )));
        }

        let response: ChatResponse = resp
            .json()
            .await
            .map_err(|e| LlamaError::Model(format!("Failed to parse response: {e}")))?;

        let stats = GenerationStats {
            prompt_tokens: response.prompt_eval_count.unwrap_or(0) as usize,
            completion_tokens: response.eval_count.unwrap_or(0) as usize,
            total_duration_ms: response.total_duration.unwrap_or(0) / 1_000_000,
        };

        Ok((response.message.content, stats))
    }

    /// Pull a model from Ollama registry.
    pub async fn pull_model(&self, model: &str) -> Result<()> {
        let resp = self
            .client
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({"name": model, "stream": false}))
            .send()
            .await
            .map_err(|e| LlamaError::OllamaConnection(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlamaError::Model(format!("Failed to pull model: {body}")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new("http://127.0.0.1:11434");
        assert_eq!(client.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn test_trailing_slash_stripped() {
        let client = OllamaClient::new("http://127.0.0.1:11434/");
        assert_eq!(client.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn test_generation_stats_default() {
        let stats = GenerationStats::default();
        assert_eq!(stats.prompt_tokens, 0);
        assert_eq!(stats.completion_tokens, 0);
    }
}
