# Clorus Quick Start

## Installation

```bash
cd ~/Learning/git/clorus
cargo build --release
```

## Usage

### 1. Interactive REPL (Recommended!)

Start the REPL for interactive development:

```bash
cargo run --bin repl
```

```clojure
λ> (+ 1 2)
=> 3

λ> (- 10 3)
=> 7

λ> (* 6 7)
=> 42

λ> (+ (* 2 3) 4)
=> 10

λ> :help
[shows available commands]

λ> :examples
[shows example expressions]

λ> :quit
Goodbye!
```

### 2. Run Examples

See all working examples:

```bash
cargo run
```

This will compile and execute a suite of test expressions.

### 3. Run Tests

Verify everything works:

```bash
cargo test
```

## What You Can Do Right Now

### Arithmetic Operations

```clojure
(+ 1 2 3 4)              ; Addition
(- 10 3)                 ; Subtraction
(- 5)                    ; Unary negation => -5
(* 2 3 4)                ; Multiplication
(/ 20 4)                 ; Division
```

### Nested Expressions

```clojure
(+ (* 2 3) 4)            ; => 10
(- (* 10 5) (+ 3 7))     ; => 40
(/ (+ 10 20) (- 10 5))   ; => 6
```

### Comparisons

```clojure
(< 5 10)                 ; => 1.0 (true)
(> 5 10)                 ; => 0.0 (false)
(= 42 42)                ; => 1.0 (true)
```

## REPL Commands

- `:help`, `:h` - Show help
- `:examples`, `:e` - Show examples
- `:quit`, `:q` - Exit REPL

## How It Works

```
Your Code → Parser → AST → LLVM CodeGen → JIT → Execute
```

Every expression you type is:
1. **Parsed** into an Abstract Syntax Tree
2. **Compiled** to LLVM IR (optimized native code)
3. **JIT compiled** to machine code
4. **Executed** immediately

## Next Steps

See `ROADMAP.md` for upcoming features:
- Variables and bindings
- Function definitions
- Control flow (if/when)
- Rust FFI for calling Rust libraries

## Project Structure

```
clorus/
├── src/
│   ├── lib.rs        # Library interface
│   ├── main.rs       # Demo binary
│   ├── bin/
│   │   └── repl.rs   # REPL binary
│   ├── ast.rs        # AST definitions
│   ├── lexer.rs      # Tokenizer
│   ├── parser.rs     # Parser
│   └── codegen.rs    # LLVM code generation
└── target/
    └── debug/
        ├── clorus    # Demo binary
        └── repl      # REPL binary
```

## Using as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
clorus = { path = "../clorus" }
inkwell = { version = "0.6.0", features = ["llvm18-1"] }
```

Then use it:

```rust
use clorus::{parse, CodeGen};
use inkwell::context::Context;

let exprs = parse("(+ 1 2)").unwrap();
let context = Context::create();
let codegen = CodeGen::new(&context, "my_module");
// ... compile and execute
```

## Tips

1. **Start with the REPL** - It's the fastest way to experiment
2. **Try nested expressions** - See how LLVM optimizes them
3. **Check the tests** - See `src/*/tests` for examples
4. **Read ROADMAP.md** - See what's coming next

Enjoy building with Clorus! 🚀
