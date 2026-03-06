# Hot Reload Architecture Design Document

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/PARITY_EXECUTION_PLAN.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Status:** 📋 DESIGN (Not Yet Implemented)
**Priority:** High (Post Core Features)
**Estimated Effort:** 3-4 days implementation
**Target:** Enable live code reloading during development

---

## Executive Summary

This document outlines the architecture for implementing hot code reload in Clorus, enabling developers to redefine functions at runtime without restarting the REPL. This matches Clojure's interactive development workflow while maintaining Clorus's native performance characteristics.

**Key Goal:** When a function is redefined, all subsequent calls use the new definition - even from previously compiled code.

---

## Problem Statement

### Current Behavior (Without Hot Reload)

```clojure
λ> (defn foo [] "old")
=> 0

λ> (defn bar [] (foo))  ; bar gets direct pointer to foo
=> 0

λ> (bar)
=> "old"  ; ✅ Works

λ> (defn foo [] "new")  ; Redefine foo
=> 0

λ> (foo)
=> "new"  ; ✅ Direct call works

λ> (bar)
=> "old"  ; ❌ bar still calls old foo!
```

**Problem:** `bar` has a direct LLVM function pointer to the original `foo`. When `foo` is redefined, `bar` doesn't see the change.

### Desired Behavior (With Hot Reload)

```clojure
λ> (defn foo [] "old")
λ> (defn bar [] (foo))

λ> (bar)
=> "old"

λ> (defn foo [] "new")  ; Redefine

λ> (bar)
=> "new"  ; ✅ bar automatically sees new foo!
```

---

## Architectural Approaches

### Approach 1: Indirect Function Dispatch (RECOMMENDED) ⭐

**Strategy:** Add one level of indirection through a global function registry.

#### Architecture

```
┌─────────────────────────────────────────┐
│  Compiled Code                          │
│                                         │
│  (bar)                                  │
│    ↓ calls                              │
│  lookup_function("foo")  ← Hash lookup  │
│    ↓ returns                            │
│  FunctionPtr from registry              │
│    ↓ indirect call                      │
│  Executes current foo implementation    │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  Global Function Registry               │
│  (HashMap<String, *mut u8>)             │
│                                         │
│  "foo" → [0x7f8a4000]  ← Points to     │
│  "bar" → [0x7f8a5000]     current      │
│  "baz" → [0x7f8a6000]     version      │
└─────────────────────────────────────────┘
```

#### Implementation Details

**1. Global Registry (Rust)**

```rust
// crates/clorus-runtime/src/dispatch.rs

use std::collections::HashMap;
use std::sync::RwLock;

lazy_static! {
    static ref FUNCTION_REGISTRY: RwLock<HashMap<String, *mut u8>> =
        RwLock::new(HashMap::new());
}

/// Register a function in the global dispatch table
#[no_mangle]
pub extern "C" fn clorus_register_function(
    name: *const c_char,
    func_ptr: *mut u8
) {
    unsafe {
        let name_str = CStr::from_ptr(name).to_str().unwrap();
        let mut registry = FUNCTION_REGISTRY.write().unwrap();
        registry.insert(name_str.to_string(), func_ptr);
    }
}

/// Look up a function by name
#[no_mangle]
pub extern "C" fn clorus_lookup_function(
    name: *const c_char
) -> *mut u8 {
    unsafe {
        let name_str = CStr::from_ptr(name).to_str().unwrap();
        let registry = FUNCTION_REGISTRY.read().unwrap();
        registry.get(name_str)
            .copied()
            .unwrap_or(std::ptr::null_mut())
    }
}

/// Unregister a function (for cleanup)
#[no_mangle]
pub extern "C" fn clorus_unregister_function(name: *const c_char) {
    unsafe {
        let name_str = CStr::from_ptr(name).to_str().unwrap();
        let mut registry = FUNCTION_REGISTRY.write().unwrap();
        registry.remove(name_str);
    }
}
```

**2. Codegen Changes (LLVM IR Generation)**

```rust
// crates/clorus-codegen/src/codegen.rs

impl<'ctx> CodeGen<'ctx> {
    /// Compile a function call with hot reload support
    fn compile_call_with_dispatch(
        &mut self,
        func_name: &str,
        args: &[Expr]
    ) -> Result<PointerValue<'ctx>, String> {
        // Declare the lookup function if not already declared
        self.declare_dispatch_functions();

        // Get the lookup function
        let lookup_fn = self.module.get_function("clorus_lookup_function")
            .ok_or("clorus_lookup_function not declared")?;

        // Create a global string constant for the function name
        let func_name_ptr = self.builder
            .build_global_string_ptr(func_name, "func_name")
            .unwrap();

        // Call clorus_lookup_function(name) -> *mut u8
        let func_ptr_call = self.builder.build_call(
            lookup_fn,
            &[func_name_ptr.as_pointer_value().into()],
            "lookup_call"
        ).unwrap();

        let func_ptr = func_ptr_call
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Compile arguments
        let mut arg_values = Vec::new();
        for arg in args {
            let val = self.compile_expr(arg)?;
            arg_values.push(val.into());
        }

        // Build function type (all functions have same signature)
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let param_types: Vec<_> = (0..args.len())
            .map(|_| value_ptr_type.into())
            .collect();
        let fn_type = value_ptr_type.fn_type(&param_types, false);

        // Build indirect call through looked-up pointer
        let call_result = self.builder.build_indirect_call(
            fn_type,
            func_ptr,
            &arg_values,
            "indirect_call"
        ).unwrap();

        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    /// Register a function after compilation
    fn register_compiled_function(&mut self, name: &str) -> Result<(), String> {
        let register_fn = self.module.get_function("clorus_register_function")
            .ok_or("clorus_register_function not declared")?;

        // Get the address of the compiled function
        let func_address = self.execution_engine
            .get_function_address(name)
            .map_err(|e| format!("Failed to get function address: {}", e))?;

        // Create function name string
        let func_name_ptr = self.builder
            .build_global_string_ptr(name, "register_name")
            .unwrap();

        // Build the function pointer as an i64 then cast to i8*
        let func_ptr_int = self.context.i64_type().const_int(func_address, false);
        let func_ptr = self.builder.build_int_to_ptr(
            func_ptr_int,
            self.context.i8_type().ptr_type(AddressSpace::default()),
            "func_ptr"
        ).unwrap();

        // Call clorus_register_function(name, ptr)
        self.builder.build_call(
            register_fn,
            &[func_name_ptr.as_pointer_value().into(), func_ptr.into()],
            "register_call"
        ).unwrap();

        Ok(())
    }

    fn declare_dispatch_functions(&mut self) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // clorus_lookup_function(name: *const c_char) -> *mut u8
        if self.module.get_function("clorus_lookup_function").is_none() {
            let lookup_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
            self.module.add_function("clorus_lookup_function", lookup_type, None);
        }

        // clorus_register_function(name: *const c_char, ptr: *mut u8)
        if self.module.get_function("clorus_register_function").is_none() {
            let register_type = self.context.void_type().fn_type(
                &[i8_ptr_type.into(), i8_ptr_type.into()],
                false
            );
            self.module.add_function("clorus_register_function", register_type, None);
        }
    }
}
```

**3. REPL Integration**

```rust
// crates/clorus-repl/src/repl_engine.rs

impl ReplEngine {
    pub fn eval(&mut self, input: &str) -> Result<String, String> {
        let exprs = parse_str(input)?;

        for expr in exprs {
            match &expr {
                Expr::Defn { name, .. } => {
                    // Compile the function
                    let result = self.codegen.compile_and_run(&expr)?;

                    // Register it in the dispatch table
                    // (happens automatically in compile_defn now)

                    return Ok(format!("Defined {}", name));
                }
                _ => {
                    let result = self.codegen.compile_and_run(&expr)?;
                    return Ok(self.format_result(result));
                }
            }
        }

        Ok("nil".to_string())
    }
}
```

#### Performance Impact

| Operation | Without Hot Reload | With Hot Reload | Overhead |
|-----------|-------------------|-----------------|----------|
| Function call | 1-2 cycles (direct) | 12-20 cycles (hash lookup + indirect) | ~10-18 cycles |
| Execution time | ~0.5ns | ~5ns | ~4.5ns per call |

**Real-world impact:** For most code, negligible. Hash lookup is very fast.

**Optimization:** Can be optimized to array-based dispatch (~2-3 cycle overhead).

#### Pros & Cons

**Pros:**
- ✅ Simple to implement (add indirection layer)
- ✅ Works immediately with existing code
- ✅ Thread-safe (RwLock on registry)
- ✅ Low overhead (~10-20 cycles per call)
- ✅ Matches Clojure semantics exactly

**Cons:**
- ⚠️ Small performance cost on every function call
- ⚠️ Requires runtime dispatch infrastructure
- ⚠️ Hash lookup overhead (can be optimized)

---

### Approach 2: Versioned Function Cells (Clojure-Style)

**Strategy:** Each function name is a "Var" that points to the current implementation.

#### Architecture

```rust
struct FunctionVar {
    name: String,
    current: AtomicPtr<CompiledFunction>,
    root_binding: *mut CompiledFunction,
}

impl FunctionVar {
    fn invoke(&self, args: &[Value]) -> Value {
        unsafe {
            let func = self.current.load(Ordering::Acquire);
            (*func).invoke(args)
        }
    }

    fn rebind(&self, new_fn: CompiledFunction) {
        let new_ptr = Box::into_raw(Box::new(new_fn));
        self.current.store(new_ptr, Ordering::Release);
    }
}
```

**Pros:**
- ✅ Atomic updates (lock-free)
- ✅ Can implement `binding` for dynamic scope
- ✅ Matches Clojure's Var semantics

**Cons:**
- ⚠️ More complex than Approach 1
- ⚠️ Requires managing function versions
- ⚠️ Memory overhead for var tracking

---

### Approach 3: JIT Code Patching

**Strategy:** When a function is redefined, patch all call sites in machine code.

#### How It Works

```rust
struct CallSite {
    caller_function: String,
    offset: usize,  // Byte offset in machine code
    target: String, // Function being called
}

fn redefine_and_patch(name: &str, new_fn: CompiledFunction) {
    let new_addr = new_fn.address();

    // Find all places that call this function
    for call_site in find_call_sites(name) {
        // Patch the machine code directly
        unsafe {
            let call_instruction = (call_site.function_base + call_site.offset) as *mut u8;
            // Write new relative jump offset
            patch_x86_call(call_instruction, new_addr);
        }
    }
}
```

**Pros:**
- ✅ Zero runtime overhead after patching
- ✅ Direct calls (maximum performance)
- ✅ Cool factor 😎

**Cons:**
- ❌ Architecture-specific (x86 vs ARM)
- ❌ Complex to implement
- ❌ Thread safety challenges
- ❌ Debugging difficulties
- ❌ May break LLVM optimizations

---

### Approach 4: Shared Library Reloading

**Strategy:** Compile changed code to `.so`/`.dylib` and reload dynamically.

```rust
fn reload_module(path: &Path) -> Result<(), String> {
    unsafe {
        // Load new shared library
        let new_lib = Library::new(path)?;

        // Update function pointers
        for (name, _) in changed_functions {
            let new_symbol = new_lib.get::<extern "C" fn() -> Value>(name.as_bytes())?;
            FUNCTION_REGISTRY.insert(name, *new_symbol as *mut u8);
        }

        // Unload old library (if safe)
        if let Some(old_lib) = CURRENT_LIB.replace(new_lib) {
            // Keep old lib alive if still referenced
            ARCHIVED_LIBS.push(old_lib);
        }
    }
    Ok(())
}
```

**Pros:**
- ✅ Proven approach (game engines use this)
- ✅ Can reload entire modules at once
- ✅ Platform-supported (dlopen/LoadLibrary)

**Cons:**
- ⚠️ Requires writing to disk
- ⚠️ Platform-specific APIs
- ⚠️ Symbol management complexity
- ⚠️ Memory leaks if old libs kept alive

---

## Recommended Implementation: Approach 1

**Rationale:**
- Simple and effective
- Low enough overhead for development
- Can be optimized later
- Matches Clojure semantics
- Enables future features (dynamic binding, alter-var-root)

---

## Implementation Plan

### Phase 1: Runtime Infrastructure (Day 1)

**Tasks:**
1. Create `crates/clorus-runtime/src/dispatch.rs`
2. Implement global function registry
3. Add FFI exports:
   - `clorus_register_function`
   - `clorus_lookup_function`
   - `clorus_unregister_function`
4. Add thread-safe RwLock wrapper

**Deliverable:** Runtime function dispatch system

---

### Phase 2: Codegen Changes (Day 2)

**Tasks:**
1. Add `declare_dispatch_functions()` to codegen
2. Modify `compile_call()` to use indirect dispatch
3. Add `register_compiled_function()` after defn
4. Update function compilation to register automatically

**Deliverable:** LLVM IR generates indirect function calls

---

### Phase 3: REPL Integration (Day 3)

**Tasks:**
1. Update REPL to register functions on definition
2. Clean up old function pointers (optional)
3. Add `:reload` command to force recompilation

**Deliverable:** Hot reload working in REPL

---

### Phase 4: Testing & Optimization (Day 4)

**Tasks:**
1. Test hot reload scenarios
2. Benchmark performance impact
3. Optimize hash lookup (consider array-based dispatch)
4. Add configuration flag for hot reload (enable/disable)

**Deliverable:** Production-ready hot reload

---

## Performance Optimization Strategies

### Optimization 1: Function ID Table

Replace hash-based lookup with array-based:

```rust
// Assign numeric IDs at compile time
const FOO_ID: usize = 0;
const BAR_ID: usize = 1;

static FUNCTION_TABLE: [AtomicPtr<u8>; 4096] = [...];

#[no_mangle]
pub extern "C" fn clorus_lookup_by_id(id: usize) -> *mut u8 {
    if id < FUNCTION_TABLE.len() {
        FUNCTION_TABLE[id].load(Ordering::Relaxed)
    } else {
        std::ptr::null_mut()
    }
}
```

**Benefit:** Reduces overhead from ~15 cycles to ~3 cycles.

---

### Optimization 2: Inline Caching

Cache the last lookup result at each call site:

```rust
struct CallSiteCache {
    last_name: String,
    last_ptr: *mut u8,
}

fn lookup_with_cache(name: &str, cache: &mut CallSiteCache) -> *mut u8 {
    if cache.last_name == name {
        return cache.last_ptr;  // Fast path: cache hit
    }

    // Slow path: hash lookup
    let ptr = FUNCTION_REGISTRY.read().unwrap().get(name).copied().unwrap();
    cache.last_name = name.to_string();
    cache.last_ptr = ptr;
    ptr
}
```

**Benefit:** First call pays hash cost, subsequent calls are fast.

---

### Optimization 3: Conditional Compilation

Add a feature flag to disable hot reload in production:

```rust
#[cfg(feature = "hot-reload")]
fn compile_call(&mut self, name: &str, args: &[Expr]) -> Result<PointerValue, String> {
    self.compile_call_with_dispatch(name, args)
}

#[cfg(not(feature = "hot-reload"))]
fn compile_call(&mut self, name: &str, args: &[Expr]) -> Result<PointerValue, String> {
    self.compile_call_direct(name, args)  // Zero overhead
}
```

**Benefit:** Zero overhead in production builds.

---

## Configuration

### Cargo.toml Features

```toml
[features]
default = ["hot-reload"]  # Enable by default for development
hot-reload = []           # Indirect function dispatch
production = []           # Direct calls, no hot reload
```

### Build Commands

```bash
# Development (hot reload enabled)
cargo build

# Production (direct calls)
cargo build --release --no-default-features --features production
```

---

## Testing Strategy

### Test 1: Basic Redefinition

```clojure
(defn foo [] 1)
(defn bar [] (foo))

(assert (= (bar) 1))

(defn foo [] 2)  ; Redefine

(assert (= (bar) 2))  ; ✅ Should see new value
```

### Test 2: Mutual Recursion

```clojure
(defn even? [n]
  (if (= n 0) true (odd? (- n 1))))

(defn odd? [n]
  (if (= n 0) false (even? (- n 1))))

(assert (even? 4))

; Redefine even? to always return false
(defn even? [n] false)

(assert (not (even? 4)))  ; ✅ Should use new even?
(assert (odd? 4))          ; ✅ odd? should call new even?
```

### Test 3: Performance Impact

```clojure
(defn hot-loop [n]
  (if (= n 0)
    0
    (+ 1 (hot-loop (- n 1)))))

; Measure: should be < 10% slower than direct calls
(time (hot-loop 10000))
```

---

## Future Enhancements

### 1. Dynamic Binding (Clojure's `binding`)

```clojure
(def ^:dynamic *debug* false)

(binding [*debug* true]
  (foo))  ; foo sees *debug* = true
```

Requires thread-local storage for var bindings.

---

### 2. Var Watchers

```clojure
(add-watch #'foo :watcher
  (fn [key ref old new]
    (println "foo changed from" old "to" new)))

(defn foo [] "new")  ; Triggers watcher
```

---

### 3. alter-var-root

```clojure
(alter-var-root #'foo (fn [old-fn]
  (fn [& args]
    (println "Calling foo")
    (apply old-fn args))))
```

Enables aspect-oriented programming and function wrapping.

---

## Comparison to Clojure

| Feature | Clojure (JVM) | Clorus (Native) |
|---------|---------------|-----------------|
| Hot reload | ✅ Vars | ✅ Dispatch table |
| alter-var-root | ✅ | 🔜 After implementation |
| binding | ✅ | 🔜 Thread-local storage |
| Overhead | ~5-10ns | ~5ns (similar) |
| Thread-safe | ✅ | ✅ RwLock |

---

## Success Criteria

1. ✅ Function redefinition works in REPL
2. ✅ Previously compiled code sees new definitions
3. ✅ Performance overhead < 10% in hot reload mode
4. ✅ Thread-safe under concurrent updates
5. ✅ Can be disabled for production builds
6. ✅ All existing tests still pass

---

## Related Features Enabled

Once hot reload is implemented, these become possible:

1. **Interactive debugging** - Redefine functions while debugging
2. **Live coding** - Change code while program runs
3. **Test-driven development** - Redefine and re-run tests instantly
4. **REPL-driven development** - True Clojure workflow
5. **alter-var-root** - Function wrapping/aspect-oriented programming
6. **Dynamic binding** - Thread-local var overrides

---

## References

- **Clojure Vars:** https://clojure.org/reference/vars
- **LLVM Indirect Calls:** https://llvm.org/docs/LangRef.html#call-instruction
- **Game Engine Hot Reload:** Casey Muratori's Handmade Hero
- **Rust libloading:** https://docs.rs/libloading/latest/libloading/

---

✅ **Hot Reload Design Complete!**

This architecture provides Clojure-style interactive development with minimal overhead, enabling true REPL-driven workflows in a native compiled language. 🔥

**Implementation Target:** After reaching 70-80% language parity (when core features are stable).
