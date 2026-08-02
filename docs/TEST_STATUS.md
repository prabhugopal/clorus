# Test Status

Live results from actual test runs on 2026-06-16.

## How to run

```bash
cargo test -p clorus-runtime
cargo test -p clorus-codegen
cargo test -p clorus-cli --lib
cargo test -p clorus --test value_star_test

CLORUS_ENTRY_FILE=tests/language/test-foo.clr target/debug/clorus run
./target/debug/clorus run tests/language/test-foo.clr
```

`clorus run <file>` now works directly for `.clr` / `.clrs` entry files.

## Rust test status

| Target | Result | Notes |
|---|---:|---|
| `clorus-runtime` | pass | runtime unit tests green |
| `clorus-codegen` | pass | codegen unit tests green |
| `clorus-cli --lib` | 222 pass, 0 fail | single-threaded and default parallel both green |
| `clorus --test value_star_test` | 4 pass, 0 fail | let-binding SIGSEGV fixed |

## Confirmed fixes from this pass

1. `dosync` retry loop is implemented.
2. `ref` / `ref-set` / `alter` / `ensure` surface works end-to-end.
3. `commute` now uses deferred replay semantics instead of the old eager `alter` path.
4. Persistent vector overflow beyond the `32^2` boundary is fixed.
5. `Value*` let-binding crash is fixed.
   - Root cause: local symbol loads incorrectly called `clorus_deref_var_value` on ordinary local values.
   - Fix: local symbol reads now return the local `Value*` directly; var auto-deref remains only for global var paths.
6. `clorus run <file>` now treats the first `.clr` / `.clrs` argument as an entry override instead of silently ignoring it.
7. Agent actions now support generic function values and 3+ extra args.
8. `alts!!` now waits on channel activity instead of spin-sleep polling.
9. Compile errors now include the source file and top-level form index in build and run paths.

## Language/runtime checks re-verified

| Test file | Result | Notes |
|---|---|---|
| `tests/language/test-completing.clr` | pass | works via direct `clorus run tests/language/test-completing.clr` |
| `tests/language/test-transducers.clr` | pass | JIT path previously verified during regression pass |
| `tests/language/test-refs-stm.clr` | pass | JIT + legacy verified after STM fixes |
| `tests/features/concurrency/refs/refs-stm-test.clr` | pass | JIT + legacy verified |
| `tests/features/concurrency/refs/stm.clr` | pass | JIT verified |
| `tests/stdlib/test-core-large-collections.clr` | pass | regression for vector overflow / large collection traversal |

## Remaining known issues worth keeping open

1. Tooling diagnostics
   - Compiler and CLI warnings are still noisy.
   - Source-location context is better, but broader error-shaping and AOT UX still need a dedicated pass.
2. Broader stdlib breadth
   - Core large-collection correctness is better now, but broader namespace parity still needs work.
3. Channel semantics depth
   - `alts!!` no longer spin-polls, but fairness and fuller core.async-grade semantics are still open.
