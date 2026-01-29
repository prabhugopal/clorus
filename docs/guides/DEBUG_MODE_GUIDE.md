# Debug Mode Implementation Guide

## Current Status

✅ **--debug Flag Added** (Task #2 Complete)

The `clorus run --debug` command now accepts the `--debug` flag:

```bash
# Run with debug mode enabled
clorus run --debug

# Or use short form
clorus run -d
```

## What Works Now

### CLI Support
- ✅ `--debug` / `-d` flag parsing
- ✅ Debug mode indicator in output
- ✅ Help text updated with debug option

### Example Output
```bash
$ cd factorial-demo
$ clorus run --debug

   Debug mode: enabled
   Note: Memory tracking requires runtime Value* types
   Currently using f64 - full tracking coming soon!

   Compiling factorial-demo v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `src/main.clrs`

=> 30
```

## What's Missing (For Full Memory Tracking)

The debug infrastructure exists in the runtime (`clorus-runtime`), but it's not connected yet because:

### 1. Type System Migration Needed

**Current State:**
- Codegen uses `f64` values directly
- `compile_expr()` returns `FloatValue<'ctx>`
- No heap allocations, everything on stack

**Needed for Memory Tracking:**
- Codegen needs to use `*mut Value` (runtime types)
- `compile_expr()` should return `PointerValue<'ctx>` (Value*)
- All operations go through runtime functions

### 2. Runtime Linking Needed

**Current State:**
- JIT execution doesn't link runtime library
- Runtime functions declared but not available at runtime

**Needed:**
- Link `libclorus_runtime.so` / `.dylib` / `.dll` in JIT engine
- Make runtime functions available for execution

### 3. Debug API Integration

**Available in Runtime** (`clorus-runtime/src/debug.rs`):
```rust
// Enable debug mode
pub extern "C" fn clorus_debug_enable();

// Disable debug mode
pub extern "C" fn clorus_debug_disable();

// Print all recorded events
pub extern "C" fn clorus_debug_print_events();

// Check for memory leaks
pub extern "C" fn clorus_debug_check_leaks() -> bool;
```

**What's Needed:**
When `--debug` flag is set:
1. Call `clorus_debug_enable()` before execution
2. Run the program
3. Call `clorus_debug_print_events()` after execution
4. Call `clorus_debug_check_leaks()` and report

## Implementation Roadmap

### Phase 1: Hybrid Mode (Recommended First Step)

Keep f64 for simple operations, use Value* for collections:

```rust
// In codegen.rs
pub enum CompiledValue<'ctx> {
    Float(FloatValue<'ctx>),      // For numbers: 42, 3.14
    Pointer(PointerValue<'ctx>),  // For collections: [1 2 3]
}
```

**Benefits:**
- Gradual migration
- Can test memory tracking on vectors/lists
- Numbers stay fast (no heap allocation)

**Changes Needed:**
- Update `compile_expr()` to return `CompiledValue`
- Add conversion between Float and Pointer types
- Use runtime functions only for collections

### Phase 2: Full Value* Migration

All values become `*mut Value`:

```rust
// Everything goes through runtime
let num = clorus_value_number(42.0);  // Heap allocated
let vec = clorus_vector_empty();       // Heap allocated
```

**Benefits:**
- Full memory tracking
- Uniform type system
- Ready for GC migration

**Trade-offs:**
- More heap allocations
- Need reference counting everywhere
- Slightly slower for simple arithmetic

### Phase 3: Runtime Library Linking

Link runtime library in JIT execution:

```rust
// In commands.rs run() function
let engine = codegen.get_module()
    .create_jit_execution_engine(OptimizationLevel::None)
    .map_err(|e| format!("JIT error: {}", e))?;

// Add runtime library
engine.add_module(&runtime_module)?;

// Or load as dynamic library
engine.add_global_mapping(&clorus_debug_enable, clorus_debug_enable_ptr);
```

## Testing Memory Tracking (When Ready)

### Example Program
```clojure
; test-memory.clrs
(def vec1 [1 2 3 4 5])
(def vec2 (conj vec1 6))
(def vec3 (conj vec2 7))

; vec1, vec2, vec3 should share structure
; Memory tracking will show:
; - 3 vector allocations
; - Shared nodes (reference counting)
; - No memory leaks on exit
```

### Expected Debug Output
```bash
$ clorus run --debug

   Debug mode: enabled

   Compiling test-memory v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `src/main.clrs`

=== Memory Tracking Events ===

[0ms] ALLOC: Vector @ 0x7f8a12000000 (size: 64 bytes, refcount: 1)
[2ms] ALLOC: VectorNode @ 0x7f8a12000100 (size: 256 bytes, refcount: 1)
[3ms] ALLOC: Vector @ 0x7f8a12000200 (size: 64 bytes, refcount: 1)
[3ms] RETAIN: VectorNode @ 0x7f8a12000100 (refcount: 1 -> 2) ← Sharing!
[5ms] ALLOC: Vector @ 0x7f8a12000300 (size: 64 bytes, refcount: 1)
[5ms] RETAIN: VectorNode @ 0x7f8a12000100 (refcount: 2 -> 3) ← Sharing!

[10ms] RELEASE: Vector @ 0x7f8a12000000 (freed)
[10ms] RELEASE: Vector @ 0x7f8a12000200 (freed)
[10ms] RELEASE: Vector @ 0x7f8a12000300 (freed)
[10ms] RELEASE: VectorNode @ 0x7f8a12000100 (refcount: 3 -> 0, freed)

=== Memory Leak Check ===
✅ No memory leaks detected!

Total allocations: 4
Total releases: 4
Peak memory usage: 384 bytes
Structural sharing saved: ~512 bytes (nodes reused 2 times)

=> 7
```

## Current Limitations

1. **No Actual Memory Tracking Yet**
   - Flag is parsed and displayed
   - But no events are recorded
   - Need Value* migration first

2. **No Runtime Linking**
   - Runtime functions declared in IR
   - But not available at execution time
   - Need dynamic library loading

3. **No Vector/List Syntax**
   - Parser doesn't support `[1 2 3]` yet
   - Would need to use function calls: `(vector 1 2 3)`
   - Parser extension needed

## Quick Reference

### Files Modified for --debug Flag
- `crates/clorus-cli/src/main.rs` - Argument parsing
- `crates/clorus-cli/src/commands.rs` - run() function signature

### Runtime Debug API
- `crates/clorus-runtime/src/debug.rs` - Full implementation exists

### Integration Example
- `crates/clorus-codegen/examples/runtime_ir_gen.rs` - Shows IR generation

## Next Steps (Priority Order)

1. ✅ Add --debug flag (DONE)
2. ⏳ Test with factorial program
3. ⏳ Add vector/list syntax to parser `[1 2 3]`
4. ⏳ Implement Hybrid Mode (f64 + Value*)
5. ⏳ Link runtime library in JIT
6. ⏳ Wire up debug API calls

## Questions?

**Q: Why doesn't --debug show memory events yet?**
A: Because we're still using f64 values (stack allocated, no tracking). Need Value* types (heap allocated with refcounting) first.

**Q: Can I track memory for my factorial program?**
A: Not yet - your program uses only f64 numbers. Memory tracking works for heap-allocated structures like vectors and lists.

**Q: When will full memory tracking work?**
A: After Phase 1 (Hybrid Mode) is implemented - estimated 2-3 hours of work.

**Q: What's the easiest way to see memory tracking work?**
A: Run the unit tests in `clorus-runtime`: `cargo test -p clorus-runtime -- --nocapture`

---

**Status**: Infrastructure ready, integration pending 🚀
