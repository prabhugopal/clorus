# Multi-Arity Closure Capture - Current Status

**Date:** 2026-02-09
**Status:** PARTIALLY FIXED (commit 0ecb22e)

---

## What Works ✅

1. **Multi-arity closures can be created**
   ```clojure
   (defn make-adder [n]
     (fn
       ([x] (+ x n))
       ([x y] (+ x y n))))
   ```
   - Compiles successfully
   - Creates a closure object
   - Verified in `tests/language/test-closure.clr`

2. **Free variable collection works**
   - The codegen correctly identifies variables to capture across all arities
   - Environment arrays are built correctly

3. **DefnMulti self-recursion works**
   - Multi-arity functions can call themselves between arities
   - Example: `(defn f ([x] (f x 0)) ([x y] ...))` works

---

## What Doesn't Work ❌

### The Critical Bug: Nested Closures Can't Capture defn Parameters

**Problem:** When a multi-arity anonymous function (FnMulti) is nested inside a `defn`, it cannot capture the `defn`'s parameters.

**Example that fails:**
```clojure
(defn completing [f]
  (fn
    ([result] result)
    ([result input] (f result input))))
    ;                ^ Error: Undefined function: f
```

**Error message:**
```
Error: Compile error: Undefined function: f (tried f and multi-arity variants)
```

**Impact:**
- Blocks stdlib/transducers.clr (100% blocked)
- Every transducer function uses this pattern
- Cannot enable transducers until this is fixed

---

## Root Cause Analysis

### Current Implementation (commit 0ecb22e)

**Location:** `crates/clorus-codegen/src/codegen.rs`

**What was fixed:**
1. Lines 2926-2946: Free variable collection for FnMulti
2. Lines 2951-3047: Environment parameter added to each arity
3. Lines 3050-3114: Environment array built and passed to closure

**What's still broken:**

When compiling a nested FnMulti inside a defn:
1. The `collect_free_vars()` function walks the FnMulti body
2. It looks for variables in `self.variables` to determine what to capture
3. **BUG:** defn parameters are stored in `self.variables` as LLVM allocas
4. **BUT:** The FnMulti compilation happens in a different scope
5. **RESULT:** The defn parameters are not visible to `collect_free_vars()`

### The Missing Piece

When we enter a `defn` body:
```rust
// In Expr::Defn handling (around line 2500):
// Parameters are added to self.variables:
for (i, param) in params.iter().enumerate() {
    let alloca = self.create_entry_block_alloca(param);
    // Store parameter value in alloca
    self.builder.build_store(alloca, param_value).unwrap();
    self.variables.insert(param.clone(), alloca);
}
```

But when we compile a nested FnMulti (around line 2926):
```rust
// The free_vars collection happens here
let mut free_vars = collect_free_vars(
    &arities,
    &bound,
    &self.variables  // <- This doesn't see parent defn's variables!
);
```

The problem is **scope visibility**: `self.variables` at the point of FnMulti compilation doesn't include the parent defn's parameters.

---

## The Fix Required

### Strategy: Parameter Context Tracking

**Goal:** Make defn parameters visible to nested FnMulti closures

**Option 1: Track Parameter Context** (Recommended)
1. Add a field to CodeGen: `parameter_context: HashMap<String, PointerValue>`
2. When entering a defn, store parameters in `parameter_context`
3. When collecting free vars for nested FnMulti, also check `parameter_context`
4. When exiting defn, clear `parameter_context`

```rust
// In codegen.rs struct
pub struct CodeGen<'ctx> {
    // ... existing fields
    parameter_context: HashMap<String, PointerValue<'ctx>>, // NEW
}

// In Expr::Defn handling (after parameter setup):
for (param, alloca) in params.iter().zip(allocas.iter()) {
    self.parameter_context.insert(param.clone(), *alloca);
}

// In collect_free_vars (or FnMulti handling):
let mut all_visible_vars = self.variables.clone();
all_visible_vars.extend(self.parameter_context.clone());

let free_vars = collect_free_vars(&arities, &bound, &all_visible_vars);

// After compiling defn body:
self.parameter_context.clear();
```

**Option 2: Lexical Scope Stack** (More complex)
1. Maintain a stack of lexical scopes
2. Each scope contains its own variables
3. Free var collection searches up the stack
4. More general but more intrusive changes

---

## Testing Plan

### Test 1: Simple Nested Closure
```clojure
(defn make-adder [n]
  (fn
    ([x] (+ x n))
    ([x y] (+ x y n))))

(def add5 (make-adder 5))
;; Need way to actually call it - pending apply implementation
```

### Test 2: Completing Function
```clojure
(defn completing [f]
  (fn
    ([result] result)
    ([result input] (f result input))))

(def my-rf (completing +))
;; Test calling with 1 and 2 args
```

### Test 3: Full Transducer
```clojure
(defn map [f]
  (fn [rf]
    (fn
      ([result] (rf result))
      ([result input] (rf result (f input))))))

(def xf (map inc))
(def plus-rf (xf +))
;; Test transduce
```

---

## Impact

**Once Fixed:**
- ✅ Enable stdlib/transducers.clr (385 lines, fully implemented)
- ✅ Unlock composable data transformations
- ✅ Match Clojure's transducer capabilities
- ✅ Enable advanced functional patterns

**Estimated Effort:** 2-4 hours
- Understand current parameter handling
- Implement parameter context tracking
- Test with transducers
- Verify no regressions

---

## Next Steps

1. **Implement parameter context tracking** in CodeGen
2. **Update free variable collection** to check parameter context
3. **Test with completing function** (simplest transducer)
4. **Test with full transducers** (map, filter, etc.)
5. **Enable stdlib/transducers.clr**
6. **Run comprehensive transducer tests**

---

## References

- **Commit:** 0ecb22e (Partial fix)
- **Files:**
  - `crates/clorus-codegen/src/codegen.rs` (lines 2500-3118)
  - `stdlib/transducers.clr.disabled` (waiting for fix)
  - `tests/language/test-transducers.clr` (comprehensive tests)
- **Related Docs:**
  - `docs/issues/KNOWN_LIMITATIONS.md`
  - `docs/issues/FIX_PLAN_NO_WORKAROUNDS.md` (Priority 0)
