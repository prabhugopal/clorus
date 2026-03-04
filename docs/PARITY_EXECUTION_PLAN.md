# Parity Execution Plan

**Date:** March 3, 2026  
**Baseline:** `CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh`  
**Baseline Result:** `Passed: 132, Failed: 0, Skipped: 0`

---

## Objective

Reach high-confidence Clojure language parity (without JVM interop) while
keeping one maintainable implementation model:

- JIT-first execution path for `run` and `repl`
- legacy engine retained as a conformance/regression check
- test-first delivery for every parity change

---

## Operating Rules

1. No temporary fixes in language/runtime/compiler paths.
2. Every semantic change requires tests in both engines.
3. JIT and legacy output must match for shared semantics.
4. New behavior must be documented in `reference/LANGUAGE_SPEC.md` and this plan.

---

## Validation Commands

```bash
# Full parity gate
CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh

# Engine-isolated checks
CLORUS_TEST_ENGINES="jit" tests/run_all_tests.sh
CLORUS_TEST_ENGINES="legacy" tests/run_all_tests.sh

# Explicit binary (CI/local reproducibility)
CLORUS_BIN=./target/release/clorus CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh
```

---

## Milestones

## M0 - Locked Baseline (Done)

- Dual-engine suite green (132/132).
- JIT default behavior documented in README + tests docs.
- Existing breakages in integration fixtures cleaned.

## M1 - Semantic Parity Matrix (In Progress)

Deliverable: explicit feature-to-test map with pass status for both engines.

Coverage buckets:
- parser/syntax forms
- macro system
- function/value semantics
- control flow
- collections/data literal semantics
- stdlib behavior
- runtime behavior and memory safety invariants
- REPL semantics vs `run` semantics

## M2 - High-Risk Semantics Hardening

Focus areas:
- `binding` semantics parity (nested, multiple vars, side effects)
- closure capture + higher-order function edge cases
- transducer completion and reducer compatibility semantics
- error/exception behavior parity for supported subset

Requirement: each area gets dedicated language tests plus dual-engine pass.

## M3 - Runtime Integrity Hardening

Focus areas:
- value lifetime invariants under stress
- release/retain consistency checks in debug instrumentation
- reproducible crash harnesses for UI/event-driven workloads

Requirement: no known heap corruption or double-free repros under current
regression scenarios.

## M4 - REPL Production Semantics

Focus areas:
- REPL semantics match `run --jit` for eval/compiler path
- namespace/module loading consistency
- deterministic project loading and package resolution

Requirement: all REPL regression tests green and mirrored by `run` semantics
tests where applicable.

---

## Priority Gap Backlog (Next 10)

1. Formalize binding parity tests (`tests/language/` + integration scenarios).
2. Add macro edge-case parity tests (macroexpand recursion + hygiene edge cases).
3. Add more multi-arity/variadic closure capture tests.
4. Add transducer completion-arity and reducer-symbol compatibility tests.
5. Add try/throw behavior matrix for currently supported exception subset.
6. Add collection equality and sequence realization edge-case tests.
7. Add REPL-vs-run semantic parity tests for same input forms.
8. Add runtime stress test for retain/release on nested collection graphs.
9. Add parity tests for keyword/symbol/string conversion edges.
10. Add CI parity gate to run dual-engine test matrix on every PR.

---

## Ownership Checklist Per Feature

For each parity item:

1. Write failing test first.
2. Fix compiler/runtime/stdlib semantics.
3. Run single test on both engines.
4. Run full dual-engine suite.
5. Update docs (`reference/LANGUAGE_SPEC.md`, `LANGUAGE_PARITY.md` if needed).
6. Commit with feature + regression context.

---

## Definition of Done (for parity increments)

- New tests exist and fail before fix.
- Both engines pass the new tests.
- Full dual-engine suite passes.
- No debug-only env var required for normal execution.
- Documentation updated.
