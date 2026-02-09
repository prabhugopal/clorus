# Clorus Test Suite

## Structure

- **language/** - Language feature tests (conditionals, loops, functions, etc.)
- **compiler/** - Compiler-specific tests (parser, codegen, etc.)
- **stdlib/** - Standard library tests (core.clr, set.clr, lazy.clr)
- **integration/** - Integration and system tests (REPL, keyboard, etc.)

## Running Tests

```bash
# Run all tests
cd ..
./scripts/test/run_all_tests.sh

# Run specific test
clorus build tests/language/test-vectors.clr
./target/test-vectors
```

## Writing Tests

Test files should be named `test-<feature>.clr` and placed in the
appropriate subdirectory based on what they're testing.
