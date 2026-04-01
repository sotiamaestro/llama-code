// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Main TUI application loop.

use crate::input::{self, SlashCommand, UserInput};
use crate::render;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use llama_code_core::agent::Agent;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use std::io;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Messages displayed in the conversation view.
struct DisplayMessage {
    role: String,
    content: String,
}

/// Run the TUI application.
pub async fn run(mut agent: Agent) -> anyhow::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut messages: Vec<DisplayMessage> = Vec::new();
    let mut input_buffer = String::new();
    let mut is_generating = false;
    let mut streaming_text = String::new();
    let mut scroll: u16 = 0;

    // Welcome message
    messages.push(DisplayMessage {
        role: "system".to_string(),
        content: format!(
            "🦙 Llama Code v{VERSION}\nModel: {}\nType a message or /help for commands.",
            agent.current_model()
        ),
    });

    loop {
        // Draw UI
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),     // Status bar
                    Constraint::Min(5),        // Messages
                    Constraint::Length(3),      // Input
                    Constraint::Length(1),      // Help bar
                ])
                .split(frame.area());

            // Status bar
            let status = render::format_status_bar(
                VERSION,
                agent.current_model(),
                &agent.context_usage(),
            );
            let status_widget = Paragraph::new(status)
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
            frame.render_widget(status_widget, chunks[0]);

            // Messages area
            let mut all_lines: Vec<Line> = Vec::new();
            for msg in &messages {
                let prefix = match msg.role.as_str() {
                    "user" => Span::styled(
                        "You: ",
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    "assistant" => Span::styled(
                        "🦙 ",
                        Style::default().fg(Color::Green),
                    ),
                    _ => Span::styled(
                        "",
                        Style::default().fg(Color::DarkGray),
                    ),
                };

                if !prefix.content.is_empty() {
                    all_lines.push(Line::from(prefix));
                }
                all_lines.extend(render::render_text(&msg.content));
                all_lines.push(Line::from(""));
            }

            // Add streaming text if generating
            if is_generating && !streaming_text.is_empty() {
                all_lines.push(Line::from(Span::styled(
                    "🦙 ",
                    Style::default().fg(Color::Green),
                )));
                all_lines.extend(render::render_text(&streaming_text));
            } else if is_generating {
                all_lines.push(Line::from(Span::styled(
                    "🦙 Thinking...",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
                )));
            }

            let messages_widget = Paragraph::new(Text::from(all_lines))
                .block(Block::default().borders(Borders::NONE))
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0));
            frame.render_widget(messages_widget, chunks[1]);

            // Input area
            let input_display = format!("> {input_buffer}█");
            let input_widget = Paragraph::new(input_display)
                .style(Style::default().fg(Color::Cyan))
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
            frame.render_widget(input_widget, chunks[2]);

            // Help bar
            let help = Paragraph::new(" [Ctrl+C: cancel] [Ctrl+D: exit] [/help] ")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(help, chunks[3]);
        })?;

        // Handle input events with timeout for updates
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    KeyEvent {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => {
                        if is_generating {
                            is_generating = false;
                            streaming_text.clear();
                            messages.push(DisplayMessage {
                                role: "system".to_string(),
                                content: "(cancelled)".to_string(),
                            });
                        } else {
                            break;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        if !is_generating && !input_buffer.trim().is_empty() {
                            let user_text = input_buffer.clone();
                            input_buffer.clear();

                            match input::parse_input(&user_text) {
                                UserInput::Message(msg) => {
                                    messages.push(DisplayMessage {
                                        role: "user".to_string(),
                                        content: msg.clone(),
                                    });

                                    // Process with agent
                                    is_generating = true;
                                    streaming_text.clear();

                                    match agent.process_turn(&msg).await {
                                        Ok(response) => {
                                            messages.push(DisplayMessage {
                                                role: "assistant".to_string(),
                                                content: response,
                                            });
                                        }
                                        Err(e) => {
                                            messages.push(DisplayMessage {
                                                role: "system".to_string(),
                                                content: format!("Error: {e}"),
                                            });
                                        }
                                    }
                                    is_generating = false;
                                }
                                UserInput::Command(cmd) => {
                                    handle_command(&mut agent, &mut messages, cmd);
                                    if matches!(
                                        input::parse_input(&user_text),
                                        UserInput::Command(SlashCommand::Exit)
                                    ) {
                                        break;
                                    }
                                }
                                UserInput::Empty => {}
                            }
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    } => {
                        if !is_generating {
                            input_buffer.push(c);
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    } => {
                        input_buffer.pop();
                    }
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => {
                        scroll = scroll.saturating_add(1);
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => {
                        scroll = scroll.saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup terminal
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn handle_command(
    agent: &mut Agent,
    messages: &mut Vec<DisplayMessage>,
    cmd: SlashCommand,
) {
    match cmd {
        SlashCommand::Help => {
            messages.push(DisplayMessage {
                role: "system".to_string(),
                content: input::help_text().to_string(),
            });
        }
        SlashCommand::Model(name) => {
            if let Some(name) = name {
                agent.switch_model(name.clone());
                messages.push(DisplayMessage {
                    role: "system".to_string(),
                    content: format!("Switched to model: {name}"),
                });
            } else {
                messages.push(DisplayMessage {
                    role: "system".to_string(),
                    content: format!("Current model: {}", agent.current_model()),
                });
            }
        }
        SlashCommand::Compact => {
            agent.session.history.compact(3);
            messages.push(DisplayMessage {
                role: "system".to_string(),
                content: "History compacted.".to_string(),
            });
        }
        SlashCommand::Clear => {
            agent.session.history.clear();
            messages.clear();
            messages.push(DisplayMessage {
                role: "system".to_string(),
                content: "Conversation cleared.".to_string(),
            });
        }
        SlashCommand::Cost => {
            messages.push(DisplayMessage {
                role: "system".to_string(),
                content: format!(
                    "Session tokens: ~{}\nContext: {}",
                    agent.session.total_tokens,
                    agent.context_usage()
                ),
            });
        }
        SlashCommand::Exit => {
            // Handled by caller
        }
        SlashCommand::Unknown(cmd) => {
            messages.push(DisplayMessage {
                role: "system".to_string(),
                content: format!("Unknown command: {cmd}. Type /help for available commands."),
            });
        }
        _ => {
            messages.push(DisplayMessage {
                role: "system".to_string(),
                content: "Command not yet implemented.".to_string(),
            });
        }
    }
}
