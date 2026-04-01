// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Output rendering for the TUI - markdown, diffs, code blocks.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Render text with basic inline formatting.
pub fn render_text(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for line_str in text.lines() {
        let line = render_line(line_str);
        lines.push(line);
    }

    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

/// Render a single line with basic formatting.
fn render_line(text: &str) -> Line<'static> {
    // Diff coloring
    if text.starts_with('+') && !text.starts_with("+++") {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Green),
        ));
    }
    if text.starts_with('-') && !text.starts_with("---") {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Red),
        ));
    }
    if text.starts_with("@@") {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Cyan),
        ));
    }

    // Headers
    if let Some(stripped) = text.strip_prefix("### ") {
        return Line::from(Span::styled(
            stripped.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }
    if let Some(stripped) = text.strip_prefix("## ") {
        return Line::from(Span::styled(
            stripped.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }
    if let Some(stripped) = text.strip_prefix("# ") {
        return Line::from(Span::styled(
            stripped.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Tool indicators
    if text.starts_with("📄") || text.starts_with("✏️") || text.starts_with("🔍") {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Yellow),
        ));
    }
    if text.starts_with("✅") {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Green),
        ));
    }
    if text.starts_with("❌") {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Red),
        ));
    }

    // Plain text
    Line::from(text.to_string())
}

/// Format a status bar string.
pub fn format_status_bar(version: &str, model: &str, context_usage: &str) -> String {
    format!(" 🦙 Llama Code {version} | model: {model} | ctx: {context_usage} ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_basic() {
        let lines = render_text("Hello world");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_multiline() {
        let lines = render_text("line 1\nline 2\nline 3");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_format_status_bar() {
        let bar = format_status_bar("0.1.0", "llama3.1:8b", "12.4k/32k");
        assert!(bar.contains("Llama Code"));
        assert!(bar.contains("llama3.1:8b"));
    }
}
