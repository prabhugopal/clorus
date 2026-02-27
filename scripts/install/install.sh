#!/bin/bash
# Clorus Installation Script
# Installs clorus unified binary and runtime libraries

set -e

# Configuration
INSTALL_DIR="${1:-$HOME/.clorus}"
BIN_DIR="$INSTALL_DIR/bin"
LIB_DIR="$INSTALL_DIR/lib"
STDLIB_DIR="$INSTALL_DIR/stdlib"

echo "╔════════════════════════════════════╗"
echo "║  Clorus Installation               ║"
echo "╚════════════════════════════════════╝"
echo ""
echo "Installing to: $INSTALL_DIR"
echo ""

# Create directories
mkdir -p "$BIN_DIR"
mkdir -p "$LIB_DIR"
mkdir -p "$STDLIB_DIR"

# Check if release build exists
if [ ! -f "target/release/clorus" ]; then
    echo "❌ Release build not found. Building..."
    if [ -f "scripts/build/build.sh" ]; then
        scripts/build/build.sh
    else
        cargo build --release --bin clorus
        cargo build --release -p clorus-runtime
        cargo build --release -p clorus-core
        cargo build --release -p clorus-std
    fi
fi

echo "📦 Copying binaries..."
cp target/release/clorus "$BIN_DIR/"
chmod +x "$BIN_DIR/clorus"

echo "📚 Copying libraries..."
# Copy runtime libraries (both static and dynamic)
find target/release -name "libclorus_runtime.*" -exec cp {} "$LIB_DIR/" \;
find target/release/deps -name "libclorus_runtime.*" -exec cp {} "$LIB_DIR/" \; 2>/dev/null || true

# Copy other essential libraries
for lib in libclorus_core.* libclorus_std.*; do
    if [ -f "target/release/$lib" ]; then
        cp "target/release/$lib" "$LIB_DIR/"
    fi
done

echo "📖 Copying standard library..."
if [ -d "stdlib" ]; then
    cp -r stdlib/* "$STDLIB_DIR/"
fi

echo ""
echo "✅ Installation complete!"
echo ""
echo "To use Clorus, add the following to your shell profile (~/.zshrc or ~/.bashrc):"
echo ""
echo "    export PATH=\"$BIN_DIR:\$PATH\""
echo "    export CLORUS_HOME=\"$INSTALL_DIR\""
echo ""
echo "Then run: source ~/.zshrc  (or source ~/.bashrc)"
echo ""
echo "Installed components:"
echo "  • clorus        - Unified compiler, build tool, and REPL"
echo "  • Runtime libs  - $LIB_DIR"
echo "  • Standard lib  - $STDLIB_DIR"
echo ""
echo "Quick start:"
echo "  clorus new my-project    - Create new project"
echo "  clorus build             - Build executable"
echo "  clorus run               - Run project"
echo "  clorus repl              - Start REPL"
echo ""
