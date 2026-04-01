// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! User input handling and slash command parsing.

/// Parsed user input.
#[derive(Debug, Clone)]
pub enum UserInput {
    /// Regular message to send to the agent.
    Message(String),
    /// Slash command.
    Command(SlashCommand),
    /// Empty input (ignored).
    Empty,
}

/// Slash commands available in the TUI.
#[derive(Debug, Clone)]
pub enum SlashCommand {
    Help,
    Model(Option<String>),
    Compact,
    Clear,
    Diff,
    Undo,
    Cost,
    Config,
    Exit,
    Unknown(String),
}

/// Parse user input into a `UserInput` variant.
pub fn parse_input(input: &str) -> UserInput {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return UserInput::Empty;
    }

    if !trimmed.starts_with('/') {
        return UserInput::Message(trimmed.to_string());
    }

    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let command = parts[0].to_lowercase();
    let args = parts.get(1).map(|s| s.to_string());

    let slash_cmd = match command.as_str() {
        "/help" | "/h" | "/?" => SlashCommand::Help,
        "/model" | "/m" => SlashCommand::Model(args),
        "/compact" => SlashCommand::Compact,
        "/clear" => SlashCommand::Clear,
        "/diff" => SlashCommand::Diff,
        "/undo" => SlashCommand::Undo,
        "/cost" => SlashCommand::Cost,
        "/config" => SlashCommand::Config,
        "/exit" | "/quit" | "/q" => SlashCommand::Exit,
        other => SlashCommand::Unknown(other.to_string()),
    };

    UserInput::Command(slash_cmd)
}

/// Get help text for slash commands.
pub fn help_text() -> &'static str {
    "\
Available commands:
  /help, /h, /?    Show this help message
  /model [name]    Switch model (or show current model)
  /compact         Manually trigger history compaction
  /clear           Clear conversation history
  /diff            Show all file changes this session
  /undo            Revert the last file change
  /cost            Show estimated token usage
  /config          Open config in $EDITOR
  /exit, /quit     Exit Llama Code

Keyboard shortcuts:
  Ctrl+C           Cancel current operation
  Ctrl+D           Exit
  Up/Down          Scroll through conversation
  Enter            Send message"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message() {
        match parse_input("fix the bug") {
            UserInput::Message(msg) => assert_eq!(msg, "fix the bug"),
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn test_parse_empty() {
        assert!(matches!(parse_input(""), UserInput::Empty));
        assert!(matches!(parse_input("  "), UserInput::Empty));
    }

    #[test]
    fn test_parse_help() {
        match parse_input("/help") {
            UserInput::Command(SlashCommand::Help) => {}
            _ => panic!("Expected Help"),
        }
    }

    #[test]
    fn test_parse_model_with_arg() {
        match parse_input("/model llama3.2:3b") {
            UserInput::Command(SlashCommand::Model(Some(name))) => {
                assert_eq!(name, "llama3.2:3b");
            }
            _ => panic!("Expected Model with arg"),
        }
    }

    #[test]
    fn test_parse_exit() {
        assert!(matches!(
            parse_input("/exit"),
            UserInput::Command(SlashCommand::Exit)
        ));
        assert!(matches!(
            parse_input("/quit"),
            UserInput::Command(SlashCommand::Exit)
        ));
    }

    #[test]
    fn test_parse_unknown() {
        match parse_input("/foobar") {
            UserInput::Command(SlashCommand::Unknown(cmd)) => {
                assert_eq!(cmd, "/foobar");
            }
            _ => panic!("Expected Unknown"),
        }
    }
}
