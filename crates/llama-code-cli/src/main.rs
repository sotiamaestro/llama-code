// Copyright 2026 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Llama Code CLI - The first coding agent built natively for open-source models.

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::generate;
use clap_complete::shells::{Bash, Fish, PowerShell, Zsh};
use llama_code_core::agent::Agent;
use llama_code_core::config::Config;
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "llama-code",
    about = "🦙 The first coding agent built natively for open-source models",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    /// Model to use (overrides config)
    #[arg(short, long)]
    model: Option<String>,

    /// Skip confirmations for most operations
    #[arg(long)]
    yolo: bool,

    /// Maximum iterations per turn
    #[arg(long, default_value = "10")]
    max_iterations: usize,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Working directory (default: current directory)
    #[arg(short = 'C', long)]
    directory: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run initial setup (install Ollama, pull default model)
    Setup,
    /// Show or manage configuration
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    /// Generate shell completion scripts to stdout
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Write a default config.toml to the standard configuration path
    Generate {
        /// Overwrite an existing config file
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let command = cli.command;

    if let Some(Commands::Completions { shell }) = command {
        return run_completions(shell);
    }

    if let Some(Commands::Config {
        command: Some(ConfigCommands::Generate { force }),
    }) = command
    {
        return run_config_generate(force);
    }

    // Setup logging (opt-in only)
    if cli.debug {
        tracing_subscriber::fmt()
            .with_env_filter("llama_code=debug")
            .with_target(false)
            .init();
    }

    // Load config
    let mut config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {e}. Using defaults.");
        Config::default()
    });

    // Apply CLI overrides
    config.apply_env_overrides();
    if let Some(model) = cli.model {
        config.model.default = model;
    }
    if cli.yolo {
        config.permissions.yolo = true;
    }
    config.permissions.max_iterations = cli.max_iterations;
    config.logging.debug = cli.debug;

    // Handle subcommands
    match command {
        Some(Commands::Setup) => {
            return run_setup(&config).await;
        }
        Some(Commands::Config { command: None }) => {
            println!("{}", toml::to_string_pretty(&config)?);
            return Ok(());
        }
        None => {}
        Some(Commands::Config { .. }) | Some(Commands::Completions { .. }) => unreachable!(),
    }

    // Get working directory
    let cwd = cli
        .directory
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Create agent
    let agent = Agent::new(config, cwd);

    // Preflight check
    if let Err(e) = agent.preflight_check().await {
        eprintln!("❌ {e}");
        eprintln!();
        eprintln!("Run 'llama-code setup' to install Ollama and pull the default model.");
        std::process::exit(1);
    }

    // Run TUI
    llama_code_tui::app::run(agent).await?;

    Ok(())
}

fn run_completions(shell: CompletionShell) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    let mut stdout = io::stdout();

    match shell {
        CompletionShell::Bash => generate(Bash, &mut cmd, bin_name, &mut stdout),
        CompletionShell::Zsh => generate(Zsh, &mut cmd, bin_name, &mut stdout),
        CompletionShell::Fish => generate(Fish, &mut cmd, bin_name, &mut stdout),
        CompletionShell::PowerShell => generate(PowerShell, &mut cmd, bin_name, &mut stdout),
    }

    Ok(())
}

fn run_config_generate(force: bool) -> anyhow::Result<()> {
    let config_dir = Config::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("config.toml");
    if config_path.exists() && !force {
        anyhow::bail!(
            "Config already exists at {}. Pass --force to overwrite.",
            config_path.display()
        );
    }

    let toml_str = toml::to_string_pretty(&Config::default())?;
    std::fs::write(&config_path, toml_str)?;
    println!("{}", config_path.display());

    Ok(())
}

async fn run_setup(config: &Config) -> anyhow::Result<()> {
    println!("🦙 Llama Code Setup");
    println!("===================");
    println!();

    // Check if Ollama is available
    let client = llama_code_core::model::OllamaClient::new(config.ollama_url());

    let healthy = client.health_check().await.unwrap_or(false);
    if !healthy {
        println!("⚠️  Ollama is not running.");
        println!("   Please install and start Ollama first:");
        println!("   https://ollama.com/download");
        println!();
        println!("   Then run: ollama serve");
        println!("   And try again: llama-code setup");
        return Ok(());
    }
    println!("✓ Ollama is running");

    // Check/pull default model
    let model = &config.model.default;
    let has_model = client.has_model(model).await.unwrap_or(false);
    if has_model {
        println!("✓ Model '{model}' is available");
    } else {
        println!("Pulling model '{model}'...");
        println!("This may take a few minutes on first run.");
        match client.pull_model(model).await {
            Ok(()) => println!("✓ Model '{model}' pulled successfully"),
            Err(e) => {
                eprintln!("❌ Failed to pull model: {e}");
                eprintln!("   Try manually: ollama pull {model}");
                return Ok(());
            }
        }
    }

    // Create config directory
    if let Some(config_dir) = Config::config_dir() {
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            let default_config = Config::default();
            let toml_str = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, toml_str)?;
            println!("✓ Config created at {}", config_path.display());
        } else {
            println!("✓ Config exists at {}", config_path.display());
        }
    }

    println!();
    println!("✓ Llama Code is ready! Run 'llama-code' in any project directory.");

    Ok(())
}
