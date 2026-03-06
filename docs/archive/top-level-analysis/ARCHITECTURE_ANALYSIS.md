# Clorus Architecture Analysis

## Built-in vs Stdlib Duplicates

### Collection Functions (Duplicates)
These exist in BOTH built-in compiler AND stdlib:

| Function | Built-in (codegen.rs) | Stdlib (core.clr) | Issue |
|----------|----------------------|-------------------|-------|
| `map` | Line 4686 | Line 464 | Built-in overrides stdlib |
| `filter` | Line 4838 | Line 471 | Built-in had boolean bug (now fixed) |
| `take` | Line 5906 | Line 38 | Duplicate implementation |
| `drop` | Line 5923 | Line 45 | Duplicate implementation |
| `concat` | Line 5940 | - | Built-in only (good) |

### Collection Access (Built-in only - Good)
These are properly in built-in only:
- `get`, `nth`, `first`, `rest`, `last`, `count`
- `conj`, `disj`, `assoc`, `dissoc`
- `keys`, `vals`, `merge`, `get-in`, `assoc-in`

### String Functions (Built-in only)
- `str`, `subs`, `split`, `join`
- `upper-case`, `lower-case`, `trim`, `trim-left`, `trim-right`
- `replace`, `replace-first`

### Concurrency (Built-in only)
- `atom`, `ref`, `ref-set`, `alter`
- `agent`, `send`, `agent-error`, `await`, `await-for`
- `chan`, `go`

### Stdlib-only Functions (Good)
These are pure Clorus implementations:
- Function utilities: `partial`, `comp`, `constantly`, `complement`, `juxt`, `identity`
- Predicates: `nil?`, `some?`, `zero?`, `pos?`, `neg?`, `even?`, `odd?`, `empty?`
- Collection utils: `zipmap`, `frequencies`, `group-by`, `partition`, `partition-all`
- Sequence ops: `take-while`, `drop-while`, `split-at`, `split-with`
- Math: `abs`, `min`, `max`, `sum`, `product`, `quot`, `rem`
- Accessors: `second`, `third`, `ffirst`, `nfirst`, `fnext`, `nnext`, `butlast`
- Sorting: `sort`, `sort-by`, `reverse`
- Generators: `range`, `repeat`, `repeatedly`, `cycle`
- Type checks: `string?`, `char?`, `keyword?`, `symbol?`, `vector?`, `map?`, `set?`, `seq?`, `coll?`, `fn?`

## Problems Identified

### 1. Duplicate Implementations
**Impact:** Maintenance burden, potential bugs, confusion

**Examples:**
- `map` exists in both built-in and stdlib (stdlib never runs due to override)
- `filter` had a bug in built-in that wouldn't exist if using stdlib version
- `take` and `drop` duplicated unnecessarily

**Recommendation:** Remove duplicates from built-in, keep only in stdlib

### 2. Hardcoded Function Names
**Location:** `crates/clorus-codegen/src/codegen.rs:3115`

```rust
"map", "filter", "reduce", "apply", "conj", "disj", "contains?", "concat",
"assoc", "dissoc", "get", "nth", "first", "rest", "last", "count"
```

These are listed in the special forms check, creating a tight coupling.

### 3. Boolean Handling (FIXED)
**Issue:** Comparison operators were returning numbers instead of booleans
**Fixed:** Now properly return `true`/`false` boolean values
**Affected:**
- Comparison operators: `<`, `>`, `<=`, `>=`, `=`
- `if` expressions
- `filter` predicate checking

### 4. Number Type Handling (FIXED)
**Issue:** `unbox_number` only handled Double, not Long (integers)
**Fixed:** Added `clorus_value_as_number` that handles both Long and Double

### 5. Generic Collection Operations (FIXED)
**Issue:** `conj` was hardcoded to call `clorus_set_conj` only
**Fixed:** Added generic `clorus_conj` that dispatches by collection type

## Hardcoded Values Across Codebase

### Runtime (clorus-runtime/src/)

#### value.rs
- Type tags enum (ValueTag): Hard-coded list of all value types
  - Long, Double, List, Vector, HashMap, String, Keyword, Symbol, Bool, Nil, HashSet, Atom, Ref, Agent, Channel, Function, Var

#### collections.rs, vector.rs, list.rs, map.rs, set.rs
- FFI function names (exported as C symbols)
- Must match exactly what codegen expects

### Codegen (clorus-codegen/src/codegen.rs)

#### Function Declarations (Lines 200-600)
All runtime functions must be declared upfront:
- `clorus_value_long`, `clorus_value_double`, `clorus_value_bool`
- `clorus_vector_empty`, `clorus_vector_conj`, `clorus_vector_nth`
- `clorus_list_empty`, `clorus_list_cons`
- `clorus_map_empty`, `clorus_map_assoc`, `clorus_map_get`
- And 50+ more...

#### Special Forms (Line ~1000+)
Hard-coded keywords: `let`, `def`, `defn`, `if`, `do`, `quote`, `fn`, `loop`, `recur`

#### Built-in Functions (Lines 4300-6300)
~50 built-in function implementations hard-coded

### Stdlib (stdlib/core.clr)
- Pure Clorus implementations
- Loaded at REPL startup
- 76 functions currently

## Recommendations

### Short Term
1. ✅ Fix boolean handling (DONE)
2. ✅ Fix number type handling for Long/Double (DONE)
3. ✅ Fix generic `conj` (DONE)
4. Remove duplicate `map` and `filter` from built-in (use stdlib only)
5. Remove duplicate `take` and `drop` from built-in

### Medium Term
1. Create a registry system for built-in functions instead of hard-coding
2. Make stdlib auto-loaded and tested comprehensively
3. Add stdlib test suite
4. Document which functions should be built-in (performance-critical) vs stdlib

### Long Term
1. Consider JIT compilation for stdlib functions that are frequently used
2. Implement function inlining for stdlib functions
3. Add profiling to identify hot paths
4. Consider making more functions be stdlib-only (reduce compiler complexity)

## Why Have Both?

### Good Reasons for Built-ins:
1. **Performance:** Direct LLVM IR generation (e.g., `nth`, `count`, `get`)
2. **FFI:** Need to call runtime functions (e.g., `atom`, `ref`, concurrency primitives)
3. **Special behavior:** Functions that need compiler support (e.g., `apply`, `reduce` with function calling)

### Good Reasons for Stdlib:
1. **Maintainability:** Easier to modify and test
2. **Dogfooding:** Tests the language itself
3. **Visibility:** Users can read the implementation
4. **Extensibility:** Users can override if needed

### Bad Reasons (Current State):
1. **Duplicates:** Having both `filter` in built-in and stdlib
2. **Override behavior:** Built-in silently overrides stdlib (confusing)
3. **Bugs:** Built-in `filter` had boolean bug, stdlib version would have worked

## Test Coverage Needed

### Critical Missing Tests:
1. Stdlib function tests (all 76 functions)
2. Comparison operator tests (just fixed)
3. Boolean value tests
4. Long/Double type coercion tests
5. Collection operation tests
6. Integration tests for stdlib + built-in interaction

### Test Strategy:
```clojure
;; Example test file: stdlib/test/core-test.clr
(ns core-test
  (:require [core :as c]))

(defn test-filter []
  (assert (= (filter even? [1 2 3 4 5 6]) [2 4 6]))
  (assert (= (filter odd? [1 2 3 4 5 6]) [1 3 5])))

(defn test-map []
  (assert (= (map inc [1 2 3]) [2 3 4])))

(defn test-comparisons []
  (assert (= (< 5 10) true))
  (assert (= (> 5 10) false))
  (assert (= (= 5 5) true)))
```

## Summary

**Current Issues Fixed:**
- ✅ Comparison operators return booleans
- ✅ Long/Double type handling
- ✅ Generic `conj` for all collection types
- ✅ `filter` boolean checking

**Remaining Issues:**
- ❌ Duplicate implementations (map, filter, take, drop)
- ❌ No stdlib test coverage
- ❌ Hardcoded function names throughout codegen
- ❌ No clear policy on built-in vs stdlib

**Priority Actions:**
1. Add stdlib test suite
2. Remove duplicate built-ins (keep only in stdlib where possible)
3. Document built-in vs stdlib policy
4. Add integration tests
