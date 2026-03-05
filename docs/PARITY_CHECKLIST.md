# Clorus Clojure Parity Checklist

Last updated: 2026-03-05
Validation baseline: `CLORUS_BIN=./target/debug/clorus CLORUS_TEST_ENGINES="jit legacy" CLORUS_TEST_JOBS=2 tests/run_all_tests.sh` -> Passed 241, Failed 0, Skipped 1.

## How to use this checklist
- `✅ Paired`: implemented and covered by tests in both `jit` and `legacy`.
- `🟡 Partial`: implemented but with known semantic or coverage gaps.
- `❌ Missing`: not implemented (or intentionally out of scope for now).

## Core Language
- ✅ Numbers, strings, keywords, symbols, bool, nil
- ✅ `def`, `fn`, `defn`, multi-arity fns, closures
- ✅ `let`, `loop/recur`
- ✅ Collections: vector, map, set basics
- ✅ Metadata primitives (`meta`, `with-meta`, `vary-meta`) + def/defn metadata forms
- ✅ Exceptions: `throw`, `try/catch` (typed + catch-all)
- ✅ Protocol/multimethod baseline (`defprotocol`, `defmulti`, `defmethod`, dispatch)
- ✅ Hierarchy + introspection (`derive`, `isa?`, `parents`, `ancestors`, `descendants`, `methods`, `get-method`, `prefers`, `satisfies?`, `extends?`, `implements?`)

## Runtime / Execution
- ✅ `run` and `repl` support JIT path and parity-tested against legacy path
- ✅ Semantics suite executes both engines (`CLORUS_TEST_ENGINES="jit legacy"`)
- 🟡 AOT path exists but needs production hardening checklist (separate track)

## Clojure-Parity Gaps (No Java Interop)
- 🟡 Reader parity (reader-discard `#_` + regex reader literal `#\"...\"` baseline implemented; remaining reader forms and deeper regex semantics)
- 🟡 Macro tooling parity (`&env`/`&form` + auto-gensym hygiene baseline implemented; deeper hygiene edge cases remain)
- 🟡 Namespace ergonomics parity (baseline `:refer`/`:rename` implemented; alias edge cases remain)
- 🟡 Dynamic vars parity (`binding` + `set!` baseline covered; deeper semantics still open)
- 🟡 Data/collection API parity long tail (core edge/error paths mostly closed; remaining deep semantics focus on sort/comparator behavior and selected long-tail contracts)
- 🟡 Exception data APIs parity (`ex-info`, `ex-data`) deep behavior checks

## Stdlib Parity Tracking
- Source-of-truth matrix: `docs/STDLIB_PARITY_MATRIX.md`

## Explicitly Out of Scope for this parity target
- ❌ JVM/Java interop

## Next Milestone Checklist (Recommended order)
- [x] Freeze this file as source-of-truth for parity status updates.
- [x] Add missing parity tests first (before new runtime/compiler work) for:
  - [x] reader discard + trailing-discard edges
  - [x] macroexpand/gensym behavior
  - [x] namespace options (`:refer`, `:rename`)
  - [x] dynamic var semantics edge cases (`binding`, nested restore, `set!` in dynamic frame)
- [x] Implement only failing parity cases revealed by those tests.
- [x] Keep `jit` and `legacy` both green for every parity PR.
- [ ] When legacy path is retired, keep one semantic matrix runner that compares modes where still relevant.

## Active Execution Queue (Feature-by-Feature)
- [x] Reader parity: cover supported non-`#_` reader forms (quote, syntax-quote, unquote, unquote-splicing error paths, deref, var-quote)
- [x] Reader parity: regex literal lowering baseline (`#\"...\"` -> `re-pattern`)
- [x] Macro tooling parity: add `&env` tests
- [x] Macro tooling parity: add `&form` tests
- [x] Macro hygiene edge cases: test + baseline fix (auto-gensym in syntax-quote)
- [x] Namespace ergonomics: alias/refer/rename edge-case matrix
  - [x] local shadowing vs `:refer`/`:rename` (alias and full-qualified access remain available)
  - [x] alias/import collision detection with deterministic compiler errors
  - [x] remaining alias/refer/rename collision and error-path cases
- [x] Dynamic vars: finish `set!` semantics and edge coverage
- [x] Dynamic vars: nested restoration and function-boundary behavior
- [x] Dynamic vars: throw/rethrow restoration paths
- [x] Collections parity: `get-in` deep edge matrix
- [x] Collections parity: `assoc-in` deep edge matrix
- [x] Collections parity: `update-in` deep edge matrix
- [x] Exception data parity: `ex-info` behavior matrix
- [x] Exception data parity: `ex-data` propagation/rethrow baseline
- [x] Exception data parity: nested cause chain (`ex-cause`) preservation on rethrow
- [x] Transducer parity: `completing` + `transduce` completion-arity behavior
- [x] Stdlib compile-arity error baseline (`update` wrong-arity compile path)
- [x] Re-run full `jit+legacy` suite and refresh baseline counts

## Step 1 Closure (Language Parity Stabilization)
- Status: ✅ Complete (strict closure met).
- Strict exit criteria met:
  - `jit` + `legacy` both green on full suite (`Passed 183, Failed 0, Skipped 1`).
  - Reader parity matrix completed for supported reader macros (not only `#_`).
  - Dynamic-var semantics matrix completed (nested/function/throw + edge behavior).
  - Exception data matrix completed (`ex-info`/`ex-data` propagation + rethrow paths).
  - Namespace resolution/error-path matrix completed (`:require`, `:refer`, `:rename`, alias/import collisions).
  - Latest suite baseline after closure work: `Passed 205, Failed 0, Skipped 1`.
