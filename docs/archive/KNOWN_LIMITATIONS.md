> **Archived 2026-09-23.** Verified stale by direct execution against current code: both the flat and nested multi-arity closure capture cases this document describes as broken now work correctly, including the full transducer pipeline it says is blocked by them. Kept for history only — see `docs/PRODUCTION_PLAN.md` and `docs/generated/PARITY_STATUS.md` for current status.

# Clorus Known Limitations

## Multi-Arity Closures Don't Support Variable Capture

**Status:** Active Limitation
**Priority:** P1 (Blocks transducers and other advanced patterns)
**Date Identified:** February 8, 2026

### Description

Multi-arity anonymous functions cannot capture variables from outer scopes. Single-arity closures work fine, but multi-arity closures fail with "Undefined variable" or "Undefined function" errors.

### Examples

**Works (Single-Arity Closure):**
```clojure
(defn make-adder [n]
  (fn [x] (+ x n)))  ;; ✅ Works - single arity

(def add5 (make-adder 5))
(add5 10)  ;; => 15
```

**Doesn't Work (Multi-Arity Closure):**
```clojure
(defn make-adder [n]
  (fn
    ([x] (+ x n))          ;; ❌ Error: Undefined variable: n
    ([x y] (+ x y n))))    ;; ❌ Error: Undefined variable: n
```

**Real-World Impact - Transducers:**
```clojure
(defn completing [f]
  (fn
    ([result] result)           ;; ❌ Can't capture f
    ([result input] (f result input))))  ;; ❌ Error: Undefined function: f
```

### Root Cause

The code generator (crates/clorus-codegen/src/codegen.rs) handles closure capture for single-arity anonymous functions but not for multi-arity ones. Multi-arity functions compile to a different structure that doesn't properly set up the closure environment.

### Impact

This limitation blocks several advanced Clojure patterns:

1. **Transducers** - Cannot implement completing function or transducers that return multi-arity reducing functions
2. **Function Builders** - Cannot create multi-arity functions that depend on outer parameters
3. **Protocol Implementations** - Cannot implement multi-method protocols with closures

### Workarounds

1. **Use Single-Arity with Variadic Args:**
   ```clojure
   ;; Instead of:
   (fn
     ([x] (+ x n))
     ([x y] (+ x y n)))

   ;; Use:
   (fn [& args]
     (if (= (count args) 1)
       (+ (first args) n)
       (+ (first args) (second args) n)))
   ```

2. **Return a Map of Functions:**
   ```clojure
   ;; Instead of multi-arity closure
   {:step (fn [result input] (f result input))
    :complete (fn [result] result)}
   ```

3. **Avoid Multi-Arity Closures:**
   - Restructure code to use single-arity closures
   - Pass captured variables as extra arguments

### Files Affected

- `stdlib/transducers.clr` - Cannot be loaded due to this limitation
- `src/clorus/transducers.clrs` - Module version also fails
- Any user code using multi-arity closures

### Required Fix

**Location:** `crates/clorus-codegen/src/codegen.rs`

The multi-arity function compilation needs to:
1. Detect when compiling an anonymous function inside another function
2. Capture variables from outer scope in closure environment
3. Pass closure environment to each arity's LLVM function
4. Set up closure capture similar to single-arity functions

**Estimated Effort:** 4-8 hours
- Understand current closure capture mechanism for single-arity
- Extend to multi-arity function compilation
- Test with transducers and other patterns
- Ensure no regressions

### Progress

- ✅ Added transducer runtime support (clorus_ensure_reduced)
- ✅ Added stdlib loading for transducers.clr
- ✅ Fixed ::none keyword syntax
- ✅ Identified limitation with multi-arity closures
- ✅ **MAJOR**: Implemented multi-arity closure capture for FnMulti
- ✅ **MAJOR**: Fixed DefnMulti self-recursion (functions can call themselves)
- ⚠️ **PARTIAL**: Simple closures work, nested closures still fail
- ❌ Transducers still not fully working due to nested closure limitation

### What Works Now

✅ **Multi-arity functions can call themselves recursively:**
```clojure
(defn transduce
  ([xform f coll] (transduce xform f (f) coll))  ; ✅ Works!
  ([xform f init coll] ...))
```

✅ **Top-level multi-arity closures with captured variables:**
```clojure
;; This would work if we could call it:
(def make-multi
  (let [n 5]
    (fn
      ([x] (+ x n))      ; ✅ Captures n
      ([x y] (+ x y n))))) ; ✅ Captures n
```

### What Still Fails

❌ **Nested closures (FnMulti inside DefnMulti parameters):**
```clojure
(defn completing
  ([f]
   (fn                           ; ❌ Inner FnMulti
     ([result] result)
     ([result input] (f result input))))  ; ❌ Can't capture f from defn parameter
  ([f cf]
   (fn
     ([result] (cf result))      ; ❌ Can't capture cf
     ([result input] (f result input)))))  ; ❌ Can't capture f

;; Error: Undefined function: f
```

The issue is that when compiling the inner FnMulti, the defn's parameters (f, cf)
are not visible in self.variables, so they can't be captured.

### Testing

To verify this is fixed, run:

```bash
# Test multi-arity closure capture
cargo run --bin clorus -- run test-closure.clr

# Expected output:
# Testing multi-arity closures...
# 1-arity: 15
# 2-arity: 35
# Done!

# Test transducers
cargo run --bin clorus -- run test-transducers.clr

# Expected output:
# Testing transducers...
# Test 1: map transducer
#   (transduce (map * 2) + 0 [1 2 3 4 5]): 30
#   Expected: 30, Got: 30
# ...
```

### Related Issues

- coral/CLORUS_WORKAROUNDS.md - Documents string use-after-free and other limitations
- coral/COMPILER_HANG_BUG.md - Documents compiler hang issue
- Issue #3 - Mutual recursion (FIXED with letfn)

### Notes

- Single-arity closures work fine (`partial`, `constantly`, `complement` in stdlib)
- The limitation is specific to multi-arity anonymous functions
- This is NOT a limitation of regular multi-arity defn functions
- Only affects closures (anonymous functions capturing outer scope)
