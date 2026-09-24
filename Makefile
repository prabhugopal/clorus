# Clorus Build System Makefile

.PHONY: all build build-release install clean test help

# Default target
all: build-release

# Help message
help:
	@echo "╔════════════════════════════════════╗"
	@echo "║  Clorus Build System               ║"
	@echo "╚════════════════════════════════════╝"
	@echo ""
	@echo "Available targets:"
	@echo "  make build          - Build debug binaries"
	@echo "  make build-release  - Build release binaries (default)"
	@echo "  make install        - Install to ~/.clorus (or INSTALL_DIR)"
	@echo "  make test           - Run tests"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make help           - Show this help"
	@echo ""
	@echo "Environment variables:"
	@echo "  INSTALL_DIR         - Installation directory (default: ~/.clorus)"
	@echo ""

# Build all components in debug mode
build:
	@echo "Building Clorus (debug)..."
	cargo build --bin clorus
	cargo build -p clorus-runtime --lib
	cargo build -p clorus-std --lib
	@echo "✅ Debug build complete"

# Build all components in release mode
build-release:
	@echo "Building Clorus (release)..."
	cargo build --release --bin clorus
	cargo build --release -p clorus-runtime --lib
	cargo build --release -p clorus-std --lib
	@echo "✅ Release build complete"
	@echo ""
	@echo "Binaries:"
	@ls -lh target/release/clorus 2>/dev/null || true
	@echo ""
	@echo "Libraries:"
	@ls -lh target/release/libclorus_runtime.* 2>/dev/null | grep -v ".d" || true
	@echo ""

# Install to INSTALL_DIR (default: ~/.clorus)
install: build-release
	@chmod +x scripts/install/install.sh
	@./scripts/install/install.sh $(INSTALL_DIR)

# Run tests
test:
	cargo test --workspace

# Clean build artifacts
clean:
	cargo clean
	@echo "✅ Cleaned build artifacts"

# Quick install for development (installs without rebuilding)
install-dev:
	@chmod +x scripts/install/install.sh
	@./scripts/install/install.sh $(INSTALL_DIR)
