# Top 3 Fixes: Detailed Plans (Code-Only)

This document expands the three proposed focus areas into concrete, test-driven plans.

## 1) Fix Macro Expansion Across Forms

**Problem**  
`parse_and_expand` expands each form with a new `MacroRegistry`. As a result, `(defmacro ...)` in one form does not affect later forms in the same file.

**Goal**  
Keep a single macro registry per file parse/expand so macros persist across forms.

**Implementation Strategy**
1. Modify `parse_and_expand` to:
   - Parse all forms.
   - Create a `MacroRegistry` once.
   - Expand forms in order, reusing the same registry.
2. Add a new function (if needed) that exposes registry-based expansion:
   - Example: `expand_macros_with_registry_sequence(exprs: &[Expr]) -> Vec<Expr>`
3. Update `clorus::parse_and_expand` to use the new flow.

**Files**
- `crates/clorus-syntax/src/lib.rs`
- `crates/clorus-syntax/src/macros.rs`

**Tests to Add**
- Parse a file with:
  - `(defmacro twice [x] \`(+ ~x ~x))`
  - `(twice 3)`
  - Ensure expansion produces `(+ 3 3)` for the second form.
- Ensure macro nesting and `gensym` still work.

**Risks**
- If external callers rely on isolated per-form expansion, behavior changes. This is expected to be more correct.

---

## 2) Map/Set Correctness: Structural Hashing + Equality

**Problem**  
Maps/Sets are hash-only with pointer equality semantics. Collisions overwrite entries; equality is pointer-based.

**Goal**  
Correctness before persistence:
- Implement structural equality for collections and complex types.
- Implement hash functions aligned with equality.
- Avoid hash-only keys; use `(hash, key)` or key equality checks.

**Implementation Strategy (Staged)**
1. **Structural hashing + equality (Stage A)**
   - Implement `clorus_hash` and `clorus_equals` for vectors/lists/maps/sets.
   - Use deep structural comparison for collections.
2. **Map/Set storage fix (Stage B)**
   - Replace `HashMap<u64, ...>` with `HashMap<u64, Vec<(key, val)>>` or a custom bucket list.
   - On lookup: compute hash, then linear scan in bucket using `clorus_equals` on keys.
   - On assoc: replace existing key or insert new.
3. **Optional persistence (Stage C)**
   - Implement HAMT/CHAMP after correctness is fixed.

**Files**
- `crates/clorus-runtime/src/value.rs`
- `crates/clorus-runtime/src/map.rs`
- `crates/clorus-runtime/src/set.rs`
- `crates/clorus-runtime/src/collections.rs`

**Tests to Add**
- Two distinct but equal vectors/maps are `=`.  
- Hash collisions do not overwrite entries.  
- `contains?` and `get` work for equal-but-not-identical keys.  

**Risks**
- Structural hashing requires cycle handling (can start without cycles and add later).

---

## 3) CAS Retry Loop for `swap!`

**Problem**  
`swap!` lacks CAS retry. Under contention, updates can be lost.

**Goal**  
`swap!` should be linearizable, retrying until it succeeds.

**Implementation Strategy**
1. Load current value.
2. Compute new value by applying function.
3. Use `compare_exchange` to swap if unchanged.
4. If CAS fails, release the computed value and retry.
5. Ensure retain/release safety for values across retries.

**Files**
- `crates/clorus-runtime/src/atom.rs`

**Tests to Add**
- Multi-threaded test with concurrent `swap!` increments should equal expected total.
- Ensure `swap!` retains/releases correctly to avoid leaks.

**Risks**
- CAS loop needs careful retain/release to avoid memory leaks.

---

## Follow-On (Optional)

- Update `contains?` to support maps/vectors, matching Clojure semantics.
- Fix quote semantics to preserve lists and support true unquote-splicing.
- Repair `update` runtime (currently stubbed).
