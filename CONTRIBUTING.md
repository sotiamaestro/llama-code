# Contributing to Llama Code

Thank you for your interest in contributing to Llama Code! This document provides guidelines for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/llama-code`
3. Install Rust: https://rustup.rs/
4. Install Ollama: https://ollama.com/download
5. Run setup: `llama-code setup`
6. Run tests: `cargo test`

## Development

### Building

```bash
cargo build           # Debug build
cargo build --release # Release build
```

### Running Tests

```bash
cargo test                    # All unit tests (no Ollama required)
cargo test --workspace        # All tests across all crates
cargo test -p llama-code-tools # Tests for a specific crate
```

### Code Quality

```bash
cargo clippy --workspace      # Lint check
cargo fmt --all               # Format code
```

## Project Structure

- `crates/llama-code-cli/` — CLI binary
- `crates/llama-code-core/` — Core agent loop
- `crates/llama-code-tools/` — Tool implementations
- `crates/llama-code-format/` — Prompt formatting
- `crates/llama-code-tui/` — Terminal UI

## Security Rules

**These are non-negotiable:**

- **NO** hardcoded API keys, tokens, or secrets anywhere
- **NO** telemetry, analytics, or phone-home behavior
- **NO** references to specific user paths or machine names
- **NO** logging of user prompts or file contents by default
- All paths must be relative or use environment variables

## Pull Request Process

1. Create a feature branch from `main`
2. Write tests for new functionality
3. Ensure `cargo test` passes
4. Ensure `cargo clippy` has no warnings
5. Update README if you've changed public-facing behavior
6. Submit a PR with a clear description of changes

## Adding a New Tool

1. Create `crates/llama-code-tools/src/your_tool.rs`
2. Implement the `Tool` trait
3. Register it in `crates/llama-code-tools/src/registry.rs`
4. Add unit tests
5. Update the tools table in README

## Code of Conduct

Be respectful, be constructive, be kind. We're all here to build something useful.

## License

By contributing, you agree that your contributions will be licensed under Apache 2.0.
