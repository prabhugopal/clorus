# Phase 2a: Interface Files (.clorus-ffi)

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/FFI_ROADMAP.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Status:** 🚧 In Progress
**Started:** January 26, 2025

---

## Overview

Interface files provide explicit FFI declarations without modifying Rust library source code. This enables:
- Using crates.io libraries directly
- Community-shareable interface definitions
- Better type documentation
- Complex type mappings (Option, Result, Vec)

---

## File Format

### Basic Syntax

```clojure
;; library-name.clorus-ffi
(interface library-name

  ;; Simple function: takes string, returns f64
  (defn show-gui [message :string] :f64
    "Show a GUI window with the given message")

  ;; No args, returns string
  (defn get-version [] :string
    "Get the library version")

  ;; Multiple params
  (defn add [x :f64 y :f64] :f64
    "Add two numbers"))
```

### Type Mappings

| Clorus Type | Rust Type | LLVM Type | Notes |
|-------------|-----------|-----------|-------|
| `:f64` | `f64` | `double` | Numbers |
| `:i32` | `i32` | `i32` | Integers |
| `:i64` | `i64` | `i64` | Long integers |
| `:bool` | `bool` | `i1` | Booleans |
| `:string` | `String` or `&str` | `i8*` | Null-terminated C strings |
| `:unit` | `()` | `void` | No return value |

### Future: Complex Types

```clojure
;; Option<T>
(defn maybe-find [key :string] [:option :string]
  :on-none nil
  :on-some identity)

;; Result<T, E>
(defn try-parse [s :string] [:result :f64 :string]
  :on-ok identity
  :on-err (fn [e] (error e)))

;; Vec<T>
(defn get-items [] [:vec :string])

;; Custom structs
(deftype HttpResponse
  {:status :i32
   :headers [:map :string :string]
   :body :string})
```

---

## Usage in Clorus.toml

### Auto-Parse (Phase 1 - Still Works)

```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello" }
```

Auto-generates FFI wrappers from Rust source.

### Interface File (Phase 2a - New)

```toml
[rust-dependencies]
egui-hello = {
  path = "../egui-hello",
  interface = "egui-hello.clorus-ffi"
}
```

Uses explicit interface file.

### Both Can Coexist

```toml
[rust-dependencies]
# Simple library: auto-parse
math-lib = { path = "../math-lib" }

# Complex library: interface file
http-client = {
  path = "../http-client",
  interface = "http.clorus-ffi"
}
```

---

## Implementation Architecture

### Data Structures

```rust
// clorus-cli/src/interface.rs

pub struct InterfaceFile {
    pub name: String,
    pub functions: Vec<InterfaceFunction>,
}

pub struct InterfaceFunction {
    pub name: String,
    pub params: Vec<InterfaceParam>,
    pub return_type: String,
    pub doc: Option<String>,
}

pub struct InterfaceParam {
    pub name: String,
    pub type_name: String,
}
```

### Parser

Uses existing `clorus-syntax` parser:

```rust
pub fn parse_interface_file(path: &Path) -> Result<InterfaceFile, String> {
    let source = fs::read_to_string(path)?;
    let exprs = clorus_syntax::parse(&source)?;

    // Parse (interface name ...)
    if let Some(Expr::List(items)) = exprs.first() {
        if let Some(Expr::Symbol(s)) = items.first() {
            if s == "interface" {
                return parse_interface_expr(&items[1..])?;
            }
        }
    }

    Err("Invalid interface file format")
}
```

### FFI Processor Integration

```rust
// clorus-cli/src/rust_ffi.rs

pub fn process_dependencies(manifest: &Manifest) -> Result<RustFfiInfo, String> {
    for (name, dep) in &manifest.rust_dependencies {
        if let Some(interface_path) = &dep.interface {
            // Phase 2a: Use interface file
            let interface = parse_interface_file(interface_path)?;
            process_interface(&interface)?;
        } else {
            // Phase 1: Auto-parse (backward compatible)
            auto_parse_rust_source(dep)?;
        }
    }
}
```

---

## Example: egui-hello Interface

### egui-hello.clorus-ffi

```clojure
;; Interface file for egui-hello library
(interface egui-hello

  (defn show-gui [message :string] :f64
    "Show a GUI window with the given message.
     Returns 0 when window is closed.")

  (defn show-gui-json [config :string] :f64
    "Show GUI configured via JSON string.
     Config format: {\"title\": \"...\", \"message\": \"...\"}")

  (defn get-gui-version [] :string
    "Get the egui version string"))
```

### Clorus.toml

```toml
[package]
name = "gui-demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
egui-hello = {
  path = "../egui-hello",
  interface = "egui-hello.clorus-ffi"
}

[link]
frameworks = ["AppKit", "CoreGraphics", "Metal", "QuartzCore", "Carbon", "OpenGL"]
```

### src/main.clrs

```clojure
(ns gui.demo
  (:rust [egui-hello :as gui]))

; Same usage - interface file is transparent to user code
(gui/show-gui "Hello from interface files!")

; Get version
(def version (gui/get-gui-version))

5
```

---

## Implementation Steps

### Step 1: Parser (1-2 hours)
- Create `clorus-cli/src/interface.rs`
- Implement `parse_interface_file()`
- Parse `(interface name ...)` structure
- Extract function signatures
- Handle docstrings

### Step 2: Data Structures (30 min)
- Define `InterfaceFile`, `InterfaceFunction`, `InterfaceParam`
- Convert to existing `RustLibrary` format
- Ensure compatibility with codegen

### Step 3: FFI Processor Update (1 hour)
- Modify `rust_ffi.rs` to check for `interface` field
- Branch: interface file vs auto-parse
- Generate same FFI wrappers
- Maintain backward compatibility

### Step 4: Manifest Parser Update (30 min)
- Add `interface` field to `RustDependency`
- Parse from Clorus.toml
- Validate file exists

### Step 5: Testing (1 hour)
- Convert gui-demo to use interface file
- Verify auto-parse still works
- Test mixed projects (some auto, some interface)

---

## Benefits

### For Library Authors
- No source modification
- Use crates.io directly
- Version interface independently

### For Users
- Clear API documentation
- Type safety
- Better error messages
- Community-maintained interfaces

### For Clorus
- Professional FFI system
- Matches OCaml, Haskell patterns
- Extensible to complex types

---

## Backward Compatibility

**100% maintained:**
- Auto-parse keeps working
- No changes to existing code
- Interface files are opt-in
- Both approaches coexist

**Migration path:**
1. Start with auto-parse (quick iteration)
2. Add interface file later (production ready)
3. Both work in same project

---

## Future: Interface Registry

```bash
# Install community interfaces
clorus install serde-json-interface
clorus install reqwest-interface

# Use in project
[rust-dependencies]
serde-json = {
  crates-io = "1.0",
  interface = "~/.clorus/interfaces/serde-json.clorus-ffi"
}
```

---

## Success Criteria

1. ✅ Parse .clorus-ffi files
2. ✅ Convert to RustLibrary format
3. ✅ Generate same FFI wrappers as auto-parse
4. ✅ gui-demo works with interface file
5. ✅ Auto-parse still works (backward compat)
6. ✅ Mixed projects work (some auto, some interface)
7. ✅ Better error messages (interface provides context)

---

**Next:** Implement parser in `clorus-cli/src/interface.rs`
