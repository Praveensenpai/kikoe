#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

REPO="Praveensenpai/kikoe"
BINARY_NAME="kikoe"
INSTALL_DIR="$HOME/.local/bin"
CACHE_DIR="$HOME/.cache/kikoe"

echo "⛩️  Installing kikoe (聴こえ)..."

OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
    echo "❌ Currently only Linux is supported by this installer." >&2
    exit 1
fi

if [ "$ARCH" != "x86_64" ]; then
    echo "❌ Currently only x86_64 architecture is supported." >&2
    exit 1
fi

mkdir -p "$INSTALL_DIR"
mkdir -p "$CACHE_DIR"

TAG=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/' || true)

if [ -z "$TAG" ]; then
    echo "⚠️  No prebuilt binary release tag found. Building from source via Cargo..."
    if command -v cargo >/dev/null 2>&1; then
        cargo install --git "https://github.com/$REPO.git"
        echo "✔ Installed kikoe via cargo!"
    else
        echo "❌ Cargo is not installed. Please install Rust or download a prebuilt release." >&2
        exit 1
    fi
else
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/kikoe-x86_64-unknown-linux-gnu.tar.gz"
    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    echo "📥 Downloading kikoe $TAG..."
    if curl -fsSL "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR" 2>/dev/null; then
        install -Dm 755 "$TMP_DIR/kikoe" "$INSTALL_DIR/kikoe"
        echo "✔ Installed kikoe to $INSTALL_DIR/kikoe"
    else
        echo "⚠️  Binary download failed. Falling back to cargo install..."
        cargo install --git "https://github.com/$REPO.git"
    fi
fi

# Ensure ONNX acoustic model is cached
MODEL_FILE="$CACHE_DIR/voxlingua107.onnx"
if [ ! -f "$MODEL_FILE" ]; then
    echo "📥 Downloading neural acoustic model voxlingua107.onnx (~83MB)..."
    MODEL_URL="https://github.com/$REPO/releases/download/v0.1.0/voxlingua107.onnx"
    curl -fsSL "$MODEL_URL" -o "$MODEL_FILE" || {
        echo "⚠️  Could not pre-fetch model. kikoe will auto-download on first execution."
    }
fi

echo "✨ 聴こえ (kikoe) successfully installed!"
echo "Run 'kikoe --help' to get started."
