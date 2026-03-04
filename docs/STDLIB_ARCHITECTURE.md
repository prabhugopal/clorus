# Clorus Standard Library Architecture

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/ROADMAP_TO_100_PARITY.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Document Version:** 1.0
**Status:** Design Specification
**Date:** 2025-01-30

---

## Executive Summary

This document defines the professional, production-ready architecture for the Clorus standard library. The design follows Clojure's proven model: a single, canonical `clorus.core` namespace that provides all essential functionality, with implementation details hidden from users.

**Key Principles:**
1. **Single namespace** - Users import one thing: `clorus.core`
2. **Implementation hiding** - Rust vs Clorus code is transparent
3. **Auto-loading** - Core functions always available
4. **Extensible** - Easy to add new modules
5. **Professional** - Clean, documented, tested

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    User Code                            │
│  (println "Hello")                                      │
│  (map inc [1 2 3])                                      │
│  (slurp "file.txt")                                     │
└────────────────┬────────────────────────────────────────┘
                 │ Uses
                 ▼
┌─────────────────────────────────────────────────────────┐
│              clorus.core (Public API)                   │
│                                                         │
│  Location: stdlib/core.clr                              │
│  Auto-loaded: Yes (REPL + Compiler)                     │
│  Namespace: clorus.core                                 │
└────────────────┬────────────────────────────────────────┘
                 │ Composed of
                 ▼
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
┌──────────────┐  ┌──────────────────┐
│ Pure Clorus  │  │  Rust Runtime    │
│ Functions    │  │  (FFI)           │
│              │  │                  │
│ inc, dec     │  │ println, slurp   │
│ map, filter  │  │ spit, read-line  │
│ range, for   │  │ file I/O         │
└──────────────┘  └──────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │ clorus.runtime.io    │
              │ (Internal dylib)     │
              │                      │
              │ Implementation only  │
              │ NOT user-facing      │
              └──────────────────────┘
```

---

## Namespace Hierarchy

### Public Namespaces (User-Facing)

```
clorus.core          - Core utilities (auto-loaded)
clorus.string        - String operations
clorus.math          - Math functions
clorus.data          - Data structure utilities
clorus.async         - Async/concurrency
clorus.test          - Testing framework
clorus.spec          - Spec/validation (future)
```

### Internal Namespaces (Implementation)

```
clorus.runtime.io    - Rust I/O implementation (dylib)
clorus.runtime.fs    - Rust filesystem implementation
clorus.runtime.ffi   - FFI utilities
```

**Rules:**
- Users **never** import `clorus.runtime.*` directly
- All runtime functions exposed through public namespaces
- Keeps implementation flexible (can swap Rust/Clorus)

---

## clorus.core Specification

**Location:** `stdlib/core.clr`
**Status:** Canonical standard library
**Auto-loaded:** Yes (in REPL and compiled programs)

### Contents

#### 1. Runtime Re-exports (from clorus.runtime.io)

```clojure
;; I/O Functions
println    - Print with newline
print      - Print without newline
pr         - Print readable representation
prn        - Print readable with newline
read-line  - Read line from stdin
slurp      - Read entire file
spit       - Write to file
flush      - Flush output buffer
```

#### 2. Basic Arithmetic

```clojure
inc        - Increment by 1
dec        - Decrement by 1
zero?      - Test if zero
pos?       - Test if positive
neg?       - Test if negative
even?      - Test if even
odd?       - Test if odd
abs        - Absolute value
min        - Minimum of args
max        - Maximum of args
```

#### 3. Collection Functions

```clojure
;; Constructors
list       - Create list
vector     - Create vector
hash-map   - Create map
hash-set   - Create set

;; Core operations
first      - First element
rest       - All but first
last       - Last element
nth        - Nth element
count      - Count elements
empty?     - Test if empty
conj       - Add element
cons       - Prepend element

;; Transformations
map        - Transform each element
filter     - Keep matching elements
reduce     - Reduce to single value
remove     - Remove matching elements
take       - Take first n
drop       - Drop first n
concat     - Concatenate collections
```

#### 4. Function Utilities

```clojure
partial    - Partial application
comp       - Function composition
identity   - Identity function
constantly - Constant function
complement - Logical complement
juxt       - Juxtapose functions
apply      - Apply function to args
```

#### 5. Iteration Macros

```clojure
for        - List comprehension
doseq      - Iterate for side effects
dotimes    - Loop n times
while      - Loop while condition
loop/recur - Tail-recursive loop
```

#### 6. Logic & Predicates

```clojure
nil?       - Test if nil
some?      - Test if not nil
true?      - Test if true
false?     - Test if false
boolean    - Coerce to boolean
not        - Logical NOT
and        - Logical AND
or         - Logical OR
```

#### 7. Type Predicates

```clojure
number?    - Test if number
string?    - Test if string
keyword?   - Test if keyword
symbol?    - Test if symbol
vector?    - Test if vector
list?      - Test if list
map?       - Test if map
set?       - Test if set
fn?        - Test if function
atom?      - Test if atom
ref?       - Test if ref
agent?     - Test if agent
```

#### 8. Sequences

```clojure
seq        - Convert to sequence
range      - Generate range
repeat     - Repeat value
repeatedly - Repeat function call
take-while - Take while predicate
drop-while - Drop while predicate
partition  - Partition into chunks
```

---

## File Organization

```
clorus/
├── stdlib/
│   ├── core.clr              # Main standard library (clorus.core)
│   ├── string.clr            # String utilities (clorus.string)
│   ├── math.clr              # Math functions (clorus.math)
│   ├── data.clr              # Data structures (clorus.data)
│   ├── async.clr             # Async/concurrency (clorus.async)
│   ├── test.clr              # Testing framework (clorus.test)
│   └── internal/
│       └── prelude.clr       # Internal: minimal bootstrap
│
├── crates/
│   ├── clorus-runtime/
│   │   └── src/
│   │       ├── io.rs         # I/O implementation
│   │       ├── fs.rs         # Filesystem operations
│   │       └── lib.rs        # Runtime library
│   │
│   └── clorus-runtime-io/    # NEW: Separate dylib crate
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs        # FFI exports for I/O
│
└── docs/
    ├── stdlib/
    │   ├── CORE.md           # clorus.core documentation
    │   ├── STRING.md         # clorus.string documentation
    │   └── ...
    └── STDLIB_ARCHITECTURE.md  # This document
```

---

## Implementation Plan

### Phase 1: Core Infrastructure (1 day)

**1.1 Create clorus-runtime-io crate**

```toml
# crates/clorus-runtime-io/Cargo.toml
[package]
name = "clorus-runtime-io"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]
name = "clorus_runtime_io"  # Note: Different from public name

[dependencies]
clorus-runtime = { path = "../clorus-runtime" }
```

```rust
// crates/clorus-runtime-io/src/lib.rs
//! Internal I/O runtime for Clorus
//!
//! This crate provides FFI bindings for I/O operations.
//! Users should NOT import this directly - use clorus.core instead.

// Re-export I/O functions from clorus-runtime
pub use clorus_runtime::io::{
    clorus_println,
    clorus_print,
    clorus_pr,
    clorus_prn,
    clorus_slurp,
    clorus_spit,
    clorus_read_line,
    clorus_flush,
};

// Re-export string functions
pub use clorus_runtime::string::{
    clorus_pr_str,
    clorus_str,
};
```

**1.2 Update workspace Cargo.toml**

```toml
[workspace]
members = [
    "crates/clorus",
    "crates/clorus-cli",
    "crates/clorus-repl",
    "crates/clorus-runtime",
    "crates/clorus-runtime-io",  # NEW
    # ... rest
]
```

**1.3 Update compiler to load clorus-runtime-io.dylib**

Location: `crates/clorus-cli/src/commands.rs`

```rust
fn load_runtime_io_library() -> Result<libloading::Library, String> {
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_runtime_io.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_runtime_io.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_runtime_io.dll";

    // Search for library in target directories
    // ... (similar to existing load_core_library)
}
```

### Phase 2: Namespace System (2 days)

**2.1 Add namespace support to parser**

Location: `crates/clorus-syntax/src/ast.rs`

```rust
/// Namespace declaration
#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub name: String,
    pub requires: Vec<RequireSpec>,
    pub uses: Vec<UseSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequireSpec {
    pub namespace: String,
    pub alias: Option<String>,
    pub refer: Option<Vec<String>>,
}
```

**2.2 Parse `ns` macro**

```clojure
(ns clorus.core
  (:require [clorus.runtime.io :as io])
  (:use [clorus.string]))
```

**2.3 Implement namespace resolution in codegen**

Location: `crates/clorus-codegen/src/codegen.rs`

```rust
pub struct NamespaceContext {
    current_ns: String,
    imports: HashMap<String, String>,  // alias -> full namespace
    refers: HashMap<String, String>,   // symbol -> source namespace
}

impl CodeGen {
    fn resolve_symbol(&self, name: &str) -> String {
        // Check if it's a qualified name
        if name.contains('/') {
            return name.to_string();
        }

        // Check refers
        if let Some(ns) = self.ns_context.refers.get(name) {
            return format!("{}/{}", ns, name);
        }

        // Default to current namespace
        format!("{}/{}", self.ns_context.current_ns, name)
    }
}
```

### Phase 3: stdlib/core.clr (1 day)

**3.1 Create professional stdlib/core.clr**

```clojure
;; Clorus Standard Library - Core Module
;;
;; This is the canonical clorus.core namespace, auto-loaded in all programs.
;; Provides essential functions and macros for Clorus programming.
;;
;; Copyright (c) 2025 Clorus Contributors
;; Licensed under MIT License

(ns clorus.core
  "Core utilities for Clorus programming.

  This namespace is automatically loaded and provides:
  - I/O functions (println, slurp, spit)
  - Collection operations (map, filter, reduce)
  - Arithmetic utilities (inc, dec)
  - Function utilities (comp, partial)
  - Control flow (for, doseq, loop)"

  (:require [clorus.runtime.io :as io]))

;; ============================================================================
;; I/O Functions (Re-exported from runtime)
;; ============================================================================

(def println
  "Print objects to stdout with newline.

  Examples:
    (println \"Hello\")           ; => Hello\\n
    (println \"x =\" 42)          ; => x = 42\\n"
  io/println)

(def print
  "Print objects to stdout without newline."
  io/print)

(def slurp
  "Read entire file into string.

  Example:
    (slurp \"file.txt\")  ; => file contents as string"
  io/slurp)

(def spit
  "Write string to file.

  Example:
    (spit \"out.txt\" \"Hello!\")"
  io/spit)

;; ============================================================================
;; Basic Arithmetic
;; ============================================================================

(defn inc
  "Increment number by 1.

  Example:
    (inc 5)  ; => 6"
  [x]
  (+ x 1))

(defn dec
  "Decrement number by 1.

  Example:
    (dec 10)  ; => 9"
  [x]
  (- x 1))

(defn zero?
  "Test if number is zero.

  Example:
    (zero? 0)   ; => true
    (zero? 5)   ; => false"
  [x]
  (= x 0))

;; ... (continue with all functions, properly documented)
```

**3.2 Update compiler auto-loading**

Location: `crates/clorus-cli/src/commands.rs`

```rust
// Load stdlib/core.clr first (provides clorus.core namespace)
let stdlib_path = Path::new("stdlib/core.clr");
let mut all_exprs = Vec::new();

if stdlib_path.exists() {
    let stdlib_source = fs::read_to_string(stdlib_path)
        .map_err(|e| format!("Failed to load clorus.core: {}", e))?;

    let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
        .map_err(|e| format!("Error in clorus.core: {}", e))?;

    all_exprs.extend(stdlib_exprs);

    println!("   Loaded clorus.core ({} forms)", stdlib_exprs.len());
} else {
    return Err("clorus.core not found! Standard library is required.".to_string());
}
```

### Phase 4: Testing & Documentation (1 day)

**4.1 Comprehensive tests**

```clojure
;; tests/stdlib/core-test.clr
(ns clorus.core-test
  (:require [clorus.test :refer [deftest is testing]]))

(deftest test-inc
  (testing "inc increments by 1"
    (is (= 6 (inc 5)))
    (is (= 0 (inc -1)))
    (is (= 1.5 (inc 0.5)))))

(deftest test-map
  (testing "map applies function to collection"
    (is (= [2 3 4] (map inc [1 2 3])))
    (is (= [] (map inc [])))
    (is (= [2] (map inc [1])))))

;; ... comprehensive tests for all functions
```

**4.2 API documentation**

Create `docs/stdlib/CORE.md` with full API reference.

### Phase 5: REPL Integration (0.5 days)

**5.1 Auto-load in REPL**

Location: `crates/clorus-repl/src/repl_engine.rs`

```rust
pub fn new() -> Self {
    let mut engine = ReplEngine { ... };

    // Auto-load clorus.core
    match engine.load_core_library() {
        Ok(_) => println!("✓ clorus.core loaded"),
        Err(e) => eprintln!("⚠ Warning: clorus.core not loaded: {}", e),
    }

    engine
}

fn load_core_library(&mut self) -> Result<(), String> {
    let core_path = Path::new("stdlib/core.clr");
    if !core_path.exists() {
        return Err("stdlib/core.clr not found".to_string());
    }

    let source = fs::read_to_string(core_path)?;
    self.eval_multiple(&source)?;
    Ok(())
}
```

---

## Migration Path

### Deprecation Timeline

**v0.3.0** (Current)
- ✅ Mark `clorus-core` dylib as deprecated
- ✅ Introduce `clorus.runtime.io` (internal)
- ✅ Create new `stdlib/core.clr` as canonical
- ⚠️ Warning: "clorus-core dylib is deprecated, use clorus.core namespace"

**v0.4.0** (Next release)
- Remove old `clorus-core` dylib
- `clorus.core` is the only public API
- All documentation updated

**v1.0.0** (Stable)
- Stable namespace API
- Comprehensive stdlib
- Full Clojure compatibility

---

## Naming Conventions

### Namespaces
- **clorus.*** - Public API
- **clorus.runtime.*** - Internal implementation

### Files
- **lowercase-with-hyphens** - Standard for Clojure
- Example: `clorus-string.clr` → namespace `clorus.string`

### Functions
- **kebab-case** - Clojure standard
- Examples: `read-line`, `hash-map`, `take-while`

### Private functions
- Prefix with `-` - Example: `-internal-helper`

---

## Quality Standards

### All stdlib functions MUST have:
1. **Docstring** - What it does, examples
2. **Type hints** (future) - Parameter and return types
3. **Tests** - Unit tests with edge cases
4. **Examples** - Runnable examples in docs

### Code review checklist:
- [ ] Docstring present and clear
- [ ] Examples provided
- [ ] Tests passing
- [ ] No breaking changes
- [ ] Performance acceptable
- [ ] Follows naming conventions

---

## Future Modules

### Priority 1 (Next 3 months)
- clorus.string - String manipulation
- clorus.test - Testing framework
- clorus.data - Data structure utilities

### Priority 2 (6 months)
- clorus.async - Async/concurrency
- clorus.json - JSON parsing
- clorus.http - HTTP client

### Priority 3 (1 year)
- clorus.spec - Specification/validation
- clorus.jdbc - Database access
- clorus.crypto - Cryptography

---

## Success Criteria

### For v1.0 (Production Ready):
1. ✅ Single `clorus.core` namespace
2. ✅ 100+ core functions
3. ✅ 95%+ test coverage
4. ✅ Complete API documentation
5. ✅ Performance benchmarks
6. ✅ Zero breaking changes policy
7. ✅ Stable namespace system

---

**Next Review Date:** 2025-02-28
**Owner:** Clorus Core Team
**Status:** Approved for Implementation
