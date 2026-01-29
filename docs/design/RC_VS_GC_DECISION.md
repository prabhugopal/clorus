# Reference Counting vs Garbage Collection for Clorus

## Decision: Which Memory Management Strategy?

### TL;DR Recommendation

**Start with Reference Counting (RC), design for GC later**

Why: Faster to implement, predictable, easier to debug, and we can migrate to GC once the language matures.

---

## Detailed Comparison

### Reference Counting (RC)

#### How It Works
```rust
struct Value {
    refcount: AtomicU64,  // Counter of how many references exist
    tag: ValueTag,
    data: *mut u8,
}

// When copying
fn clone_value(v: &Value) {
    v.refcount.fetch_add(1, Ordering::Relaxed);
}

// When dropping
fn drop_value(v: &Value) {
    if v.refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        // Last reference - free memory
        unsafe { deallocate(v.data) };
    }
}
```

#### Generated LLVM IR
```llvm
; Copy operation
%val_ptr = load ptr, ptr %source
%rc_ptr = getelementptr %Value, ptr %val_ptr, i32 0, i32 0
call void @llvm.atomic.add.i64(ptr %rc_ptr, i64 1)

; Drop operation
call void @clorus_release(ptr %val_ptr)
```

#### ✅ Pros

1. **Simpler Implementation** (~1-2 weeks)
   - Just atomic inc/dec operations
   - No complex runtime needed
   - Easier to reason about

2. **Deterministic Cleanup**
   - Object freed immediately when last reference drops
   - No unpredictable pauses
   - Better for systems programming

3. **Predictable Performance**
   - Constant overhead per operation
   - No GC pauses during execution
   - Real-time friendly

4. **Easier Debugging**
   - Can track refcounts
   - Memory issues are local
   - Simpler to understand leaks

5. **Better FFI/Interop**
   - Works well with Rust (Rc/Arc)
   - C libraries expect manual management
   - No GC state to manage across FFI

6. **Lower Memory Usage**
   - Objects freed immediately
   - No memory retained for GC

#### ❌ Cons

1. **Cannot Handle Cycles** ⚠️
   ```clojure
   ; This leaks memory!
   (def a (atom nil))
   (def b (atom a))
   (reset! a b)
   ; a points to b, b points to a
   ; Refcounts never reach 0
   ```

2. **Performance Overhead**
   - Atomic inc/dec on every copy/drop
   - Cache thrashing (touching refcounts)
   - ~5-15% overhead on operations

3. **Not True Clojure Semantics**
   - Clojure assumes GC
   - Some patterns don't work
   - Can't have arbitrary cycles

4. **Complexity for User**
   - Need to think about cycles
   - Might need weak references

#### Real-World Examples Using RC
- **Swift**: RC + weak references
- **Python (CPython)**: RC + cycle detector
- **Rust**: Rc/Arc for shared ownership
- **Objective-C**: ARC (Automatic Reference Counting)

---

### Garbage Collection (GC)

#### How It Works
```rust
// GC allocates from heap
fn gc_alloc(size: usize) -> *mut u8 {
    GC_RUNTIME.allocate(size)
}

// Periodically or on memory pressure:
fn gc_collect() {
    // 1. Mark: Find all reachable objects
    mark_roots();      // Scan stack, globals
    mark_reachable();  // Trace object graph

    // 2. Sweep: Free unreachable objects
    sweep_heap();
}
```

#### ✅ Pros

1. **True Clojure Semantics** ⭐
   - Matches Clojure's model exactly
   - Arbitrary cycles work fine
   - No mental overhead for users

2. **Handles Cycles Naturally**
   ```clojure
   ; This works fine!
   (def a (atom nil))
   (def b (atom a))
   (reset! a b)
   ; GC traces reachability, cleans up when neither is reachable
   ```

3. **Better for Functional Style**
   - Lots of intermediate objects? No problem
   - No refcount overhead per operation
   - Allocate fast, collect in batch

4. **Lower Per-Operation Overhead**
   - No atomic inc/dec
   - Just allocate and forget
   - Faster individual operations

5. **Optimization Opportunities**
   - Generational GC (young objects die fast)
   - Concurrent collection
   - Escape analysis

#### ❌ Cons

1. **Complex Implementation** (~4-8 weeks)
   - Need GC runtime
   - Mark/sweep algorithm
   - Root scanning (stack/globals)
   - Integration with LLVM

2. **Non-Deterministic Pauses**
   - Collection happens at unpredictable times
   - Can cause latency spikes
   - Harder for real-time systems

3. **More Memory Usage**
   - Objects retained until collection
   - Need extra heap space
   - Memory overhead for GC metadata

4. **Harder Debugging**
   - Use-after-free bugs can be subtle
   - Memory corruption harder to track
   - Need GC-aware tools

5. **FFI Complications**
   - C libraries don't know about GC
   - Need to pin objects
   - Can't collect during FFI call

6. **Runtime Dependency**
   - Need GC runtime in final binary
   - Increases binary size
   - More complexity

#### Real-World Examples Using GC
- **Clojure**: JVM GC (G1, ZGC, etc.)
- **Go**: Concurrent mark-sweep
- **JavaScript**: V8 generational GC
- **OCaml**: Generational GC
- **Java**: Various (G1, ZGC, Shenandoah)

---

## Hybrid Approaches

### Option 1: RC + Cycle Detector
Use RC normally, but periodically run cycle detection:

```rust
fn detect_cycles() {
    // Trial deletion algorithm
    // Temporarily decrement refcounts
    // See what would be freed
    // Restore if object is in cycle
}
```

**Example**: Python uses this!

### Option 2: RC + Weak References
Use strong refs normally, weak refs for potential cycles:

```clojure
(def a (atom nil))
(def b (atom (weak-ref a)))  ; Weak reference doesn't increment refcount
```

**Example**: Swift uses this!

### Option 3: Start with RC, Migrate to GC
1. Implement RC first (quick win)
2. Design API to hide implementation
3. Later swap in GC runtime
4. Keep both? Let user choose?

```clojure
; User code stays the same
(def v [1 2 3])
(conj v 4)

; Under the hood: RC today, GC tomorrow
```

---

## Recommendation for Clorus

### Phase 1: Reference Counting (Immediate - 2 weeks)

**Why start with RC:**
1. **Faster to implement** - Get persistent data structures working NOW
2. **Simpler to debug** - Learn the system before adding GC complexity
3. **Good for systems programming** - Aligns with Rust-level control
4. **Validates the design** - Proves out the runtime/codegen architecture

**Implementation Plan:**
```rust
// Week 1: Foundation
- Tagged Value type
- Heap allocation (malloc wrapper)
- RC operations (retain/release)
- Runtime library skeleton

// Week 2: First Data Structure
- Persistent List (simple RC test)
- Code generation for literals
- Integration tests
- Memory leak checking (Valgrind)
```

**Cycle Handling:**
For now, document that cycles leak. In practice, persistent data structures rarely create cycles:
```clojure
; These are fine (no cycles):
(def v [1 2 3])
(def m {:a 1 :b 2})
(def nested [{:x [1 2]} {:y [3 4]}])

; This would leak (rare in practice):
(def a (atom nil))
(reset! a a)  ; Self-reference
```

### Phase 2: Cycle Detection (Later - 2-3 weeks)

Once RC is working, add Python-style cycle detection:
- Run periodically or on memory pressure
- Trial deletion algorithm
- Only for potentially cyclic types (Atom, custom types)

### Phase 3: Consider GC Migration (Future - 4-6 weeks)

Once the language matures, evaluate:
- Is cycle detection enough?
- Are GC pauses acceptable?
- Do we need true Clojure semantics?

If yes, implement **conservative Boehm GC** or **precise generational GC**.

---

## Detailed RC Implementation Plan

### Step 1: Value Type
```rust
// runtime/src/value.rs

#[repr(C)]
pub struct Value {
    header: Header,
    data: ValueData,
}

#[repr(C)]
struct Header {
    refcount: AtomicU64,
    tag: ValueTag,
}

#[repr(C)]
enum ValueTag {
    Number = 0,
    Vector = 1,
    HashMap = 2,
    List = 3,
}

#[repr(C)]
union ValueData {
    number: f64,           // Inline for small values
    ptr: *mut u8,          // Pointer for heap-allocated
}
```

### Step 2: RC Operations
```rust
#[no_mangle]
pub extern "C" fn clorus_retain(val: *mut Value) {
    if val.is_null() { return; }
    unsafe {
        (*val).header.refcount.fetch_add(1, Ordering::Relaxed);
    }
}

#[no_mangle]
pub extern "C" fn clorus_release(val: *mut Value) {
    if val.is_null() { return; }
    unsafe {
        if (*val).header.refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            // Last reference - deallocate
            deallocate_value(val);
        }
    }
}

unsafe fn deallocate_value(val: *mut Value) {
    match (*val).header.tag {
        ValueTag::Number => {
            // Numbers are inline, just free the Value
            libc::free(val as *mut libc::c_void);
        }
        ValueTag::Vector => {
            // Recursively release vector contents
            let vec = (*val).data.ptr as *mut PersistentVector;
            release_vector(vec);
            libc::free(val as *mut libc::c_void);
        }
        // ... other types
    }
}
```

### Step 3: Code Generation
```rust
// In codegen.rs

impl<'ctx> CodeGen<'ctx> {
    fn compile_variable_ref(&mut self, name: &str) -> Result<PointerValue<'ctx>, String> {
        let val_ptr = self.variables.get(name)?;

        // Increment refcount when copying
        let retain_fn = self.module.get_function("clorus_retain").unwrap();
        self.builder.build_call(retain_fn, &[val_ptr.into()], "").unwrap();

        Ok(val_ptr)
    }

    fn compile_let_binding(&mut self, bindings: &[(String, Expr)], body: &Expr) -> Result<Value, String> {
        // ... compile bindings ...

        let result = self.compile_expr(body)?;

        // Release all bindings when scope ends
        let release_fn = self.module.get_function("clorus_release").unwrap();
        for (name, _) in bindings {
            let val = self.variables.get(name)?;
            self.builder.build_call(release_fn, &[val.into()], "").unwrap();
        }

        Ok(result)
    }
}
```

### Step 4: Testing
```rust
#[test]
fn test_refcounting() {
    // Create value
    let v = clorus_vector_new();
    assert_eq!(refcount(v), 1);

    // Clone
    clorus_retain(v);
    assert_eq!(refcount(v), 2);

    // Release
    clorus_release(v);
    assert_eq!(refcount(v), 1);

    // Final release should free
    clorus_release(v);
    // Valgrind should show no leaks
}
```

---

## Migration Path: RC → GC

Design the API so implementation can change:

```rust
// Public API (stable)
#[no_mangle]
pub extern "C" fn clorus_value_clone(val: *mut Value) -> *mut Value;

#[no_mangle]
pub extern "C" fn clorus_value_drop(val: *mut Value);

// Implementation (can change)
// Version 1: RC
pub extern "C" fn clorus_value_clone(val: *mut Value) -> *mut Value {
    clorus_retain(val);
    val
}

// Version 2: GC (future)
pub extern "C" fn clorus_value_clone(val: *mut Value) -> *mut Value {
    gc_write_barrier(val);  // Tell GC about new reference
    val
}
```

Code generation calls the public API, not RC-specific functions:
```llvm
%new_ref = call @clorus_value_clone(ptr %val)
call @clorus_value_drop(ptr %old_val)
```

---

## Final Decision Framework

Choose **Reference Counting** if:
- ✅ You want to ship persistent data structures quickly (2 weeks)
- ✅ Deterministic cleanup is important
- ✅ You're okay with cycle restrictions
- ✅ You want Rust-like control
- ✅ You're targeting systems programming

Choose **Garbage Collection** if:
- ✅ True Clojure semantics are critical
- ✅ You can invest 4-8 weeks upfront
- ✅ You expect lots of cyclic structures
- ✅ You're okay with GC pauses
- ✅ You're targeting high-level programming

Choose **Hybrid (RC + Cycle Detection)** if:
- ✅ You want RC's determinism
- ✅ You need to handle some cycles
- ✅ You can tolerate occasional cycle scans
- ✅ Python-like model is acceptable

---

## My Specific Recommendation

**Start with Reference Counting for these reasons:**

1. **Speed to Market**: Working persistent vectors in 2 weeks vs 8+ weeks
2. **Learning**: Understand the system before adding GC complexity
3. **Flexibility**: Can add cycle detection or migrate to GC later
4. **Systems Programming**: Aligns with Clorus's Rust-level goals
5. **Debugging**: Easier to validate correctness initially

**Implementation Order:**
```
Week 1-2:   RC + Persistent List
Week 3-4:   Persistent Vector
Week 5-6:   HashMap (HAMT)
Week 7-8:   Optimization + profiling
Week 9+:    Evaluate: cycle detection vs GC migration
```

**Accept this trade-off:**
- Cyclic structures will leak (document it)
- In practice, persistent data structures rarely have cycles
- If it becomes a problem, add cycle detection
- Long-term, might migrate to GC for true Clojure semantics

**The beauty of this approach**: You can ship working persistent data structures quickly, then evolve the memory management as needs become clear.

Ready to implement RC-based persistent data structures?
