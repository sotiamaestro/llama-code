// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Configuration loading and validation.

use crate::errors::{LlamaError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default Ollama host.
const DEFAULT_OLLAMA_HOST: &str = "http://127.0.0.1:11434";
/// Default model.
const DEFAULT_MODEL: &str = "llama3.1:8b-instruct-q4_K_M";
/// Default context window.
const DEFAULT_NUM_CTX: usize = 32768;
/// Default max tokens per response.
const DEFAULT_NUM_PREDICT: usize = 4096;
/// Default temperature for code generation.
const DEFAULT_TEMPERATURE: f64 = 0.1;
/// Default max iterations per turn.
const DEFAULT_MAX_ITERATIONS: usize = 10;

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub model: ModelConfig,

    #[serde(default)]
    pub permissions: PermissionsConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Default model for general tasks.
    #[serde(default = "default_model")]
    pub default: String,

    /// Heavy model for complex reasoning (optional).
    pub heavy: Option<String>,

    /// Light model for simple tasks (optional).
    pub light: Option<String>,

    #[serde(default)]
    pub ollama: OllamaConfig,

    #[serde(default)]
    pub parameters: ModelParameters,
}

/// Ollama connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_host")]
    pub host: String,
}

/// Model generation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    #[serde(default = "default_top_p")]
    pub top_p: f64,

    #[serde(default = "default_num_ctx")]
    pub num_ctx: usize,

    #[serde(default = "default_num_predict")]
    pub num_predict: usize,

    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f64,
}

/// Permissions configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    /// Skip confirmations for most operations.
    #[serde(default)]
    pub yolo: bool,

    /// Maximum iterations per agent turn.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable debug logging (opt-in only).
    #[serde(default)]
    pub debug: bool,

    /// Log directory (within project's .llama-code/ directory).
    pub log_dir: Option<PathBuf>,
}

// Default value functions
fn default_model() -> String { DEFAULT_MODEL.to_string() }
fn default_ollama_host() -> String { DEFAULT_OLLAMA_HOST.to_string() }
fn default_temperature() -> f64 { DEFAULT_TEMPERATURE }
fn default_top_p() -> f64 { 0.95 }
fn default_num_ctx() -> usize { DEFAULT_NUM_CTX }
fn default_num_predict() -> usize { DEFAULT_NUM_PREDICT }
fn default_repeat_penalty() -> f64 { 1.1 }
fn default_max_iterations() -> usize { DEFAULT_MAX_ITERATIONS }

impl Default for Config {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            permissions: PermissionsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model(),
            heavy: None,
            light: None,
            ollama: OllamaConfig::default(),
            parameters: ModelParameters::default(),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: default_ollama_host(),
        }
    }
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            top_p: default_top_p(),
            num_ctx: default_num_ctx(),
            num_predict: default_num_predict(),
            repeat_penalty: default_repeat_penalty(),
        }
    }
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            yolo: false,
            max_iterations: default_max_iterations(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            debug: false,
            log_dir: None,
        }
    }
}

impl Config {
    /// Load configuration from the standard config path.
    ///
    /// Checks in order:
    /// 1. `$LLAMA_CODE_CONFIG` env var
    /// 2. `$XDG_CONFIG_HOME/llama-code/config.toml`
    /// 3. `~/.config/llama-code/config.toml`
    /// 4. Falls back to defaults
    pub fn load() -> Result<Self> {
        // Check env var first
        if let Ok(path) = std::env::var("LLAMA_CODE_CONFIG") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Self::load_from_file(&path);
            }
        }

        // Check XDG config
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("llama-code").join("config.toml");
            if config_path.exists() {
                return Self::load_from_file(&config_path);
            }
        }

        // Fall back to defaults
        Ok(Self::default())
    }

    /// Load from a specific file path.
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LlamaError::Config(format!("Failed to read config: {e}")))?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| LlamaError::Config(format!("Failed to parse config: {e}")))?;
        Ok(config)
    }

    /// Apply environment variable overrides.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("LLAMA_CODE_OLLAMA_HOST") {
            self.model.ollama.host = host;
        }
        if let Ok(model) = std::env::var("LLAMA_CODE_MODEL") {
            self.model.default = model;
        }
        if let Ok(ctx) = std::env::var("LLAMA_CODE_NUM_CTX") {
            if let Ok(n) = ctx.parse() {
                self.model.parameters.num_ctx = n;
            }
        }
    }

    /// Get the Ollama base URL.
    pub fn ollama_url(&self) -> &str {
        &self.model.ollama.host
    }

    /// Get the config directory path.
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("llama-code"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model.default, DEFAULT_MODEL);
        assert_eq!(config.model.ollama.host, DEFAULT_OLLAMA_HOST);
        assert_eq!(config.model.parameters.num_ctx, DEFAULT_NUM_CTX);
        assert!(!config.permissions.yolo);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
[model]
default = "llama3.2:3b"

[model.ollama]
host = "http://127.0.0.1:11434"

[model.parameters]
temperature = 0.2
num_ctx = 16384

[permissions]
yolo = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.model.default, "llama3.2:3b");
        assert_eq!(config.model.parameters.temperature, 0.2);
        assert_eq!(config.model.parameters.num_ctx, 16384);
        assert!(config.permissions.yolo);
    }

    #[test]
    fn test_partial_config() {
        // Should use defaults for missing fields
        let toml_str = r#"
[model]
default = "custom-model"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.model.default, "custom-model");
        assert_eq!(config.model.parameters.num_ctx, DEFAULT_NUM_CTX); // default
        assert!(!config.permissions.yolo); // default
    }
}
