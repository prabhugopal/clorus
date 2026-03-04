# Test Status

Canonical test entry points:

- Full language/runtime/compiler/integration sweep:
  - `tests/run_all_tests.sh`
- Clojure parity smoke set:
  - `tests/parity/run_core_parity.py`

Current policy:

- Do not keep disabled/temporary test files in active test paths.
- Replace flaky tests with deterministic coverage before removing originals.
- Any removed test must be mentioned in commit message with replacement test(s).
