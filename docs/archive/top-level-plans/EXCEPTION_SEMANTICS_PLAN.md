# Exception Semantics Plan

**Status:** Planned  
**Scope:** Language-level `throw` / `try` / `catch` / `finally` semantics (no JVM interop)

## Problem

Current state:
- `try/catch/finally` syntax parses.
- `throw` currently uses foreign exception ABI behavior and can abort.
- `catch` does not provide reliable runtime catching semantics for thrown paths.

Goal:
- Deterministic, safe language exceptions in both `jit` and `legacy` engines.

## Target Semantics

1. `(throw x)` produces a language exception carrying payload `x`.
2. `(try expr (catch e handler))` catches thrown language exceptions from `expr`.
3. `(try expr (catch Type e handler))` supports typed catches (initially optional type-match policy).
4. `finally` always runs exactly once on both success and throw paths.
5. If uncaught, exception is surfaced as runtime error (not heap corruption / abort trap).

## Implementation Strategy

1. Runtime model:
   - Introduce explicit exception object representation.
   - Provide safe construction, payload access, and type predicate.
2. Codegen model:
   - Avoid foreign unwinding path for language exceptions.
   - Represent propagation via explicit value/control flow (engine-consistent).
   - Make `try` branch correctly between success and exception paths.
3. Engine parity:
   - JIT and legacy must implement the same semantics.
4. Error boundary:
   - CLI/REPL top-level catches uncaught language exception and prints payload.

## Test-First Sequence

1. `throw-catch-basic`:
   - direct throw + catch payload.
2. `throw-through-call`:
   - exception thrown in called function, caught by caller.
3. `finally-order`:
   - finally executes on success and throw.
4. `rethrow`:
   - catch handler may rethrow.
5. `uncaught-surface`:
   - uncaught exception produces stable runtime error path.

## Safety Constraints

- No temporary env-var-only behavior.
- No reintroduction of known crash patterns (SIGSEGV / abort trap).
- Full `CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh` must stay green.

## Rollout

Phase A:
- Add tests under `tests/features/exceptions/` (manual run only initially).

Phase B:
- Implement runtime + codegen behavior to pass tests.

Phase C:
- Move stable exception tests into main language/integration suite.
