# Clorus Modular Architecture

## Workspace Structure

Clorus is organized as a Cargo workspace with 5 focused crates:

```
clorus/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── clorus-syntax/          # Frontend: Lexer, Parser, AST
│   ├── clorus-codegen/         # Backend: LLVM code generation
│   ├── clorus/                 # Main library (re-exports all)
│   ├── clorus-cli/             # CLI tool and demos
│   └── clorus-repl/            # Interactive REPL
└── target/                     # Build artifacts

```

## Crate Breakdown

### 1. `clorus-syntax` - Frontend (No Dependencies!)

**Purpose:** Pure Rust syntax processing

**Contains:**
- `ast.rs` - Abstract Syntax Tree definitions
- `lexer.rs` - Tokenizer (Clojure syntax → tokens)
- `parser.rs` - Parser (tokens → AST)

**Dependencies:** None! Pure Rust.

**Use Cases:**
- Syntax highlighting in editors
- Code formatters
- Linters and static analysis
- IDE integrations
- Any tool that needs to parse Clorus code

**Example:**
```rust
use clorus_syntax::{parse, Expr};

let exprs = parse("(+ 1 2)").unwrap();
// AST: [List([Symbol("+"), Number(1), Number(2)])]
```

---

### 2. `clorus-codegen` - Backend (LLVM)

**Purpose:** Compile AST to native code

**Contains:**
- `codegen.rs` - LLVM IR generation
- Symbol table management
- Function definitions
- Variable handling

**Dependencies:**
- `clorus-syntax` - For AST types
- `inkwell` - LLVM bindings

**Use Cases:**
- Compile Clorus to native binaries
- JIT execution
- AOT compilation
- Custom compilation pipelines

**Example:**
```rust
use clorus_syntax::parse;
use clorus_codegen::CodeGen;
use inkwell::context::Context;

let exprs = parse("(+ 1 2)").unwrap();
let context = Context::create();
let mut codegen = CodeGen::new(&context, "my_module");

codegen.wrap_in_function(&exprs[0], "eval").unwrap();
// Now JIT execute or emit object file
```

---

### 3. `clorus` - Main Library (Re-exports)

**Purpose:** Convenient all-in-one library

**Contains:**
- Re-exports from `clorus-syntax`
- Re-exports from `clorus-codegen`
- Re-exports `inkwell` for convenience

**Dependencies:**
- `clorus-syntax`
- `clorus-codegen`

**Use Cases:**
- Simple imports: `use clorus::*;`
- End-to-end compilation
- Embedding Clorus in applications

**Example:**
```rust
use clorus::{parse, CodeGen};
use clorus::inkwell::context::Context;

// Everything from one import!
let exprs = parse("(+ 1 2)").unwrap();
let context = Context::create();
let mut codegen = CodeGen::new(&context, "main");
```

---

### 4. `clorus-cli` - Command-Line Tool

**Purpose:** Standalone CLI binary

**Binary:** `clorus`

**Contains:**
- Demo programs
- End-to-end examples
- Showcases language features

**Dependencies:**
- `clorus` (main library)

**Usage:**
```bash
cargo run --bin clorus
# Runs demo showing all features
```

---

### 5. `clorus-repl` - Interactive REPL

**Purpose:** Read-Eval-Print Loop

**Binary:** `repl`

**Contains:**
- Interactive shell
- Expression evaluation
- Help system
- Examples

**Dependencies:**
- `clorus` (main library)

**Usage:**
```bash
cargo run --bin repl

λ> (+ 1 2)
=> 3

λ> (let [x 10] (* x x))
=> 100

λ> :help
```

---

## Benefits of This Architecture

### 1. **Modularity**
- Each crate has a single responsibility
- Clear boundaries between components
- Easy to understand and navigate

### 2. **Reusability**
- Use just `clorus-syntax` for tooling (no LLVM dependency!)
- Use just `clorus-codegen` for custom backends
- Mix and match as needed

### 3. **Independent Versioning**
- Syntax can evolve independently of codegen
- Breaking changes isolated to specific crates
- Semantic versioning per component

### 4. **Faster Compilation**
- Parallel compilation of crates
- Only recompile what changed
- Incremental builds more effective

### 5. **Testing**
- Test each component in isolation
- Clear test boundaries
- Easier to maintain test suites

### 6. **Future Extensions**

Easy to add:
- `clorus-lsp` - Language Server Protocol
- `clorus-fmt` - Code formatter
- `clorus-std` - Standard library
- `clorus-wasm` - WebAssembly backend
- `clorus-cranelift` - Alternative backend

---

## Dependency Graph

```
clorus-cli ─────┐
                ├──> clorus ──┬──> clorus-syntax (no deps!)
clorus-repl ────┘             └──> clorus-codegen ──> inkwell
                                              └──> clorus-syntax
```

**Key Point:** `clorus-syntax` has NO dependencies!
This means tools can use just the parser without needing LLVM.

---

## Building and Testing

```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p clorus-syntax

# Test everything
cargo test

# Test specific crate
cargo test -p clorus-codegen

# Run CLI
cargo run --bin clorus

# Run REPL
cargo run --bin repl

# Build release
cargo build --release
```

---

## Using as Dependencies

### In Your Project

**For tooling (syntax only):**
```toml
[dependencies]
clorus-syntax = { path = "path/to/clorus/crates/clorus-syntax" }
```

**For compilation:**
```toml
[dependencies]
clorus-codegen = { path = "path/to/clorus/crates/clorus-codegen" }
inkwell = { version = "0.6.0", features = ["llvm18-1"] }
```

**For everything:**
```toml
[dependencies]
clorus = { path = "path/to/clorus/crates/clorus" }
```

---

## Migration from Monolithic

The transition from the previous monolithic structure was seamless:
- All functionality preserved
- All tests still pass
- Same public API
- Better organization

Old structure was in `src/`, new structure is in `crates/`.

---

## Summary

| Crate | Purpose | Dependencies | Size |
|-------|---------|--------------|------|
| `clorus-syntax` | Parse Clojure code | None | Small |
| `clorus-codegen` | Compile to LLVM | syntax, inkwell | Medium |
| `clorus` | All-in-one library | syntax, codegen | Tiny (re-exports) |
| `clorus-cli` | Demo binary | clorus | Small |
| `clorus-repl` | Interactive shell | clorus | Small |

**Total:** Clean, modular, maintainable architecture! 🚀
