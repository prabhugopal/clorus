# Implementing Clojure-like Persistent Data Structures in Clorus

## What Are Persistent Data Structures?

**Persistent** = Previous versions remain accessible after "modification"

```clojure
(def v1 [1 2 3])
(def v2 (conj v1 4))  ; v2 = [1 2 3 4]
; v1 is still [1 2 3] - unchanged!
; v2 shares structure with v1 (efficient)
```

## Core Requirements

### 1. Heap Allocation ⚠️ **Currently Missing**

**Need**: Dynamic memory allocation for structures that outlive function scope

**Implementation Options**:

#### Option A: Use System Allocator (malloc/free)
```rust
// In codegen.rs
fn create_heap_alloc(&mut self, size: usize) -> PointerValue<'ctx> {
    // Call libc malloc
    let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
        self.module.add_function("malloc", malloc_type, None)
    });

    let size_val = self.context.i64_type().const_int(size as u64, false);
    let ptr = self.builder.build_call(malloc_fn, &[size_val.into()], "malloc")
        .unwrap()
        .try_as_basic_value()
        .left()
        .unwrap();

    ptr.into_pointer_value()
}
```

#### Option B: Custom Allocator
```rust
// Memory pool for better performance
struct ClorusAllocator {
    pools: Vec<MemoryPool>,
    bump_allocator: BumpAllocator,
}
```

### 2. Memory Management ⚠️ **Currently Missing**

**Need**: Automatic cleanup of heap-allocated data

#### Option A: Reference Counting (Recommended for Start)

**Pros**:
- Simple to implement
- Deterministic cleanup
- No GC pauses
- Good for systems programming

**Cons**:
- Can't handle cycles
- Some overhead (atomic inc/dec)

**Implementation**:
```rust
// Tagged pointer with reference count
struct ClorusValue {
    tag: ValueTag,      // 8 bytes
    refcount: AtomicU64, // 8 bytes
    data: *mut u8,      // 8 bytes (pointer to actual data)
}

impl ClorusValue {
    fn retain(&self) {
        self.refcount.fetch_add(1, Ordering::Relaxed);
    }

    fn release(&self) {
        if self.refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            unsafe { self.deallocate() };
        }
    }
}
```

**LLVM IR Generation**:
```llvm
; Increment refcount when copying
%new_ref = load ptr, ptr %vec_ptr
call void @clorus_retain(ptr %new_ref)

; Decrement refcount when dropping
call void @clorus_release(ptr %vec_ptr)
```

#### Option B: Tracing Garbage Collection (Better Long-term)

**Pros**:
- Handles cycles
- True Clojure semantics
- Less overhead per operation

**Cons**:
- Complex implementation
- GC pauses
- Need GC runtime

**Implementation Strategy**:
1. **Conservative GC** (like Boehm GC) - scan stack/globals
2. **Generational GC** - young/old generations
3. **Concurrent GC** - minimal pauses

### 3. Type System with Tagged Values ⚠️ **Currently Missing**

**Need**: Represent different types (numbers, vectors, maps, etc.)

```rust
#[repr(C)]
enum ValueTag {
    Number = 0,
    Vector = 1,
    HashMap = 2,
    List = 3,
    String = 4,
    Keyword = 5,
    Symbol = 6,
}

#[repr(C)]
struct Value {
    tag: u64,           // 8 bytes - type tag
    data: [u64; 2],     // 16 bytes - inline or pointer
}
```

**Small value optimization**:
```
If value fits in 16 bytes (data field):
  Store inline (no heap allocation)

Otherwise:
  data[0] = pointer to heap
  data[1] = metadata (size, etc.)
```

### 4. Persistent Vector Implementation

Clojure uses a **32-way branching tree** (trie structure).

#### Structure
```
Vector with 100 elements:

Root (level 2)
├─ Node 0-31 (level 1)
│  ├─ Leaf 0-31 (elements 0-31)
│  └─ ...
├─ Node 32-63 (level 1)
│  ├─ Leaf 32-63 (elements 32-63)
│  └─ ...
└─ Node 64-95 (level 1)
   └─ Leaf 64-95 (elements 64-95)

Tail (elements 96-99) - optimization
```

#### Data Structure Definition
```rust
const BRANCHING_FACTOR: usize = 32;

#[repr(C)]
struct PersistentVector {
    refcount: AtomicU64,
    count: u64,              // Total elements
    shift: u8,               // Tree depth (shift = depth * 5)
    root: *mut VectorNode,   // Root node
    tail: *mut [Value; 32],  // Tail optimization
    tail_len: u8,            // Elements in tail
}

#[repr(C)]
struct VectorNode {
    refcount: AtomicU64,
    children: [*mut VectorNode; 32], // Either nodes or leaves
}
```

#### Core Operations

**1. Create Empty Vector**
```clojure
[]  ; Empty vector
```
```rust
fn vector_empty() -> *mut PersistentVector {
    let vec = Box::new(PersistentVector {
        refcount: AtomicU64::new(1),
        count: 0,
        shift: 5,
        root: null_mut(),
        tail: null_mut(),
        tail_len: 0,
    });
    Box::into_raw(vec)
}
```

**2. Append (conj)**
```clojure
(conj [1 2 3] 4)  ; => [1 2 3 4]
```
```rust
fn vector_conj(vec: *mut PersistentVector, value: Value) -> *mut PersistentVector {
    let vec = unsafe { &*vec };

    // If tail has space, use it (fast path)
    if vec.tail_len < 32 {
        return vector_append_to_tail(vec, value);
    }

    // Otherwise, push tail to tree and start new tail
    vector_push_tail(vec, value)
}
```

**3. Get by Index**
```clojure
(nth [1 2 3 4] 2)  ; => 3
```
```rust
fn vector_nth(vec: *mut PersistentVector, index: u64) -> Value {
    let vec = unsafe { &*vec };

    // Check tail first (optimization)
    if index >= vec.count - vec.tail_len as u64 {
        let tail_index = index - (vec.count - vec.tail_len as u64);
        return unsafe { (*vec.tail)[tail_index as usize] };
    }

    // Traverse tree
    let mut node = vec.root;
    let mut level = vec.shift;

    while level > 0 {
        let child_index = (index >> level) & 0x1F; // & 31
        node = unsafe { (*node).children[child_index as usize] };
        level -= 5;
    }

    // Now at leaf level
    let leaf_index = index & 0x1F;
    unsafe { (*node).children[leaf_index as usize] }
}
```

**4. Update (assoc)**
```clojure
(assoc [1 2 3] 1 99)  ; => [1 99 3]
```
```rust
fn vector_assoc(vec: *mut PersistentVector, index: u64, value: Value) -> *mut PersistentVector {
    // Create new vector sharing structure
    // Only copy path from root to changed leaf
    // This is O(log32 n) which is effectively O(1) for practical sizes

    if index >= vec.count {
        panic!("Index out of bounds");
    }

    // Clone vector metadata
    let new_vec = clone_vector_shallow(vec);

    // Clone path and update
    new_vec.root = clone_path_and_update(vec.root, vec.shift, index, value);

    new_vec
}
```

**Structural Sharing Example**:
```
Original:      [1 2 3 4 5]
                   │
                  Root
                 /    \
              Node1   Node2
              /  \     /  \
             1   2    3   4,5

After (assoc 2 99):  [1 2 99 4 5]
                        │
                      Root' (new)
                     /      \
                  Node1    Node2' (new)
                  /  \      /  \
                 1   2    99'  4,5

Only 3 allocations: Root', Node2', value 99
Original still accessible and unchanged!
```

### 5. Persistent HashMap (HAMT)

**Hash Array Mapped Trie** - even more complex than vector

#### Structure
```
HashMap {:a 1, :b 2, :c 3}

Uses hash of keys to navigate tree:
hash(:a) = 0b10110...
           ││└── bits 0-4  = index in level 0
           │└─── bits 5-9  = index in level 1
           └──── bits 10-14 = index in level 2

Each node has 32 slots (like vector)
But uses bitmap to track which slots are occupied
```

#### Data Structure
```rust
#[repr(C)]
struct PersistentHashMap {
    refcount: AtomicU64,
    count: u64,
    root: *mut HashMapNode,
}

#[repr(C)]
struct HashMapNode {
    refcount: AtomicU64,
    bitmap: u32,              // Which of 32 slots are used
    children: Vec<*mut u8>,   // Nodes or key-value pairs
}

#[repr(C)]
struct HashMapEntry {
    key: Value,
    value: Value,
    hash: u64,
}
```

#### Bitmap Trick
```rust
// Check if slot is occupied
fn has_slot(bitmap: u32, index: u8) -> bool {
    (bitmap & (1 << index)) != 0
}

// Find array index from bitmap
fn array_index(bitmap: u32, index: u8) -> usize {
    // Count number of set bits below index
    (bitmap & ((1 << index) - 1)).count_ones() as usize
}
```

**Example**:
```
bitmap = 0b0000_0000_0000_0101_0001  (slots 0, 2, 8 occupied)
children = [value_at_0, value_at_2, value_at_8]

To access slot 8:
  has_slot(bitmap, 8) = true
  array_index = count_ones(0b0000_0001) = 2
  children[2] = value_at_8
```

### 6. LLVM Type Representations

```rust
// In codegen.rs - new type system

impl<'ctx> CodeGen<'ctx> {
    fn value_type(&self) -> StructType<'ctx> {
        // struct Value { tag: i64, data: [i64; 2] }
        self.context.struct_type(
            &[
                self.context.i64_type().into(),  // tag
                self.context.i64_type().array_type(2).into(), // data
            ],
            false,
        )
    }

    fn vector_type(&self) -> StructType<'ctx> {
        // struct PersistentVector { ... }
        self.context.struct_type(
            &[
                self.context.i64_type().into(),  // refcount
                self.context.i64_type().into(),  // count
                self.context.i8_type().into(),   // shift
                self.context.ptr_type(AddressSpace::default()).into(), // root
                self.context.ptr_type(AddressSpace::default()).into(), // tail
                self.context.i8_type().into(),   // tail_len
            ],
            false,
        )
    }
}
```

### 7. Runtime Library

You'll need a **runtime library** written in Rust:

```rust
// runtime/src/lib.rs

#[no_mangle]
pub extern "C" fn clorus_vector_new() -> *mut PersistentVector {
    // Implementation
}

#[no_mangle]
pub extern "C" fn clorus_vector_conj(vec: *mut PersistentVector, value: Value) -> *mut PersistentVector {
    // Implementation
}

#[no_mangle]
pub extern "C" fn clorus_vector_nth(vec: *mut PersistentVector, index: u64) -> Value {
    // Implementation
}

#[no_mangle]
pub extern "C" fn clorus_hashmap_new() -> *mut PersistentHashMap {
    // Implementation
}

#[no_mangle]
pub extern "C" fn clorus_hashmap_assoc(
    map: *mut PersistentHashMap,
    key: Value,
    value: Value
) -> *mut PersistentHashMap {
    // Implementation
}

#[no_mangle]
pub extern "C" fn clorus_retain(ptr: *mut u8) {
    // Increment refcount
}

#[no_mangle]
pub extern "C" fn clorus_release(ptr: *mut u8) {
    // Decrement refcount, free if 0
}
```

### 8. Code Generation for Vectors

```clojure
; Clorus code
(def v [1 2 3])
(conj v 4)
```

```rust
// Generated LLVM IR (simplified)
impl CodeGen {
    fn compile_vector_literal(&mut self, elements: &[Expr]) -> FloatValue<'ctx> {
        // Call runtime to create empty vector
        let vec_new_fn = self.module.get_function("clorus_vector_new").unwrap();
        let empty_vec = self.builder.build_call(vec_new_fn, &[], "vec").unwrap();

        // Add each element
        let vec_conj_fn = self.module.get_function("clorus_vector_conj").unwrap();
        let mut vec = empty_vec;

        for elem in elements {
            let elem_val = self.compile_expr(elem)?;
            vec = self.builder.build_call(
                vec_conj_fn,
                &[vec.into(), elem_val.into()],
                "vec"
            ).unwrap();
        }

        vec.try_as_basic_value().left().unwrap()
    }
}
```

## Implementation Roadmap

### Phase 1: Foundation (2-4 weeks)
- [ ] Tagged value system (Value struct)
- [ ] Heap allocation (malloc/free wrappers)
- [ ] Reference counting infrastructure
- [ ] Runtime library skeleton

### Phase 2: Basic Collections (2-3 weeks)
- [ ] Persistent List (simplest - linked list)
- [ ] Basic vector operations (create, conj, nth)
- [ ] Tests for memory safety

### Phase 3: Full Vector (2-3 weeks)
- [ ] 32-way trie implementation
- [ ] Tail optimization
- [ ] assoc, pop, subvec
- [ ] Transients (mutable builders)

### Phase 4: HashMap (3-4 weeks)
- [ ] HAMT implementation
- [ ] Hash functions
- [ ] assoc, dissoc, get
- [ ] Hash collisions (ArrayNode)

### Phase 5: Optimization (2-3 weeks)
- [ ] Structural sharing verification
- [ ] Memory usage profiling
- [ ] Performance benchmarks
- [ ] Consider GC instead of RC

## Memory Usage Comparison

### Current Clorus
```clojure
(def x 10)  ; 8 bytes (f64)
```

### With Persistent Data Structures
```clojure
(def v [1 2 3 4 5])
```

**Memory layout**:
```
PersistentVector struct:        48 bytes
├─ refcount:                     8 bytes
├─ count:                        8 bytes
├─ shift:                        1 byte
├─ root pointer:                 8 bytes
├─ tail pointer:                 8 bytes
└─ tail_len:                     1 byte

Tail array:                    256 bytes (32 * 8-byte Values)
├─ value 1:                     24 bytes (tag + data)
├─ value 2:                     24 bytes
├─ value 3:                     24 bytes
├─ value 4:                     24 bytes
└─ value 5:                     24 bytes

Total: ~304 bytes for 5 elements
```

**But with sharing**:
```clojure
(def v1 [1 2 3 4 5])        ; 304 bytes
(def v2 (conj v1 6))        ; +72 bytes (shared structure!)
```

## Example: Complete Vector Implementation

Here's a minimal example:

```rust
// runtime/src/vector.rs

const SHIFT_INCREMENT: u8 = 5;
const BRANCH_FACTOR: usize = 32;

#[repr(C)]
pub struct PersistentVector {
    rc: AtomicUsize,
    cnt: usize,
    shift: u8,
    root: *mut Node,
    tail: *mut [Value; BRANCH_FACTOR],
}

#[repr(C)]
struct Node {
    rc: AtomicUsize,
    array: [*mut Node; BRANCH_FACTOR],
}

impl PersistentVector {
    pub fn new() -> *mut Self {
        Box::into_raw(Box::new(Self {
            rc: AtomicUsize::new(1),
            cnt: 0,
            shift: SHIFT_INCREMENT,
            root: null_mut(),
            tail: null_mut(),
        }))
    }

    pub fn conj(self: *mut Self, val: Value) -> *mut Self {
        unsafe {
            let vec = &*self;

            // Room in tail?
            let tail_len = vec.cnt - (vec.cnt / BRANCH_FACTOR) * BRANCH_FACTOR;
            if tail_len < BRANCH_FACTOR {
                return self.append_tail(val);
            }

            // Need to push tail to tree
            self.overflow_tail(val)
        }
    }

    fn append_tail(self: *mut Self, val: Value) -> *mut Self {
        // Clone vector with new tail
        let new_vec = self.shallow_clone();

        // Clone tail array
        let new_tail = clone_array((*self).tail);
        let tail_offset = (*new_vec).cnt % BRANCH_FACTOR;
        (*new_tail)[tail_offset] = val;

        (*new_vec).tail = new_tail;
        (*new_vec).cnt += 1;

        new_vec
    }

    pub fn nth(self: *const Self, index: usize) -> Value {
        unsafe {
            let vec = &*self;

            // Bounds check
            assert!(index < vec.cnt);

            // In tail?
            if index >= vec.cnt - (vec.cnt % BRANCH_FACTOR) {
                let tail_index = index % BRANCH_FACTOR;
                return (*vec.tail)[tail_index];
            }

            // Navigate tree
            let mut node = vec.root;
            let mut level = vec.shift;

            while level > 0 {
                node = (*node).array[(index >> level) & 0x1F];
                level -= SHIFT_INCREMENT;
            }

            // Leaf node
            (*node).array[index & 0x1F] as Value
        }
    }
}
```

## Recommended Next Steps

1. **Start Simple**: Implement persistent list first (easier than vector)
2. **Add Tagged Values**: Create Value enum/struct
3. **Reference Counting**: Basic retain/release
4. **Runtime Library**: Rust functions callable from LLVM
5. **Code Gen Updates**: Generate calls to runtime
6. **Test Thoroughly**: Memory leaks, correctness
7. **Optimize**: Profile and improve

## Resources

- **Clojure Implementation**: Study `clojure.lang.PersistentVector`
- **Paper**: "Ideal Hash Trees" by Phil Bagwell
- **Paper**: "RRB-Trees: Efficient Immutable Vectors" by Stucki et al.
- **Immutable.js**: Good reference implementation
- **Rust im crate**: Persistent data structures in Rust

## Performance Characteristics

| Operation | Time Complexity | Note |
|-----------|----------------|------|
| Vector conj | O(1)* | Amortized, usually just tail append |
| Vector nth | O(log32 n) ≈ O(1) | Max 7 levels for 2^32 elements |
| Vector assoc | O(log32 n) ≈ O(1) | Path copying |
| HashMap assoc | O(log32 n) ≈ O(1) | Path copying |
| HashMap get | O(log32 n) ≈ O(1) | Tree traversal |

*Effectively constant time for practical sizes!*
