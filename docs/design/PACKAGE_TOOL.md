# Clorus Package Tool

A Cargo-like build tool for the Clorus programming language.

**Last Updated:** February 2026
**Status:** Production-ready with workspace support

---

## Overview

The Clorus CLI provides a complete build toolchain for Clorus projects:
- Project scaffolding (`clorus new`)
- Compilation to native executables (`clorus build`)
- Package management (`.clip` format)
- Workspace/monorepo support
- Rust FFI integration
- Interactive REPL

## Installation

Build the Clorus CLI tool:
```bash
cargo build --release --bin clorus
```

Optionally install it:
```bash
cargo install --path crates/clorus-cli
```

## Quick Start

### Create a New Project
```bash
clorus new my-project
cd my-project
```

This creates:
```
my-project/
├── Clorus.toml      # Project manifest
├── .gitignore       # Git ignore file
└── src/
    └── main.clrs    # Main source file
```

### Run Your Project
```bash
clorus run
```

Output:
```
   Compiling my-project v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `src/main.clrs`

=> 30
```

## Commands

### Core Commands

| Command | Description | Status |
|---------|-------------|--------|
| `new <name>` | Create a new project | ✅ Complete |
| `build` | Compile to native executable | ✅ Complete |
| `run` | Compile and execute | ✅ Complete |
| `check` | Syntax checking only | ✅ Complete |
| `clean` | Remove build artifacts | ✅ Complete |
| `pack` | Package as .clip library | ✅ Complete |
| `install` | Install .clip package | ✅ Complete |
| `repl` | Interactive REPL | ✅ Complete |
| `replx` | Extended REPL | ✅ Complete |

### Workspace Commands

| Command | Description | Status |
|---------|-------------|--------|
| `build --workspace` | Build all members | ✅ Complete |
| `clean --workspace` | Clean all members | ✅ Complete |
| `pack --workspace` | Package all libraries | ✅ Complete |

### Build Features

**✅ Implemented:**
- AOT compilation to native executables
- JIT mode for fast iteration (`--jit`)
- Automatic dependency resolution (.clip packages)
- Module system with recursive loading
- Rust FFI with automatic interface generation
- Debug mode with memory tracking (`--debug`)
- Workspace/monorepo support
- Static and dynamic linking

---

## Project Manifest (Clorus.toml)

**Application Example:**
```toml
[package]
name = "my-app"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "A Clorus application"

[build]
entry = "src/main.clrs"  # Entry point for applications

[dependencies]
json-lib = { path = "libs/json-lib-1.0.0.clip" }
math-utils = { workspace = true }  # From workspace

[rust-dependencies]
graphics-lib = { path = "../graphics-rs", interface = true }

[link]
frameworks = ["OpenGL", "CoreFoundation"]  # macOS
libraries = ["pthread", "m"]
```

**Library Example:**
```toml
[package]
name = "mylib"
version = "1.0.0"

[build]
# No entry = library
# Creates mylib.o and mylib.bc for linking

[dependencies]
string-utils = "0.5.0"
```

**Workspace Example:**
```toml
[workspace]
members = ["lib1", "lib2", "examples/*"]
exclude = ["examples/experimental"]

[workspace.package]
version = "1.0.0"
authors = ["Team"]

[workspace.dependencies]
lib1 = { path = "lib1" }
lib2 = { path = "lib2" }
```

---

## Examples

### Simple Arithmetic
```clojure
; src/main.clrs
(def x 10)
(def y 20)
(+ x y)
```

Run with:
```bash
$ clorus run
=> 30
```

### Functions
```clojure
; src/main.clrs
(defn square [x]
  (* x x))

(defn sum-of-squares [a b]
  (+ (square a) (square b)))

(sum-of-squares 3 4)
```

Run with:
```bash
$ clorus run
=> 25
```

### Recursion - Fibonacci
```clojure
; src/main.clrs
(def n 10)

(defn fib [x]
  (if (< x 2)
    x
    (+ (fib (- x 1)) (fib (- x 2)))))

(fib n)
```

Run with:
```bash
$ clorus run
=> 55
```

### Recursion - Factorial
```clojure
; src/main.clrs
(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(factorial 5)
```

Run with:
```bash
$ clorus run
=> 120
```

### Control Flow
```clojure
; src/main.clrs
(def age 25)

(if (< age 18)
  (def status 100)  ; Minor
  (def status 200)) ; Adult

status
```

Run with:
```bash
$ clorus run
=> 200
```

## Current Features (February 2026)

### Language Features ✅
- ✅ **Complete type system** - Long, Double, String, Bool, Nil
- ✅ **Collections** - Vectors, Maps, HashSets, Lists
- ✅ **Functions** - defn with parameters, recursion, closures
- ✅ **Control flow** - if, cond, case, when, when-not
- ✅ **Local bindings** - let, letfn
- ✅ **Loops** - loop/recur, doseq, for
- ✅ **Macros** - Full macro system
- ✅ **Namespaces** - ns, require, :as, :refer
- ✅ **Concurrency** - Atoms, Refs/STM, Agents, Channels/CSP
- ✅ **Lazy sequences** - Full lazy evaluation support
- ✅ **String operations** - str, subs, split, join, etc.
- ✅ **Transducers** - Composable transformations

### Build Tool Features ✅
- ✅ **Project scaffolding** - `clorus new`
- ✅ **AOT compilation** - Native executables
- ✅ **JIT mode** - Fast iteration (`--jit`)
- ✅ **Dependency management** - .clip packages
- ✅ **Package system** - `clorus pack`, `clorus install`
- ✅ **Workspaces** - Multi-package monorepo support
- ✅ **Rust FFI** - Automatic interface generation
- ✅ **Interactive REPL** - With .clip support
- ✅ **Debug mode** - Memory tracking (`--debug`)

### Future Features 🚧
- 🚧 **Package registry** - Central package hosting
- 🚧 **Testing framework** - `clorus test`
- 🚧 **Documentation generation** - `clorus doc`
- 🚧 **Optimization levels** - `--release` flag
- 🚧 **Cross-compilation** - Multi-platform builds
- 🚧 **Benchmarking** - `clorus bench`

---

## Comparison with Cargo

| Feature | Cargo | Clorus | Status |
|---------|-------|--------|--------|
| `new` | ✅ | ✅ | Complete |
| `build` | ✅ | ✅ | AOT + JIT modes |
| `run` | ✅ | ✅ | Complete |
| `check` | ✅ | ✅ | Complete |
| `clean` | ✅ | ✅ | Complete |
| `pack` (publish) | ✅ | ✅ | Local only |
| `install` | ✅ | ✅ | Local/global |
| Dependencies | ✅ | ✅ | .clip packages |
| Workspaces | ✅ | ✅ | Complete |
| Publishing | ✅ | 🚧 | Planned |
| Native executables | ✅ | ✅ | Complete |
| `test` | ✅ | 🚧 | Planned |
| `bench` | ✅ | 🚧 | Planned |
| `doc` | ✅ | 🚧 | Planned |

---

## Implementation Details

### AOT Mode (Default)
- Ahead-of-time compilation to native code
- Creates standalone executables
- Can distribute binaries without Clorus toolchain
- Full .clip dependency support
- Static and dynamic linking

### JIT Mode (`--jit`)
- Uses LLVM JIT compilation
- Compiles and runs code in memory
- Fast iteration during development
- Limited .clip support (bitcode only)
- Best for quick testing

---

## Contributing

The Clorus project is under active development. Contributions are welcome!

## License

MIT OR Apache-2.0
