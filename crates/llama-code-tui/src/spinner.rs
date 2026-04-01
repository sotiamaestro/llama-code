// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Thinking/loading spinner indicators.

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A braille spinner for loading indicators.
pub struct Spinner {
    frame: usize,
    message: String,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            frame: 0,
            message: message.into(),
        }
    }

    /// Advance to the next frame and return the display string.
    pub fn tick(&mut self) -> String {
        let frame = FRAMES[self.frame % FRAMES.len()];
        self.frame += 1;
        format!("{frame} {}", self.message)
    }

    /// Update the spinner message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get the current display string without advancing.
    pub fn current(&self) -> String {
        let frame = FRAMES[self.frame % FRAMES.len()];
        format!("{frame} {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_tick() {
        let mut spinner = Spinner::new("Loading...");
        let first = spinner.tick();
        let second = spinner.tick();
        assert_ne!(first, second);
        assert!(first.contains("Loading..."));
    }
}
