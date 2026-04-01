# Contributing to Llama Code

Thanks for your interest in contributing. Here's how to get started.

## Development Setup

1. Install Rust 1.75+ via [rustup](https://rustup.rs/)
2. Install Ollama via [ollama.com](https://ollama.com/)
3. Pull a model: `ollama pull llama3.1:8b`
4. Clone and build:

```bash
git clone https://github.com/sotiamaestro/llama-code.git
cd llama-code
cargo build
cargo test
```

## Project Structure

```
crates/
  llama-code-cli/      # Binary entrypoint, argument parsing
  llama-code-core/     # Agent loop, config, context management, model client
  llama-code-format/   # Llama-native prompt formatting, tool call schemas
  llama-code-tools/    # Built-in tool implementations
  llama-code-tui/      # Terminal UI (ratatui)
```

## Making Changes

1. Fork the repo and create a feature branch from `main`
2. Make your changes
3. Add tests for new functionality
4. Run `cargo test` and ensure all tests pass
5. Run `cargo clippy` and fix any warnings
6. Run `cargo fmt` to format code
7. Open a PR against `main`

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write doc comments for public APIs
- Prefer returning `Result` over panicking
- No `unwrap()` in library code (use `?` or `expect()` with descriptive messages)

## Security

- **Never commit API keys, tokens, or secrets**
- **Never hardcode file paths** - use environment variables or platform defaults
- **Never add telemetry or analytics** without explicit opt-in
- **Never send data over the network** except to the local Ollama instance
- Run the security audit in the README before submitting PRs

## Areas for Contribution

### Good First Issues
- Add doc-tests to public API functions
- Improve error messages for common failure modes
- Add more models to the "Supported Models" table with quality ratings

### Medium
- Integration test suite that runs against real Ollama
- Support for Mistral/Phi/Gemma prompt formats in `llama-code-format`
- Improve JSON repair heuristics in `constrained.rs`
- Session persistence (save/resume conversations)

### Advanced
- Constrained decoding via llama.cpp grammar sampling
- Benchmark suite against SWE-bench Lite
- npm wrapper (`npx llama-code`)
- Homebrew formula

## Reporting Issues

Include:
- OS and architecture
- Rust version (`rustc --version`)
- Ollama version (`ollama --version`)
- Model being used
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs (with any personal info redacted)

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.
