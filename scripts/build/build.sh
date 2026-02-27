#!/bin/bash
# Clorus Build Script
# Builds clorus binaries and core/runtime libraries.

set -euo pipefail

PROFILE="release"

if [[ "${1:-}" == "--debug" ]]; then
  PROFILE="debug"
fi

echo "╔════════════════════════════════════╗"
echo "║  Clorus Build                      ║"
echo "╚════════════════════════════════════╝"
echo ""
echo "Profile: $PROFILE"
echo ""

if [[ "$PROFILE" == "release" ]]; then
  echo "🔨 Building clorus (release)..."
  cargo build --release --bin clorus

  echo "🔨 Building clorus-runtime (release)..."
  cargo build --release -p clorus-runtime

  echo "🔨 Building clorus-core (release)..."
  cargo build --release -p clorus-core

  echo "🔨 Building clorus-std (release)..."
  cargo build --release -p clorus-std
else
  echo "🔨 Building clorus (debug)..."
  cargo build --bin clorus

  echo "🔨 Building clorus-runtime (debug)..."
  cargo build -p clorus-runtime

  echo "🔨 Building clorus-core (debug)..."
  cargo build -p clorus-core

  echo "🔨 Building clorus-std (debug)..."
  cargo build -p clorus-std
fi

echo ""
echo "✅ Build complete"
