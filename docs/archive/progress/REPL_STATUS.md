# REPL Status - January 28, 2026

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/PARITY_EXECUTION_PLAN.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


## ✅ What's Working

### Core REPL Functionality
- ✓ Basic expression evaluation: `(+ 1 2)` => `3`
- ✓ Variable definitions: `(def x 10)` => `#'user/x`
- ✓ Function definitions: `(defn add [x y] (+ x y))` => `#'user/add`
- ✓ Function calls across history
- ✓ Namespace switching with `(ns my.namespace)`
- ✓ TAB autocomplete for symbols
- ✓ History (↑↓ arrows)
- ✓ Saved history file: `~/.clorus_history`

### Clojure-Style Output
- ✓ `(def x 10)` displays `#'user/x` (not `=> 0`)
- ✓ `(defn foo [] ...)` displays `#'user/foo`
- ✓ Regular expressions display their value
- ✓ `(ns ...)` and `(require ...)` display `nil`

### Project Detection
- ✓ Detects `Clorus.toml` in current directory
- ✓ Shows project info in banner:
  ```
  📦 Project: my-app v0.1.0
  📂 Namespace: my_app.main
  ```
- ✓ Derives namespace from entry point filename
- ✓ Works from any directory with `clorus repl` command

### Library Loading
- ✓ Loads `clorus-runtime.dylib` for Value* operations
- ✓ Loads `clorus-std.dylib` for rust.fs functions (optional)
- ✓ Loads `clorus-core.dylib` for slurp/spit (optional)
- ✓ Graceful fallback if libraries not found

## ❌ Known Limitations

### Project Auto-Loading
**Status:** Partial - Only first expression loads

When you run `clorus repl` in a project directory, it attempts to load the entry file but **only the first expression (usually `ns` form) is evaluated**.

**Example:**
```clojure
;; src/main.clrs
(ns my.app)          ; ← This loads ✓
(defn add [x y] ...)  ; ← This doesn't load ✗
(def value 42)        ; ← This doesn't load ✗
```

**Workaround:** Manually evaluate definitions in REPL, or use `clorus run`.

**Why:** The REPL engine was designed to process one expression per `eval()` call. To fix this, we need one of:
1. Implement batch eval mode in repl_engine.rs
2. Add Expr → String serialization to evaluate forms individually
3. Parse source into individual form strings

### No :reload Command
Can't reload changed files without restarting REPL.

**Workaround:** Restart REPL or re-evaluate individual forms.

### No Rust FFI in REPL
Can't use Rust libraries interactively via `:rust` imports.

**Workaround:** Use `clorus run` which supports full Rust FFI.

## 🎯 Comparison with Clojure REPL

| Feature | Clojure | Clorus | Status |
|---------|---------|--------|--------|
| Basic evaluation | ✓ | ✓ | Working |
| Def/defn output | `#'ns/name` | `#'ns/name` | Working |
| Namespace switching | ✓ | ✓ | Working |
| Load file | `(load-file "...")` | Partial | Needs work |
| Reload namespace | `(require ... :reload)` | ✗ | Not implemented |
| Pretty printing | ✓ | Basic | Works for numbers |
| Doc strings | `(doc symbol)` | ✗ | Not implemented |
| Source viewing | `(source symbol)` | ✗ | Not implemented |
| Stacktraces | ✓ | Basic | JIT errors only |
| nREPL support | ✓ | ✗ | Not planned |

## 📋 TODO List (Priority Order)

### High Priority
1. **Multi-expression file loading** - Make project auto-loading work properly
2. **`:reload` command** - Reload changed files
3. **Better error messages** - Show line/column for parse errors

### Medium Priority
4. **`:doc` command** - Show documentation for symbols
5. **`:source` command** - Show source code of functions
6. **Pretty printing** - Format maps, vectors nicely
7. **String display** - Show strings properly (currently shows pointers)

### Low Priority
8. **History search** - Ctrl-R to search history
9. **Multi-line editing** - Detect incomplete forms
10. **Syntax highlighting** - Color code in terminal

## 🔧 Technical Notes

### JIT Compilation
Clorus REPL uses LLVM JIT compilation, not interpretation. This means:
- **Fast execution** - compiled to native code
- **Complex state management** - must recompile history on each eval
- **Library dependencies** - requires runtime libraries to be loaded

### Runtime Library Loading
Critical fix (Jan 28): The REPL must load `libclorus_runtime.dylib` with `RTLD_GLOBAL` flag before creating JIT engine. Without this, JIT-compiled code crashes when calling Value* functions.

### Namespace Context
Each eval() creates a fresh CodeGen with namespace context. Historical definitions are preserved by:
1. Keeping source in `history: Vec<String>`
2. Recompiling all history on each eval
3. Executing def/defn to initialize globals
4. Only returning value from latest expression

## 🚀 Usage Examples

### Basic REPL
```clojure
userλ> (def x 10)
#'user/x

userλ> (defn square [n] (* n n))
#'user/square

userλ> (square x)
100
```

### Namespace Switching
```clojure
userλ> (ns my.app)
nil

my.appλ> (def config {:port 8080})
#'my.app/config
```

### Project Aware
```bash
$ cd my-project
$ clorus repl

📦 Project: my-project v1.0.0
📂 Namespace: my_project.main

my_project.mainλ> ; Your definitions from src/main.clrs available here (eventually)
```

## 📚 Related Files

- **REPL Engine:** `crates/clorus-repl/src/repl_engine.rs`
- **REPL Main:** `crates/clorus-repl/src/main.rs`
- **Project Config:** `crates/clorus-repl/src/project.rs`
- **CLI Integration:** `crates/clorus-cli/src/commands.rs`

## 🎉 Recent Fixes (Jan 28, 2026)

1. **Fixed JIT crashes** - Added runtime library loading
2. **Project detection** - Reads Clorus.toml, shows project info
3. **Clojure-style output** - `#'namespace/name` for def/defn
4. **Namespace derivation** - No hardcoded .core suffix
5. **CLI fix** - `clorus repl` works from any directory

---

**Status:** Production-ready for basic use. Advanced features pending.
