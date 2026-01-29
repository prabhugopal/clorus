# Clorus Memory Model

## Current Implementation (v0.1.0)

### Overview
Clorus currently uses a **simple, stack-based memory model** with LLVM-managed global variables. There is **no garbage collection** or heap allocation yet.

### Memory Layout

#### 1. Local Variables (`let` bindings)
**Storage**: Stack-allocated via LLVM `alloca`
**Lifetime**: Lexically scoped, automatic cleanup
**Implementation**:
```rust
// In codegen.rs
fn create_entry_block_alloca(&self, name: &str) -> PointerValue<'ctx> {
    let builder = self.context.create_builder();
    let entry = self.builder.get_insert_block()
        .and_then(|block| block.get_first_instruction())
        .unwrap();
    builder.position_before(&entry);
    builder.build_alloca(self.context.f64_type(), name).unwrap()
}
```

**Example**:
```clojure
(let [x 10      ; Stack: allocate 8 bytes, store 10.0
      y 20]     ; Stack: allocate 8 bytes, store 20.0
  (+ x y))      ; Use values, then deallocate when scope ends
```

**LLVM IR**:
```llvm
%x = alloca double        ; Stack allocation
store double 10.0, ptr %x ; Store value
%x_val = load double, ptr %x  ; Load value
```

#### 2. Global Variables (`def`)
**Storage**: LLVM global variables (static data section)
**Lifetime**: Program lifetime
**Implementation**:
```rust
// Create global variable
let global = self.module.add_global(f64_type, Some(AddressSpace::default()), name);
global.set_initializer(&f64_type.const_float(0.0));
```

**Example**:
```clojure
(def pi 3.14159)  ; Global section: allocate 8 bytes, init to 0.0
                  ; Then store 3.14159
```

**LLVM IR**:
```llvm
@pi = global double 0.0   ; Global variable in .data section
store double 3.14159, ptr @pi
```

#### 3. Function Parameters
**Storage**: Stack-allocated, passed by value (copy)
**Lifetime**: Function scope
**Implementation**:
```rust
// Parameters are copied to allocas for mutation
for (i, param_name) in params.iter().enumerate() {
    let param_val = function.get_nth_param(i as u32).unwrap();
    let alloca = self.create_entry_block_alloca(param_name);
    self.builder.build_store(alloca, param_val).unwrap();
}
```

**Example**:
```clojure
(defn square [x]  ; x is copied to stack
  (* x x))
```

#### 4. Function Definitions
**Storage**: LLVM function table (code section)
**Lifetime**: Program lifetime
**Implementation**:
```rust
let function = self.module.add_function(name, fn_type, None);
self.functions.insert(name.clone(), function);
```

### Current Type System

**All values are `f64` (64-bit IEEE 754 floating point)**
- Size: 8 bytes
- Range: ±1.7E±308 (~15 decimal digits of precision)
- No integers, booleans, or strings yet

```clojure
42       ; Actually 42.0 (f64)
true     ; Will be 1.0 (f64) when implemented
"hello"  ; Not yet supported
```

## Memory Characteristics

### ✅ What Works Well

1. **No Memory Leaks**: Everything is either stack-allocated (automatic cleanup) or global (program lifetime)
2. **No Garbage Collection Needed**: Simple value types, no heap allocation
3. **Fast Allocation**: Stack allocation is O(1)
4. **LLVM-Optimized**: LLVM can optimize stack frame layout

### ⚠️ Current Limitations

1. **No Heap Allocation**: Can't create dynamic data structures
2. **No Complex Types**: Only f64 values
3. **Copy Semantics**: Everything is copied (fine for f64, problematic for future types)
4. **No Move Semantics**: Can't transfer ownership
5. **No Reference Counting**: Not needed yet, but will be for heap types

## Memory Safety

### Stack Overflow
**Current Protection**: None explicit, relies on OS stack limits
**Risk**: Deep recursion can overflow

```clojure
(defn infinite [x]
  (infinite (+ x 1)))  ; Stack overflow!

(infinite 0)
```

**Future**: Add tail-call optimization or stack depth limits

### Use-After-Free
**Current Protection**: LLVM guarantees (impossible with current model)
**Status**: ✅ Safe - no pointers to freed memory

### Double-Free
**Current Protection**: LLVM handles all deallocation
**Status**: ✅ Safe - automatic memory management

### Memory Alignment
**Current Protection**: LLVM handles alignment for f64 (8-byte aligned)
**Status**: ✅ Safe

## Future Memory Model Evolution

### Phase 1: More Value Types (Planned)
```clojure
42         ; i64 (8 bytes, stack)
true       ; bool (1 byte, stack)
3.14       ; f64 (8 bytes, stack)
```

**Memory Model**: Still stack-based, no GC needed

### Phase 2: Strings (Requires Heap)
```clojure
"hello"    ; Heap-allocated, needs memory management
```

**Options**:
1. **Reference Counting**: Like Swift/Python
   - Pro: Deterministic cleanup
   - Con: Cycle issues, overhead

2. **Garbage Collection**: Like Clojure/Java
   - Pro: Simple semantics, handles cycles
   - Con: Pause times, complexity

3. **Ownership (Rust-like)**: Like Rust
   - Pro: Zero-cost, memory safe
   - Con: Complex, different from Clojure

### Phase 3: Collections (Requires Heap)
```clojure
[1 2 3]              ; Vector (heap)
{:name "Alice"}      ; Map (heap)
```

**Likely Approach**:
- **Immutable persistent data structures** (like Clojure)
- **Structural sharing** (minimize copies)
- **Reference counting or GC**

## Comparison with Other Languages

| Language | Memory Model | Management | Semantics |
|----------|-------------|------------|-----------|
| **Clorus (current)** | Stack + Globals | Automatic | Value (copy) |
| **Clojure** | Heap | GC (JVM) | Reference (immutable) |
| **Rust** | Stack + Heap | Ownership | Move + Borrow |
| **C** | Stack + Heap | Manual | Pointer |
| **Go** | Stack + Heap | GC | Reference |
| **Swift** | Stack + Heap | ARC | Reference |

## Recommendations for Future

### Short-term (Next Version)
1. **Add integer types** (i64, i32) - still stack-based
2. **Add booleans** - still stack-based
3. **Keep it simple** - no GC complexity yet

### Medium-term
1. **Add strings** with reference counting
2. **Add vectors/lists** with structural sharing
3. **Implement simple mark-and-sweep GC** or **reference counting**

### Long-term (Self-hosting Goal)
Consider **Rust-like ownership** for:
- Zero-cost abstraction
- Memory safety without GC
- Systems programming capability

But this conflicts with Clojure semantics (immutability + sharing).

**Hybrid Approach** (Recommended):
- **Value types** (numbers, bools): Stack, copy semantics
- **Immutable collections**: Heap, reference counted, structural sharing
- **Mutable data**: Owned pointers (Rust-like) for systems programming

## Example Memory Layouts

### Current (Simple)
```
Stack Frame for (let [x 10] (* x x)):
┌─────────────┐
│ x: 10.0     │ 8 bytes (f64)
└─────────────┘

Global Section:
┌─────────────┐
│ pi: 3.14159 │ 8 bytes (f64)
└─────────────┘
```

### Future (With Heap)
```
Stack:
┌──────────────────┐
│ name: ptr → heap │ 8 bytes (pointer)
│ age: 25          │ 8 bytes (i64)
└──────────────────┘
          │
          ↓
Heap:
┌────────────────┐
│ "Alice"        │ 6 bytes + metadata
│ refcount: 1    │
└────────────────┘
```

## Technical Details

### Current Memory Layout (x86_64)
```
Program Memory:
├── Code Section (.text)
│   └── LLVM-generated functions
├── Data Section (.data)
│   └── Global variables (@pi, @x, etc.)
├── Stack
│   ├── Function frames
│   └── Local variables (allocas)
└── Heap (unused currently)
```

### LLVM Optimizations

LLVM can optimize:
1. **Register allocation**: Keep values in registers when possible
2. **Stack frame elimination**: Inline small functions
3. **Constant folding**: Compute constants at compile time
4. **Dead store elimination**: Remove unused allocas

Example optimization:
```clojure
(let [x 10]
  (+ x 5))
```

Unoptimized LLVM IR:
```llvm
%x = alloca double
store double 10.0, ptr %x
%x_val = load double, ptr %x
%result = fadd double %x_val, 5.0
```

Optimized (with -O2):
```llvm
%result = fadd double 10.0, 5.0
; Or even better:
ret double 15.0  ; Constant folded!
```

## Checking Memory Usage

You can inspect LLVM IR to see memory layout:

```bash
# Add this to clorus-cli
clorus build --emit-llvm  # Output .ll file

# Example output shows allocas and globals
```

## Summary

**Current State**:
- ✅ Simple and safe
- ✅ No GC needed
- ✅ LLVM-optimized
- ✅ No memory leaks
- ⚠️ Limited to f64 values
- ⚠️ No heap allocation
- ⚠️ No complex data structures

**Next Steps**:
1. Add more primitive types (integers, booleans)
2. Design heap allocation strategy
3. Choose GC approach (RC vs. tracing GC)
4. Implement strings and collections

**For Self-Hosting Goal**:
- Need to support arrays, structs, pointers
- Probably need manual memory management option
- Could use hybrid: GC for high-level, manual for low-level
