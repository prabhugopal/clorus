# Core Parity Harness

This directory provides a parity harness to compare pure `clojure.core`-style
expressions across Clojure, Clorus JIT, and Clorus AOT.

## Run

```bash
python3 tests/parity/run_core_parity.py
```

Optional:

```bash
CLORUS_BIN=./target/release/clorus python3 tests/parity/run_core_parity.py
```

Run one Clorus engine while investigating a failure:

```bash
python3 tests/parity/run_core_parity.py --engines jit
python3 tests/parity/run_core_parity.py --engines aot
```

## Scope

- Uses `tests/parity/core_cases.json` as the source of expressions.
- Runs each expression in:
  - Clojure (`clj`)
  - Clorus JIT (`clorus run`)
  - Clorus AOT (`clorus run --legacy-run`)
- Compares printed value equality (`pr-str` in Clojure vs `=>` value from Clorus).

## Notes

- This is intentionally a pure-expression contract suite; add cases
  incrementally. Namespace/module and exception behavior belongs in the
  dual-engine `.clr` integration tests because those cases need files and
  process-level setup.
- Exclude JVM interop and host-specific behavior from this harness.
