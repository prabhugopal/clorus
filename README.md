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
clorus run               # Compile and run
clorus check             # Check syntax
clorus build             # Build to object file
clorus repl              # Start project-aware REPL
clorus help              # Show help
```

See `PACKAGE_TOOL.md` for complete documentation.

### Example Workflow

```bash
# Create a new project
$ clorus new hello-world
     Created binary (application) `hello-world` package

# Edit src/main.clrs
$ cd hello-world
$ cat > src/main.clrs << EOF
(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(factorial 5)
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
- 50+ utility functions in pure Clorus (`stdlib/core.clr`)
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
