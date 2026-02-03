# Namespace System Design for Clorus

## Overview

Implement a Clojure-style namespace system to enable modular code organization.

---

## Goals

1. **Namespace Declaration**: `(ns my.app.core)`
2. **Module Loading**: Load `.clrs` files based on namespace paths
3. **Dependencies**: `require`, `use`, `import` with options
4. **Qualified Symbols**: `my.module/func`, `alias/func`
5. **Symbol Resolution**: Proper scoping and visibility

---

## Syntax Design

### 1. Namespace Declaration

```clojure
; Simple namespace
(ns my.app.core)

; Namespace with requires
(ns my.app.core
  (:require [my.lib.util :as util]
            [my.lib.math :refer [add multiply]]))

; With Rust FFI imports
(ns my.app.gui
  (:require [my.app.config :as cfg])
  (:rust [egui-hello :as gui]))
```

### 2. Standalone Require

```clojure
; Require with alias
(require '[clorus.string :as str])

; Require with selective import
(require '[clorus.math :refer [sin cos pi]])

; Multiple requires
(require '[clorus.io :as io]
         '[clorus.json :as json])
```

### 3. Use (Import All)

```clojure
; Import all symbols from namespace
(use 'clorus.test)

; Now can call directly:
(is (= 1 1))      ; Instead of (clorus.test/is (= 1 1))
```

### 4. Qualified Symbols

```clojure
; Using alias
(str/join "," ["a" "b" "c"])

; Using full namespace
(my.app.util/process-data {:x 10})

; Defining qualified function
(defn my-func [x] (+ x 1))
; Accessible as: my.app.core/my-func
```

---

## File Structure

### Namespace to File Mapping

```
Namespace: my.app.core
File:      src/my/app/core.clrs

Namespace: clorus.string
File:      stdlib/clorus/string.clrs  (or in clorus installation)

Namespace: my.utils
File:      src/my/utils.clrs
```

### Example Project Structure

```
my-project/
├── Clorus.toml
├── src/
│   ├── main.clrs          ; Entry point
│   └── my/
│       ├── app/
│       │   ├── core.clrs  ; my.app.core
│       │   └── util.clrs  ; my.app.util
│       └── lib/
│           └── math.clrs  ; my.lib.math
└── target/
```

---

## Implementation Plan

### Phase 1: AST Extensions

**File**: `crates/clorus-syntax/src/ast.rs`

Add new expression types:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ... existing variants ...

    /// Namespace declaration: (ns my.app.core (:require ...))
    Ns {
        name: String,              // my.app.core
        requires: Vec<Require>,    // (:require clauses)
        rust_imports: Vec<RustImport>,  // (:rust clauses)
    },

    /// Require clause: (require '[my.lib :as lib])
    Require {
        module: String,            // my.lib
        alias: Option<String>,     // :as lib
        refer: Vec<String>,        // :refer [func1 func2]
        refer_all: bool,           // :refer :all
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequireSpec {
    pub module: String,
    pub alias: Option<String>,
    pub refer: Vec<String>,
    pub refer_all: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustImport {
    pub library: String,           // egui-hello
    pub alias: Option<String>,     // :as gui
}
```

### Phase 2: Parser Updates

**File**: `crates/clorus-syntax/src/parser.rs`

Add parsing for:

```rust
impl Parser {
    fn parse_ns(&mut self) -> Result<Expr, String> {
        // (ns my.app.core
        //   (:require [my.lib :as lib])
        //   (:rust [egui-hello :as gui]))
    }

    fn parse_require(&mut self) -> Result<Expr, String> {
        // (require '[my.lib :as lib :refer [func1 func2]])
    }

    fn parse_require_spec(&mut self) -> Result<RequireSpec, String> {
        // [my.lib :as lib :refer [func1 func2]]
    }
}
```

### Phase 3: Module Loader

**New File**: `crates/clorus/src/module_loader.rs`

```rust
pub struct ModuleLoader {
    /// Search paths for modules (src/, stdlib/, etc.)
    search_paths: Vec<PathBuf>,

    /// Cache of loaded modules: namespace -> parsed expressions
    cache: HashMap<String, Vec<Expr>>,
}

impl ModuleLoader {
    /// Find and load a module by namespace name
    pub fn load_module(&mut self, namespace: &str) -> Result<&Vec<Expr>, String> {
        // Check cache first
        if let Some(cached) = self.cache.get(namespace) {
            return Ok(cached);
        }

        // Convert namespace to file path
        // my.app.core -> src/my/app/core.clrs
        let file_path = self.namespace_to_path(namespace)?;

        // Read and parse the file
        let source = fs::read_to_string(&file_path)?;
        let exprs = clorus::parse(&source)?;

        // Cache and return
        self.cache.insert(namespace.to_string(), exprs);
        Ok(self.cache.get(namespace).unwrap())
    }

    fn namespace_to_path(&self, namespace: &str) -> Result<PathBuf, String> {
        // my.app.core -> my/app/core.clrs
        let rel_path = namespace.replace('.', "/") + ".clrs";

        // Try each search path
        for search_path in &self.search_paths {
            let full_path = search_path.join(&rel_path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }

        Err(format!("Module not found: {}", namespace))
    }
}
```

### Phase 4: Symbol Resolution

**New File**: `crates/clorus/src/namespace.rs`

```rust
pub struct NamespaceContext {
    /// Current namespace
    current: String,

    /// Namespace aliases: alias -> full namespace
    /// e.g., "str" -> "clorus.string"
    aliases: HashMap<String, String>,

    /// Imported symbols: symbol -> (namespace, original_name)
    /// e.g., "sin" -> ("clorus.math", "sin")
    imports: HashMap<String, (String, String)>,

    /// All defined symbols in current namespace
    definitions: HashSet<String>,
}

impl NamespaceContext {
    pub fn resolve_symbol(&self, name: &str) -> ResolvedSymbol {
        // Check for qualified symbol: namespace/symbol or alias/symbol
        if let Some((prefix, symbol)) = name.split_once('/') {
            if let Some(full_ns) = self.aliases.get(prefix) {
                return ResolvedSymbol::Qualified {
                    namespace: full_ns.clone(),
                    symbol: symbol.to_string(),
                };
            }
            return ResolvedSymbol::Qualified {
                namespace: prefix.to_string(),
                symbol: symbol.to_string(),
            };
        }

        // Check imported symbols
        if let Some((ns, orig_name)) = self.imports.get(name) {
            return ResolvedSymbol::Imported {
                namespace: ns.clone(),
                symbol: orig_name.clone(),
            };
        }

        // Local symbol in current namespace
        ResolvedSymbol::Local {
            namespace: self.current.clone(),
            symbol: name.to_string(),
        }
    }
}

pub enum ResolvedSymbol {
    Local { namespace: String, symbol: String },
    Qualified { namespace: String, symbol: String },
    Imported { namespace: String, symbol: String },
}
```

### Phase 5: Codegen Integration

**File**: `crates/clorus-codegen/src/codegen.rs`

Update to use namespaces:

```rust
impl<'ctx> CodeGen<'ctx> {
    /// Set current namespace context
    pub fn set_namespace(&mut self, ns: NamespaceContext) {
        self.namespace = ns;
    }

    /// Compile expression with namespace-aware symbol resolution
    fn compile_expr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            Expr::Symbol(name) => {
                let resolved = self.namespace.resolve_symbol(name);
                let mangled_name = self.mangle_symbol(&resolved);
                // Look up variable by mangled name
                self.lookup_variable(&mangled_name)
            }

            Expr::Defn { name, params, body } => {
                let resolved = ResolvedSymbol::Local {
                    namespace: self.namespace.current.clone(),
                    symbol: name.clone(),
                };
                let mangled_name = self.mangle_symbol(&resolved);
                // Define function with mangled name
                self.compile_defn(&mangled_name, params, body)
            }

            // ...
        }
    }

    /// Mangle symbol name with namespace
    /// my.app.core/add -> clorus_my_app_core_add
    fn mangle_symbol(&self, resolved: &ResolvedSymbol) -> String {
        match resolved {
            ResolvedSymbol::Local { namespace, symbol } |
            ResolvedSymbol::Qualified { namespace, symbol } |
            ResolvedSymbol::Imported { namespace, symbol } => {
                format!("clorus_{}_{}",
                    namespace.replace('.', "_"),
                    symbol.replace('-', "_"))
            }
        }
    }
}
```

---

## Usage Examples

### Example 1: Simple Module

**src/math.clrs**:
```clojure
(ns math)

(defn add [x y]
  (+ x y))

(defn multiply [x y]
  (* x y))
```

**src/main.clrs**:
```clojure
(ns main
  (:require [math :as m]))

(def result (m/add 10 20))
(println result)  ; => 30
```

### Example 2: Selective Imports

**src/util.clrs**:
```clojure
(ns util)

(defn format-number [n]
  (str "Number: " n))

(defn format-string [s]
  (str "String: " s))

(defn helper [x]
  (* x 2))
```

**src/main.clrs**:
```clojure
(ns main
  (:require [util :refer [format-number format-string]]))

; Can use directly (no util/ prefix)
(println (format-number 42))    ; => "Number: 42"
(println (format-string "hi"))  ; => "String: hi"

; helper is NOT imported
; (helper 5)  ; Error: undefined symbol
```

### Example 3: GUI with Namespaces

**src/config.clrs**:
```clojure
(ns config)

(def window-title "My App")
(def window-width 800.0)
(def window-height 600.0)
```

**src/main.clrs**:
```clojure
(ns main
  (:require [config :as cfg])
  (:rust [egui-hello :as gui]))

(def window-config (str "{"
  "\"title\": \"" cfg/window-title "\","
  "\"width\": " cfg/window-width ","
  "\"height\": " cfg/window-height
  "}"))

(gui/show-gui-json window-config)
```

---

## Migration Path

### Current Code (No Namespaces)

```clojure
; main.clrs
(use rust.egui-hello)

(defn my-func [x]
  (+ x 1))

(egui-hello/show-gui "Hello")
```

**Works without changes!** - Files without `(ns ...)` use a default namespace.

### With Namespaces

```clojure
; main.clrs
(ns main
  (:rust [egui-hello :as gui]))

(defn my-func [x]
  (+ x 1))

(gui/show-gui "Hello")
```

**Explicit and modular!**

---

## Implementation Checklist

### Phase 1: Core Foundation
- [ ] Add `Ns` and `Require` to AST
- [ ] Implement `parse_ns()` in parser
- [ ] Implement `parse_require()` in parser
- [ ] Add tests for parsing

### Phase 2: Module Loading
- [ ] Create `ModuleLoader` struct
- [ ] Implement `load_module()`
- [ ] Implement `namespace_to_path()`
- [ ] Add search path configuration
- [ ] Add module caching

### Phase 3: Symbol Resolution
- [ ] Create `NamespaceContext` struct
- [ ] Implement `resolve_symbol()`
- [ ] Handle aliases (`:as`)
- [ ] Handle selective imports (`:refer`)
- [ ] Add tests for resolution

### Phase 4: Codegen Integration
- [ ] Add namespace context to CodeGen
- [ ] Implement symbol mangling
- [ ] Update function definitions to use mangled names
- [ ] Update function calls to resolve namespaces
- [ ] Update variable resolution

### Phase 5: REPL Support
- [ ] Track current namespace in REPL
- [ ] Support `(ns ...)` command
- [ ] Support `(require ...)` at REPL
- [ ] Display current namespace in prompt

### Phase 6: Testing
- [ ] Test simple module loading
- [ ] Test qualified symbols
- [ ] Test aliases
- [ ] Test selective imports
- [ ] Test circular dependencies (error handling)

---

## Timeline Estimate

| Phase | Duration | Description |
|-------|----------|-------------|
| Phase 1 | 1-2 days | AST and parser |
| Phase 2 | 1-2 days | Module loading |
| Phase 3 | 2-3 days | Symbol resolution |
| Phase 4 | 3-4 days | Codegen integration |
| Phase 5 | 1 day | REPL support |
| Phase 6 | 1-2 days | Testing |
| **Total** | **9-14 days** | Full implementation |

---

## Benefits

1. **Code Organization** - Logical separation of concerns
2. **Reusability** - Share code across projects
3. **Name Collision Prevention** - Namespaces prevent conflicts
4. **Explicit Dependencies** - Clear what each module needs
5. **Standard Library** - Foundation for stdlib modules
6. **Better IDEs** - Tools can understand module structure

---

## Future Enhancements

1. **Private Symbols** - `(defn- private-func [])`
2. **Namespace Metadata** - `(ns ^{:author "..." :doc "..."} my.ns)`
3. **Conditional Loading** - `(require '[my.lib :refer :all :when *compiled*])`
4. **Auto-aliasing** - Common aliases like `str`, `io`, `json`
5. **Namespace Reloading** - For REPL development
