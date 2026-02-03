# Clorus Namespace Conventions & Clojure Parity

## Question 1: Folder Structure Convention

### YES - Clorus Follows Clojure Folder Structure! ✅

**Current Implementation** (commands.rs:410-412):
```rust
// Convert module name to file path: math -> src/math.clrs
let module_path = module_name.replace('.', "/");
let module_file = project_root.join("src").join(format!("{}.clrs", module_path));
```

**Mapping:**
```
com.example.core    →  src/com/example/core.clrs
examples.factorial  →  src/examples/factorial.clrs
my.app.server       →  src/my/app/server.clrs
```

### Example Project Structure

```
my-project/
├── Clorus.toml
└── src/
    ├── main.clrs                    ; (ns main)
    └── com/
        └── example/
            ├── core.clrs            ; (ns com.example.core)
            ├── server.clrs          ; (ns com.example.server)
            └── util/
                ├── string.clrs      ; (ns com.example.util.string)
                └── math.clrs        ; (ns com.example.util.math)
```

### Current State vs Recommendation

**Current Examples (Flat Structure):**
```
namespace-test/src/
├── main.clrs    ; (ns main)
├── math.clrs    ; (ns math)
└── util.clrs    ; (ns util)
```

**Recommended Structure (Clojure Convention):**
```
namespace-test/src/
├── main.clrs           ; (ns namespace-test.main)
├── namespace_test/     ; Create directory
│   ├── math.clrs       ; (ns namespace-test.math)
│   └── util.clrs       ; (ns namespace-test.util)
```

---

## Question 2: Interface File Extensions

### Current: `.clorus-ffi`

**Location:** `interfaces/library-name.clorus-ffi`

**Example:**
```
my-project/
└── interfaces/
    ├── egui-hello.clorus-ffi
    ├── serde-json.clorus-ffi
    └── async-hello.clorus-ffi
```

### Proposed Alternatives

#### Option 1: `.cli` (Clorus Interface)
```
interfaces/
├── egui-hello.cli
├── serde-json.cli
└── async-hello.cli
```

**Pros:**
- ✅ Shorter, cleaner
- ✅ Follows common convention (.h, .hpp, .d.ts)
- ✅ Language-agnostic (not tied to FFI term)

**Cons:**
- ⚠️ Could conflict with "Command Line Interface" tools
- ⚠️ Less descriptive

#### Option 2: `.rsi` (Rust Interop/Interface)
```
interfaces/
├── egui-hello.rsi
├── serde-json.rsi
└── async-hello.rsi
```

**Pros:**
- ✅ Clearly indicates Rust interface
- ✅ Unique extension (no conflicts)
- ✅ Language-specific clarity

**Cons:**
- ⚠️ Locks us into Rust (what about C/C++ FFI?)
- ⚠️ Less generic

#### Option 3: Keep `.clorus-ffi`
**Pros:**
- ✅ Extremely clear and descriptive
- ✅ No ambiguity about purpose
- ✅ Already established

**Cons:**
- ⚠️ Longer file extension
- ⚠️ Verbose

### Recommendation

**Short term:** Keep `.clorus-ffi` (already working, clear)

**Long term:** Consider `.cli` when:
- We add non-Rust FFI support (C, C++, etc.)
- We have multiple interface types
- Community consensus emerges

**Implementation:** Make it configurable in Clorus.toml:
```toml
[build]
interface-extension = "cli"  # Default: "clorus-ffi"
```

---

## Question 3: Namespace Declaration - Inside vs Outside

### Current State: BOTH are supported! ✅

#### Style 1: Clojure-style (Inside `ns` form) ✅ **SUPPORTED**

**Example:** namespace-test/src/main.clrs
```clojure
(ns main
  (:require [math :as m]
            [util :refer [double triple]]))

(def result (m/add 10 20))
```

**Parser Support:** parser.rs:1399-1451
```rust
fn parse_ns(&mut self) -> Result<Expr, String> {
    // (:require [my.lib :as lib] [other.lib :refer [func1 func2]])
    // (:rust [egui-hello :as gui])

    // Parse optional clauses: (:require ...) (:rust ...)
    match self.current_token() {
        Token::Keyword(kw) if kw == "require" => {
            // Parse require specs...
        }
        Token::Keyword(kw) if kw == "rust" => {
            // Parse rust import specs...
        }
        _ => return Err("Unknown ns clause. Expected :require or :rust")
    }
}
```

#### Style 2: Separate `use` form ✅ **ALSO SUPPORTED**

**Example:** string-value-test/src/main.clrs
```clojure
(ns examples.value-operations)

(use clorus.core)

(def x 10)
```

**Parser Support:** parser.rs:1174-1184
```rust
fn parse_use(&mut self) -> Result<Expr, String> {
    // (use rust.fs) or (use rust.fs [read write])
    self.advance(); // Skip 'use'

    // Parse module path (e.g., rust.fs)
    let module = match self.current_token() {
        Token::Symbol(name) => name.clone(),
        // ...
    }
}
```

### Why Both Exist?

1. **`use` form** - Simpler, backwards compatible:
   ```clojure
   (ns my.app)
   (use clorus.core)
   (use rust.fs)
   ```

2. **`:require` inside ns** - Clojure idiomatic, better organization:
   ```clojure
   (ns my.app
     (:require [clorus.core :refer :all]
               [my.lib :as lib]))
   ```

3. **`:rust` inside ns** - FFI imports grouped with namespace:
   ```clojure
   (ns gui.demo
     (:rust [egui-hello :as gui]))
   ```

---

## Clojure Parity Analysis

### What Clorus Currently Supports ✅

| Feature | Clojure | Clorus | Status |
|---------|---------|--------|--------|
| Namespace declaration | `(ns ...)` | `(ns ...)` | ✅ Full |
| `:require` clause | ✅ | ✅ | ✅ Supported |
| `:as` alias | ✅ | ✅ | ✅ Supported |
| `:refer [...]` | ✅ | ✅ | ✅ Supported |
| `:refer :all` | ✅ | ✅ | ✅ Supported |
| Folder structure | `src/com/example/core.clj` | `src/com/example/core.clrs` | ✅ Supported |
| Module loading | Automatic | Automatic | ✅ Supported |

### What Clorus Doesn't Support Yet ❌

| Feature | Clojure | Clorus | Impact |
|---------|---------|--------|--------|
| `:import` clause | Java classes | N/A | ⚠️ Different model (uses `:rust`) |
| `:use` inside ns | ✅ Deprecated | ❌ Not in ns | ⚠️ Use separate `(use ...)` |
| `:gen-class` | ✅ | ❌ | ⚠️ No AOT compilation yet |
| `:load` | ✅ | ❌ | ⚠️ Not needed yet |
| `:only` in require | ✅ | ❌ | ⚠️ Use `:refer` instead |
| `:exclude` | ✅ | ❌ | ⚠️ Not implemented |
| `:rename` | ✅ | ❌ | ⚠️ Not implemented |

### Clorus Extensions (Not in Clojure)

| Feature | Purpose | Example |
|---------|---------|---------|
| `:rust` clause | Rust FFI imports | `(:rust [lib :as l])` |
| `(use rust.fs)` | Direct Rust module use | Simple FFI access |
| `.clorus-ffi` files | Interface definitions | FFI contracts |

---

## Recommendations

### 1. Update Examples to Follow Clojure Convention ✅

**Current:**
```
factorial-demo/src/
└── main.clrs    ; (ns examples.factorial)
```

**Should be:**
```
factorial-demo/src/
└── examples/
    └── factorial.clrs    ; (ns examples.factorial)
```

### 2. Standardize on `:require` Inside `ns` Form ✅

**Instead of:**
```clojure
(ns examples.value-operations)
(use clorus.core)
```

**Prefer:**
```clojure
(ns examples.value-operations
  (:require [clorus.core :refer :all]))
```

### 3. Add `:use` Support Inside `ns` Form (Future)

**Clojure way** (deprecated but still works):
```clojure
(ns my.app
  (:use [clojure.string :only [split join]]))
```

**Implementation:** Add `:use` keyword to parser.rs parse_ns():
```rust
Token::Keyword(kw) if kw == "use" => {
    // Parse use specs (same as :require but implies :refer :all)
}
```

### 4. Interface File Extension Decision

**Option A:** Keep `.clorus-ffi` (stable, clear)

**Option B:** Migrate to `.cli` (shorter, modern)
```bash
# Migration script
find interfaces/ -name "*.clorus-ffi" -exec rename 's/\.clorus-ffi$/.cli/' {} \;
```

---

## Implementation Roadmap

### Phase 1: Documentation Update (This Week)
- [ ] Document folder structure convention
- [ ] Update all examples to use proper structure
- [ ] Add migration guide from flat to nested

### Phase 2: Namespace Improvements (Next Week)
- [ ] Add `:use` support inside `ns` form
- [ ] Add `:only`, `:exclude`, `:rename` to `:require`
- [ ] Better error messages for namespace issues

### Phase 3: Interface Standardization (Future)
- [ ] Decide on final extension (keep `.clorus-ffi` or move to `.cli`)
- [ ] Make it configurable in Clorus.toml
- [ ] Add migration tooling

---

## Examples: Correct Structure

### Example 1: Web Server

```
my-web-server/
├── Clorus.toml
└── src/
    ├── main.clrs                    ; Entry point
    └── my_web_server/
        ├── server.clrs              ; (ns my-web-server.server)
        ├── routes.clrs              ; (ns my-web-server.routes)
        └── middleware/
            ├── auth.clrs            ; (ns my-web-server.middleware.auth)
            └── logging.clrs         ; (ns my-web-server.middleware.logging)
```

**main.clrs:**
```clojure
(ns my-web-server.main
  (:require [my-web-server.server :as server]
            [my-web-server.routes :refer [app-routes]])
  (:rust [httpkit :as http]))

(defn -main [args]
  (server/start app-routes 8080))
```

### Example 2: GUI Application

```
gui-app/
├── Clorus.toml
├── interfaces/
│   └── egui-hello.clorus-ffi
└── src/
    └── gui_app/
        ├── core.clrs                ; (ns gui-app.core)
        ├── components/
        │   ├── button.clrs          ; (ns gui-app.components.button)
        │   └── window.clrs          ; (ns gui-app.components.window)
        └── util/
            └── layout.clrs          ; (ns gui-app.util.layout)
```

**core.clrs:**
```clojure
(ns gui-app.core
  (:require [gui-app.components.button :as btn]
            [gui-app.components.window :as win])
  (:rust [egui-hello :as gui]))

(defn -main [args]
  (win/create-window "My App"))
```

---

## Summary

### Current State: 🟢 **EXCELLENT Clojure Parity**

1. ✅ **Folder Structure** - Full Clojure convention support
2. ✅ **Namespace Declaration** - `:require` and `:rust` inside `ns` work
3. ✅ **Module System** - Automatic loading like Clojure
4. ✅ **Aliases & Refers** - Full support

### Action Items:

1. **Update Examples** - Use proper folder structure (com.example.core style)
2. **Standardize** - Prefer `:require` inside `ns` over separate `use`
3. **Document** - Make folder convention explicit in guides
4. **Interface Extension** - Keep `.clorus-ffi` for now, revisit later

### The Gap:

- ❌ `:use` inside ns form (minor - use separate `use` statement)
- ❌ `:only`, `:exclude`, `:rename` in require (low priority)
- ❌ `:import` (not applicable - we use `:rust` instead)

**Clorus is very close to Clojure namespace conventions!** 🎉
