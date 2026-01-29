# Rust Interop Roadmap - Automatic FFI Generation

## Vision
Make Rust crates usable from Clorus with ZERO wrapper code - just declare the dependency and use it.

## Current Status (Jan 2026)

### ✅ Working
- Manual FFI (rust.fs, clorus.core)
- Value* type system with String support
- Binary compilation
- Reference counting

### 🚧 In Progress
- Automatic binding generation

### 📋 Planned
- Complex type mappings (Vec, HashMap, structs)
- Generic support
- Trait objects

---

## Phase 1: Simple Function Wrapping (1-2 weeks)

**Goal:** Auto-wrap Rust functions with simple types (primitives, String)

**Example:**
```rust
// In a Rust crate "mylib"
pub fn add(x: f64, y: f64) -> f64 {
    x + y
}

pub fn greet(name: String) -> String {
    format!("Hello, {}", name)
}
```

```clojure
; In Clorus - automatic!
(use rust.mylib)
(mylib/add 1 2)           ; => 3
(mylib/greet "Alice")     ; => "Hello, Alice"
```

**Implementation Steps:**
1. ✅ Create FFI generator crate (`clorus-ffi-gen`)
2. Parse Rust function signatures from compiled crates
3. Generate C-compatible wrapper functions
4. Generate LLVM declarations in codegen
5. Update clorus CLI to auto-generate on build

**Type Mappings:**
- `f64` ↔ `f64` (direct)
- `String` ↔ `Value*` (box/unbox)
- `bool` ↔ `f64` (0.0/1.0)
- `()` → void

---

## Phase 2: Collections (2-3 weeks)

**Goal:** Support Vec, HashMap automatically

**Example:**
```rust
pub fn sum_vec(nums: Vec<f64>) -> f64 {
    nums.iter().sum()
}

pub fn get_names(map: HashMap<String, String>) -> Vec<String> {
    map.keys().cloned().collect()
}
```

```clojure
(mylib/sum-vec [1 2 3 4 5])           ; => 15
(mylib/get-names {:alice "Alice"
                  :bob "Bob"})         ; => ["alice" "bob"]
```

**Implementation:**
- Vector ↔ PersistentVector (already exists!)
- HashMap ↔ Clorus Map
- Automatic conversion at FFI boundary

---

## Phase 3: Structs & Enums (3-4 weeks)

**Goal:** Rust structs become Clorus maps/records

**Example:**
```rust
#[derive(Clone)]
pub struct Person {
    pub name: String,
    pub age: u32,
}

pub fn create_person(name: String, age: u32) -> Person {
    Person { name, age }
}

pub fn get_age(person: &Person) -> u32 {
    person.age
}
```

```clojure
(def alice (mylib/create-person "Alice" 30))
(mylib/get-age alice)                  ; => 30

; Or access like a map:
(get alice :name)                      ; => "Alice"
```

**Implementation:**
- Serialize structs to Clorus maps
- Use reflection or derive macros
- Handle references (&T) with lifetimes

---

## Phase 4: Real-World Crates (4+ weeks)

**Goal:** Popular crates work out of the box

### Example 1: HTTP Requests
```clojure
[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }

(use rust.reqwest.blocking)
(def response (reqwest/get "https://api.github.com/users/octocat"))
(def body (response/text))
(println body)
```

### Example 2: JSON
```clojure
[dependencies]
serde_json = "1.0"

(use rust.serde-json)
(def json "{\"name\":\"Alice\",\"age\":30}")
(def data (serde-json/from-str json))
(println (get data :name))  ; => "Alice"
```

### Example 3: Regex
```clojure
[dependencies]
regex = "1.5"

(use rust.regex)
(def re (regex/Regex::new "[0-9]+"))
(regex/is-match re "abc123")  ; => true
```

**Challenges:**
- Error handling (Result<T, E>)
- Options (Option<T>)
- Lifetimes & borrows
- Generics

---

## Phase 5: OS & System Programming (Future)

**Goal:** Build operating systems, games, distributed systems

### Example: Tokio (Async)
```clojure
[dependencies]
tokio = { version = "1", features = ["full"] }

(use rust.tokio)
(defn async-main []
  (tokio/spawn
    (fn [] (println "Hello from async!"))))
```

### Example: Game Engine
```clojure
[dependencies]
bevy = "0.12"

(use rust.bevy)
(defn main []
  (bevy/App::new
    (bevy/add-plugins bevy/DefaultPlugins)
    (bevy/add-systems startup setup)
    (bevy/run)))
```

---

## Technical Architecture

### 1. Build Process
```
Clorus.toml → cargo fetch → extract metadata → generate FFI → compile
```

### 2. FFI Generator
- Uses `syn` crate to parse Rust AST
- Generates C-compatible wrappers
- Creates LLVM declarations
- Handles memory management

### 3. Type Bridge
```
Rust Type        C FFI            Clorus Type
---------        -----            -----------
f64           → f64            → Number
String        → *mut c_char   → Value*(String)
Vec<T>        → *mut array    → Vector
HashMap<K,V>  → *mut hashmap  → Map
struct Foo    → *mut Foo      → Map {:field value}
```

### 4. Memory Management
- Rust → Clorus: Transfer ownership, increment refcount
- Clorus → Rust: Clone data, Rust owns the copy
- Reference counting prevents leaks

---

## Success Metrics

### Phase 1 Complete When:
- ✅ Can call any Rust fn with primitives/String
- ✅ No manual wrapper code needed
- ✅ Works in both JIT and AOT modes

### Phase 2 Complete When:
- ✅ Vec/HashMap work automatically
- ✅ Can use std::collections crates

### Phase 3 Complete When:
- ✅ Custom structs work as maps
- ✅ Can define data structures in Rust, use in Clorus

### Phase 4 Complete When:
- ✅ reqwest, serde_json, regex work
- ✅ Can build real applications

---

## Next Immediate Steps

1. **Create `clorus-ffi-gen` crate** - FFI binding generator
2. **Simple proof of concept** - One function, end-to-end
3. **Integrate with CLI** - Auto-generate on build
4. **Add to examples** - Show it working

---

## Design Decisions

### Why Automatic?
Manual FFI is tedious and error-prone. Clojure's Java interop is seamless - we want the same.

### Why Zero Copy When Possible?
Performance. Direct memory sharing when safe.

### Why Maps for Structs?
Clojure philosophy - data as maps. Plus it's dynamic and flexible.

### Why Not bindgen?
bindgen generates C bindings. We need higher-level, type-aware Clorus bindings.

---

**Built with ❤️ for seamless Rust-Clorus interop**
