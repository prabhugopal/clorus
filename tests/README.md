# Clorus Test Suite

## Structure

- **language/** - Language feature tests (conditionals, loops, functions, etc.)
- **compiler/** - Compiler-specific tests (parser, codegen, etc.)
- **stdlib/** - Standard library tests (core.clr, set.clr, lazy.clr)
- **integration/** - Integration and system tests (REPL, keyboard, etc.)

## Running Tests

```bash
# Run all Clorus language tests (both engines)
CLORUS_TEST_ENGINES="jit legacy" ./tests/run_all_tests.sh

# Run all Clorus tests (JIT only)
CLORUS_TEST_ENGINES="jit" ./tests/run_all_tests.sh

# Override clorus binary (useful in CI/local builds)
CLORUS_BIN=./target/release/clorus CLORUS_TEST_ENGINES="jit legacy" ./tests/run_all_tests.sh

# Legacy-only smoke run (optional)
CLORUS_TEST_ENGINES="legacy" ./tests/run_all_tests.sh

# Default run (uses script defaults)
./tests/run_all_tests.sh

# Run all tests (Rust + Clorus + REPL)
./scripts/test/run-all.sh

# Run specific test
clorus build tests/language/test-vectors.clr
./target/test-vectors
```

`clorus run` and `clorus repl` use JIT by default. The test matrix above keeps
legacy engine parity checked continuously.

## Writing Tests

Test files should be named `test-<feature>.clr` and placed in the
appropriate subdirectory based on what they're testing.
