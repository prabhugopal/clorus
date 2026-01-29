# Clorus FFI Roadmap - Complete Picture

**Last Updated:** January 26, 2025

---

## Three Phases of FFI

### Phase 1: Auto-Parse (✅ Complete)
**Status:** Production ready, in use today

**How it works:**
```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello" }
```

**Process:**
1. Parse Rust source with `syn`
2. Auto-generate `ffi.rs` wrappers
3. Compile to static + dynamic libraries
4. Link automatically

**Strengths:**
- ✅ Zero configuration
- ✅ Just reference and use
- ✅ Works today

**Limitations:**
- ⚠️ Limited to simple types (f64, i32, String, bool)
- ⚠️ Modifies library source (adds ffi.rs)
- ⚠️ Can't handle complex generics
- ⚠️ String returns use pointer→f64 hacks

---

### Phase 2b: Value* Type System (🚧 In Progress)
**Status:** Starting now (9-14 days)

**What it changes:**
```rust
// Before: Everything is f64
pub fn compile_expr(&mut self, expr: &Expr) -> Result<FloatValue<'ctx>, String>

// After: Everything is Value*
pub fn compile_expr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String>
```

**Impact on FFI:**
- ✅ Phase 1 auto-parse **keeps working**
- ✅ Cleaner internal implementation (no hacks)
- ✅ Can return proper strings (not f64 hacks)
- ✅ Foundation for collections
- ✅ Better type conversions

**Example transformation:**
```rust
// Phase 1: Hack
"slurp" => {
    let ptr_as_int = self.builder.build_ptr_to_int(...);
    let ptr_as_float = self.builder.build_unsigned_int_to_float(...);
    Ok(ptr_as_float)  // 😱 Pointer disguised as f64
}

// Phase 2b: Clean
"slurp" => {
    let str_ptr = ...;
    Ok(self.box_string(str_ptr))  // 😊 Proper Value*
}
```

**User Impact:** **ZERO** - existing code keeps working, just cleaner internally

---

### Phase 2a: Interface Files (📅 Future - After 2b)
**Status:** Planned for Week 3+

**How it works:**
```clojure
;; egui-hello.clorus-ffi (OCaml .mli style)
(interface egui-hello

  (defn show-gui [message :string] :f64
    "Show a GUI window with the given message")

  (defn show-gui-json [config :string] :f64
    "Show GUI configured via JSON")

  (defn get-gui-version [] :string
    "Get the egui version string")

  ;; Advanced: Map Rust Option<T>
  (defn maybe-find [key :string] [:option :string]
    :on-none nil
    :on-some identity)

  ;; Advanced: Map Rust Vec<T>
  (defn get-items [] [:vec :string]))
```

**Usage:**
```toml
[rust-dependencies]
egui-hello = {
  path = "../egui-hello",
  interface = "egui.clorus-ffi"  # Explicit interface
}
```

**Benefits:**
- ✅ No source modification (no ffi.rs in library)
- ✅ Can use crates.io directly
- ✅ Community-shareable interface files
- ✅ Handle complex types (Option, Result, Vec)
- ✅ Full control over exposed API
- ✅ Better documentation

**Why after Phase 2b?**
- Need Value* types to properly represent complex types
- Can cleanly map Rust types to Clorus types
- Better foundation for type conversions

---

## Complete Timeline

```
Week 0 (Current)
├─ Phase 1: Auto-Parse ✅ DONE
│  └─ Works in production today
│
Week 1-2: Phase 2b (Value* System)
├─ Day 1-2:   Runtime string support
├─ Day 3-5:   Codegen infrastructure
├─ Day 6-9:   Update all expressions
├─ Day 10-11: Fix FFI functions
├─ Day 12:    Memory management
└─ Day 13-14: Testing & validation
│
Week 3+: Phase 2a (Interface Files)
├─ Day 1-2: .clorus-ffi parser
├─ Day 3-4: Interface-based FFI generation
├─ Day 5:   Backward compat testing
└─ Result: Both auto-parse AND interface files work
```

---

## Developer Choice: Mix and Match

After all phases complete, developers can choose per-dependency:

```toml
[rust-dependencies]
# Simple library: Auto-parse (Phase 1)
math-lib = { path = "../math-lib" }

# Complex library: Interface file (Phase 2a)
http-client = {
  path = "../http-client",
  interface = "http.clorus-ffi"
}

# Community library: Pre-made interface
serde-json = {
  git = "https://github.com/clorus/serde-json-interface",
  interface = "serde.clorus-ffi"
}
```

**All three work together in same project!**

---

## Backward Compatibility Matrix

| Feature | Phase 1 | Phase 2b | Phase 2a |
|---------|---------|----------|----------|
| Auto-parse | ✅ Yes | ✅ Yes | ✅ Yes |
| Interface files | ❌ No | ❌ No | ✅ Yes |
| String returns | ⚠️ Hack | ✅ Clean | ✅ Clean |
| Collections | ❌ No | ✅ Yes | ✅ Yes |
| Complex types | ❌ No | ⚠️ Basic | ✅ Full |
| Source modification | ⚠️ Yes | ⚠️ Yes | ✅ No |
| Existing code works | ✅ Yes | ✅ Yes | ✅ Yes |

---

## Interface File Examples

### Simple Library
```clojure
;; math.clorus-ffi
(interface math
  (defn add [x :f64 y :f64] :f64)
  (defn multiply [x :f64 y :f64] :f64))
```

### String Library
```clojure
;; string-utils.clorus-ffi
(interface string-utils
  (defn to-upper [s :string] :string)
  (defn to-lower [s :string] :string)
  (defn split [s :string delimiter :string] [:vec :string]))
```

### Complex Types
```clojure
;; http.clorus-ffi
(interface http

  ;; Simple GET
  (defn get [url :string] [:result :string :string]
    :on-ok identity
    :on-err (fn [e] (error e)))

  ;; POST with JSON
  (defn post [url :string body :map] [:result :map :string])

  ;; Custom types
  (deftype HttpResponse
    {:status :i32
     :headers [:map :string :string]
     :body :string}))
```

### Platform Abstraction
```clojure
;; Same interface, multiple backends
;; http-reqwest.clorus-ffi
;; http-curl.clorus-ffi
;; Both implement same interface!

(interface http
  (defn fetch [url :string] :string)
  (defn post [url :string data :string] :string))
```

---

## Architecture: How They Work Together

```
┌────────────────────────────────────────────────────┐
│  Clorus Code                                       │
│  (ns app (:rust [lib :as l]))                      │
└────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────┐
│  FFI Processor (clorus-cli/rust_ffi.rs)            │
│                                                     │
│  if dep.interface.is_some() {                      │
│      // Phase 2a: Use interface file               │
│      process_interface_file()                      │
│  } else {                                          │
│      // Phase 1: Auto-parse (backward compat)      │
│      auto_parse_rust_source()                      │
│  }                                                  │
│                                                     │
│  // Both produce same output:                      │
│  FunctionMetadata { name, params, return_type }    │
└────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────┐
│  Wrapper Generator                                 │
│  - Generate ffi.rs (Phase 1) OR                    │
│  - Generate standalone wrapper (Phase 2a)          │
└────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────┐
│  Code Generation (clorus-codegen)                  │
│                                                     │
│  Phase 1:  FloatValue (f64 only)                   │
│  Phase 2b: PointerValue (Value* - all types)       │
└────────────────────────────────────────────────────┘
```

---

## Why This Approach?

### Compared to Other Languages

**Julia:** Direct C calls, simple syntax
- ✅ Easy to use
- ❌ No type safety
- ❌ Manual memory management

**Python ctypes/cffi:**
- ⚠️ Runtime overhead (100-500ns)
- ❌ No auto-generation
- ❌ Easy to crash

**Rust bindgen:**
- ✅ Type safe
- ❌ C/C++ only
- ❌ Complex setup

**Node.js N-API:**
- ⚠️ Moderate overhead (50-200ns)
- ❌ Manual wrappers
- ⚠️ Runtime type checks

**Clorus Approach:**
- ✅ Type safe (compile time)
- ✅ Low overhead (2-60ns)
- ✅ Auto-generation (Phase 1)
- ✅ OR explicit control (Phase 2a)
- ✅ Developer choice!

---

## Community Impact

### Phase 1 (Now)
Developers can:
- Use local Rust libraries
- Zero configuration
- Fast iteration

### Phase 2b (Soon)
Developers get:
- Cleaner internals
- Better type support
- Collections working
- Still zero config for simple libs

### Phase 2a (Later)
Community can:
- Share interface files
- Publish to registry
- Use crates.io libraries directly
- No source modification

**Example ecosystem:**
```
# Install pre-made interface files
clorus install reqwest-interface
clorus install serde-json-interface
clorus install tokio-interface

# Use in project
[rust-dependencies]
reqwest = { crates-io = "0.11", interface = "~/.clorus/interfaces/reqwest.clorus-ffi" }
```

---

## Success Metrics

### Phase 1 (✅ Complete)
- [x] Zero configuration
- [x] Auto-generation works
- [x] Good performance (70-99%)
- [x] Production ready

### Phase 2b (🚧 In Progress)
- [ ] No breaking changes
- [ ] All existing code works
- [ ] Clean Value* implementation
- [ ] Collections enabled
- [ ] 60% language coverage

### Phase 2a (📅 Future)
- [ ] Interface file parser works
- [ ] Both auto-parse AND interface files supported
- [ ] Community can share interfaces
- [ ] crates.io integration
- [ ] Complex types supported

---

## References

- **Phase 1 Documentation:** `/Users/prabhugopal/Learning/clorus/PHASE1_COMPLETE.md`
- **Phase 2b Plan:** `/Users/prabhugopal/Learning/git/clorus/docs/PHASE2_VALUE_SYSTEM.md`
- **FFI Guide:** `/Users/prabhugopal/Learning/clorus/RUST_FFI_GUIDE.md`
- **Quick Reference:** `/Users/prabhugopal/Learning/clorus/RUST_FFI_QUICKREF.md`

---

**Key Takeaway:** We're building a flexible, professional FFI system with backward compatibility at every step. Developers can choose the approach that fits their needs, and all approaches work together.
