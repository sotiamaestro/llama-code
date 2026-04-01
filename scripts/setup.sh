#!/usr/bin/env bash
set -euo pipefail

echo "🦙 Llama Code Setup"
echo "==================="
echo ""

# 1. Check/install Ollama
if ! command -v ollama &> /dev/null; then
    echo "⚠️  Ollama is not installed."
    echo "   Please install Ollama first: https://ollama.com/download"
    echo ""
    echo "   On macOS:  brew install ollama"
    echo "   On Linux:  curl -fsSL https://ollama.com/install.sh | sh"
    echo ""
    echo "   Then re-run this script."
    exit 1
else
    echo "✓ Ollama is installed ($(ollama --version 2>/dev/null || echo 'version unknown'))"
fi

# 2. Start Ollama if not running
if ! curl -s http://127.0.0.1:11434/api/tags &> /dev/null; then
    echo "Starting Ollama..."
    ollama serve &
    sleep 3
    if ! curl -s http://127.0.0.1:11434/api/tags &> /dev/null; then
        echo "⚠️  Failed to start Ollama. Please start it manually: ollama serve"
        exit 1
    fi
fi
echo "✓ Ollama is running"

# 3. Pull default model
MODEL="llama3.1:8b-instruct-q4_K_M"
echo ""
echo "Pulling default model (${MODEL})..."
echo "This may take a few minutes on first run."
ollama pull "${MODEL}"
echo "✓ Model '${MODEL}' is ready"

# 4. Create default config
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/llama-code"
mkdir -p "${CONFIG_DIR}"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    cat > "${CONFIG_DIR}/config.toml" << 'EOF'
[model]
default = "llama3.1:8b-instruct-q4_K_M"

[model.ollama]
host = "http://127.0.0.1:11434"

[model.parameters]
temperature = 0.1
num_ctx = 32768
num_predict = 4096

[permissions]
yolo = false
max_iterations = 10
EOF
    echo "✓ Config created at ${CONFIG_DIR}/config.toml"
else
    echo "✓ Config already exists at ${CONFIG_DIR}/config.toml"
fi

echo ""
echo "✓ Llama Code is ready! Run 'llama-code' in any project directory."
