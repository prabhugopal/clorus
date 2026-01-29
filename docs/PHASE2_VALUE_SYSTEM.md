# Phase 2b: Value* Type System - Implementation Plan

**Status:** 🚧 In Progress
**Started:** January 26, 2025
**Estimated:** 9-14 days
**Goal:** Transform Clorus from f64-only to full multi-type system

---

## Executive Summary

Phase 2b upgrades Clorus from a single-type (f64) system to a complete Value* type system, enabling:
- ✅ Proper strings (not pointer hacks)
- ✅ Collections: vectors, maps, sets, lists
- ✅ Clean FFI integration
- ✅ Foundation for 60% Clojure language coverage

**Critical Design Decision:** Zero breaking changes. All existing code keeps working.

---

## Current State vs Target State

### Current (Phase 1): f64-Only
```rust
// codegen.rs
pub fn compile_expr(&mut self, expr: &Expr) -> Result<FloatValue<'ctx>, String> {
    match expr {
        Expr::Number(n) => Ok(self.context.f64_type().const_float(*n)),
        Expr::String(_) => Err("Strings not supported!"),
        _ => // ...
    }
}
```

**Problems:**
- ❌ Can only handle numbers
- ❌ String returns are pointer→f64 hacks
- ❌ Collections completely blocked
- ❌ Stuck at 35% language coverage

### Target (Phase 2b): Value* System
```rust
// codegen.rs
pub fn compile_expr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
    match expr {
        Expr::Number(n) => {
            let float_val = self.context.f64_type().const_float(*n);
            Ok(self.box_number(float_val))  // Returns Value*
        }
        Expr::String(s) => {
            let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
            Ok(self.box_string(c_str.as_pointer_value()))  // Returns Value*
        }
        Expr::Vector(items) => {
            // Now possible!
            Ok(self.compile_vector(items)?)
        }
        _ => // ...
    }
}
```

**Benefits:**
- ✅ All types work naturally
- ✅ Clean FFI (no hacks)
- ✅ Collections enabled
- ✅ Jump to 60% coverage

---

## Backward Compatibility Guarantee

### User Code: Zero Changes
```clojure
; This code works BEFORE Phase 2b
(ns gui.demo
  (:rust [egui-hello :as gui]))

(gui/show-gui "Hello")
(def x 42)

; This SAME code works AFTER Phase 2b
; No migration needed!
```

### FFI: Both Approaches Keep Working

**Phase 1 Auto-Parse (still supported):**
```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello" }  # No changes needed
```

**Phase 2a Interface Files (coming later):**
```toml
[rust-dependencies]
complex-lib = {
  path = "../complex-lib",
  interface = "complex.clorus-ffi"  # Optional upgrade
}
```

### Internal: Cleaner Implementation
```rust
// Phase 1: Hack
let str_ptr = ...; // String pointer
let ptr_int = self.builder.build_ptr_to_int(str_ptr, i64_type, "ptr_to_int");
let ptr_float = self.builder.build_unsigned_int_to_float(ptr_int, f64_type, "hack");

// Phase 2b: Clean
let str_ptr = ...;
Ok(self.box_string(str_ptr))  // Returns Value*
```

---

## Architecture: Three-Layer System

```
┌─────────────────────────────────────────────┐
│  SYNTAX LAYER (clorus-syntax)               │  ✅ COMPLETE
│  - Parser: handles all types                │     (No changes needed)
│  - AST: Expr enum with all variants         │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│  CODEGEN LAYER (clorus-codegen)             │  🚧 PHASE 2B
│  - compile_expr() returns PointerValue      │     (Main work here)
│  - All variables are Value* allocations     │
│  - Type conversions via runtime helpers     │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│  RUNTIME LAYER (clorus-runtime)             │  ✅ MOSTLY COMPLETE
│  - Value type with ValueTag enum            │     (Minor additions)
│  - Reference counting implemented           │
│  - Vector & List work                       │
└─────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Runtime String Support (1-2 days)

**Goal:** Add String type to the runtime Value system

**File:** `crates/clorus-runtime/src/value.rs`

**Changes:**

1. **Implement String value creation:**
```rust
impl Value {
    pub fn string(s: &str) -> *mut Value {
        let data = ValueData {
            ptr: Box::into_raw(Box::new(s.to_string())) as *mut u8
        };
        let header = Header::new(ValueTag::String);
        Box::into_raw(Box::new(Value { header, data }))
    }

    pub fn as_string(&self) -> &str {
        unsafe {
            let ptr = self.data.ptr as *const String;
            &*ptr
        }
    }
}
```

2. **Add String deallocation:**
```rust
// In dealloc() method
ValueTag::String => {
    let ptr = self.data.ptr as *mut String;
    let _ = Box::from_raw(ptr);
}
```

3. **Add FFI helpers:**
```rust
#[no_mangle]
pub extern "C" fn clorus_value_string(ptr: *const c_char) -> *mut Value {
    if ptr.is_null() { return Value::nil(); }
    unsafe {
        let c_str = CStr::from_ptr(ptr);
        let s = c_str.to_str().unwrap_or("");
        Value::string(s)
    }
}

#[no_mangle]
pub extern "C" fn clorus_value_as_cstring(val: *mut Value) -> *mut c_char {
    if val.is_null() { return std::ptr::null_mut(); }
    unsafe {
        if (*val).header.tag() == ValueTag::String {
            let s = (*val).as_string();
            CString::new(s).unwrap().into_raw()
        } else {
            std::ptr::null_mut()
        }
    }
}
```

**Testing:** Compile and run string creation/deallocation tests

---

### Phase 2: Codegen Infrastructure Changes (2-3 days)

**Goal:** Change codegen to work with Value* instead of f64

**File:** `crates/clorus-codegen/src/codegen.rs`

**Major Changes:**

#### 2.1: Change compile_expr signature
```rust
// Before
pub fn compile_expr(&mut self, expr: &Expr) -> Result<FloatValue<'ctx>, String>

// After
pub fn compile_expr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String>
```

#### 2.2: Update variable allocations
```rust
// Before
fn create_entry_block_alloca(&self, name: &str) -> PointerValue<'ctx> {
    let f64_type = self.context.f64_type();
    builder.build_alloca(f64_type, name).unwrap()
}

// After
fn create_entry_block_alloca(&self, name: &str) -> PointerValue<'ctx> {
    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
    builder.build_alloca(value_ptr_type, name).unwrap()
}
```

#### 2.3: Add runtime helper declarations
```rust
fn declare_value_functions(&mut self) {
    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
    let f64_type = self.context.f64_type();
    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

    // clorus_value_number(f64) -> Value*
    let value_number_type = value_ptr_type.fn_type(&[f64_type.into()], false);
    self.module.add_function("clorus_value_number", value_number_type, None);

    // clorus_value_as_number(Value*) -> f64
    let value_as_number_type = f64_type.fn_type(&[value_ptr_type.into()], false);
    self.module.add_function("clorus_value_as_number", value_as_number_type, None);

    // clorus_value_string(*const c_char) -> Value*
    let value_string_type = value_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    self.module.add_function("clorus_value_string", value_string_type, None);

    // clorus_value_as_cstring(Value*) -> *mut c_char
    let value_as_cstring_type = i8_ptr_type.fn_type(&[value_ptr_type.into()], false);
    self.module.add_function("clorus_value_as_cstring", value_as_cstring_type, None);
}
```

#### 2.4: Helper methods for boxing/unboxing
```rust
impl<'ctx> CodeGen<'ctx> {
    /// Box an f64 into a Value*
    fn box_number(&self, float_val: FloatValue<'ctx>) -> PointerValue<'ctx> {
        let value_number_fn = self.module.get_function("clorus_value_number").unwrap();
        let call_result = self.builder.build_call(
            value_number_fn,
            &[float_val.into()],
            "box_number"
        ).unwrap();
        call_result.try_as_basic_value().left().unwrap().into_pointer_value()
    }

    /// Unbox a Value* to f64 (for arithmetic operations)
    fn unbox_number(&self, value_ptr: PointerValue<'ctx>) -> FloatValue<'ctx> {
        let value_as_number_fn = self.module.get_function("clorus_value_as_number").unwrap();
        let call_result = self.builder.build_call(
            value_as_number_fn,
            &[value_ptr.into()],
            "unbox_number"
        ).unwrap();
        call_result.try_as_basic_value().left().unwrap().into_float_value()
    }

    /// Box a C string into Value*
    fn box_string(&self, str_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let value_string_fn = self.module.get_function("clorus_value_string").unwrap();
        let call_result = self.builder.build_call(
            value_string_fn,
            &[str_ptr.into()],
            "box_string"
        ).unwrap();
        call_result.try_as_basic_value().left().unwrap().into_pointer_value()
    }

    /// Extract C string from Value*
    fn extract_cstring_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let value_as_cstring_fn = self.module.get_function("clorus_value_as_cstring").unwrap();
        let call_result = self.builder.build_call(
            value_as_cstring_fn,
            &[value_ptr.into()],
            "extract_cstring"
        ).unwrap();
        call_result.try_as_basic_value().left().unwrap().into_pointer_value()
    }
}
```

---

### Phase 3: Update Expression Compilation (3-4 days)

**Goal:** Rewrite all compile_expr branches to return Value*

#### 3.1: Number literals
```rust
// Before
Expr::Number(n) => {
    let float_type = self.context.f64_type();
    Ok(float_type.const_float(*n))
}

// After
Expr::Number(n) => {
    let float_type = self.context.f64_type();
    let float_val = float_type.const_float(*n);
    Ok(self.box_number(float_val))
}
```

#### 3.2: String literals
```rust
// Before
Expr::String(_s) => {
    Err("String literals cannot be used in numeric expressions yet".to_string())
}

// After
Expr::String(s) => {
    let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
    Ok(self.box_string(c_str.as_pointer_value()))
}
```

#### 3.3: Variable loading
```rust
// Before
Expr::Symbol(name) => {
    let ptr = self.variables.get(name)?;
    let val = self.builder.build_load(self.context.f64_type(), *ptr, name).unwrap();
    Ok(val.into_float_value())
}

// After
Expr::Symbol(name) => {
    let ptr = self.variables.get(name)?;
    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
    let val = self.builder.build_load(value_ptr_type, *ptr, name).unwrap();
    Ok(val.into_pointer_value())
}
```

#### 3.4: Arithmetic operations
```rust
// Before
fn compile_add(&mut self, args: &[Expr]) -> Result<FloatValue<'ctx>, String> {
    let mut result = self.compile_expr(&args[0])?;  // f64
    for arg in &args[1..] {
        let val = self.compile_expr(arg)?;
        result = self.builder.build_float_add(result, val, "add").unwrap();
    }
    Ok(result)
}

// After
fn compile_add(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
    // Unbox first arg
    let first_val_ptr = self.compile_expr(&args[0])?;
    let mut result = self.unbox_number(first_val_ptr);

    // Add remaining args
    for arg in &args[1..] {
        let val_ptr = self.compile_expr(arg)?;
        let val = self.unbox_number(val_ptr);
        result = self.builder.build_float_add(result, val, "add").unwrap();
    }

    // Box result
    Ok(self.box_number(result))
}
```

Apply same pattern to: subtract, multiply, divide, lt, gt, eq

---

### Phase 4: Fix FFI Functions (1-2 days)

**Goal:** Remove pointer→f64 hacks, return proper Value*

#### 4.1: slurp - Return string Value*
```rust
// Before
"slurp" => {
    // ... call clorus_slurp, get *mut c_char ...
    // Cast pointer to i64 then to f64 (HACK)
    let ptr_as_int = self.builder.build_ptr_to_int(ptr_result, ...);
    let ptr_as_float = self.builder.build_unsigned_int_to_float(ptr_as_int, ...);
    Ok(ptr_as_float)
}

// After
"slurp" => {
    let path_ptr = self.compile_expr(&args[0])?;
    let path_cstr = self.extract_cstring_from_value(path_ptr);

    let slurp_fn = self.module.get_function("clorus_slurp").unwrap();
    let result = self.builder.build_call(slurp_fn, &[path_cstr.into()], "slurp_call").unwrap();

    let str_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();
    Ok(self.box_string(str_ptr))  // Clean!
}
```

---

### Phase 5: Memory Management (1 day)

**Goal:** Add reference counting for proper memory management

Already implemented in runtime:
```rust
#[no_mangle]
pub extern "C" fn clorus_value_retain(val: *mut Value) -> *mut Value { ... }

#[no_mangle]
pub extern "C" fn clorus_value_release(val: *mut Value) { ... }
```

Add automatic release for let scopes in codegen.

---

### Phase 6: Testing and Validation (1-2 days)

**Goal:** Verify all features work with Value* system

#### 6.1: Update tests
Add tests for:
- String literals and operations
- String return values from FFI
- Mixed number/string expressions
- Reference counting (no leaks)

#### 6.2: Manual REPL testing
```clojure
λ> (use clorus.core)
=> 0

λ> (def content (slurp "test.txt"))
=> "Hello World!"   # NOT a pointer number!

λ> (def x [1 2 3])
=> [1 2 3]

λ> (+ 1 2)
=> 3
```

---

## Performance Tradeoffs

### Arithmetic: 5-10x Slower (Acceptable)

**Current (f64 only):**
```
(+ 1 2)  ; ~1 nanosecond (raw LLVM f64 add)
```

**Phase 2b (Value*):**
```
(+ 1 2)  ; ~5-10 nanoseconds
         ; 1. Unbox 1 from Value* → f64
         ; 2. Unbox 2 from Value* → f64
         ; 3. LLVM f64 add
         ; 4. Box result → Value*
```

**Why This Is Fine:**
1. Most programs bottleneck on I/O, not arithmetic
2. Clojure/JVM has same tradeoff (everything boxed)
3. Still 10-100x faster than Python/Ruby
4. Can optimize hot paths later

### Real-World Impact

```clojure
; Typical program
(defn process-users []
  (let [users (slurp "users.json")]        ; I/O: 1ms
    (->> users
         (parse-json)                       ; Parsing: 5ms
         (filter active?)                   ; 100 users: 10μs
         (map transform)                    ; 100 transforms: 20μs
         (db/save!))))                      ; Database: 50ms

; Total: ~56ms
; Arithmetic overhead: ~30μs (0.05% of total time!)
```

### Language Comparison

| Language | Implementation | Arithmetic Speed | Production Ready? |
|----------|---------------|------------------|-------------------|
| C | Raw CPU | 1x | ✅ Yes |
| Rust | Raw CPU | 1x | ✅ Yes |
| Go | Mostly raw | 1-2x | ✅ Yes |
| Java | JIT optimized | 1-3x | ✅ Yes |
| **Clojure/JVM** | **Boxed objects** | **5-10x** | **✅ Yes (production!)** |
| Python | Boxed PyObject* | 50-100x | ✅ Yes |
| Ruby | Boxed VALUE | 50-100x | ✅ Yes |
| **Clorus (f64)** | **Raw LLVM** | **1x** | **❌ Can't build apps** |
| **Clorus (Value*)** | **Boxed Value*** | **5-10x** | **✅ Can build apps!** |

**Key Insight:** Clojure on JVM proves boxed systems work in production at scale (Nubank: 40M users).

---

## Migration Path

### Phase 1 → Phase 2b: Automatic

**No user action required:**
- Existing code keeps working
- Same Clorus syntax
- Same FFI declarations
- Cleaner internals

### Example: gui-demo

**Before Phase 2b:**
```clojure
(ns gui.demo
  (:rust [egui-hello :as gui]))

(gui/show-gui "Hello")  ; Works (with internal hacks)
```

**After Phase 2b:**
```clojure
(ns gui.demo
  (:rust [egui-hello :as gui]))

(gui/show-gui "Hello")  ; Works (clean implementation)
```

**Zero changes needed!**

---

## Future: Phase 2a (Interface Files)

After Phase 2b completes, we'll add `.clorus-ffi` interface files:

```clojure
;; egui-hello.clorus-ffi
(interface egui-hello
  (defn show-gui [message :string] :f64
    "Show a GUI window with the given message")

  (defn get-gui-version [] :string
    "Get the egui version string"))
```

**Benefits of doing Value* first:**
- Can properly express complex types in interfaces
- Have clean Value* conversions implemented
- Better error handling available

---

## Success Criteria

**Phase 2b is complete when:**

1. ✅ `(slurp "file.txt")` returns actual string, not pointer number
2. ✅ String literals work: `(def s "hello")`
3. ✅ Arithmetic still works: `(+ 1 2)` => 3
4. ✅ Functions work with Value*: `(defn add [x y] (+ x y))`
5. ✅ No memory leaks (valgrind clean)
6. ✅ REPL shows proper values, not pointers
7. ✅ All existing tests pass
8. ✅ All FFI examples keep working (gui-demo, namespace-test, etc.)

---

## Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| Phase 1 | 1-2 days | Runtime string support |
| Phase 2 | 2-3 days | Codegen infrastructure |
| Phase 3 | 3-4 days | Update all expressions |
| Phase 4 | 1-2 days | Fix FFI functions |
| Phase 5 | 1 day | Memory management |
| Phase 6 | 1-2 days | Testing & validation |
| **Total** | **9-14 days** | **Full implementation** |

---

## Risk Mitigation

### Low Risk
- ✅ Runtime Value infrastructure exists
- ✅ Parser handles all types
- ✅ Reference counting implemented

### Medium Risk
- ⚠️ Large refactor of codegen (500+ lines)
- ⚠️ All existing code must be updated
- ⚠️ FFI boundary changes

### High Risk
- 🔴 LLVM type mismatches (pointer vs float)
- 🔴 Memory leaks if refcount wrong
- 🔴 Subtle bugs in boxing/unboxing

### Mitigation Strategy
1. Implement in phases with testing at each step
2. Keep backup branches
3. Add comprehensive tests before refactoring
4. Use valgrind/ASAN to detect memory issues
5. Test all existing projects after each phase

---

## References

- **Phase 1 Complete:** `/Users/prabhugopal/Learning/clorus/PHASE1_COMPLETE.md`
- **Original Plan:** `~/.claude/plans/humble-herding-lightning.md`
- **Runtime Source:** `crates/clorus-runtime/src/value.rs`
- **Codegen Source:** `crates/clorus-codegen/src/codegen.rs`

---

**Document Version:** 1.0
**Last Updated:** January 26, 2025
**Status:** Ready to implement
