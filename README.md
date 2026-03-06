# Clorus

A **Clojure-inspired systems programming language** that combines Clojure's elegant syntax with native performance and Rust interop, compiled via LLVM.

**Version:** 0.5.0 Beta
**Status:** ~75-80% Clojure parity - Core language complete!
**Latest:** Polymorphism system + Lazy sequences + Project-aware REPL

## Features

- ✅ **Full Clojure syntax**: S-expressions, destructuring, macros
- ✅ **Native compilation**: LLVM → machine code (via Inkwell)
- ✅ **Interactive REPL**: Project-aware with Clojure-style output
- ✅ **Polymorphism**: Records, Protocols, Multimethods
- ✅ **Lazy sequences**: Memory-efficient stream processing
- ✅ **Package tool**: Cargo-like CLI (`clorus new`, `clorus run`, etc.)
- ✅ **Rust FFI**: Direct Rust integration
- ✅ **Concurrency**: Atoms, channels, go blocks, agents
- ✅ **Standard library**: 50+ functions in pure Clorus

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/clorus.git
cd clorus

# Build the compiler and tools
cargo build --release

# Add to PATH (optional)
export PATH="$PWD/target/release:$PATH"
```

### Create Your First Project

```bash
# Create a new project
clorus new my-app
cd my-app

# Run it
clorus run
```

### Try the REPL

```bash
# In a project directory - loads project context
clorus repl

# Or run directly
cargo run --bin repl
```

## Building from Source

### Prerequisites
- Rust 1.70+ (2024 edition)
- LLVM 17+ (via Inkwell)
- Cargo

### Build All Binaries

```bash
# Build everything in release mode
cargo build --release

# This creates:
# - target/release/clorus         # Main CLI tool
# - target/release/repl            # Interactive REPL
# - target/release/clorus-ffi-gen  # FFI generator
```

### Build Individual Components

```bash
# CLI tool only
cargo build --release --bin clorus

# REPL only
cargo build --release --bin repl

# Run tests
cargo test

# Check code
cargo check
```

### Development Build (faster)

```bash
# Unoptimized debug build
cargo build

# Run without installing
cargo run --bin clorus -- new my-project
cargo run --bin repl
```

## Usage

### Package Tool Commands

```bash
clorus new <name>        # Create new project
clorus run               # Compile and run (JIT engine by default)
clorus run --legacy-run  # Compile and run with legacy non-JIT engine
clorus check             # Check syntax
clorus build             # Build to object file
clorus repl              # Start project-aware REPL
clorus help              # Show help
```

See `PACKAGE_TOOL.md` for complete documentation.

## Execution Modes

- `run` and `repl` use the JIT path by default.
- Legacy engine is still available for parity and regression checks via `--legacy-run` in `run`.
- Production packaging/build flows should use AOT artifacts (`clorus build` / `clorus pack`).
- CI/regression should run both engines:

```bash
CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh
```

## Environment Variables

This is the current complete list of `CLORUS_*` environment variables used by
runtime code, REPL/CLI code, and first-party test/debug scripts.

### Runtime + REPL/CLI

| Variable | Default | Scope | Purpose |
|---|---|---|---|
| `CLORUS_HOME` | unset | CLI, REPL, REPLx, pack/build helpers | Installation root used to locate `lib/` and `stdlib/` (fallback is `~/.clorus/...` in several paths). |
| `CLORUS_REPL_NO_CORE_LIB` | `0` (disabled) | REPL | If set to any value except `"0"`, skip loading `clorus-core` dylib fallback. |
| `CLORUS_REPL_LOAD_STDLIB` | `1` (enabled) | REPL | If set to any value except `"0"`, JIT-load stdlib (`clorus.core`) at REPL startup (default path). |
| `CLORUS_DEBUG_REPL` | unset | REPL, REPL engine | Verbose REPL debug logging. |
| `CLORUS_DEBUG_IR` | unset | REPL engine | On LLVM verify failure, dumps REPL IR to `/tmp/clorus_repl_ir.ll`. |
| `CLORUS_DEBUG_RUNTIME` | unset | REPL internals | Debug logging for runtime-library discovery (diagnostics only). |
| `CLORUS_SAFE_VALUE` | unset | Runtime | Safety valve: skips `Value` deallocation (leaks memory) to avoid crashes while debugging memory corruption. |
| `CLORUS_SAFE_VECTOR` | unset | Runtime | Safety valve: skips vector internals release (leaks vector memory) for debugging. |
| `CLORUS_DEBUG_RELEASE` | unset | Runtime | Enables verbose retain/release/deallocation logging. |
| `CLORUS_GUARD_RELEASE` | unset | Runtime | Adds guard checks (notably release on `refcount=0`). Used with `CLORUS_DEBUG_RELEASE=1`. |
| `CLORUS_DEBUG_RELEASE_BT` | unset | Runtime | Adds backtraces for retain/release/double-free diagnostic logs. |
| `CLORUS_DEBUG_ALLOC_BT` | unset | Runtime | Captures allocation backtraces for values (high overhead). |
| `CLORUS_DEBUG_PTR` | unset | Runtime | Pointer filter for debug logs (hex `0x...` or decimal). |
| `CLORUS_DEBUG_TAG` | unset | Runtime | Tag filter for debug logs (example: `Vector`, `Long`, `Double`). |
| `CLORUS_DEBUG_VECTOR_RELEASE` | unset | Runtime/vector | Extra vector double-release diagnostics. |
| `CLORUS_ENTRY_FILE` | unset | CLI `run`/`build` | Overrides manifest entry file for one invocation (used by test runners to execute per-file suites without rewriting `Clorus.toml`). |

### Test + Debug Script Variables

| Variable | Default | Used by | Purpose |
|---|---|---|---|
| `CLORUS_BIN` | Script-specific | `tests/run_all_tests.sh`, `tests/generate_metrics.sh`, `scripts/test/run-all.sh`, `trace_double_free.sh` | Path to `clorus` binary used by scripts. |
| `CLORUS_TEST_ENGINES` | `jit legacy` | `tests/run_all_tests.sh` | Engine matrix for semantics runs. Values: `jit`, `legacy`. |
| `CLORUS_TEST_JOBS` | `2` | `tests/run_all_tests.sh` | Per-test engine parallelism (`1` = sequential, `2` = parallel `jit`+`legacy`). |
| `CLORUS_TEST_TIMEOUT_SECONDS` | `20` | `tests/run_all_tests.sh` | Per-test timeout used via `timeout`/`gtimeout` when available. |
| `CLORUS_TEST_ISOLATE` | `1` | `tests/run_all_tests.sh` | Isolate per-engine test artifacts (`CARGO_TARGET_DIR`, `TMPDIR`) to avoid parallel engine collisions. |
| `CLORUS_TEST_ISOLATION_ROOT` | `/tmp/clorus-test-isolation` | `tests/run_all_tests.sh` | Root directory for isolated per-engine test artifacts. |
| `CLORUS_APP_DIR` | `~/Workspace/github/coral/coral-examples/gallery` | `trace_double_free.sh` | Working directory for gallery-based double-free tracing. |

### Notes

- `CLORUS_CACHE` is mentioned in some docs, but is not currently consumed by
  runtime/CLI code paths.
- `clorus run` now defaults to JIT. Legacy non-JIT run path is explicit:
  `clorus run --legacy-run`.
- `clorus repl` also runs on the JIT execution path by default.

## Rust Interop (Current)

- Canonical file/module import style is `ns` + `:rust`:

```clojure
(ns main
  (:rust [libm :as m]))
```

- In interactive REPL usage, `(use rust.<lib>)` is still supported for compatibility.
- Preferred interop contract file is `.clri` (legacy `.clorus-ffi` remains supported).
- Canonical status and limits are tracked in:
  - `docs/RUST_INTEROP_STATUS.md`

### Example Workflow

```bash
# Create a new project
$ clorus new hello-world
     Created binary (application) `hello-world` package

# Edit src/main.clrs
$ cd hello-world
$ cat > src/main.clrs << EOF
(ns main)

(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(defn -main [& _args]
  (factorial 5))
EOF

# Run it
$ clorus run
   Compiling hello-world v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `src/main.clrs`

=> 120
```

## Project Structure

**Modular Workspace:**

```
clorus/
├── crates/
│   ├── clorus-syntax/      # Lexer, Parser, AST (no deps!)
│   ├── clorus-codegen/     # LLVM code generation
│   ├── clorus/             # Main library (re-exports all)
│   ├── clorus-cli/         # Package tool (like Cargo)
│   └── clorus-repl/        # Interactive REPL
└── Cargo.toml              # Workspace root
```

See `MODULAR_ARCHITECTURE.md` for details.

## Current Status (v0.5.0 Beta)

### ✅ Core Language (~75-80% Clojure Parity)

**Data Types**
- Numbers (f64), Strings, Booleans, Keywords, Nil
- Vectors `[1 2 3]`, Maps `{:a 1}`, Lists `'(1 2)`, Sets `#{1 2}`

**Functions**
- Named functions `defn`, anonymous `fn`, shorthand `#(+ % 1)`
- Multi-arity, variadic (`& rest`), closures
- Higher-order: `map`, `filter`, `reduce`, `apply`

**Control Flow**
- `if`, `when`, `when-not`, `if-let`, `when-let`
- `cond`, `case`, `and`, `or`
- `loop`/`recur` with tail-call optimization
- `while`, `dotimes`, `doseq`

**Macros**
- `defmacro`, `quote`, syntax-quote, unquote, `gensym`
- All control flow implemented as macros

**Destructuring**
- Vector: `[a b c]`, `[a & rest]`, `[a [b c]]`
- Map: `{:keys [x y]}`
- Works in `let` and function parameters

**Polymorphism** - **NEW!**
- Records: `defrecord` with constructor functions
- Protocols: `defprotocol`/`extend-type` for type-based dispatch
- Multimethods: `defmulti`/`defmethod` for custom dispatch

**Lazy Sequences** - **NEW!**
- Infinite sequences: `lazy-range`, `lazy-repeat`, `lazy-cycle`
- Transformations: `lazy-map`, `lazy-filter`, `lazy-take`
- Memory-efficient stream processing

**State Management**
- Atoms: `atom`, `swap!`, `reset!`, `deref`/@
- Refs, Agents, Channels (CSP), Transactions (STM)

**Namespaces**
- `ns`, `require`, `use`, `:as`, `:refer`

**Collections API**
- 50+ functions: `get`, `assoc`, `dissoc`, `conj`, `keys`, `vals`
- `merge`, `get-in`, `assoc-in`, `select-keys`
- `take`, `drop`, `concat`, `flatten`, `distinct`

**Standard Library**
- 50+ utility functions in pure Clorus (`stdlib/clorus/core.clr`)
- `partial`, `comp`, `zipmap`, `frequencies`, `group-by`
- Predicates: `nil?`, `even?`, `empty?`, etc.
- Math: `abs`, `min`, `max`, `sum`

**String Operations**
- `str`, `subs`, `split`, `join`, `upper-case`, `lower-case`
- `trim`, `replace`, `starts-with?`, `ends-with?`

**Exception Handling**
- `try`, `catch`, `finally`, `throw`

**FFI**
- Direct Rust integration
- File I/O: `slurp`, `spit`
- rust.fs module for file operations

### 🚧 Not Yet Implemented

- AOT compilation to standalone executables
- Automatic dispatch for protocols/multimethods (requires explicit function names)
- Regular expressions
- Advanced STM features
- Transducers
- Full Clojure.spec compatibility

## Syntax Examples

### Simple Arithmetic
```clojure
(+ 1 2 3)        ; => 6
(* 2 (+ 3 4))    ; => 14
```

### Variables
```clojure
(def x 10)
(def y 20)
(+ x y)          ; => 30
```

### Functions
```clojure
(defn square [x]
  (* x x))

(square 5)       ; => 25
```

### Recursion - Fibonacci
```clojure
(defn fib [n]
  (if (< n 2)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(fib 10)         ; => 55
```

### Recursion - Factorial
```clojure
(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(factorial 5)    ; => 120
```

### Control Flow
```clojure
(def age 25)

(if (< age 18)
  100  ; Minor
  200) ; Adult
       ; => 200
```

### Let Bindings
```clojure
(let [x 10
      y 20
      z (+ x y)]
  (* z 2))       ; => 60
```

## Interactive REPL

The REPL is **project-aware** and shows **Clojure-style output**:

```bash
$ clorus repl

╔════════════════════════════════════╗
║  Clorus REPL v0.2.0                ║
║  Clojure-inspired systems language ║
╚════════════════════════════════════╝

📦 Project: my-app v0.1.0
📂 Namespace: my_app.core

✓ rust.fs module available
✓ clorus.core module available

my_app.coreλ> (def x 10)
#'my_app.core/x

my_app.coreλ> (defn square [n] (* n n))
#'my_app.core/square

my_app.coreλ> (square x)
100

my_app.coreλ> :quit
Goodbye!
```

### REPL Features

- ✅ **Project-aware**: Auto-loads namespace from `Clorus.toml`
- ✅ **Clojure output**: Shows `#'namespace/name` for def/defn
- ✅ **Persistent state**: Variables and functions persist across lines
- ✅ **Auto-complete**: TAB completion for built-in functions
- ✅ **History**: ↑/↓ navigation with persistent history file
- ✅ **Commands**: `:help`, `:examples`, `:quit`

See `docs/REPL_AND_CLEANUP_COMPLETE.md` for details.

## Usage as a Library

```rust
use clorus::{parse, CodeGen};
use inkwell::context::Context;
use inkwell::OptimizationLevel;

fn main() {
    let code = "(+ (* 2 3) 4)";  // Result: 10

    // Parse
    let exprs = parse(code).unwrap();

    // Compile
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "my_module");
    codegen.wrap_in_function(&exprs[0], "eval").unwrap();

    // Execute with JIT
    let engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    unsafe {
        type EvalFunc = unsafe extern "C" fn() -> f64;
        let jit_fn = engine.get_function::<EvalFunc>("eval").unwrap();
        println!("Result: {}", jit_fn.call());  // Prints: Result: 10
    }
}
```

## Rust Interop Example (Future)

```clojure
; Declare external Rust function
(extern "Rust" calculate_pi [] f64)

; Call Rust function from Clorus
(def pi (calculate_pi))
(println "Pi is approximately" pi)
```

## Design Philosophy

1. **Simple and modular**: Each component is a self-contained crate
2. **Clojure syntax**: Keep the elegant, minimal syntax of Clojure
3. **Rust-level performance**: Compile to efficient native code
4. **Easy interop**: First-class Rust FFI support
5. **Developer-friendly**: Cargo-like tooling for familiar workflow

## Roadmap

### Phase 1: Core Language ✅ COMPLETE
- ✅ Basic expressions and arithmetic
- ✅ Functions and recursion
- ✅ Control flow (all macros)
- ✅ Global and local variables
- ✅ Data types (numbers, strings, booleans, collections)
- ✅ Destructuring (vector and map)
- ✅ Macros system
- ✅ Polymorphism (records, protocols, multimethods)
- ✅ Lazy sequences

### Phase 2: Tooling ✅ MOSTLY COMPLETE
- ✅ Project scaffolding (`clorus new`)
- ✅ JIT compilation (`clorus run`)
- ✅ Syntax checking (`clorus check`)
- ✅ Interactive REPL (project-aware)
- ⏳ AOT compilation to native executables
- ⏳ Testing framework (`clorus test`)
- ⏳ Package management (dependencies)

### Phase 3: Polish & Advanced Features (In Progress)
- ✅ Standard library (50+ functions)
- ⏳ Automatic protocol/multimethod dispatch
- ⏳ Regular expressions
- ⏳ Transducers
- ⏳ Full STM implementation
- ⏳ Advanced documentation tooling
- ⏳ **Self-hosting** (compiler in Clorus!)

## Documentation

- [`docs/LANGUAGE_PARITY.md`](docs/LANGUAGE_PARITY.md) - Complete feature comparison with Clojure
- [`docs/COVERAGE_ASSESSMENT.md`](docs/COVERAGE_ASSESSMENT.md) - Implementation status
- [`docs/REPL_AND_CLEANUP_COMPLETE.md`](docs/REPL_AND_CLEANUP_COMPLETE.md) - Latest improvements
- [`docs/implementation/`](docs/implementation/) - Feature implementation docs
- `PACKAGE_TOOL.md` - Package tool documentation
- `MODULAR_ARCHITECTURE.md` - Crate architecture

## Contributing

Clorus is under active development. Contributions welcome!

## License

MIT OR Apache-2.0
