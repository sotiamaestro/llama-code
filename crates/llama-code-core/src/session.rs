// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Session management and persistence.

use crate::errors::{LlamaError, Result};
use crate::history::History;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A coding session with conversation history.
#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub cwd: PathBuf,
    pub model: String,
    pub history: History,
    /// Estimated total tokens used this session.
    pub total_tokens: usize,
}

/// Serializable session metadata for persistence.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub cwd: String,
    pub model: String,
    pub total_tokens: usize,
    pub exchange_count: usize,
}

impl Session {
    /// Create a new session.
    pub fn new(cwd: PathBuf, model: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            cwd,
            model,
            history: History::new(),
            total_tokens: 0,
        }
    }

    /// Get session metadata.
    pub fn metadata(&self) -> SessionMetadata {
        SessionMetadata {
            id: self.id.clone(),
            created_at: self.created_at,
            cwd: self.cwd.to_string_lossy().to_string(),
            model: self.model.clone(),
            total_tokens: self.total_tokens,
            exchange_count: self.history.len(),
        }
    }

    /// Get the session data directory.
    pub fn session_dir(&self) -> PathBuf {
        self.cwd.join(".llama-code").join("sessions").join(&self.id)
    }

    /// Save session to disk.
    pub fn save(&self) -> Result<()> {
        let dir = self.session_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| LlamaError::Session(format!("Failed to create session dir: {e}")))?;

        let metadata_path = dir.join("metadata.json");
        let metadata = self.metadata();
        let json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| LlamaError::Session(format!("Failed to serialize session: {e}")))?;
        std::fs::write(metadata_path, json)
            .map_err(|e| LlamaError::Session(format!("Failed to write session: {e}")))?;

        Ok(())
    }

    /// Add to the token count.
    pub fn add_tokens(&mut self, tokens: usize) {
        self.total_tokens += tokens;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = Session::new(
            PathBuf::from("/tmp/test"),
            "llama3.1:8b".to_string(),
        );
        assert!(!session.id.is_empty());
        assert_eq!(session.model, "llama3.1:8b");
        assert!(session.history.is_empty());
        assert_eq!(session.total_tokens, 0);
    }

    #[test]
    fn test_session_metadata() {
        let session = Session::new(
            PathBuf::from("/tmp/test"),
            "llama3.1:8b".to_string(),
        );
        let meta = session.metadata();
        assert_eq!(meta.id, session.id);
        assert_eq!(meta.model, "llama3.1:8b");
        assert_eq!(meta.exchange_count, 0);
    }
}
