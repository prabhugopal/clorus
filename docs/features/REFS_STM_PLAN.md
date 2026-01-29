# Refs & STM Implementation Plan

**Feature:** Software Transactional Memory (STM) with Refs
**Priority:** 🔥 High - Completes state management
**Effort:** 1 week
**Status:** Planning

---

## Overview

Implement Clojure-style refs with STM to enable **coordinated, synchronous state changes** across multiple references. Unlike atoms (single-value updates), refs allow multiple values to be updated atomically within a transaction.

**Current State:**
- ✅ Atoms - Single mutable reference (swap!, reset!, deref)
- ❌ Refs - Coordinated transactional updates
- ❌ STM - Transaction system

**Goal State:**
- ✅ `ref` - Create transactional reference
- ✅ `dosync` - Transaction block
- ✅ `alter` - Update ref with function in transaction
- ✅ `ref-set` - Set ref value in transaction
- ✅ `commute` - Commutative update
- ✅ `@ref` / `deref` - Read ref value

---

## Key Concepts

### What is STM?

**Software Transactional Memory** provides ACID properties for memory operations:
- **Atomicity** - All updates succeed or all fail
- **Consistency** - Refs move from one valid state to another
- **Isolation** - Concurrent transactions don't see partial updates
- **Durability** - N/A (in-memory only)

### Atoms vs Refs

| Feature | Atoms | Refs |
|---------|-------|------|
| Coordination | ❌ No | ✅ Yes |
| Transactions | ❌ No | ✅ Yes |
| Synchronous | ✅ Yes | ✅ Yes |
| Use case | Single value | Multiple coordinated values |

**Example Problem Refs Solve:**
```clojure
;; WRONG - Race condition with atoms!
(def account-a (atom 1000))
(def account-b (atom 500))

;; Transfer money - NOT ATOMIC!
(swap! account-a - 100)  ; If this succeeds but next fails...
(swap! account-b + 100)  ; ...money disappears!

;; RIGHT - Atomic with refs
(def account-a (ref 1000))
(def account-b (ref 500))

(dosync
  (alter account-a - 100)  ; Both succeed or both fail
  (alter account-b + 100))
```

---

## Design Decisions

### STM Strategy: MVCC (Multi-Version Concurrency Control)

**Why MVCC?**
- ✅ Lock-free reads (fast)
- ✅ Optimistic concurrency (rarely conflicts)
- ✅ Snapshot isolation
- ✅ Proven design (Clojure uses this)

**How It Works:**
1. Each ref has a version counter
2. Transactions read current version
3. Writes create new versions
4. Commit checks if versions changed (CAS)
5. Retry on conflict

### Transaction Lifecycle

```
dosync starts → Read refs (snapshot) → Execute body → Validate → Commit/Retry
                                                          ↓
                                                    Conflict? → Retry
                                                    Valid? → Commit
```

---

## Architecture

### Data Structures

#### Ref Structure
```rust
pub struct ClorusRef {
    /// Current value (versioned)
    value: Arc<Mutex<RefValue>>,
    /// Validators (functions that check value is valid)
    validators: Vec<fn(*mut Value) -> bool>,
}

struct RefValue {
    /// Current value
    value: *mut Value,
    /// Version number (incremented on write)
    version: u64,
    /// Transaction ID that last wrote
    last_writer: Option<TxId>,
}
```

#### Transaction Context (Thread-Local)
```rust
thread_local! {
    static TX_CONTEXT: RefCell<Option<Transaction>> = RefCell::new(None);
}

struct Transaction {
    /// Transaction ID
    id: TxId,
    /// Read set: (ref, version)
    reads: HashMap<RefId, u64>,
    /// Write set: (ref, new_value)
    writes: HashMap<RefId, *mut Value>,
    /// Commute set: (ref, function, args)
    commutes: HashMap<RefId, Vec<CommuteFn>>,
    /// Start time
    start_time: Instant,
}
```

---

## API Design

### 1. Creating Refs

#### `ref` - Create a ref
```clojure
(ref initial-value)
(ref initial-value :validator validate-fn)

;; Examples
(def counter (ref 0))
(def inventory (ref {:apples 100 :oranges 50}))
(def balance (ref 1000 :validator #(>= % 0)))  ; Must be non-negative
```

**Signature:** `(ref value & {:keys [validator]}) -> Ref`
**FFI:** `clorus_ref(value: *mut Value) -> *mut Value`

#### `deref` / `@` - Read ref value
```clojure
@counter         ; => 0
(deref counter)  ; => 0
```

**Note:** Same as atoms, already works!

---

### 2. Transactions

#### `dosync` - Transaction block
```clojure
(dosync
  (alter account-a - 100)
  (alter account-b + 100))
```

**Signature:** `(dosync & body) -> result`
**Implementation:** Macro that wraps body in transaction context

**Behavior:**
- Starts transaction
- Executes body
- Commits or retries on conflict
- Returns body result
- Max retries: 10000 (configurable)

---

### 3. Updating Refs

#### `alter` - Update ref with function
```clojure
(alter ref f & args)

;; Examples
(dosync
  (alter counter inc)
  (alter balance - 100)
  (alter inventory update :apples + 10))
```

**Signature:** `(alter ref f & args) -> new-value`
**FFI:** `clorus_alter(ref: *mut Value, f: *mut Value, args: *mut Value) -> *mut Value`

**Behavior:**
- Must be inside dosync
- Reads current value
- Applies function
- Stages write for commit
- Validates result
- Returns new value

#### `ref-set` - Set ref value directly
```clojure
(ref-set ref new-value)

;; Example
(dosync
  (ref-set counter 42))
```

**Signature:** `(ref-set ref value) -> value`
**FFI:** `clorus_ref_set(ref: *mut Value, value: *mut Value) -> *mut Value`

**Behavior:**
- Must be inside dosync
- Sets value directly (no function)
- Stages write for commit
- Validates result

#### `commute` - Commutative update
```clojure
(commute ref f & args)

;; Example - order doesn't matter
(dosync
  (commute counter inc))  ; Safe even if other transactions increment
```

**Signature:** `(commute ref f & args) -> new-value`
**FFI:** `clorus_commute(ref: *mut Value, f: *mut Value, args: *mut Value) -> *mut Value`

**Behavior:**
- Like alter but **order-independent**
- Can be applied at commit time
- Reduces conflicts
- Use for operations like +, *, conj

**When to use commute:**
- Increment/decrement counters
- Add to collections
- Any operation where order doesn't matter

---

### 4. Advanced Operations

#### `ensure` - Add ref to read set
```clojure
(ensure ref)

;; Example - prevent phantom reads
(dosync
  (ensure inventory)
  (when (contains? @inventory :rare-item)
    (alter sold-items conj :rare-item)))
```

**Signature:** `(ensure ref) -> ref-value`

**Behavior:**
- Adds ref to transaction read set
- Prevents value from changing during transaction
- Use to prevent phantom reads

---

## Implementation Plan

### Phase 1: Runtime Ref Structure (Day 1-2)

**File:** `/crates/clorus-runtime/src/ref.rs`

#### Step 1.1: Basic Ref Structure
```rust
use crate::value::{Value, ValueTag};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct ClorusRef {
    value: Arc<Mutex<RefValue>>,
}

struct RefValue {
    value: *mut Value,
    version: u64,
}

impl ClorusRef {
    pub fn new(initial: *mut Value) -> Self {
        unsafe { (*initial).header().retain(); }
        ClorusRef {
            value: Arc::new(Mutex::new(RefValue {
                value: initial,
                version: 0,
            })),
        }
    }

    pub fn deref(&self) -> *mut Value {
        let guard = self.value.lock().unwrap();
        let val = guard.value;
        unsafe { (*val).header().retain(); }
        val
    }
}

#[no_mangle]
pub extern "C" fn clorus_ref(initial: *mut Value) -> *mut Value {
    let ref_obj = Box::new(ClorusRef::new(initial));
    Value::from_ptr(ValueTag::Ref, Box::into_raw(ref_obj) as *mut u8)
}

#[no_mangle]
pub extern "C" fn clorus_ref_deref(ref_val: *mut Value) -> *mut Value {
    if ref_val.is_null() {
        return Value::nil();
    }
    unsafe {
        if (*ref_val).header().tag() == ValueTag::Ref {
            let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
            (*ref_ptr).deref()
        } else {
            Value::nil()
        }
    }
}
```

#### Step 1.2: Add ValueTag::Ref
```rust
// In value.rs
pub enum ValueTag {
    // ... existing
    Ref = 11,  // New!
}
```

---

### Phase 2: Transaction Context (Day 2-3)

**File:** `/crates/clorus-runtime/src/transaction.rs`

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

thread_local! {
    static TX_CONTEXT: RefCell<Option<Transaction>> = RefCell::new(None);
}

type TxId = u64;
type RefId = usize;

pub struct Transaction {
    id: TxId,
    reads: HashMap<RefId, u64>,        // (ref_id, version)
    writes: HashMap<RefId, *mut Value>, // (ref_id, new_value)
    start_time: Instant,
}

impl Transaction {
    fn new() -> Self {
        static TX_COUNTER: AtomicU64 = AtomicU64::new(0);
        Transaction {
            id: TX_COUNTER.fetch_add(1, Ordering::SeqCst),
            reads: HashMap::new(),
            writes: HashMap::new(),
            start_time: Instant::now(),
        }
    }
}

#[no_mangle]
pub extern "C" fn clorus_tx_begin() {
    TX_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(Transaction::new());
    });
}

#[no_mangle]
pub extern "C" fn clorus_tx_commit() -> bool {
    TX_CONTEXT.with(|ctx| {
        if let Some(tx) = ctx.borrow_mut().take() {
            // Validate all reads still have same version
            // Apply all writes atomically
            // Return true if success
            commit_transaction(tx)
        } else {
            false
        }
    })
}

#[no_mangle]
pub extern "C" fn clorus_tx_abort() {
    TX_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
}

fn commit_transaction(tx: Transaction) -> bool {
    // 1. Lock all refs in write set (in order to prevent deadlock)
    // 2. Validate all reads (check versions)
    // 3. Apply all writes
    // 4. Increment versions
    // 5. Release locks
    // 6. Return true

    // TODO: Full implementation
    true
}
```

---

### Phase 3: Ref Operations (Day 3-4)

**File:** `/crates/clorus-runtime/src/ref.rs` (continued)

```rust
#[no_mangle]
pub extern "C" fn clorus_alter(
    ref_val: *mut Value,
    func: *mut Value,
    args: *mut Value,
) -> *mut Value {
    // 1. Check we're in transaction
    // 2. Read current value
    // 3. Apply function
    // 4. Stage write in transaction
    // 5. Return new value

    unsafe {
        if (*ref_val).header().tag() != ValueTag::Ref {
            return Value::nil();
        }

        let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
        let current = (*ref_ptr).deref();

        // Call function with current value + args
        // let new_value = call_function(func, current, args);

        // Stage in transaction write set
        // transaction::stage_write(ref_ptr, new_value);

        // Return new value
        // new_value

        current  // TODO: Implement
    }
}

#[no_mangle]
pub extern "C" fn clorus_ref_set(
    ref_val: *mut Value,
    new_value: *mut Value,
) -> *mut Value {
    // Similar to alter but no function call
    unsafe {
        (*new_value).header().retain();
    }
    new_value
}
```

---

### Phase 4: Codegen Integration (Day 4-5)

**File:** `/crates/clorus-codegen/src/codegen.rs`

#### Step 4.1: FFI Declarations
```rust
// In declare_runtime_functions()

// clorus_ref(value: *mut Value) -> *mut Value
let ref_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
self.module.add_function("clorus_ref", ref_type, None);

// clorus_ref_deref(ref: *mut Value) -> *mut Value
let ref_deref_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
self.module.add_function("clorus_ref_deref", ref_deref_type, None);

// Transaction operations
let tx_begin_type = self.context.void_type().fn_type(&[], false);
self.module.add_function("clorus_tx_begin", tx_begin_type, None);

let tx_commit_type = self.context.bool_type().fn_type(&[], false);
self.module.add_function("clorus_tx_commit", tx_commit_type, None);

// clorus_alter(ref, func, args) -> *mut Value
let alter_type = i8_ptr_type.fn_type(
    &[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()],
    false
);
self.module.add_function("clorus_alter", alter_type, None);

// clorus_ref_set(ref, value) -> *mut Value
let ref_set_type = i8_ptr_type.fn_type(
    &[i8_ptr_type.into(), i8_ptr_type.into()],
    false
);
self.module.add_function("clorus_ref_set", ref_set_type, None);
```

#### Step 4.2: Builtin Dispatch
```rust
// In compile_core_call()

"ref" => {
    if args.len() != 1 {
        return Err("ref requires 1 argument: initial-value".to_string());
    }

    let initial = self.compile_expr(&args[0])?;
    let ref_fn = self.module.get_function("clorus_ref")
        .ok_or("clorus_ref not declared")?;

    let result = self.builder.build_call(
        ref_fn,
        &[initial.into()],
        "ref_call"
    ).unwrap();

    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
}

"alter" => {
    // Must be inside dosync - check at runtime
    if args.len() < 2 {
        return Err("alter requires at least 2 args: ref, function, [args]".to_string());
    }

    let ref_val = self.compile_expr(&args[0])?;
    let func_val = self.compile_expr(&args[1])?;

    // Pack remaining args into vector
    let mut args_vec = vec![];
    for arg in &args[2..] {
        args_vec.push(self.compile_expr(arg)?);
    }

    // TODO: Create vector from args_vec

    let alter_fn = self.module.get_function("clorus_alter")
        .ok_or("clorus_alter not declared")?;

    let result = self.builder.build_call(
        alter_fn,
        &[ref_val.into(), func_val.into(), /* args_vec */],
        "alter_call"
    ).unwrap();

    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
}
```

---

### Phase 5: dosync Macro (Day 5-6)

**File:** `/crates/clorus-macros/src/lib.rs` or builtin macros

```rust
// dosync is a special form that needs retry logic
// Pseudocode:

(defmacro dosync [& body]
  `(do
     (tx-begin)
     (loop [retry-count 0]
       (if (> retry-count 10000)
         (throw "Transaction failed after 10000 retries")
         (let [result (try
                        (do ~@body)
                        (catch e
                          (tx-abort)
                          (throw e)))]
           (if (tx-commit)
             result
             (recur (inc retry-count))))))))
```

**Better:** Implement as special form in compiler for efficiency

---

### Phase 6: Testing (Day 6-7)

#### Unit Tests (Rust)
```rust
#[test]
fn test_ref_create_and_deref() {
    unsafe {
        let val = Value::number(42.0);
        let ref_val = clorus_ref(val);
        let deref_val = clorus_ref_deref(ref_val);
        assert_eq!((*deref_val).as_number(), 42.0);
    }
}

#[test]
fn test_transaction_basic() {
    clorus_tx_begin();
    // ... do work
    assert!(clorus_tx_commit());
}
```

#### Integration Tests (Clorus)
```clojure
;; Test 1: Basic ref
(def counter (ref 0))
@counter  ; => 0

;; Test 2: Simple transaction
(dosync
  (ref-set counter 10))
@counter  ; => 10

;; Test 3: Alter
(dosync
  (alter counter inc))
@counter  ; => 11

;; Test 4: Multiple refs - COORDINATED
(def a (ref 100))
(def b (ref 50))

(dosync
  (alter a - 25)
  (alter b + 25))

@a  ; => 75
@b  ; => 75

;; Test 5: Bank account transfer
(def account-a (ref 1000))
(def account-b (ref 500))

(defn transfer [from to amount]
  (dosync
    (alter from - amount)
    (alter to + amount)))

(transfer account-a account-b 100)
@account-a  ; => 900
@account-b  ; => 600

;; Test 6: Retry on conflict (concurrent)
;; TODO: Spawn threads to test conflict resolution
```

---

## Success Criteria

### Functional Requirements ✅
- [x] ref creates a transactional reference
- [x] @ / deref reads ref value
- [x] dosync creates transaction block
- [x] alter updates ref with function
- [x] ref-set sets ref value directly
- [x] commute provides commutative updates
- [x] Multiple refs can be updated atomically
- [x] Conflicts cause retry (up to limit)
- [x] Validators work

### Performance Requirements ✅
- [x] Read operations are fast (lock-free)
- [x] Most transactions commit first try
- [x] Retry limit prevents infinite loops
- [x] Memory cleanup (reference counting)

### Testing Requirements ✅
- [x] Unit tests for ref operations
- [x] Integration tests for transactions
- [x] Concurrent transaction tests
- [x] Bank account transfer example
- [x] Conflict resolution tests

---

## Timeline

### Week 1: Full STM Implementation

**Day 1-2:** Runtime ref structure
- Create ClorusRef type
- Add ValueTag::Ref
- Basic ref creation and deref

**Day 3:** Transaction context
- Thread-local transaction
- Begin/commit/abort
- Read/write sets

**Day 4:** Ref operations
- alter implementation
- ref-set implementation
- Function calling in transactions

**Day 5:** Codegen integration
- FFI declarations
- Builtin dispatch
- dosync special form

**Day 6:** Testing
- Unit tests (Rust)
- Integration tests (Clorus)
- Concurrent tests

**Day 7:** Documentation & Examples
- API documentation
- Tutorial examples
- Performance guide

---

## Future Enhancements (Post-MVP)

### Phase 2 Features:
- Validators - `(ref value :validator fn)`
- Watchers - `(add-watch ref key fn)`
- Min/max history - Version retention
- Barging - Older transactions can steal from newer

### Phase 3 Features:
- Agents - Async state updates
- Channels - CSP concurrency
- STM performance optimizations

---

## Risk Assessment

### Low Risk ✅
- Basic ref structure
- Deref operations
- Single-ref transactions

### Medium Risk ⚠️
- Multi-ref coordination
- Conflict detection
- Retry logic

### High Risk ❌
- Concurrent transaction conflicts
- Deadlock prevention
- Performance under contention

---

## Dependencies

### Required ✅
- Value system (already have)
- Reference counting (already have)
- Function calling (already have)
- Atoms implementation (reference)

### Optional
- Threading (for concurrent tests)
- Async runtime (for agents later)

---

## Comparison with Clojure

| Feature | Clojure | Clorus (Planned) | Status |
|---------|---------|------------------|--------|
| ref | ✅ | 🚧 | Planned |
| dosync | ✅ | 🚧 | Planned |
| alter | ✅ | 🚧 | Planned |
| ref-set | ✅ | 🚧 | Planned |
| commute | ✅ | 🚧 | Planned |
| ensure | ✅ | 🚧 | Planned |
| Validators | ✅ | ❌ | Future |
| Watchers | ✅ | ❌ | Future |
| Barging | ✅ | ❌ | Future |

**MVP Coverage:** ~70% of Clojure ref features
**Production Ready:** Yes, for basic coordinated state

---

## Conclusion

Refs & STM will complete Clorus's state management story:
- ✅ **Atoms** - Single value updates
- 🚧 **Refs** - Coordinated updates (THIS!)
- ⏭️ **Agents** - Async updates (NEXT)

**Timeline:** 1 week for full implementation
**Complexity:** Medium (build on atoms, simpler than polymorphism)
**Impact:** High (enables real concurrent programs)

---

**Status:** 📋 Plan Complete - Ready to Implement
**Next Step:** Create `/crates/clorus-runtime/src/ref.rs` and begin Phase 1

---

*Last Updated: January 27, 2026*
*Contributors: Prabhu Gopal + Claude Code*
