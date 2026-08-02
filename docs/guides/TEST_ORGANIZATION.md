# Test Organization

There is one test tree: `tests/`. It has five suites, all run by
[`tests/run_all_tests.sh`](../../tests/run_all_tests.sh):

| Suite | Contents |
|---|---|
| `tests/language/` | Core language: special forms, macros, destructuring, protocols, concurrency, error handling. |
| `tests/stdlib/` | `stdlib/clorus/core.clr` and friends: sequence/collection/string/math functions. |
| `tests/compiler/` | Compiler/codegen edge cases, including sub-project tests (`Clorus.toml` + `src/`) that exercise a full build. |
| `tests/integration/` | End-to-end project builds. |
| `tests/parity/` | Differential harness against a real Clojure install (`run_core_parity.py`) — optional, requires `clj` locally, not part of the standard sweep. |

Rust unit/integration tests live alongside the code they test, under each
crate's `src/` (`#[cfg(test)] mod tests`), and run via `cargo test --workspace`.

## Running

```bash
./tests/run_all_tests.sh          # full .clr sweep, JIT + AOT where applicable
cargo test --workspace            # Rust unit/integration tests
python3 tests/parity/run_core_parity.py   # optional, needs local `clj`
```

The runner accumulates failures instead of stopping at the first one, and
prints a summary at the end — treat that summary, not any status doc, as the
current pass/fail count.

There used to be a second, parallel `tests/features/` tree and a
`tests/_archived/` tree. Both were dead (unreferenced by any runner,
duplicating `tests/language/`) and have been removed — if you're looking for
a test by a name that doesn't exist under the suites above, it isn't run
anywhere and should be added under `tests/language/` or `tests/stdlib/`
instead of resurrected.
