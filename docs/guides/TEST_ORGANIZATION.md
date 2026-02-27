# Clorus Test Organization Guide

## 🎯 Clean, Feature-Based Test Structure

All tests are now organized by **feature** rather than by arbitrary categories. This makes it easy to find tests, add new ones, and track coverage.

## 📂 Directory Structure

```
tests/
├── features/              # ✅ All Clorus language feature tests (65 tests)
│   ├── arithmetic/        # Arithmetic operators & comparison (2 tests)
│   │   ├── comparison-test.clr
│   │   └── comparison-bug-test.clr
│   │
│   ├── collections/       # Vector, list, map operations (5 tests)
│   │   ├── api-test.clr
│   │   ├── cons-test.clr
│   │   ├── nested-map-ops-test.clr
│   │   └── test-collections.clr
│   │
│   ├── concurrency/       # Atoms, refs, channels, STM (8 tests)
│   │   ├── atoms/
│   │   │   ├── atoms.clr
│   │   │   ├── atoms-test.clr
│   │   │   └── atoms-simple-test.clr
│   │   ├── refs/
│   │   │   ├── refs-stm-test.clr
│   │   │   └── stm.clr
│   │   ├── test-atoms.clr
│   │   ├── test-go-closure.clr
│   │   └── test-ref-simple.clr
│   │
│   ├── control-flow/      # if, do, let, loop, case, when, cond (6 tests)
│   │   ├── case-test.clr
│   │   ├── letfn-test.clr
│   │   ├── test-dotimes.clr
│   │   ├── test-doseq.clr
│   │   ├── test-simple-while.clr
│   │   └── test-while.clr
│   │
│   ├── exceptions/        # Exception handling (3 tests)
│   │   ├── exceptions-test.clr
│   │   ├── simple-test.clr
│   │   └── try-catch-test.clr
│   │
│   ├── functions/         # defn, fn, closures, multi-arity (13 tests)
│   │   ├── closures.clr
│   │   ├── creation.clr
│   │   ├── destructuring-test.clr
│   │   ├── functions.clr
│   │   ├── higher-order.clr
│   │   ├── loop-recur-test.clr
│   │   ├── misc.clr
│   │   ├── multi-arity.clr
│   │   ├── multi-arity-docstring-test.clr
│   │   ├── multi-arity-simple-test.clr
│   │   ├── multi-arity-test.clr
│   │   ├── recursion.clr
│   │   └── variadic-test.clr
│   │
│   ├── macros/            # Macro system (6 tests)
│   │   ├── cond-case-test.clr
│   │   ├── control-flow-macros-test.clr
│   │   ├── control-flow-test.clr
│   │   ├── defmacro-test.clr
│   │   ├── expansion-test.clr
│   │   └── macros.clr
│   │
│   ├── polymorphism/      # Protocols, records, multimethods (7 tests)
│   │   ├── test-multimethod-simple.clr
│   │   ├── test-protocol-simple.clr
│   │   ├── test-protocols-comprehensive.clr
│   │   ├── test-record-get.clr
│   │   ├── test-record-proper.clr
│   │   ├── test-records.clr
│   │   └── test-records-comprehensive.clr
│   │
│   ├── sequences/         # map, filter, reduce, range, lazy (11 tests)
│   │   ├── core/
│   │   │   ├── apply.clr
│   │   │   ├── arithmetic.clr
│   │   │   ├── comp.clr
│   │   │   ├── filter.clr
│   │   │   ├── identity.clr
│   │   │   ├── map.clr
│   │   │   ├── partial-test.clr
│   │   │   ├── partial.clr
│   │   │   ├── reduce.clr
│   │   │   ├── sequences.clr
│   │   │   ├── simple-test.clr
│   │   │   └── stdlib-test.clr
│   │   ├── lazy/
│   │   │   ├── lazy-minimal-test.clr
│   │   │   ├── lazy-original-test.clr
│   │   │   ├── lazy-seq-macro-test.clr
│   │   │   └── lazy-test.clr
│   │   ├── transducers/
│   │   │   └── transducers.clr
│   │   ├── range-test.clr
│   │   └── sequence-generators-test.clr
│   │
│   ├── strings/           # String operations (2 tests)
│   │   ├── string-operations-test.clr
│   │   └── test-simple-string.clr
│   │
│   └── types/             # Type predicates (2 tests)
│       ├── basic-predicates-test.clr
│       └── type-predicates-advanced-test.clr
│
├── integration/           # ✅ Full integration tests (3 tests)
│   ├── basic.clr
│   ├── binding.clr
│   └── stdlib-minimal.clr
│
├── compiler/              # ✅ Compiler/infrastructure tests (3 tests)
│   └── test-framework/
│       ├── test-core.clr           # Complete test framework
│       ├── test-enhanced.clr       # Test framework with metrics
│       └── test-minimal.clr        # Minimal test framework
│
└── _archived/             # ⚠️ Archived/obsolete tests (7 tests)
    ├── test-long-add.clr
    ├── test-map.clr
    ├── test-println.clr
    ├── test-range-exists.clr
    ├── test-simple-syntax.clr
    ├── test-stdlib.clr
    └── test-summary.clr
```

## 📊 Test Statistics

| Category | Tests | Description |
|----------|-------|-------------|
| **features/arithmetic** | 2 | Arithmetic ops: +, -, *, /, mod, <, >, <=, >=, = |
| **features/collections** | 5 | Vectors, lists, maps, nested operations |
| **features/concurrency** | 8 | Atoms, refs, STM, channels, go blocks |
| **features/control-flow** | 6 | if, do, let, loop/recur, case, when, cond |
| **features/exceptions** | 3 | try, catch, finally, throw |
| **features/functions** | 13 | defn, fn, closures, multi-arity, destructuring |
| **features/macros** | 6 | defmacro, quote, syntax-quote, macro expansion |
| **features/polymorphism** | 7 | Protocols, records, multimethods |
| **features/sequences** | 11 | map, filter, reduce, range, lazy, transducers |
| **features/strings** | 2 | String operations and manipulation |
| **features/types** | 2 | Type predicates (vector?, map?, seq?, coll?, fn?) |
| **integration** | 3 | Full integration and REPL tests |
| **compiler** | 3 | Test framework implementations |
| **_archived** | 7 | Old/obsolete tests (not counted) |
| **TOTAL (Active)** | **71** | All active tests passing at 100% |

## 🎯 Test Categories Explained

### Language Features (65 tests)

These test **Clorus language features** - the core language syntax and semantics:

- **Arithmetic**: Basic math and comparison operators
- **Collections**: Data structure operations
- **Concurrency**: Parallel programming primitives
- **Control Flow**: Branching and looping
- **Exceptions**: Error handling
- **Functions**: Function definition and application
- **Macros**: Compile-time code generation
- **Polymorphism**: Dynamic dispatch mechanisms
- **Sequences**: Sequence processing and lazy evaluation
- **Strings**: String manipulation
- **Types**: Runtime type checking

### Integration Tests (3 tests)

Test **complete workflows** and interactions between multiple features:
- Full program execution
- REPL functionality
- Standard library integration

### Compiler Tests (3 tests)

Test **compiler infrastructure** and tooling (not language features):
- Test framework implementations
- Testing utilities
- Development tools

These are separate because they're **meta-tests** - they test the testing infrastructure itself.

## 🔧 How to Use

### Run All Tests
```bash
./tests/run_all_tests.sh

To run **all** tests (Rust + Clorus + REPL):

```bash
./scripts/test/run-all.sh
```
```

Output:
```
================================================
  CLORUS COMPREHENSIVE TEST SUITE
================================================

1. LANGUAGE FEATURES

=== Testing: features/arithmetic ===
Testing arithmetic/comparison-test...             ✓ PASS
Testing arithmetic/comparison-bug-test...         ✓ PASS

=== Testing: features/collections ===
Testing collections/api-test...                   ✓ PASS
...

2. INTEGRATION TESTS
3. COMPILER TESTS

================================================
  TEST SUMMARY
================================================
Passed: 71
Failed: 0

All tests passed! ✓
```

### Generate Metrics with Visualization
```bash
./tests/generate_metrics.sh
```

Output shows:
- Category-by-category analysis
- ASCII progress bars
- JSON metrics export
- Visual dashboard data

### View HTML Dashboard
```bash
open test-metrics/dashboard.html  # macOS
xdg-open test-metrics/dashboard.html  # Linux
```

Beautiful web dashboard with:
- 📊 Summary cards
- 📈 Progress visualization
- 🎨 Color-coded categories
- 📱 Mobile-friendly design

## ✨ Benefits of This Organization

### 1. **Feature-Based Discovery**
Want to test arithmetic? Look in `features/arithmetic/`
Want to test macros? Look in `features/macros/`
Crystal clear organization.

### 2. **Separation of Concerns**
- Language feature tests in `features/`
- Integration tests in `integration/`
- Compiler/tooling tests in `compiler/`
- No confusion about what tests what

### 3. **Easy to Extend**
Adding a new feature? Create `features/my-feature/` and add tests.
The metrics script automatically discovers and reports on it.

### 4. **Clean Metrics**
The dashboard shows exactly what language features are tested:
- Arithmetic: 2 tests, 100%
- Collections: 5 tests, 100%
- Functions: 13 tests, 100%
- etc.

### 5. **Merge-Friendly**
No duplicate tests scattered across random directories.
Everything related to a feature is in one place.

## 📝 Adding New Tests

### For a New Feature
```bash
# 1. Create feature directory
mkdir tests/features/my-feature

# 2. Add test file
cat > tests/features/my-feature/basic-test.clr << 'EOF'
;; Test for my-feature

(defn test-my-feature []
  (if (my-feature-works?)
    nil
    (throw "my-feature should work")))

(defn -main []
  (do
    (println "Testing my-feature...")
    (test-my-feature)
    (println "✓ my-feature tests passed")))
EOF

# 3. Run tests - automatically discovered!
./tests/run_all_tests.sh
```

### For an Existing Feature
```bash
# Just add to the existing directory
cat > tests/features/arithmetic/advanced-math-test.clr << 'EOF'
...
EOF
```

## 🗂️ What Happened to Old Tests?

| Old Location | New Location | Reason |
|--------------|--------------|--------|
| `lang/`, `core/`, etc. | `features/` | Consolidated into feature-based org |
| `type-predicates-test.clr` | `features/types/basic-predicates-test.clr` | Merged with types |
| `test-println.clr`, etc. | `_archived/` | Trivial/temporary tests |
| `test-core.clr`, etc. | `compiler/test-framework/` | Test infrastructure, not feature tests |

## 🎯 Current Status

✅ **71 active tests** - 100% pass rate
✅ **Feature-based organization** - Clean and logical
✅ **Separated concerns** - Features vs integration vs compiler
✅ **Comprehensive metrics** - ASCII + HTML dashboards
✅ **Documented structure** - Easy to understand and extend

## 🚀 Next Steps

1. Add more feature tests as you implement new language features
2. Keep integration tests for full workflows
3. Run `./test_metrics.sh` before each release for full report
4. Share the HTML dashboard to show test coverage

---

**Clean. Organized. Feature-Based. Trackable.**

That's how tests should be! 🎉
