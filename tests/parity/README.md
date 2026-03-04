# Core Parity Harness

This directory provides a small parity harness to compare Clorus results against
Clojure for pure `clojure.core`-style expressions.

## Run

```bash
python3 tests/parity/run_core_parity.py
```

Optional:

```bash
CLORUS_BIN=./target/release/clorus python3 tests/parity/run_core_parity.py
```

## Scope

- Uses `tests/parity/core_cases.json` as the source of expressions.
- Runs each expression in:
  - Clojure (`clj`)
  - Clorus (`clorus run`)
- Compares printed value equality (`pr-str` in Clojure vs `=>` value from Clorus).

## Notes

- This is intentionally a smoke set; add cases incrementally.
- Exclude JVM interop and host-specific behavior from this harness.
