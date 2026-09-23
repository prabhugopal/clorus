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

### P0-2 STM retry semantics — FIXED 2026-09-23
- Two distinct real bugs found and fixed in `crates/clorus-runtime/src/transaction.rs`, both required for correctness (fixing only one was not enough — proved by a failing stress test in between):
  1. `commit_transaction` computed a sorted `ref_ids` list (comment: "sorted by address to prevent deadlock") but never used it — validation, writes, and commutes each locked and released one ref's mutex per iteration, across three separate loops, instead of holding all of a transaction's locks for the whole critical section. Another thread's transaction could interleave between a ref being validated and a different ref in the same transaction being written.
  2. `Transaction::stage_write` unconditionally called `self.reads.remove(&ref_id)`. Since `alter` reads a ref (recording its version) then stages a write computed from that read, this silently deleted the only record commit-time validation had to check — meaning **conflict detection was completely disabled for the single most common STM pattern (read-then-write)**. This was the dominant bug: a 2-ref concurrent-transfer stress test showed the total drifting under contention with *zero* commit conflicts ever detected, which is what exposed it — bug #1 alone did not explain that.
- Verified with a new test, `transaction::tests::test_tx_concurrent_transfer_preserves_total`: 8 threads x 300 transfers between two refs, asserting the total is exactly preserved. Failed before the `stage_write` fix (drifted, zero retries observed — proving validation wasn't running), passes reliably after (run 6x, no flakes). Full test suite: 129/129, no regressions.
- Not yet done: the retry loop (`crates/clorus-codegen/src/codegen/mod.rs` `Expr::Dosync`) has no backoff, so heavy contention means CPU-spin rather than a crash — a liveness/perf concern, not a correctness one.
- Why first: current surface over-promises concurrency safety

### P0-3 REPL O(n²) recompilation
- Impact: `clorus repl` creates a fresh `CodeGen` per evaluated form instead of reusing one incrementally, so every eval recompiles the entire session history — REPL sessions on nontrivial projects slow down and can hang as history grows
- Primary area: `crates/clorus-repl/src/repl_engine.rs`
- Why first: directly undermines the REPL-driven workflow the language is designed around
- **The obvious fix is a trap, confirmed by direct implementation attempt (2026-09-23):** a "persistent `CodeGen`, keep growing one module, skip re-executing historical `def`/`defn`, clone the module + create a fresh JIT engine per eval" approach compiles and runs, but is **silently wrong**: `def` (`crates/clorus-codegen/src/codegen/mod.rs` ~line 1858) stores its value in a genuine LLVM `@global`, populated only when its wrapping function *executes*. A fresh `ExecutionEngine` created from a cloned module gets its own independent, null-initialized copy of every global — completely disconnected from whatever a previous eval's (now-discarded) engine wrote. Skipping re-execution of history means every reference to a previously-defined symbol silently evaluates to `nil` instead of erroring. Verified by direct REPL session testing (`(def x 10)` in one eval, `x` in the next → `nil` instead of `10`) before this was reverted, uncommitted.
- This is almost certainly why the 2026-02-15 orphaned commit (see git history / `orphaned-repl-incremental-fix` tag) was never merged despite its own status doc claiming success — its own testing only exercised self-contained expressions (`(+ 10 20)`) and it admits the one real multi-def project it tried "hangs during form loading," i.e. it likely never got far enough to hit this bug either.
- **A real fix needs one long-lived `ExecutionEngine` for the whole session** (not a fresh one per eval), with each new eval adding a small new module to it via incremental module-linking (verify inkwell/LLVM support for this), so previously-defined globals' memory is never discarded. This is a bigger change than it looks; budget accordingly and write a multi-eval cross-reference regression test (`(def x 10)` then a later eval reading `x`) before considering it done — clorus-repl currently has zero automated tests.

### P0-4 Compiler hangs instead of erroring on unrecognized/malformed syntax — FIXED 2026-09-23
- Root cause was in the lexer, not the parser: `crates/clorus-syntax/src/lexer.rs` `next_token()`'s catch-all branch fell through to `read_symbol()` for any unmatched character; `read_symbol()`'s allowed-character set excluded `\`/`|`/`$`/etc., so it read zero characters without advancing the cursor, and `tokenize()`'s loop spun forever re-tokenizing the same position
- The unbalanced-delimiter case (the other symptom `docs/issues/LANGUAGE_ISSUES.md` Issue #8 described) was already fixed separately — verified it errors cleanly today
- Fix: `next_token()` now returns an error when `read_symbol()` makes zero progress, instead of returning an empty-string token. Verified against `\`, `|`, `$` directly, and against the full test suite (129/129 passing)

> Note: an earlier version of this list included "nested multi-arity closure capture" as P0-1. Verified fixed by direct execution on 2026-09-23 (`completing`-style nested multi-arity closures and full `transduce`/`map` pipelines both run correctly) — removed from blockers below.

## Priority Roadmap

### Sprint 1: Correctness
- Rewrite recursive stdlib hot paths with `loop/recur`
- Fix `conj`-builder performance mistakes in stdlib
- Re-implement REPL incremental compilation (persistent `CodeGen`) to fix O(n²) recompilation
- Add regression tests for all of the above

### Sprint 2: Concurrency semantics
- ~~Implement STM retry/validation correctly~~ — done, see P0-2
- Add backoff to the `dosync` retry loop (currently an unbounded tight retry under contention)
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
- STM retry/validation correctness fixed (see P0-2); no backoff on retry yet
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
- AOT path is reliable enough for shipping binaries
- runtime/compiler diagnostics are materially better
- Rust interop covers more than bridge-crate basics
