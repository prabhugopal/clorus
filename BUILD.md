# Clorus Build & Installation Guide

This guide covers building, installing, and distributing Clorus.

## Quick Start

### Building from Source

```bash
# Build all components (release mode)
make build-release

# Or using cargo directly
cargo build --release --bin clorus --bin repl
cargo build --release -p clorus-runtime --lib
cargo build --release -p clorus-core --lib
```

### Installing

```bash
# Install to ~/.clorus (recommended)
make install

# Or install to custom location
make install INSTALL_DIR=/usr/local/clorus

# Or use install script directly
./install.sh ~/.clorus
```

After installation, add to your shell profile (~/.zshrc or ~/.bashrc):

```bash
export PATH="$HOME/.clorus/bin:$PATH"
export CLORUS_HOME="$HOME/.clorus"
```

Then reload: `source ~/.zshrc`

## Build System Components

### Binaries

1. **clorus** (55MB)
   - Main compiler and build tool
   - Commands: `new`, `build`, `run`, `check`, `repl`
   - Location: `target/release/clorus`

2. **repl** (55MB)
   - Standalone REPL with JIT compilation
   - Interactive development environment
   - Location: `target/release/repl`

### Libraries

1. **libclorus_runtime.a** (16MB)
   - Static library for linking compiled executables
   - Contains Value* runtime, collections, atoms, etc.
   - Location: `target/release/deps/libclorus_runtime.a`

2. **libclorus_runtime.dylib** (755KB)
   - Dynamic library for REPL/JIT execution
   - Loaded at runtime by `clorus run` and `repl`
   - Location: `target/release/deps/libclorus_runtime.dylib`

3. **libclorus_core.dylib** (756KB)
   - Core Clojure-style functions (slurp, spit)
   - Optional, loaded if available
   - Location: `target/release/libclorus_core.dylib`

### Standard Library

- **stdlib/core.clr** - Core functions and macros
- **stdlib/lazy.clr** - Lazy sequences
- **stdlib/transducers.clr** - Transducers (if implemented)

## Build Targets

### `make build`
Build debug versions of all components. Faster compilation, slower runtime.

```bash
make build
```

### `make build-release`
Build optimized release versions. Slower compilation, fast runtime. **This is the default.**

```bash
make build-release
```

### `make install`
Build release versions and install to INSTALL_DIR (default: ~/.clorus).

```bash
make install

# Custom installation directory
make install INSTALL_DIR=/opt/clorus
```

### `make test`
Run the test suite.

```bash
make test
```

### `make clean`
Remove all build artifacts.

```bash
make clean
```

## Directory Structure After Installation

```
~/.clorus/
├── bin/
│   ├── clorus          # Main CLI
│   └── repl            # REPL binary
├── lib/
│   ├── libclorus_runtime.a      # Static library
│   ├── libclorus_runtime.dylib  # Dynamic library
│   └── libclorus_core.dylib     # Core functions
└── stdlib/
    ├── core.clr        # Core stdlib
    └── lazy.clr        # Lazy sequences
```

## How It Works

### Building Projects

When you run `clorus build` in a project directory:

1. Reads `Clorus.toml` for configuration
2. Parses entry file (default: `src/main.clrs`)
3. Compiles to LLVM IR
4. Generates object file
5. **Searches for runtime library:**
   - Current project: `target/{release,debug}/deps/`
   - Parent directories: `../target/`, `../../target/`
   - Clorus installation: `$CLORUS_HOME/lib/`
   - Relative to clorus binary: `../lib/`
6. Links with system frameworks (macOS: AppKit, Metal, etc.)
7. Creates standalone executable

### Running Projects

When you run `clorus run`:

1. Compiles code to LLVM IR (like build)
2. Creates JIT execution engine
3. **Dynamically loads libraries:**
   - libclorus_runtime.dylib (required)
   - libclorus_core.dylib (optional)
   - Rust FFI libraries (if configured)
4. Executes code in JIT
5. Calls `-main` function if present

### REPL

When you run `clorus repl`:

1. Loads project config from `Clorus.toml` (if present)
2. Dynamically loads runtime and core libraries
3. Processes Rust FFI dependencies
4. Loads entry file (evaluates forms)
5. Starts interactive prompt

## Distribution

### Creating a Distribution Package

For distributing Clorus to users:

```bash
# Build release version
make build-release

# Create distribution tarball
tar -czf clorus-$(cargo pkgid | cut -d# -f2).tar.gz \
  target/release/clorus \
  target/release/repl \
  target/release/deps/libclorus_runtime.* \
  target/release/libclorus_core.* \
  stdlib/ \
  install.sh \
  README.md

# Creates: clorus-0.1.0.tar.gz
```

Users can then:

```bash
tar -xzf clorus-0.1.0.tar.gz
cd clorus-0.1.0
./install.sh
```

### Binary Distribution

For maximum simplicity, you can create a self-contained binary distribution:

```bash
# Build release
make build-release

# Create distribution directory
mkdir -p dist/clorus
cp target/release/clorus dist/clorus/
cp target/release/repl dist/clorus/
cp -r target/release/deps/libclorus_runtime.* dist/clorus/
cp -r target/release/libclorus_core.* dist/clorus/ 2>/dev/null || true
cp -r stdlib dist/clorus/

# Create tarball
cd dist && tar -czf clorus-bin-$(uname -s)-$(uname -m).tar.gz clorus/
# Creates: clorus-bin-Darwin-arm64.tar.gz (or Linux-x86_64, etc.)
```

## Troubleshooting

### Runtime library not found

If `clorus build` reports "Runtime library not found":

1. **Check if runtime is built:**
   ```bash
   ls target/release/deps/libclorus_runtime.a
   ```

2. **Rebuild runtime:**
   ```bash
   cargo build --release -p clorus-runtime --lib
   ```

3. **Set CLORUS_HOME:**
   ```bash
   export CLORUS_HOME=/path/to/clorus/installation
   ```

### REPL can't find libraries

If `clorus repl` reports library loading errors:

1. **Rebuild dynamic libraries:**
   ```bash
   cargo build --release -p clorus-runtime --lib
   cargo build --release -p clorus-core --lib
   ```

2. **Check library locations:**
   ```bash
   ls target/release/deps/libclorus_runtime.dylib
   ls target/release/libclorus_core.dylib
   ```

### Built executable doesn't work

If executables built with `clorus build` fail to run:

1. **Check dynamic library dependencies:**
   ```bash
   otool -L ./target/your-app      # macOS
   ldd ./target/your-app           # Linux
   ```

2. **Verify runtime is statically linked:**
   ```bash
   nm ./target/your-app | grep clorus_value
   # Should show symbols like _clorus_value_long
   ```

3. **Check for Rust FFI issues:**
   - Ensure all Rust dependencies are compiled
   - Check `target/rust-ffi/` for generated wrappers

## Development Workflow

### Iterative Development

```bash
# 1. Make code changes
vim crates/clorus-codegen/src/codegen.rs

# 2. Rebuild
cargo build --release --bin clorus

# 3. Test
cd ~/my-project
clorus build
./target/my-project
```

### Testing Changes

```bash
# Run workspace tests
cargo test --workspace

# Test specific component
cargo test -p clorus-runtime
cargo test -p clorus-codegen

# Integration test with example project
cd examples/gui-demo
clorus build
./target/gui-demo
```

### Quick Development Install

After making changes, quickly update your installation:

```bash
# Rebuild just the binaries
cargo build --release --bin clorus --bin repl

# Install without rebuilding libraries
make install-dev
```

## Platform-Specific Notes

### macOS

- AppKit framework required for GUI applications
- Metal framework for graphics acceleration
- Libraries use `.dylib` extension
- May need to allow binaries in Security & Privacy settings

### Linux

- X11 or Wayland required for GUI
- Libraries use `.so` extension
- May need to install LLVM development libraries

### Windows

- Libraries use `.dll` extension
- May need Visual Studio Build Tools
- Path handling uses `\` instead of `/`

## See Also

- [README.md](README.md) - Project overview
- [LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md) - Language specification
- [RUST_FFI_GUIDE.md](docs/guides/RUST_FFI_GUIDE.md) - Rust FFI integration
