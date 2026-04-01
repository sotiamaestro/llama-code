# 🦙 Llama Code

**The first coding agent built natively for open-source models.**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

<!-- Demo GIF placeholder -->
<!-- ![Llama Code Demo](docs/demo.gif) -->

## Why Llama Code?

- **🔓 Fully open source** — Apache 2.0 licensed, end to end. No proprietary dependencies anywhere in the stack.
- **📡 Works offline** — Runs entirely on your machine. No cloud APIs, no accounts, no data leaves your computer.
- **🦙 Llama-native** — Prompt templates, tool calling, and constrained decoding built specifically for Llama's architecture.

## Quick Start

```bash
# Install
cargo install llama-code

# Setup (installs Ollama + pulls default model)
llama-code setup

# Run in any project directory
cd your-project
llama-code
```

## Supported Models

| Model | Size | Quality | Speed | Use Case |
|-------|------|---------|-------|----------|
| `llama3.2:3b-instruct` | 3B | ⭐⭐ | ⚡⚡⚡ | Quick reads, simple edits |
| `llama3.1:8b-instruct-q4_K_M` | 8B | ⭐⭐⭐ | ⚡⚡ | **Default** — general tasks |
| `llama3.1:70b-instruct-q4_K_M` | 70B | ⭐⭐⭐⭐ | ⚡ | Complex refactoring |

Any Ollama-compatible model works. The above are tested and recommended.

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  llama-code  │────▶│  llama-code  │────▶│  llama-code  │
│     cli      │     │     tui      │     │     core     │
│  (clap args) │     │  (ratatui)   │     │ (agent loop) │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                  │
                                    ┌─────────────┼─────────────┐
                                    │             │             │
                              ┌─────▼─────┐ ┌────▼────┐ ┌──────▼──────┐
                              │ llama-code│ │ llama-  │ │   Ollama    │
                              │   tools   │ │ code-   │ │   (local)   │
                              │(8 tools)  │ │ format  │ │             │
                              └───────────┘ └─────────┘ └─────────────┘
```

**Crates:**
- `llama-code-cli` — Binary entrypoint, argument parsing
- `llama-code-tui` — Terminal UI with ratatui
- `llama-code-core` — Agent loop, config, context management, Ollama client
- `llama-code-tools` — 8 built-in tools (file_read, file_write, file_edit, bash, grep, ls, git, think)
- `llama-code-format` — Llama 3.x prompt template, ChatML fallback, JSON repair

## Configuration

Config file: `~/.config/llama-code/config.toml`

```toml
[model]
default = "llama3.1:8b-instruct-q4_K_M"
heavy = "llama3.1:70b-instruct-q4_K_M"   # optional
light = "llama3.2:3b-instruct-q4_K_M"     # optional

[model.ollama]
host = "http://127.0.0.1:11434"

[model.parameters]
temperature = 0.1
num_ctx = 32768
num_predict = 4096

[permissions]
yolo = false           # skip confirmations
max_iterations = 10    # max tool calls per turn
```

Environment variable overrides:
- `LLAMA_CODE_OLLAMA_HOST` — Ollama server URL
- `LLAMA_CODE_MODEL` — Default model name
- `LLAMA_CODE_NUM_CTX` — Context window size

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/model [name]` | Switch model or show current |
| `/compact` | Manually trigger history compaction |
| `/clear` | Clear conversation history |
| `/diff` | Show all file changes this session |
| `/undo` | Revert the last file change |
| `/cost` | Show estimated token usage |
| `/config` | Open config in $EDITOR |
| `/exit` | Exit Llama Code |

## CLI Options

```
llama-code [OPTIONS] [COMMAND]

Commands:
  setup    Run initial setup (install Ollama, pull model)
  config   Show current configuration

Options:
  -m, --model <MODEL>         Model to use
  -C, --directory <DIR>       Working directory
      --yolo                  Skip confirmations
      --max-iterations <N>    Max iterations per turn (default: 10)
      --debug                 Enable debug logging
  -h, --help                  Print help
  -V, --version               Print version
```

## Built-in Tools

| Tool | Description |
|------|-------------|
| `file_read` | Read files with line numbers, smart truncation |
| `file_write` | Create/overwrite files with diff preview |
| `file_edit` | Surgical string replacement (str_replace) |
| `bash` | Execute shell commands with allowlist/timeout |
| `grep` | Ripgrep-powered codebase search |
| `ls` | Tree-style directory listing |
| `git` | Git operations with read/write permission tiers |
| `think` | Extended reasoning scratchpad |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache 2.0 — see [LICENSE](LICENSE).

## Acknowledgments

Inspired by the work of:
- [Ollama](https://ollama.com) — Local model serving
- [llama.cpp](https://github.com/ggerganov/llama.cpp) — Model inference
- [Aider](https://github.com/paul-gauthier/aider) — AI coding assistant
- [OpenCode](https://github.com/opencode-ai/opencode) — Terminal AI coding tool
- [Claw Code](https://github.com/instructkr/claw-code) — Architecture inspiration
- [Meta](https://ai.meta.com/llama/) — Llama models
