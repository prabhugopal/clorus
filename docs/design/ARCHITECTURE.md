# Clorus Crate Architecture

## Current Structure (Monolithic)

Currently, everything is in one crate:

```
clorus/
└── src/
    ├── lib.rs      # Main library
    ├── ast.rs      # AST definitions
    ├── lexer.rs    # Lexer
    ├── parser.rs   # Parser
    ├── codegen.rs  # LLVM codegen
    ├── main.rs     # Demo binary
    └── bin/
        └── repl.rs # REPL binary
```

**Pros:**
- Simple to work with
- Fast iteration
- No dependency management between crates
- Good for early development

**Cons:**
- All components coupled together
- Cannot version independently
- Harder to reuse parts in other projects

---

## Proposed Modular Structure

Split into multiple crates for better modularity:

```
clorus/
├── crates/
│   ├── clorus-syntax/      # Lexer, Parser, AST
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ast.rs
│   │   │   ├── lexer.rs
│   │   │   └── parser.rs
│   │   └── Cargo.toml
│   │
│   ├── clorus-codegen/     # LLVM code generation
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── codegen.rs
│   │   └── Cargo.toml
│   │
│   ├── clorus/             # Main library (re-exports)
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   └── clorus-repl/        # REPL binary
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
│
├── Cargo.toml              # Workspace root
└── README.md
```

### Crate Breakdown

#### 1. `clorus-syntax`
**Purpose:** Frontend - parsing source code into AST

**Dependencies:** None (pure Rust)

**Exports:**
- `Expr` (AST)
- `Lexer`
- `Parser`
- `parse()` function

**Use case:** Can be used independently for:
- Syntax highlighting
- Code formatters
- Linters
- IDEs/editors

#### 2. `clorus-codegen`
**Purpose:** Backend - compiling AST to machine code

**Dependencies:**
- `clorus-syntax` (for AST types)
- `inkwell` (LLVM bindings)

**Exports:**
- `CodeGen`
- Compilation functions

**Use case:**
- Different codegen backends (LLVM, Cranelift, custom)
- AOT vs JIT compilation
- Optimization passes

#### 3. `clorus`
**Purpose:** Main library - convenient all-in-one

**Dependencies:**
- `clorus-syntax`
- `clorus-codegen`

**Exports:** Re-exports everything

**Use case:** Simple `use clorus::*` for users

#### 4. `clorus-repl`
**Purpose:** Interactive REPL binary

**Dependencies:**
- `clorus` (main library)

**Use case:** Standalone REPL application

---

## Recommended Approach

### Option 1: Keep it Simple (Now)
✅ **Recommended for current stage**

Stay with monolithic structure until:
- Language features are more stable
- API boundaries are clear
- You have 3-5 major features working

**When to split:**
- After implementing variables + functions
- When you want to add alternative backends
- When external tools want to use just the parser

### Option 2: Refactor to Modules (Later)
Do this when:
- You want to version components separately
- External projects want to use parts of clorus
- You're building IDE tools (syntax highlighting, etc.)
- You want to swap LLVM for another backend

### Option 3: Hybrid Approach
Keep main code in one crate, but create separate crates for:
- `clorus-lsp` - Language Server Protocol
- `clorus-fmt` - Code formatter
- `clorus-std` - Standard library (when you have one)

---

## Decision Matrix

| Aspect | Monolithic | Modular |
|--------|-----------|---------|
| Development speed | ⚡ Fast | 🐢 Slower |
| Compile time | ✅ Quick | ⏱️ Longer |
| Reusability | ❌ Limited | ✅ High |
| Versioning | Simple | Complex |
| Testing | Easy | Per-crate |
| Best for | Early dev | Mature projects |

---

## My Recommendation

**Stay monolithic for now.** Here's why:

1. You're still building core features (variables, functions, control flow)
2. Fast iteration is more important than perfect structure
3. You can refactor to modules later (Rust makes this easy)
4. The current structure is already well-organized with modules

**Refactor to multi-crate when:**
- Someone wants to use just the parser for syntax highlighting
- You want to experiment with different backends (not just LLVM)
- The project grows beyond 10k lines of code
- You have external contributors working on different parts

---

## How to Refactor (When Ready)

1. Create workspace `Cargo.toml`:
```toml
[workspace]
members = [
    "crates/clorus-syntax",
    "crates/clorus-codegen",
    "crates/clorus",
    "crates/clorus-repl",
]
resolver = "2"
```

2. Move code into separate crates
3. Update dependencies between crates
4. Test everything still works
5. Update documentation

The refactor takes ~2-3 hours but is straightforward in Rust.

---

## Questions?

- Want to stay monolithic? ✅ Keep current structure
- Want to split now? I can help refactor
- Hybrid approach? We can extract specific parts

What do you prefer?
