# Clorus Production Plan

> Status: planning document, not the canonical status source.
> Canonical status lives in `docs/generated/PARITY_STATUS.md` and `docs/generated/STATUS.json`.
> Any percentages or completion scores here are planning estimates, not measured facts.

## Purpose

This is the single planning document for Clorus production-readiness work.

Use it for:
- priority order
- execution roadmap
- architecture risks
- effort estimates
- gap ownership by subsystem

Do not use it as the source of truth for current status. For that, use:
- `docs/generated/PARITY_STATUS.md`
- `docs/generated/STATUS.json`

## Current Position

Planning estimate:

| Domain | Done | Remaining | Notes |
|---|---:|---:|---|
| Core Language | 82% | 18% | Close to strong parity; a few real blockers remain |
| Standard Library | 58% | 42% | Broad surface exists; correctness and scale need work |
| Concurrency | 65% | 35% | Atoms/vars/channels baseline exists; STM/core.async depth missing |
| Rust Interop | 45% | 55% | Bridge path is real; type/model depth is the gap |
| Tooling / DX | 32% | 68% | Weakest area; AOT/errors/editor story still behind |
| Overall | 61% | 39% | Good foundation, not production-complete |

## What Is Already Strong

- Core forms, macros, metadata, destructuring, protocols, multimethods, hierarchy
- JIT execution path and REPL-driven workflow
- Substantial stdlib core and collection operations
- Real Rust interop foundation with package/build integration
- Deterministic RC-based runtime model
- Better concurrency foundation than most non-JVM Clojure implementations

## Production Blockers First

These are the top items to close before broader feature expansion.

### P0-1 Recursive stdlib stack overflow / quadratic builders
- Impact: `map`, `filter`, `take`, `drop`, `range`, `repeat`, `reverse`, and related sequence builders fail or scale badly on large inputs
- Primary area: `stdlib/clorus/core.clr`
- Why first: correctness bug at production scale

### P0-2 STM retry semantics
- Impact: `dosync` / `alter` / `commute` are not yet trustworthy under real contention
- Primary areas:
  - `crates/clorus-runtime/src/transaction.rs`
  - `crates/clorus-runtime/src/ref_type.rs`
- Why first: current surface over-promises concurrency safety

### P0-3 REPL O(n²) recompilation
- Impact: `clorus repl` creates a fresh `CodeGen` per evaluated form instead of reusing one incrementally, so every eval recompiles the entire session history — REPL sessions on nontrivial projects slow down and can hang as history grows
- Primary area: `crates/clorus-repl/src/repl_engine.rs`
- Why first: directly undermines the REPL-driven workflow the language is designed around; a fix for this existed once (2026-02-15) but was never merged into the main development line and needs to be re-implemented against the current REPL architecture

### P0-4 Compiler hangs instead of erroring on unrecognized/malformed syntax
- Impact: a parse error (mismatched delimiters, unsupported reader syntax such as a `\a` char literal) causes `clorus build`/`clorus run` to hang indefinitely instead of reporting an error and exiting non-zero; requires force-killing the process
- Primary area: `crates/clorus-syntax/src/parser.rs` (and/or the compiler driver in `crates/clorus-cli`)
- Why first: silent hangs are far worse for adoption than a missing feature erroring cleanly; verified reproducible by direct execution on 2026-09-23 (`(println \a)` hangs past a 20s timeout); previously logged in `docs/issues/LANGUAGE_ISSUES.md` Issue #8, never fixed

> Note: an earlier version of this list included "nested multi-arity closure capture" as P0-1. Verified fixed by direct execution on 2026-09-23 (`completing`-style nested multi-arity closures and full `transduce`/`map` pipelines both run correctly) — removed from blockers below.

## Priority Roadmap

### Sprint 1: Correctness
- Rewrite recursive stdlib hot paths with `loop/recur`
- Fix `conj`-builder performance mistakes in stdlib
- Re-implement REPL incremental compilation (persistent `CodeGen`) to fix O(n²) recompilation
- Add regression tests for all of the above

### Sprint 2: Concurrency semantics
- Implement STM retry/validation correctly
- Harden `alter` / `commute` / `ensure`
- Add concurrency stress tests
- Decide the minimal supported async model explicitly

### Sprint 3: Missing core language/platform pieces
- Character type
- `delay` / `force`
- `future`
- `promise` / `deliver`
- `volatile!`
- namespace/macro long-tail fixes

### Sprint 4: Stdlib breadth
- `clojure.string`
- `clojure.walk`
- `clojure.data`
- EDN roundtrip (`pr-str`, `edn/read-string`)
- `memoize`
- sort/transient/sorted collection follow-up if still open

### Sprint 5: Tooling and DX
- AOT standalone binary path
- source-location error reporting
- package caching / module caching
- CLI/project scaffold cleanup
- editor/LSP strategy

### Sprint 6: Rust interop depth
- richer struct/impl bridging
- generic/container signatures
- `Result` / `Option` mapping policy
- async/native interop model
- better interface generation and diagnostics

## Domain Gaps

### Core Language
Still missing or incomplete:
- syntax-quote namespace preservation edge cases
- character type
- ratio literals
- BigInt / BigDecimal
- `delay` / `future` / `promise` / `volatile!`
- deeper macro hygiene edge cases
- namespace long-tail ergonomics

### Standard Library
Main issues:
- recursion-based implementations that fail at scale
- missing namespaces: `clojure.string`, `clojure.walk`, `clojure.data`
- missing EDN roundtrip and broader serialization story
- more scale/correctness than breadth in some areas

### Concurrency
Main issues:
- STM semantics not complete enough yet
- no complete `core.async`-grade model
- no fully specified `go`/parking/alts story
- futures/promises/delays absent

### Rust Interop
Main issues:
- primitive/function path is solid
- richer host-model interop is still shallow
- more work needed for structs, impl methods, generics, container/result types

### Tooling / DX
Main issues:
- AOT and binary production path
- better compiler/runtime diagnostics
- source-location fidelity
- editor integration, formatting, linting, debugger story

## Architecture Risks

### Risk 1: Codegen concentration
- `crates/clorus-codegen/src/codegen/mod.rs` is too large
- This increases change risk and slows feature work
- Recommendation: split by language surface after the P0 correctness work

### Risk 2: Stdlib correctness hidden by small tests
- Small collection tests can mask recursion/scaling failures
- Recommendation: add large-input regression suite

### Risk 3: Concurrency surface ahead of semantics
- Surface APIs exist, but semantics are not equally hardened
- Recommendation: either harden or narrow claims immediately

### Risk 4: Status drift between narrative docs and code
- Recommendation: keep generated status canonical and keep this file planning-only

## How To Use This Doc

When choosing work:
1. verify current status in `docs/generated/PARITY_STATUS.md`
2. pick the next item from `Production Blockers First`
3. implement with tests
4. regenerate status docs
5. update this planning doc only if priority/order changed

## Success Criteria

Clorus is ready to claim serious production direction when:
- stdlib hot paths scale to large collections
- STM semantics are trustworthy
- AOT path is reliable enough for shipping binaries
- runtime/compiler diagnostics are materially better
- Rust interop covers more than bridge-crate basics
