# 🦙 Llama Code

**The first coding agent built natively for open-source models.**

Llama Code is a terminal-based AI coding agent that runs entirely on your machine. No API keys, no cloud, no telemetry. Just you, your code, and a local Llama model.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/sotiamaestro/llama-code/ci.yml?branch=main&label=CI)](https://github.com/sotiamaestro/llama-code/actions)
[![Tests](https://img.shields.io/badge/tests-84%20passing-brightgreen.svg)]()

---

## Why Llama Code?

Most AI coding tools are wrappers around proprietary APIs. They send your code to someone else's server, charge per token, and stop working when the internet goes down.

Llama Code is different:

- **Fully local.** Runs on Ollama. Your code never leaves your machine.
- **Llama-native.** Prompt templates, tool calling format, and context management are purpose-built for Llama 3.x - not adapted from a Claude/GPT harness.
- **Model ladder.** Automatically routes simple tasks to small models (3B) and complex reasoning to large models (70B). All local, all configurable.
- **Zero config.** Install, pull a model, run. That's it.

---

## Quick Start

```bash
# Install
cargo install llama-code

# Setup (installs Ollama + pulls default model if needed)
llama-code setup

# Run in any project directory
cd your-project
llama-code
```

Or build from source:

```bash
git clone https://github.com/sotiamaestro/llama-code.git
cd llama-code
cargo build --release
./target/release/llama-code
```

---

## Demo

![Llama Code Demo](docs/demo.gif)

*Fix a fibonacci performance bug — Plan → Read → Edit with diff → Verify → Done in 2.8s*

---

## Supported Models

Llama Code works with any model available through Ollama. These are tested and recommended:

| Model | Size | VRAM | Best For | Quality |
|-------|------|------|----------|---------|
| `llama3.2:3b` | ~2 GB | 4 GB | Quick edits, file reads, simple tasks | ⭐⭐⭐ |
| `llama3.1:8b` | ~4.7 GB | 8 GB | General coding, bug fixes, test writing | ⭐⭐⭐⭐ |
| `llama3.1:70b-q4_K_M` | ~40 GB | 48 GB | Complex refactors, architecture, multi-file | ⭐⭐⭐⭐⭐ |
| `codellama:13b` | ~7 GB | 10 GB | Code-focused tasks, completions | ⭐⭐⭐⭐ |
| `deepseek-coder-v2:16b` | ~9 GB | 12 GB | Code generation, debugging | ⭐⭐⭐⭐ |
| `qwen2.5-coder:7b` | ~4.4 GB | 8 GB | Code-focused alternative | ⭐⭐⭐⭐ |

**Don't have a GPU?** Ollama runs on CPU too. Start with `llama3.2:3b` - it's fast even on a MacBook Air.

**Model ladder:** Configure light/default/heavy models in `~/.config/llama-code/config.toml` and Llama Code auto-routes tasks to the right size.

---

## Tools

Llama Code ships with 8 built-in tools:

| Tool | Description |
|------|-------------|
| `file_read` | Read files with smart truncation and line ranges |
| `file_write` | Create/overwrite files with diff preview |
| `file_edit` | Surgical string replacement (like find-and-replace) |
| `bash` | Execute shell commands with allowlist + timeout |
| `grep` | Ripgrep-powered codebase search |
| `ls` | Tree-style directory listing |
| `git` | Git operations with read/write permission tiers |
| `think` | Extended reasoning scratchpad |

---

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────────┐
│  llama-code  │────▶│  llama-code  │────▶│   llama-code     │
│    -cli      │     │    -tui      │     │     -core        │
│  (entrypoint)│     │  (ratatui)   │     │  (agent loop)    │
└──────────────┘     └──────────────┘     └────────┬─────────┘
                                                   │
                                          ┌────────┴─────────┐
                                          │                  │
                                   ┌──────▼──────┐   ┌──────▼───────┐
                                   │ llama-code  │   │  llama-code  │
                                   │   -tools    │   │   -format    │
                                   │ (8 tools)   │   │(Llama prompts│
                                   └─────────────┘   └──────────────┘
```

**Agent loop:** Plan → Execute → Validate → repeat or respond.

**Model ladder:** Light (3B) → Default (8B) → Heavy (70B) with automatic escalation on failure.

**Permission system:** Three tiers (auto-approve, confirm-once, always-confirm) so the model can't `rm -rf` your project without asking.

---

## Configuration

Config lives at `~/.config/llama-code/config.toml`:

```toml
[model]
default = "llama3.1:8b-instruct-q4_K_M"
heavy = "llama3.1:70b-instruct-q4_K_M"    # optional
light = "llama3.2:3b-instruct-q4_K_M"     # optional

[model.ollama]
host = "http://127.0.0.1:11434"

[model.parameters]
temperature = 0.1
num_ctx = 32768
num_predict = 4096

[permissions]
yolo = false    # set true to skip most confirmations
```

---

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/model [name]` | Switch model mid-session |
| `/compact` | Manually trigger history compaction |
| `/clear` | Clear conversation history |
| `/diff` | Show all file changes this session |
| `/undo` | Revert last file change |
| `/cost` | Show estimated token usage |
| `/config` | Open config in $EDITOR |
| `/exit` | Exit Llama Code |

---

## Building From Source

Requirements:
- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Ollama (install via [ollama.com](https://ollama.com/))

```bash
git clone https://github.com/sotiamaestro/llama-code.git
cd llama-code
cargo build --release
```

Run tests:

```bash
cargo test
```

---

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where help is needed:
- Integration tests with real Ollama models
- Constrained decoding improvements for smaller models
- Support for additional model families (Mistral, Phi, Gemma)
- Performance profiling and optimization
- Doc-tests and documentation
- Packaging (Homebrew, AUR, Nix, etc.)

---

## Acknowledgments

Llama Code stands on the shoulders of giants:

- [Ollama](https://ollama.com/) - Local model serving
- [Meta Llama](https://llama.meta.com/) - Open-source language models
- [Claw Code](https://github.com/instructkr/claw-code) - Harness architecture inspiration
- [Aider](https://github.com/paul-gauthier/aider) - Pioneering open-source coding agents
- [OpenCode](https://github.com/opencode-ai/opencode) - Model-agnostic agent design
- [ratatui](https://ratatui.rs/) - Terminal UI framework

---

## License

Apache 2.0. See [LICENSE](LICENSE) for details.
