# Patch Plan: Top 3 Fixes (Step-by-Step)

This is a concrete patch plan (file-by-file, incremental) for the top 3 fixes.

## 1) Macro Expansion Across Forms

**Goal**  
Ensure `defmacro` in one form affects later forms in the same file.

**Steps**
1. Add a new public helper in `crates/clorus-syntax/src/macros.rs`:
   - `expand_macros_sequence(exprs: &[Expr]) -> Vec<Expr>`
   - Internally:
     - Create one `MacroRegistry`.
     - For each expr: `expand_macros_with_registry`.
2. Update `crates/clorus-syntax/src/lib.rs`:
   - Change `parse_and_expand` to parse once, then call `expand_macros_sequence`.
3. Add unit tests:
   - New test: parse two forms `(defmacro twice ...)` and `(twice 3)`.
   - Assert the second expands to `(+ 3 3)` or equivalent AST.

**Files**
- `crates/clorus-syntax/src/macros.rs`
- `crates/clorus-syntax/src/lib.rs`

**Tests**
- Add in `crates/clorus-syntax/src/macros.rs` or `crates/clorus-syntax/src/lib.rs`.

---

## 2) Map/Set Correctness (Structural Hash + Equality)

**Goal**  
Prevent hash-collision bugs and make equality structural for collections.

**Steps**
1. Implement `clorus_hash` in `crates/clorus-runtime/src/value.rs`:
   - Hash primitives by value.
   - Hash vectors/lists by element hashes in order.
   - Hash maps/sets by unordered combination (commutative).
2. Update `clorus_equals`:
   - For vectors/lists: element-wise equality.
   - For maps: count + key/value equality.
   - For sets: membership equality.
3. Fix map/set storage to avoid hash-only keys:
   - Change `ClorusHashMap.entries` to `HashMap<u64, Vec<(key, val)>>`.
   - On assoc: scan bucket for key equality; update or insert.
   - On get: scan bucket for key equality.
4. Fix set storage similarly:
   - `HashMap<u64, Vec<val>>` or `HashMap<u64, Vec<*mut Value>>`.
   - `contains` checks key equality.
5. Update `distinct`/`dedupe` to use new hash+equals.

**Files**
- `crates/clorus-runtime/src/value.rs`
- `crates/clorus-runtime/src/map.rs`
- `crates/clorus-runtime/src/set.rs`
- `crates/clorus-runtime/src/collections.rs`

**Tests**
- New runtime tests for hash collisions and equality.
- Add a test for structural equality of two equal-but-distinct vectors/maps.

---

## 3) `swap!` CAS Retry Loop

**Goal**  
Make `swap!` correct under contention.

**Steps**
1. In `ClorusAtom::swap`:
   - Loop: load current value.
   - Call function to get new value.
   - Attempt CAS with `compare_exchange`.
   - If CAS fails, release computed value and retry.
2. Ensure refcounting is correct:
   - Retain new value before CAS.
   - Release new value on failed CAS.
   - Release old value on successful CAS.

**Files**
- `crates/clorus-runtime/src/atom.rs`

**Tests**
- Multi-thread test: multiple threads `swap!` increment and assert final count.

---

## Optional Follow-Ons (Not in Top 3)

- `contains?` for maps/vectors.  
- Quote semantics: preserve lists; implement `~@` splicing.  
- Implement `update` runtime.
