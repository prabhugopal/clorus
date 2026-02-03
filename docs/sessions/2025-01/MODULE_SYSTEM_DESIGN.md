# Clorus Module System Design

## Goals

Clojure-style module system with:
- **Namespaces** - Organize code into logical units
- **Load** - Simple file loading
- **Require** - Import with aliasing
- **Use** - Import all symbols

## Phase 1: Simple File Loading

### Syntax

```clojure
; utils.clrs
(defn add [x y] (+ x y))
(defn multiply [x y] (* x y))

; main.clrs
(load "utils.clrs")  ; Load and evaluate utils.clrs
(add 10 20)          ; => 30
(multiply 3 4)       ; => 12
```

### Implementation

1. **AST Node**:
```rust
Expr::Load { path: String }
```

2. **Parser**: Recognize `(load "path")`

3. **Codegen**:
   - Read file at path
   - Parse it
   - Compile it into the same module
   - All definitions become available

4. **Relative Paths**: Resolve relative to current file's directory

---

## Phase 2: Namespaces

### Syntax

```clojure
; math/utils.clrs
(ns math.utils)

(defn add [x y] (+ x y))
(defn subtract [x y] (- x y))

; main.clrs
(ns main)

(require 'math.utils)
(math.utils/add 10 20)      ; Qualified call

; Or with alias
(require '[math.utils :as m])
(m/add 10 20)               ; Aliased call

; Or refer specific symbols
(require '[math.utils :refer [add]])
(add 10 20)                 ; Direct call
```

### Implementation

1. **AST Nodes**:
```rust
// Declare namespace for current file
Expr::Ns { name: String }

// Import another namespace
Expr::Require {
    namespace: String,
    alias: Option<String>,
    refer: Vec<String>,
}
```

2. **Symbol Table**:
```rust
struct SymbolTable {
    current_namespace: String,
    namespaces: HashMap<String, HashMap<String, Value>>,
    aliases: HashMap<String, String>,
}
```

3. **Symbol Resolution**:
   - `add` → Look in current namespace
   - `math.utils/add` → Look in math.utils namespace
   - `m/add` → Resolve alias `m` → `math.utils`, then look up `add`

---

## Phase 3: Advanced Features

### Use (Import All)

```clojure
(use 'math.utils)
(add 10 20)        ; All symbols from math.utils available
```

### Private Definitions

```clojure
(ns math.utils)

(defn- helper [x]  ; Private (notice -)
  (* x 2))

(defn public-fn [x]
  (helper x))
```

### Import from Rust

Keep existing:
```clojure
(use rust.async-hello)
(async-hello/add-blocking 10 20)
```

---

## Implementation Phases

### Phase 1: Load (Simplest)
**Files to Modify:**
1. `clorus-syntax/src/ast.rs` - Add `Expr::Load`
2. `clorus-syntax/src/parser.rs` - Parse `(load "path")`
3. `clorus-codegen/src/codegen.rs` - Implement file loading

**Timeline:** 1-2 days

### Phase 2: Namespaces
**Files to Modify:**
1. `clorus-syntax/src/ast.rs` - Add `Expr::Ns`, `Expr::Require`
2. `clorus-syntax/src/parser.rs` - Parse ns/require forms
3. `clorus-codegen/src/codegen.rs` - Symbol table with namespaces
4. Add namespace resolution logic

**Timeline:** 3-4 days

### Phase 3: Advanced
**Features:**
- `use` (import all)
- Private definitions
- Re-export
- Cyclic dependency detection

**Timeline:** 2-3 days

---

## Example Usage

### Simple Project Structure

```
my-project/
├── Clorus.toml
├── src/
│   ├── main.clrs           (ns main)
│   ├── utils.clrs          (ns utils)
│   └── math/
│       ├── core.clrs       (ns math.core)
│       └── stats.clrs      (ns math.stats)
└── target/
```

### main.clrs
```clojure
(ns main)

(require '[utils :as u])
(require '[math.core :as m])

(defn -main []
  (let [data [1 2 3 4 5]
        sum (m/sum data)
        formatted (u/format-number sum)]
    (println formatted)))
```

### utils.clrs
```clojure
(ns utils)

(defn format-number [n]
  (str "Result: " n))
```

### math/core.clrs
```clojure
(ns math.core)

(defn sum [numbers]
  (reduce + 0 numbers))

(defn avg [numbers]
  (/ (sum numbers) (count numbers)))
```

---

## Comparison to Other Languages

### Clojure
```clojure
(ns my.app
  (:require [clojure.string :as str]
            [my.utils :refer [add]]))
```

### Python
```python
import my.utils as utils
from my.math import add
```

### Rust
```rust
use my::utils;
use my::math::add;
```

### Our Clorus (Phase 2)
```clojure
(ns my.app)
(require '[my.utils :as utils])
(require '[my.math :refer [add]])
```

Very similar to Clojure!

---

## Benefits

1. **Code Organization** - Logical separation of concerns
2. **Reusability** - Share code across projects
3. **Avoid Conflicts** - Namespace prefixes prevent name collisions
4. **Familiar** - Clojure developers feel at home
5. **Scalable** - Support large codebases

---

## Next Steps

1. Start with Phase 1 (Load) - Simple and immediately useful
2. Test thoroughly
3. Then add Phase 2 (Namespaces) - Full power
4. Document and create examples

Ready to implement?
