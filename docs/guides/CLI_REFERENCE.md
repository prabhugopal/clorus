# Clorus CLI Reference

**Complete guide to the Clorus command-line interface**

Version: 0.1.0
Last Updated: February 2026

---

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Commands Overview](#commands-overview)
4. [Project Management](#project-management)
5. [Build Commands](#build-commands)
6. [Package Management](#package-management)
7. [Workspace Commands](#workspace-commands)
8. [Development Tools](#development-tools)
9. [Configuration](#configuration)
10. [Examples](#examples)

---

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/clorus.git
cd clorus

# Build the CLI
cargo build --release -p clorus-cli

# Optionally install globally
cargo install --path crates/clorus-cli
```

### Verify Installation

```bash
$ clorus version
clorus 0.1.0
```

---

## Quick Start

```bash
# Create a new project
$ clorus new my-app
     Created binary (application) `my-app` package

# Enter the project
$ cd my-app

# Run the project
$ clorus run
   Compiling my-app v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `target/my-app`

=> 30
```

---

## Commands Overview

| Command | Description | Workspace Support |
|---------|-------------|-------------------|
| `new` | Create a new project | No |
| `build` | Compile project | Yes (--workspace) |
| `run` | Compile and run | No |
| `check` | Check syntax | No |
| `clean` | Remove build artifacts | Yes (--workspace) |
| `pack` | Package as .clip library | Yes (--workspace) |
| `install` | Install .clip package | No |
| `repl` | Start interactive REPL | No |
| `replx` | Start extended REPL | No |
| `help` | Show help | No |
| `version` | Show version | No |

---

## Project Management

### `clorus new <name>`

Create a new Clorus project.

**Usage:**
```bash
clorus new <project-name>
```

**Example:**
```bash
$ clorus new hello-world
     Created binary (application) `hello-world` package

To get started:
  cd hello-world
  clorus run
```

**Created Structure:**
```
hello-world/
├── Clorus.toml      # Project manifest
├── .gitignore       # Git ignore (target/, *.clrs.tmp)
└── src/
    └── main.clrs    # Entry point
```

**Default Clorus.toml:**
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = []

[build]
entry = "src/main.clrs"
```

---

## Build Commands

### `clorus build`

Compile the current project to a native executable.

**Usage:**
```bash
clorus build [OPTIONS]
```

**Options:**
- `--workspace` - Build all workspace members
- `--debug`, `-d` - Enable debug mode (memory tracking)

**Examples:**

**Basic build:**
```bash
$ clorus build
   Resolving dependencies...
   Compiling my-project v0.1.0
    Generated object file: target/my-project.o
    Finished dev [unoptimized] target(s) in 0.00s

   Executable: target/my-project
   Run with: ./target/my-project
```

**With debug mode:**
```bash
$ clorus build --debug
   Debug mode: enabled
   Compiling my-project v0.1.0
   [DEBUG] Loaded 42 expressions from stdlib
   [DEBUG] Total expressions to compile: 156
    Finished dev [unoptimized] target(s) in 0.00s
```

**What it does:**
1. Loads stdlib/core.clr (standard library)
2. Resolves and loads dependencies from Clorus.toml
3. Recursively loads required modules
4. Compiles all expressions to LLVM IR
5. Generates object file (target/project-name.o)
6. Links with runtime library
7. Creates executable (target/project-name)

**For Libraries:**
If no entry point is specified, builds as library:
```bash
$ clorus build
   Skipping build for library package mylib v1.0.0
   (No entry point or src/lib.clrs found)
```

Or if src/lib.clrs exists:
```bash
$ clorus build
   Compiling mylib v1.0.0
    Generated object file: target/mylib.o
    Generated bitcode file: target/mylib.bc
    Finished lib build in 0.00s
```

---

### `clorus run`

Compile and execute the current project.

**Usage:**
```bash
clorus run [OPTIONS] [ARGS...]
```

**Options:**
- `--jit` - Use JIT mode (faster, but no .clip support)
- `--debug`, `-d` - Enable debug mode
- `[ARGS...]` - Arguments passed to -main function

**Examples:**

**Basic run:**
```bash
$ clorus run
   Compiling my-project v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `target/my-project`

=> 42
```

**With arguments:**
```bash
$ clorus run arg1 arg2
   Compiling my-project v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `target/my-project`

Args: ["arg1" "arg2"]
```

**JIT mode (faster for small projects):**
```bash
$ clorus run --jit
   Mode: JIT compilation (--jit flag)
   Compiling my-project v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `src/main.clrs`

=> 42
```

**How arguments work:**
```clojure
;; src/main.clrs
(defn -main [& args]
  (println "Arguments:" args)
  (count args))
```

```bash
$ clorus run hello world
Arguments: ["hello" "world"]
=> 2
```

**JIT vs AOT Mode:**

| Feature | JIT (--jit) | AOT (default) |
|---------|-------------|---------------|
| Speed | Faster compilation | Slower compilation |
| .clip dependencies | ❌ Not supported | ✅ Fully supported |
| Executable | None (memory only) | target/project-name |
| Best for | Quick iteration | Production, libraries |

---

### `clorus check`

Check syntax without building.

**Usage:**
```bash
clorus check
```

**Example:**
```bash
$ clorus check
   Checking my-project v0.1.0
    Finished checking my-project in 0.00s
```

**What it does:**
- Parses entry file
- Expands macros
- Checks for syntax errors
- Does NOT compile or generate code

---

### `clorus clean`

Remove build artifacts.

**Usage:**
```bash
clorus clean [OPTIONS]
```

**Options:**
- `--workspace` - Clean all workspace members

**Examples:**

**Clean single project:**
```bash
$ clorus clean
   Cleaning my-project v0.1.0
      Removed target/
      Removed my-project-0.1.0.clip
    Finished cleaning
```

**Clean workspace:**
```bash
$ clorus clean --workspace
🧹 Cleaning workspace with 4 member(s)

   Cleaning coral-gfx...
      Removed target/
   Cleaning coral-ui...
      Removed target/
   Cleaning workspace target/...

✅ Workspace cleaned
```

**What it removes:**
- `target/` directory
- All `.clip` files matching package name
- Workspace-level `target/` (in workspace mode)

---

## Package Management

### `clorus pack`

Package project as a .clip library.

**Usage:**
```bash
clorus pack [OPTIONS]
```

**Options:**
- `--workspace` - Package all workspace libraries
- `--output <file>` - Output filename (single) or directory (workspace)

**Examples:**

**Package single library:**
```bash
$ clorus pack
📦 Packaging Clorus library...
   📝 Package: json-lib v0.1.0
   📄 Output: json-lib-0.1.0.clip
   🔨 Building project...
   Compiling json-lib v0.1.0
    Generated object file: target/json-lib.o
    Generated bitcode file: target/json-lib.bc
   📋 Creating metadata...
   📦 Packaging compiled artifacts...
      ✓ Packaged json-lib.o
      ✓ Packaged json-lib.bc
   🗜️  Creating .clip archive...
✅ Successfully created json-lib-0.1.0.clip
```

**Custom output:**
```bash
$ clorus pack --output libs/mylib.clip
   📄 Output: libs/mylib.clip
```

**Package workspace:**
```bash
$ clorus pack --workspace
📦 Packaging workspace to dist/

   Packaging coral-gfx...
   Packaging coral-ui...

✅ Packaged 2 member(s):
   📦 coral-gfx-1.0.0.clip
   📦 coral-ui-1.0.0.clip

Output directory: /path/to/workspace/dist
```

**Custom output directory:**
```bash
$ clorus pack --workspace --output release/
📦 Packaging workspace to release/
   ...
```

**What it creates:**

**.clip File Structure:**
```
json-lib-0.1.0.clip (ZIP archive)
├── clip.toml           # Package metadata
├── lib/
│   ├── json-lib.o      # Native object file
│   └── json-lib.bc     # LLVM bitcode (for JIT)
├── api/
│   └── exports.json    # Exported functions
└── docs/
    └── README.md       # Optional documentation
```

**Workspace behavior:**
- Only packages libraries (packages without entry point)
- Skips applications (packages with entry point)
- Shows which members were packaged/skipped
- All .clip files go to output directory (default: dist/)

---

### `clorus install`

Install a .clip package.

**Usage:**
```bash
clorus install <clip-file> [OPTIONS]
```

**Options:**
- `--local` - Install to current project (default: global)

**Examples:**

**Install globally:**
```bash
$ clorus install json-lib-0.1.0.clip
📦 Installing json-lib v0.1.0
   Extracting to ~/.clorus/packages/json-lib-0.1.0/
   ✅ Installed successfully

Add to your Clorus.toml:
[dependencies]
json-lib = { path = "~/.clorus/packages/json-lib-0.1.0" }
```

**Install locally:**
```bash
$ clorus install json-lib-0.1.0.clip --local
📦 Installing json-lib v0.1.0 (local)
   Extracting to libs/json-lib-0.1.0/
   ✅ Installed successfully

Add to your Clorus.toml:
[dependencies]
json-lib = { path = "libs/json-lib-0.1.0" }
```

**Installation paths:**
- **Global:** `~/.clorus/packages/<package-name>-<version>/`
- **Local:** `./libs/<package-name>-<version>/`

---

## Workspace Commands

### What is a Workspace?

A workspace is a collection of related Clorus packages managed together, similar to Cargo workspaces.

**Benefits:**
- Build all packages with one command
- Share dependencies across packages
- Consistent versioning
- Easier CI/CD
- Better monorepo support

### Workspace Structure

```
my-workspace/
├── Clorus.toml                # Workspace root manifest
├── coral-gfx/                 # Member 1 (library)
│   ├── Clorus.toml
│   └── src/lib.clrs
├── coral-ui/                  # Member 2 (library)
│   ├── Clorus.toml
│   └── src/lib.clrs
├── examples/                  # Member group
│   ├── gallery/               # Member 3 (app)
│   │   ├── Clorus.toml
│   │   └── src/main.clrs
│   └── test-app/              # Member 4 (app)
│       ├── Clorus.toml
│       └── src/main.clrs
└── target/                    # Workspace-level builds
```

### Workspace Configuration

**Root Clorus.toml:**
```toml
[workspace]
members = [
    "coral-gfx",
    "coral-ui",
    "examples/*",      # Glob patterns supported!
]

exclude = [
    "examples/experimental",
]

# Optional workspace-level metadata
[workspace.package]
version = "1.0.0"
authors = ["Your Team"]
repository = "https://github.com/your-org/workspace"

# Shared dependencies
[workspace.dependencies]
coral-gfx = { path = "coral-gfx" }
coral-ui = { path = "coral-ui" }
```

**Member Clorus.toml:**
```toml
[package]
name = "gallery"
version = "1.0.0"

[dependencies]
# Reference workspace dependency
coral-ui = { workspace = true }   # Auto-resolved!
```

### `clorus build --workspace`

Build all workspace members.

**Example:**
```bash
$ clorus build --workspace
📦 Building workspace with 4 member(s)

   Building coral-gfx...
   Compiling coral-gfx v1.0.0
    Finished dev [unoptimized] target(s) in 0.5s

   Building coral-ui...
   Compiling coral-ui v1.0.0
    Finished dev [unoptimized] target(s) in 0.6s

   Building gallery...
   Compiling gallery v1.0.0
    Finished dev [unoptimized] target(s) in 0.4s

   Building test-app...
   Compiling test-app v1.0.0
    Finished dev [unoptimized] target(s) in 0.3s

✅ Workspace build complete - 4 member(s) built
```

**With errors:**
```bash
$ clorus build --workspace
...
   Building coral-ui...
   ❌ Failed to build coral-ui: Parse error

⚠️  Workspace build completed with errors:
   ✓ Succeeded: coral-gfx, gallery
   ❌ Failed: coral-ui, test-app
```

### `clorus clean --workspace`

Clean all workspace members.

**Example:**
```bash
$ clorus clean --workspace
🧹 Cleaning workspace with 4 member(s)

   Cleaning coral-gfx...
      Removed target/
   Cleaning coral-ui...
      Removed target/
   Cleaning gallery...
      Removed target/
   Cleaning test-app...
      Removed target/
   Cleaning workspace target/...

✅ Workspace cleaned
```

### `clorus pack --workspace`

Package all workspace libraries.

**Example:**
```bash
$ clorus pack --workspace
📦 Packaging workspace to dist/

   Packaging coral-gfx...
   Packaging coral-ui...
      Skipped (not a library)

✅ Packaged 2 member(s):
   📦 coral-gfx-1.0.0.clip
   📦 coral-ui-1.0.0.clip

ℹ️  Skipped 2 member(s) (applications):
   → gallery
   → test-app

Output directory: /path/to/workspace/dist
```

**Custom output:**
```bash
$ clorus pack --workspace --output release/
```

### Workspace Dependencies

**Shared Dependencies:**
```toml
# Root Clorus.toml
[workspace.dependencies]
json-lib = { path = "libs/json-lib-0.1.0.clip" }
math-utils = { path = "math-utils" }
```

**Member Usage:**
```toml
# coral-ui/Clorus.toml
[dependencies]
json-lib = { workspace = true }    # References workspace dep
math-utils = { workspace = true }
```

**How it works:**
1. Member specifies `{ workspace = true }`
2. Compiler looks up dependency in `[workspace.dependencies]`
3. Automatically resolves paths from workspace root
4. No manual path calculation needed!

---

## Development Tools

### `clorus repl`

Start an interactive REPL (Read-Eval-Print Loop).

**Usage:**
```bash
clorus repl [OPTIONS]
```

**Options:**
- `--main-thread` - Run on main thread (for GUI on macOS)

**Example:**
```bash
$ clorus repl
Clorus REPL v0.1.0
Type :help for help, :quit to exit

userλ> (+ 1 2)
=> 3

userλ> (defn square [x] (* x x))
=> #'user/square

userλ> (square 5)
=> 25

userλ> :quit
Goodbye!
```

**With .clip dependencies:**
```bash
$ cd my-project  # Has json-lib dependency
$ clorus repl

Clorus REPL v0.1.0
📦 Loaded 1 .clip package(s) for REPL
   ✓ json-lib v0.1.0

my-projectλ> (json/parse "{\"x\": 42}")
=> {:x 42}
```

**REPL Commands:**
- `:help` - Show help
- `:quit` - Exit REPL
- `:doc <symbol>` - Show documentation (future)
- `:reload` - Reload current namespace (future)

---

### `clorus replx`

Start extended REPL with smart adaptive execution.

**Usage:**
```bash
clorus replx [OPTIONS]
```

**Options:**
- `--main-thread` - Run on main thread
- `--warn` - Warn only, don't auto-fix
- `--silent` - Auto-fix silently
- `--no-adapt` - Disable adaptive features

**Example:**
```bash
$ clorus replx
Clorus Extended REPL v0.1.0 (replx)
Smart adaptive mode enabled

userλ> (+ 1 2)
=> 3
```

**Features:**
- Auto-completion
- Syntax highlighting
- Smart error recovery
- Adaptive evaluation strategies

---

## Configuration

### Clorus.toml

The project manifest file.

**Full Example:**
```toml
[package]
name = "my-project"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "A Clorus project"

[build]
entry = "src/main.clrs"        # Entry point (apps)
src = ["src", "resources"]     # Source directories

[dependencies]
# Clorus .clip packages
json-lib = { path = "libs/json-lib-0.1.0.clip" }
math-utils = { path = "../math-utils" }
http-client = "1.2.0"          # From registry (future)
coral-gfx = { workspace = true }  # From workspace

[rust-dependencies]
# Rust libraries with automatic FFI generation
example-lib = { path = "../example-rust-lib", interface = true }
# Or without interface file (uses function signatures)
coral-gfx-rs = { path = "../coral-core/coral-gfx" }

[link]
# macOS frameworks
frameworks = ["CoreFoundation", "Security", "OpenGL"]
# System libraries
libraries = ["pthread", "m"]

[workspace]
members = ["lib1", "lib2", "examples/*"]
exclude = ["examples/experimental"]

[workspace.package]
version = "1.0.0"
authors = ["Team"]

[workspace.dependencies]
shared-lib = { path = "shared" }
```

### Rust Dependencies

Clorus can automatically compile and link Rust libraries with FFI generation.

**How it works:**
1. Specify Rust crate path in `[rust-dependencies]`
2. Clorus automatically:
   - Compiles the Rust crate with `cargo build`
   - Generates FFI interface from signatures
   - Links the static library into your Clorus executable

**Example - coral-gfx:**

**Rust Library (coral-core/coral-gfx/src/lib.rs):**
```rust
#[no_mangle]
pub extern "C" fn gfx_init() -> i32 {
    // Initialize graphics
    0
}

#[no_mangle]
pub extern "C" fn gfx_clear(r: f64, g: f64, b: f64, a: f64) {
    // Clear screen with color
}
```

**Clorus App (coral-examples/gallery/Clorus.toml):**
```toml
[package]
name = "gallery"
version = "1.0.0"

[rust-dependencies]
# Link to Rust graphics library
coral-gfx = { path = "../../coral-core/coral-gfx", interface = true }

[build]
entry = "src/main.clrs"
```

**Use in Clorus Code:**
```clojure
;; src/main.clrs
(ns gallery.main
  (:rust-import [coral-gfx :as gfx]))

(defn -main []
  (gfx/init)
  (gfx/clear 0.0 0.0 0.0 1.0)  ; Clear to black
  (println "Graphics initialized!"))
```

**Build Process:**
```bash
$ clorus build
   Processing Rust dependencies...
      Compiling coral-gfx (Rust)...
      Generated FFI interface: coral-gfx.clorus-ffi
   Compiling gallery v1.0.0
      Linking coral-gfx.a
    Finished dev [unoptimized] target(s) in 1.2s
```

**Workspace + Rust Dependencies:**

You can combine workspace dependencies with Rust dependencies:

```toml
# Root workspace Clorus.toml
[workspace]
members = ["coral-ui", "examples/*"]

[workspace.dependencies]
# Clorus library
coral-ui = { path = "coral-ui" }

# Member Clorus.toml
[dependencies]
coral-ui = { workspace = true }

[rust-dependencies]
coral-gfx = { path = "../../coral-core/coral-gfx" }
```

This allows mixing Clorus and Rust code seamlessly!

---

## Examples

### Create and Run a Project

```bash
# Create
$ clorus new factorial
     Created binary (application) `factorial` package

# Enter
$ cd factorial

# Edit src/main.clrs
$ cat > src/main.clrs << 'EOF'
(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(factorial 10)
EOF

# Run
$ clorus run
   Compiling factorial v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `target/factorial`

=> 3628800
```

### Create and Use a Library

```bash
# Create library
$ clorus new math-lib
$ cd math-lib

# Edit to be a library (remove entry)
$ cat > Clorus.toml << 'EOF'
[package]
name = "math-lib"
version = "1.0.0"

[build]
# No entry = library
EOF

# Create src/lib.clrs
$ mkdir src
$ cat > src/lib.clrs << 'EOF'
(ns math-lib.core)

(defn square [x] (* x x))
(defn cube [x] (* x x x))
EOF

# Package
$ clorus pack
✅ Successfully created math-lib-1.0.0.clip

# Use in another project
$ cd ../my-app
$ mkdir libs
$ cp ../math-lib/math-lib-1.0.0.clip libs/

# Add to Clorus.toml
$ cat >> Clorus.toml << 'EOF'

[dependencies]
math-lib = { path = "libs/math-lib-1.0.0.clip" }
EOF

# Use in code
$ cat > src/main.clrs << 'EOF'
(require [math-lib.core :as math])

(math/square 5)
EOF

# Build and run
$ clorus run
   Resolving dependencies...
   📦 Loading dependency: math-lib from libs/math-lib-1.0.0.clip
   Compiling my-app v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `target/my-app`

=> 25
```

### Create a Workspace

```bash
# Create workspace root
$ mkdir my-workspace && cd my-workspace

# Create workspace manifest
$ cat > Clorus.toml << 'EOF'
[workspace]
members = ["lib1", "lib2", "app"]

[workspace.dependencies]
lib1 = { path = "lib1" }
lib2 = { path = "lib2" }
EOF

# Create library 1
$ clorus new lib1
$ cat > lib1/Clorus.toml << 'EOF'
[package]
name = "lib1"
version = "1.0.0"

[build]
# No entry = library
EOF

# Create library 2 (depends on lib1)
$ clorus new lib2
$ cat > lib2/Clorus.toml << 'EOF'
[package]
name = "lib2"
version = "1.0.0"

[dependencies]
lib1 = { workspace = true }

[build]
# No entry = library
EOF

# Create app (uses both)
$ clorus new app
$ cat > app/Clorus.toml << 'EOF'
[package]
name = "app"
version = "1.0.0"

[dependencies]
lib1 = { workspace = true }
lib2 = { workspace = true }

[build]
entry = "src/main.clrs"
EOF

# Build everything
$ clorus build --workspace
📦 Building workspace with 3 member(s)
   ...
✅ Workspace build complete - 3 member(s) built

# Package libraries
$ clorus pack --workspace
📦 Packaging workspace to dist/
✅ Packaged 2 member(s):
   📦 lib1-1.0.0.clip
   📦 lib2-1.0.0.clip
```

---

## Tips and Tricks

### Debug Build Output

```bash
# Show detailed compilation info
$ clorus build --debug
   [DEBUG] Loaded 42 expressions from stdlib
   [DEBUG] Loading 3 module(s):
      → coral.widgets
      → coral.utils
      → demos.shapes-demo
   [DEBUG] Total expressions to compile: 156
```

### JIT for Quick Iteration

```bash
# Faster compilation for development
$ clorus run --jit
# Note: Won't work with .clip dependencies
```

### Clean Before Release

```bash
# Ensure fresh build
$ clorus clean
$ clorus build
$ clorus pack
```

### Workspace Package Distribution

```bash
# Package all libraries for distribution
$ clorus pack --workspace --output release/
$ ls release/
lib1-1.0.0.clip
lib2-1.0.0.clip
lib3-1.0.0.clip

# Create tarball
$ tar czf my-libs-v1.0.0.tar.gz release/
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CLORUS_HOME` | Clorus installation directory | (auto-detected) |
| `CLORUS_CACHE` | Cache directory | `~/.clorus/cache` |

---

## See Also

- [CLIP User Guide](CLIP_USER_GUIDE.md) - Package management details
- [Workspace Guide](../WORKSPACE_DEPENDENCY_RESOLUTION.md) - Workspace dependency resolution
- [Rust FFI Guide](RUST_FFI_GUIDE.md) - Rust interop
- [REPL Guide](../features/repl/REPL_FEATURES.md) - REPL features

---

**Documentation Version:** 1.0
**Last Updated:** February 2026
