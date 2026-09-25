# Refactor Plan: Maintainability and Smaller Files

Goal: reduce file size, improve readability, and make changes safer with clear module boundaries. This plan is code-only and aims to preserve behavior.

## Principles
- Keep each module focused on a single concern.
- Minimize cyclic dependencies.
- Keep public interfaces stable.
- Avoid large match blocks in one file.

## Phase 1: Codegen Split

**Current pain**  
`crates/clorus-codegen/src/codegen.rs` is monolithic and difficult to change safely.

**Target structure**
- `crates/clorus-codegen/src/codegen/mod.rs` (new entry that re-exports)
- `crates/clorus-codegen/src/codegen/expr.rs`
  - `compile_expr`, `compile_quoted`, `compile_syntax_quoted*`
- `crates/clorus-codegen/src/codegen/special_forms.rs`
  - `def`, `defn`, `fn`, `let`, `letfn`, `if`, `do`, `loop`, `recur`, `try`, `throw`, `dosync`
- `crates/clorus-codegen/src/codegen/builtins.rs`
  - Builtin dispatch (`core_functions` table and `compile_core_call`)
- `crates/clorus-codegen/src/codegen/ffi.rs`
  - Rust FFI calls and declarations
- `crates/clorus-codegen/src/codegen/namespace.rs`
  - `ns`, `require`, `use`, alias/refs tracking
- `crates/clorus-codegen/src/codegen/closures.rs`
  - `find_free_variables`, environment capture, closure creation

**Steps**
1. Move helper functions into new modules with `pub(crate)` visibility.
2. Keep `CodeGen` struct in `codegen/mod.rs` and `impl CodeGen` split across modules.
3. Update `lib.rs` to re-export `CodeGen` from new `codegen/mod.rs`.

**Risks**
- Minor compile errors during moves. Keep behavior identical.

---

## Phase 2: Runtime Collections Split

**Current pain**  
`crates/clorus-runtime/src/collections.rs` is large and mixes unrelated functionality.

**Target structure**
- `crates/clorus-runtime/src/collections/mod.rs`
  - Expose stable C ABI functions
- `crates/clorus-runtime/src/collections/seq.rs`
  - `nth`, `first`, `rest`, `last`, `count`
- `crates/clorus-runtime/src/collections/map_ops.rs`
  - `get-in`, `assoc-in`, `map_dissoc`, `keys`, `vals`, `merge`
- `crates/clorus-runtime/src/collections/set_ops.rs`
  - `distinct`, `dedupe`
- `crates/clorus-runtime/src/collections/concat.rs`
  - `concat`, `interleave`, `interpose`, `flatten`

**Steps**
1. Move functions into the new submodules.
2. Re-export `extern "C"` symbols from `collections/mod.rs`.
3. Keep function names and signatures unchanged to avoid breaking codegen.

---

## Phase 3: Runtime Hash/Equality Utilities

**Current pain**  
`value.rs` is growing with hash/equality logic.

**Target structure**
- `crates/clorus-runtime/src/hash.rs`
  - `clorus_hash`, `hash_combine`, structural hashing helpers
- `crates/clorus-runtime/src/value.rs`
  - Keep `clorus_equals`, but move hashing helpers out

**Steps**
1. Move `clorus_hash` and `hash_combine` to `hash.rs`.
2. Update `map.rs`, `set.rs`, and `collections.rs` to call `crate::hash::clorus_hash`.

---

## Phase 4: Stdlib Cleanup

**Current pain**  
`stdlib/core.clr` contains stubs and duplicates that drift from runtime behavior.

**Steps**
1. Remove duplicate predicates (`odd?`, `even?`, etc.).
2. Replace stubs (`string?`, `map?`, `vector?`, etc.) with calls to builtins.
3. Remove stdlib `conj` or align it with runtime `conj` semantics.

---

## Optional: REPL Debug Gating

**Current pain**  
Unconditional `eprintln!` makes REPL noisy.

**Steps**
1. Add a `debug` flag to `ReplEngine`.
2. Gate all debug prints behind that flag.

---

## Execution Approach

1. Apply Phase 1 and Phase 2 first (largest file reductions).
2. Run tests (compiler + runtime tests).
3. Apply Phase 3 and Phase 4.

If you want, I can start Phase 1 immediately.
