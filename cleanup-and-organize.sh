#\!/bin/bash
# cleanup-and-organize.sh

set -e

echo "🧹 Starting Clorus codebase cleanup and organization..."

# Phase 1: Create directories
echo "📁 Creating directory structure..."
mkdir -p stdlib
mkdir -p tests/{core,macros,collections,atoms,exceptions}
mkdir -p docs/{features,guides,technical,archive}

# Phase 2: Move test files
echo "🔄 Moving test files..."

# Core tests
[ -f test-loop-recur.clr ] && mv test-loop-recur.clr tests/core/loop-recur-test.clr && echo "  ✓ Moved loop-recur test"
[ -f test-multi-arity.clr ] && mv test-multi-arity.clr tests/core/multi-arity-test.clr && echo "  ✓ Moved multi-arity test"
[ -f test-multi-simple.clr ] && mv test-multi-simple.clr tests/core/multi-arity-simple-test.clr && echo "  ✓ Moved multi-arity-simple test"
[ -f test-variadic.clr ] && mv test-variadic.clr tests/core/variadic-test.clr && echo "  ✓ Moved variadic test"
[ -f test-destructuring.clr ] && mv test-destructuring.clr tests/core/destructuring-test.clr && echo "  ✓ Moved destructuring test"

# Macro tests
[ -f test-defmacro.clr ] && mv test-defmacro.clr tests/macros/defmacro-test.clr && echo "  ✓ Moved defmacro test"
[ -f test-macro-expansion.clr ] && mv test-macro-expansion.clr tests/macros/expansion-test.clr && echo "  ✓ Moved macro-expansion test"
[ -f test-control-flow-macros.clr ] && mv test-control-flow-macros.clr tests/macros/control-flow-test.clr && echo "  ✓ Moved control-flow test"
[ -f test-cond-case.clr ] && mv test-cond-case.clr tests/macros/cond-case-test.clr && echo "  ✓ Moved cond-case test"

# Collection tests
[ -f test-collections.clr ] && mv test-collections.clr tests/collections/api-test.clr && echo "  ✓ Moved collections test"

# Atom tests
[ -f test-atoms.clr ] && mv test-atoms.clr tests/atoms/atoms-test.clr && echo "  ✓ Moved atoms test"
[ -f test-atoms-simple.clr ] && mv test-atoms-simple.clr tests/atoms/atoms-simple-test.clr && echo "  ✓ Moved atoms-simple test"

# Exception tests
[ -f test-exceptions.clr ] && mv test-exceptions.clr tests/exceptions/exceptions-test.clr && echo "  ✓ Moved exceptions test"
[ -f test-simple-exception.clr ] && mv test-simple-exception.clr tests/exceptions/simple-test.clr && echo "  ✓ Moved simple-exception test"
[ -f test-try-catch.clr ] && mv test-try-catch.clr tests/exceptions/try-catch-test.clr && echo "  ✓ Moved try-catch test"

# Lazy tests
[ -f test-lazy-minimal.clr ] && mv test-lazy-minimal.clr tests/lazy-minimal-test.clr && echo "  ✓ Moved lazy-minimal test"
[ -f test-lazy.clr ] && mv test-lazy.clr tests/lazy-original-test.clr && echo "  ✓ Moved lazy test"

# Phase 3: Move documentation
echo "📚 Moving documentation files..."

# Feature docs
[ -f COLLECTIONS_API_COMPLETE.md ] && mv COLLECTIONS_API_COMPLETE.md docs/features/collections-api.md && echo "  ✓ Moved collections-api doc"
[ -f CONTROL_FLOW_MACROS_COMPLETE.md ] && mv CONTROL_FLOW_MACROS_COMPLETE.md docs/features/control-flow-macros.md && echo "  ✓ Moved control-flow-macros doc"
[ -f EXCEPTION_HANDLING_COMPLETE.md ] && mv EXCEPTION_HANDLING_COMPLETE.md docs/features/exception-handling.md && echo "  ✓ Moved exception-handling doc"
[ -f ASYNC_INTEGRATION_PROVEN.md ] && mv ASYNC_INTEGRATION_PROVEN.md docs/features/async-integration.md && echo "  ✓ Moved async-integration doc"
[ -f FFI_COMPLETE.md ] && mv FFI_COMPLETE.md docs/features/ffi.md && echo "  ✓ Moved ffi doc"
[ -f AUTOMATIC_FFI_COMPLETE.md ] && mv AUTOMATIC_FFI_COMPLETE.md docs/features/automatic-ffi.md && echo "  ✓ Moved automatic-ffi doc"

# Technical docs
[ -f LAZY_SEQUENCES_PURE_CLORUS.md ] && mv LAZY_SEQUENCES_PURE_CLORUS.md docs/technical/lazy-sequences-implementation.md && echo "  ✓ Moved lazy-sequences-implementation doc"
[ -f LAZY_VS_POLYMORPHISM.md ] && mv LAZY_VS_POLYMORPHISM.md docs/technical/lazy-vs-polymorphism.md && echo "  ✓ Moved lazy-vs-polymorphism doc"

# Archive old docs
[ -f MAJOR_FEATURES_PROGRESS.md ] && mv MAJOR_FEATURES_PROGRESS.md docs/archive/features-progress-jan27.md && echo "  ✓ Archived features-progress doc"
[ -f TRY_CATCH_STATUS.md ] && mv TRY_CATCH_STATUS.md docs/archive/try-catch-status-jan27.md && echo "  ✓ Archived try-catch-status doc"

# Keep the summary docs in root
echo "  ℹ Keeping LAZY_SEQUENCES_COMPLETE.md in root (summary doc)"

# Phase 4: Cleanup temporary files
echo "🗑️  Removing temporary files..."
[ -f test_expr.sh ] && rm -f test_expr.sh && echo "  ✓ Removed test_expr.sh"
[ -f repl.sh ] && rm -f repl.sh && echo "  ✓ Removed repl.sh"
[ -f test_value_star.txt ] && rm -f test_value_star.txt && echo "  ✓ Removed test_value_star.txt"
[ -f test_value_system.rs ] && rm -f test_value_system.rs && echo "  ✓ Removed test_value_system.rs"
[ -f repl-test.txt ] && rm -f repl-test.txt && echo "  ✓ Removed repl-test.txt"
[ -d test-fs-demo ] && rm -rf test-fs-demo && echo "  ✓ Removed test-fs-demo/"

echo ""
echo "✅ Cleanup and organization complete\!"
echo ""
echo "📊 New structure:"
echo "  crates/            - Compiler & runtime (Rust)"
echo "  stdlib/            - Standard library (Clorus)"
echo "  examples/          - Example programs"
echo "  tests/             - Test suite"
echo "    ├── core/        - Core language tests"
echo "    ├── macros/      - Macro system tests"
echo "    ├── collections/ - Collection API tests"
echo "    ├── atoms/       - Atom tests"
echo "    └── exceptions/  - Exception handling tests"
echo "  docs/              - Documentation"
echo "    ├── features/    - Feature documentation"
echo "    ├── technical/   - Technical specs"
echo "    └── archive/     - Archived docs"
echo ""
echo "🎉 Codebase is now organized\!"
