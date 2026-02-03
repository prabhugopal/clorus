# 🔧 Clorus Build Scripts

Professional build scripts with test metrics and detailed reporting.

---

## 📜 Available Scripts

### 1. `build-compiler.sh` - Compiler & Runtime Build

Builds the Clorus compiler and runtime (all Rust crates).

**Usage:**
```bash
./scripts/build-compiler.sh [OPTIONS]

Options:
  --no-tests    Build without running Rust tests
  --release     Build in release mode (optimized)
  --verbose     Show detailed build output
  --help        Show help message
```

**Examples:**
```bash
# Debug build with tests (default)
./scripts/build-compiler.sh

# Release build with tests
./scripts/build-compiler.sh --release

# Fast build without tests
./scripts/build-compiler.sh --no-tests

# Verbose release build
./scripts/build-compiler.sh --release --verbose
```

**Output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Clorus Compiler/Runtime Build Script
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Build Mode: Debug (fast compile)
Tests: Enabled

[1/3] Building Clorus...
✓ Build successful

[2/3] Running Rust tests...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Test Summary:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Total:   42 tests
  Passed:  42
  Failed:  0
  ✓ All tests passed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[3/3] Build artifacts:
  Binary: target/debug/clorus (12M)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ Build Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Build time: 45s
  Mode: debug
  Tests: ✓ 42/42 passed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

### 2. `test-stdlib.sh` - Standard Library Tests

Runs all .clr test files in the tests/ directory and provides detailed metrics.

**Usage:**
```bash
./scripts/test-stdlib.sh [OPTIONS]

Options:
  --no-tests     Skip running tests (just discover)
  --verbose      Show detailed test output
  --pattern PAT  Run only tests matching pattern
  --help         Show help message
```

**Examples:**
```bash
# Run all stdlib tests
./scripts/test-stdlib.sh

# Run only concurrency tests
./scripts/test-stdlib.sh --pattern concurrency

# Run only lang tests with verbose output
./scripts/test-stdlib.sh --pattern lang --verbose

# Run tests matching "atom"
./scripts/test-stdlib.sh --pattern atom
```

**Output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Clorus Standard Library Test Script
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Binary: target/debug/clorus
Tests: Enabled
Pattern: *

[1/2] Discovering test files...
  Found 67 test files

[2/2] Running tests...

...........F...............F............................

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Test Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Total Files:    67
  Passed Files:   65
  Failed Files:   2
  Success Rate:   97%
  Test Time:      12s

Category Breakdown:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  concurrency:     12 passed,   0 failed  [100%]
  integration:      8 passed,   0 failed  [100%]
  lang:            25 passed,   1 failed  [ 96%]
  polymorphism:     7 passed,   0 failed  [100%]
  repl:             3 passed,   0 failed  [100%]
  stdlib:          10 passed,   1 failed  [ 91%]
  types:            0 passed,   0 failed  [  0%]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✗ Some tests failed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

### 3. `build-all.sh` - Full Build & Test

Runs both compiler build and stdlib tests in sequence.

**Usage:**
```bash
./scripts/build-all.sh [OPTIONS]

Options:
  --no-compiler-tests   Skip Rust/compiler tests
  --no-stdlib-tests     Skip stdlib .clr tests
  --no-tests            Skip all tests
  --release             Build in release mode
  --verbose             Show detailed output
  --help                Show help message
```

**Examples:**
```bash
# Full build and test suite (default)
./scripts/build-all.sh

# Release build with all tests
./scripts/build-all.sh --release

# Fast build without tests
./scripts/build-all.sh --no-tests

# Build with only Rust tests (skip stdlib)
./scripts/build-all.sh --no-stdlib-tests
```

**Output:**
```
╔══════════════════════════════════════════════════╗
║        Clorus Full Build & Test Suite           ║
╚══════════════════════════════════════════════════╝

[Step 1/2] Compiler & Runtime
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[... compiler build output ...]

[Step 2/2] Standard Library Tests
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[... stdlib test output ...]

╔══════════════════════════════════════════════════╗
║              Build Summary                       ║
╚══════════════════════════════════════════════════╝

  Compiler:       ✓ Success
  Stdlib Tests:   ✓ Success
  Total Time:     58s

✓ All builds and tests passed!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## 🎯 Common Workflows

### Development Workflow
```bash
# Quick iteration (no tests)
./scripts/build-compiler.sh --no-tests

# Full check before commit
./scripts/build-all.sh

# Test specific feature
./scripts/test-stdlib.sh --pattern atoms --verbose
```

### CI/CD Workflow
```bash
# Full build and test (fails on any error)
./scripts/build-all.sh --release
```

### Pre-Release Workflow
```bash
# Release build with all tests
./scripts/build-all.sh --release --verbose
```

---

## 📊 Metrics Provided

Each script provides detailed metrics:

### Compiler Build Metrics
- ✅ Build time
- ✅ Build mode (debug/release)
- ✅ Rust test count (total/passed/failed)
- ✅ Binary size
- ✅ Success/failure status

### Stdlib Test Metrics
- ✅ Total test files discovered
- ✅ Passed/failed counts
- ✅ Success rate percentage
- ✅ Test execution time
- ✅ Category breakdown (lang, stdlib, concurrency, etc.)
- ✅ Per-category success rates

### Full Build Metrics
- ✅ Combined metrics from both scripts
- ✅ Overall success/failure
- ✅ Total build + test time

---

## 🔍 Exit Codes

All scripts follow standard Unix conventions:

- **0**: Success (all tests passed)
- **1**: Failure (build failed or tests failed)

This makes them CI/CD friendly:

```bash
# CI script
./scripts/build-all.sh || exit 1
```

---

## 🎨 Output Features

- ✅ Colored output (green = success, red = failure, yellow = warning)
- ✅ Progress indicators (dots for tests)
- ✅ Category breakdown
- ✅ Professional formatting
- ✅ Clear success/failure messages
- ✅ Quick start instructions

---

## 🚀 Quick Reference

```bash
# Daily development
./scripts/build-compiler.sh              # Fast build with tests
./scripts/test-stdlib.sh --pattern lang  # Test specific area

# Before commit
./scripts/build-all.sh                   # Full validation

# Release preparation
./scripts/build-all.sh --release --verbose

# CI/CD
./scripts/build-all.sh --release || exit 1
```

---

**All scripts are in `/scripts/` and are executable. Run with `--help` for detailed usage!**
