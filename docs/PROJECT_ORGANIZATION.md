# Clorus Project Organization Guide

## Overview

This document describes the recommended organization of the Clorus project, separating runtime/compiler code (Rust) from standard library code (Clorus).

**Principle:** Keep compiler/runtime infrastructure separate from language stdlib to maintain clear boundaries and enable stdlib-in-Clorus development.

**Date:** January 27, 2026
**Version:** 1.0.0

---

## Proposed Directory Structure

```
clorus/
├── crates/                      # Rust compiler & runtime infrastructure
│   ├── clorus-syntax/           # Parser, lexer, AST
│   ├── clorus-codegen/          # LLVM code generation
│   ├── clorus-runtime/          # Runtime primitives (FFI)
│   ├── clorus-cli/              # CLI tool
│   └── clorus-repl/             # REPL implementation
│
├── stdlib/                      # Standard library (Pure Clorus)
│   ├── core.clr                 # Core utilities
│   ├── lazy.clr                 # Lazy sequences ✅
│   ├── functions.clr            # Function utilities (partial, comp, etc.)
│   ├── collections.clr          # Collection utilities
│   ├── string.clr               # String operations
│   ├── math.clr                 # Mathematical functions
│   ├── io.clr                   # I/O utilities
│   └── README.md                # Stdlib documentation
│
├── examples/                    # Example Clorus programs
│   ├── lazy-examples.clr        # Lazy sequence examples ✅
│   ├── basic-examples.clr       # Basic language features
│   ├── macro-examples.clr       # Macro usage
│   └── ffi-examples.clr         # Rust FFI examples
│
├── tests/                       # Test suites (Clorus)
│   ├── lazy-test.clr            # Lazy sequence tests ✅
│   ├── core-test.clr            # Core language tests
│   ├── stdlib-test.clr          # Stdlib tests
│   └── integration-test.clr     # Integration tests
│
├── docs/                        # Documentation
│   ├── LANGUAGE_PARITY.md       # Clojure feature comparison ✅
│   ├── LAZY_SEQUENCES_GUIDE.md  # Lazy sequence user guide ✅
│   ├── FFI_GUIDE.md             # Rust FFI documentation
│   ├── STDLIB_GUIDE.md          # Standard library guide
│   └── CONTRIBUTING.md          # Contribution guidelines
│
├── build/                       # Build artifacts (gitignored)
│   └── ...
│
├── Cargo.toml                   # Rust workspace manifest
├── Cargo.lock                   # Rust dependencies
├── README.md                    # Project readme
└── LICENSE                      # License file
```

---

## Directory Responsibilities

### `/crates` - Compiler & Runtime (Rust)

**Purpose:** Core infrastructure written in Rust.

**Contains:**
- Lexer and parser
- AST definitions
- Macro expansion
- LLVM code generation
- Runtime primitives (FFI functions)
- REPL and CLI tools

**Who modifies:** Core maintainers when adding language features

**Examples:**
- `clorus-syntax/src/parser.rs` - Parse Clorus source code
- `clorus-codegen/src/codegen.rs` - Generate LLVM IR
- `clorus-runtime/src/collections.rs` - Runtime collection functions
- `clorus-runtime/src/map.rs` - Hash map implementation

### `/stdlib` - Standard Library (Pure Clorus)

**Purpose:** High-level libraries written in Clorus itself.

**Contains:**
- Pure Clorus functions built on runtime primitives
- Function utilities (partial, comp, juxt)
- Collection utilities (frequencies, group-by)
- String operations
- Math utilities
- I/O helpers

**Who modifies:** Core maintainers + community contributors

**Key files:**
- `lazy.clr` ✅ - Lazy sequence library (completed)
- `functions.clr` - Function composition utilities
- `collections.clr` - Collection manipulation
- `string.clr` - String operations
- `math.clr` - Math functions

**Philosophy:** Once core language features are complete, most stdlib can be written in pure Clorus without touching Rust code.

### `/examples` - Example Programs (Clorus)

**Purpose:** Demonstrate language features and stdlib usage.

**Contains:**
- Tutorial examples
- Real-world use cases
- Best practices

**Examples:**
- `lazy-examples.clr` ✅ - Lazy sequence examples
- `macro-examples.clr` - Macro usage patterns
- `ffi-examples.clr` - Rust FFI integration

### `/tests` - Test Suites (Clorus)

**Purpose:** Validate language and stdlib functionality.

**Contains:**
- Unit tests for stdlib functions
- Integration tests
- Regression tests

**Examples:**
- `lazy-test.clr` ✅ - Lazy sequence test suite
- `core-test.clr` - Core language tests
- `stdlib-test.clr` - Standard library tests

### `/docs` - Documentation (Markdown)

**Purpose:** Comprehensive documentation for users and contributors.

**Contains:**
- User guides
- Technical documentation
- API references
- Design documents

**Examples:**
- `LAZY_SEQUENCES_GUIDE.md` ✅ - Lazy sequence user guide
- `LANGUAGE_PARITY.md` ✅ - Feature comparison
- `FFI_GUIDE.md` - Rust FFI guide
- `STDLIB_GUIDE.md` - Standard library overview

---

## What Goes Where?

### Runtime Primitives (Rust) vs Stdlib (Clorus)

#### Runtime Primitives (`/crates/clorus-runtime`)

Implement in Rust when:
- ✅ Performance critical (tight loops, memory ops)
- ✅ Requires type-specific operations (hash map internals)
- ✅ Foundation for stdlib (nth, get, conj, assoc)
- ✅ Memory management (retain, release, GC)
- ✅ FFI boundaries (calling Rust/C code)

**Examples:**
- `clorus_vector_nth` - Direct memory access
- `clorus_map_assoc` - Hash table operations
- `clorus_atom_swap` - Atomic operations

#### Standard Library (`/stdlib`)

Implement in Clorus when:
- ✅ Can be built using runtime primitives
- ✅ Benefits from Clorus' expressiveness
- ✅ Doesn't require low-level operations
- ✅ Community can contribute easily

**Examples:**
- `lazy-map`, `lazy-filter` - Built on closures and recursion
- `partial`, `comp` - Function composition
- `frequencies` - Uses reduce and assoc
- `zipmap` - Combines map and reduce

### Decision Matrix

| Feature | Runtime (Rust) | Stdlib (Clorus) | Reason |
|---------|---------------|-----------------|--------|
| Vector nth | ✅ | ❌ | Direct memory access |
| Vector take | ❌ | ✅ | Can use nth in loop |
| Map assoc | ✅ | ❌ | Hash table internals |
| Map merge | ❌ | ✅ | Can use assoc repeatedly |
| Lazy sequences | ❌ | ✅ | Closures + atoms sufficient |
| Atoms | ✅ | ❌ | Requires atomic operations |
| Partial | ❌ | ✅ | Pure Clorus function composition |
| String concat | ⚠️ | ⚠️ | Could be either |

---

## Current Status (January 27, 2026)

### ✅ Completed

**Runtime primitives:**
- Core data types (vectors, maps, sets, lists)
- Collection operations (get, nth, first, rest, count, conj, assoc)
- 14 new collection functions (dissoc, keys, vals, merge, get-in, assoc-in, take, drop, concat, interleave, interpose, distinct, dedupe, flatten)
- Atoms (swap!, reset!, deref)
- Control flow (if, loop/recur, try/catch/finally)
- Macros (defmacro, quote, syntax-quote, unquote, gensym)
- Destructuring (vectors, maps in let/defn)
- Exception handling (throw, try, catch, finally)

**Standard library:**
- ✅ Lazy sequence library (`stdlib/lazy.clr`) - Complete!
  - Core construction (make-lazy, lazy-cons, force)
  - Infinite sequences (range, repeat, cycle, iterate)
  - Transformations (map, filter, take, drop, etc.)
  - Combination (concat, interleave, interpose)
  - Realization (realize, realize-n)
  - Advanced operations (distinct, partition)

**Documentation:**
- ✅ LAZY_SEQUENCES_GUIDE.md - User guide
- ✅ LAZY_SEQUENCES_PURE_CLORUS.md - Technical details
- ✅ LAZY_VS_POLYMORPHISM.md - Feature comparison
- ✅ LANGUAGE_PARITY.md - Current ~60% Clojure parity

**Examples & Tests:**
- ✅ lazy-examples.clr - Comprehensive examples
- ✅ lazy-test.clr - Full test suite

### 🚧 In Progress / Next

**Standard library (Pure Clorus):**
- Function utilities (partial, comp, juxt, complement)
- Collection utilities (frequencies, group-by, zipmap)
- Predicates (nil?, empty?, even?, odd?, zero?, some?)
- Math utilities (abs, min, max, mod)
- String operations (str, subs, split, join)

**Runtime (if needed):**
- String primitive operations (only if performance-critical)
- I/O primitives (file reading, network)

---

## Development Workflow

### Adding a New Runtime Feature

1. **Design** - Determine if it belongs in runtime or stdlib
2. **Implement** - Add to appropriate `/crates/clorus-runtime/src/*.rs`
3. **Declare** - Add FFI declaration in `codegen.rs`
4. **Dispatch** - Add builtin case in `codegen.rs`
5. **Test** - Add Rust tests in runtime crate
6. **Document** - Update docs with new capability

### Adding a New Stdlib Function

1. **Design** - Ensure runtime primitives are sufficient
2. **Implement** - Add to appropriate `/stdlib/*.clr` file
3. **Example** - Add usage examples in `/examples/`
4. **Test** - Add tests in `/tests/`
5. **Document** - Update stdlib guide

### Example: Adding `frequencies`

```clojure
;; stdlib/collections.clr

(defn frequencies [coll]
  "Count occurrences of each element in collection.

  Examples:
    (frequencies [1 2 1 3 2 1])  ; => {1 3, 2 2, 3 1}"
  (reduce (fn [m x]
            (assoc m x (inc (get m x 0))))
          {}
          coll))
```

No runtime changes needed! Uses existing primitives: `reduce`, `assoc`, `get`, `inc`.

---

## Migration Plan

### Current Structure
```
clorus/
├── crates/                    # Runtime & compiler ✅
├── test-lazy.clr              # Tests scattered ⚠️
├── test-collections.clr       # Tests scattered ⚠️
├── examples/                  # Good! ✅
└── docs/                      # Good! ✅
```

### Proposed Migration

1. **Create `/stdlib` directory**
   ```bash
   mkdir -p stdlib
   mv stdlib/lazy.clr stdlib/lazy.clr  # Already in place ✅
   ```

2. **Create `/tests` directory**
   ```bash
   mkdir -p tests
   mv test-lazy.clr tests/lazy-test.clr
   mv test-collections.clr tests/collections-test.clr
   ```

3. **Organize examples** (already good! ✅)
   ```bash
   ls examples/
   # lazy-examples.clr ✅
   ```

4. **Update documentation**
   - Add `/stdlib/README.md` - Stdlib overview
   - Add `/docs/STDLIB_GUIDE.md` - How to contribute
   - Update main README.md with new structure

---

## File Naming Conventions

### Runtime (Rust)
- `snake_case.rs` - Rust convention
- Examples: `collections.rs`, `map.rs`, `value.rs`

### Stdlib (Clorus)
- `kebab-case.clr` - Lisp convention
- Examples: `lazy.clr`, `string.clr`, `math.clr`

### Tests (Clorus)
- `name-test.clr` - Test suffix
- Examples: `lazy-test.clr`, `core-test.clr`

### Examples (Clorus)
- `name-examples.clr` - Examples suffix
- Examples: `lazy-examples.clr`, `macro-examples.clr`

### Documentation (Markdown)
- `SCREAMING_SNAKE_CASE.md` - Important docs
- `kebab-case.md` - Guides
- Examples: `README.md`, `CONTRIBUTING.md`, `lazy-sequences-guide.md`

---

## Dependency Flow

```
┌─────────────────────────────────────────────┐
│  User Clorus Programs (.clr)                │
│  - Applications                              │
│  - Scripts                                   │
└────────────────┬────────────────────────────┘
                 │ uses
                 ▼
┌─────────────────────────────────────────────┐
│  Standard Library (/stdlib/*.clr)           │
│  - lazy.clr ✅                              │
│  - functions.clr                             │
│  - collections.clr                           │
│  - string.clr                                │
└────────────────┬────────────────────────────┘
                 │ uses
                 ▼
┌─────────────────────────────────────────────┐
│  Runtime Primitives (/crates/clorus-runtime)│
│  - collections.rs (nth, get, conj, assoc)   │
│  - map.rs (hash map internals)              │
│  - value.rs (memory management)              │
└────────────────┬────────────────────────────┘
                 │ compiled by
                 ▼
┌─────────────────────────────────────────────┐
│  Compiler (/crates/clorus-{syntax,codegen}) │
│  - Parser (source → AST)                     │
│  - Macro expansion                           │
│  - Codegen (AST → LLVM IR)                   │
└─────────────────────────────────────────────┘
```

**Key insight:** Most new features go in `/stdlib`, not `/crates`.

---

## Best Practices

### DO ✅
- ✅ Implement in Clorus stdlib when possible
- ✅ Keep runtime primitives minimal and focused
- ✅ Document both what and why
- ✅ Add examples for all stdlib functions
- ✅ Write tests for everything
- ✅ Use clear naming conventions

### DON'T ❌
- ❌ Add to runtime when stdlib can do it
- ❌ Mix compiler code with stdlib code
- ❌ Scatter test files around project
- ❌ Skip documentation
- ❌ Duplicate functionality

---

## Summary

**Principle:** Separate infrastructure (Rust) from library code (Clorus).

**Structure:**
- `/crates` - Compiler & runtime (Rust)
- `/stdlib` - Standard library (Clorus)
- `/examples` - Example programs (Clorus)
- `/tests` - Test suites (Clorus)
- `/docs` - Documentation (Markdown)

**Philosophy:** Build advanced features in Clorus itself once core language is complete.

**Status:** Organization in progress - lazy sequences demonstrate the stdlib-in-Clorus approach works! ✅

---

**Next Steps:**
1. Create `/stdlib` directory structure
2. Migrate test files to `/tests`
3. Add stdlib README
4. Create STDLIB_GUIDE.md
5. Continue implementing stdlib functions in pure Clorus

---

**Last Updated:** January 27, 2026
**Version:** 1.0.0
