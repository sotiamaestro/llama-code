// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Color themes for the TUI.

use ratatui::style::{Color, Modifier, Style};

/// Theme colors for the application.
pub struct Theme;

impl Theme {
    /// Primary accent color.
    pub fn accent() -> Color {
        Color::Cyan
    }

    /// Color for user messages.
    pub fn user_message() -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    /// Color for assistant messages.
    pub fn assistant_message() -> Style {
        Style::default().fg(Color::Green)
    }

    /// Color for tool names.
    pub fn tool_name() -> Style {
        Style::default().fg(Color::Yellow)
    }

    /// Color for success indicators.
    pub fn success() -> Style {
        Style::default().fg(Color::Green)
    }

    /// Color for error indicators.
    pub fn error() -> Style {
        Style::default().fg(Color::Red)
    }

    /// Color for diff additions.
    pub fn diff_add() -> Style {
        Style::default().fg(Color::Green)
    }

    /// Color for diff deletions.
    pub fn diff_remove() -> Style {
        Style::default().fg(Color::Red)
    }

    /// Color for the status bar.
    pub fn status_bar() -> Style {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    }

    /// Color for the input prompt.
    pub fn prompt() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    /// Color for thinking/planning indicator.
    pub fn thinking() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC)
    }

    /// Color for dimmed/secondary text.
    pub fn dimmed() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    /// Color for help text.
    pub fn help() -> Style {
        Style::default().fg(Color::DarkGray)
    }
}
