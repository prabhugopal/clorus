# Clorus Codebase Cleanup and Organization Plan

## Overview

This document provides a step-by-step plan to organize and clean up the Clorus codebase, moving files to their proper locations according to the project organization guidelines.

**Date:** January 27, 2026
**Status:** Ready to Execute

---

## Current State Analysis

### Root Directory (Scattered Files)

**Documentation Files (*.md):**
```
ASYNC_INTEGRATION_PROVEN.md
AUTOMATIC_FFI_COMPLETE.md
COLLECTIONS_API_COMPLETE.md
CONTROL_FLOW_MACROS_COMPLETE.md
EXCEPTION_HANDLING_COMPLETE.md
FFI_COMPLETE.md
LAZY_SEQUENCES_PURE_CLORUS.md
LAZY_VS_POLYMORPHISM.md
MAJOR_FEATURES_PROGRESS.md
TRY_CATCH_STATUS.md
```

**Test Files (test-*.clr):**
```
test-atoms-simple.clr
test-atoms.clr
test-collections.clr
test-cond-case.clr
test-control-flow-macros.clr
test-defmacro.clr
test-destructuring.clr
test-exceptions.clr
test-lazy-minimal.clr
test-lazy.clr
test-loop-recur.clr
test-macro-expansion.clr
test-multi-arity.clr
test-multi-simple.clr
test-simple-exception.clr
test-try-catch.clr
test-variadic.clr
```

---

## Target Structure

```
clorus/
├── crates/              # Runtime & compiler (already organized ✅)
├── stdlib/              # Standard library (Clorus)
│   └── lazy.clr         # ✅ Already created
├── examples/            # Example programs
│   └── lazy-examples.clr # ✅ Already created
├── tests/               # Test suites
│   └── lazy-test.clr    # ✅ Already created
└── docs/                # Documentation
    ├── features/        # Feature documentation
    ├── guides/          # User guides
    └── technical/       # Technical specs
```

---

## Migration Plan

### Phase 1: Create Directory Structure

```bash
# Create stdlib directory (if not exists)
mkdir -p stdlib

# Create organized test directories
mkdir -p tests/core
mkdir -p tests/macros
mkdir -p tests/collections
mkdir -p tests/atoms
mkdir -p tests/exceptions

# Create organized docs directories
mkdir -p docs/features
mkdir -p docs/guides
mkdir -p docs/technical
mkdir -p docs/archive
```

### Phase 2: Move Test Files

#### Core Language Tests → `/tests/core/`
```bash
mv test-loop-recur.clr tests/core/loop-recur-test.clr
mv test-multi-arity.clr tests/core/multi-arity-test.clr
mv test-multi-simple.clr tests/core/multi-arity-simple-test.clr
mv test-variadic.clr tests/core/variadic-test.clr
mv test-destructuring.clr tests/core/destructuring-test.clr
```

#### Macro Tests → `/tests/macros/`
```bash
mv test-defmacro.clr tests/macros/defmacro-test.clr
mv test-macro-expansion.clr tests/macros/expansion-test.clr
mv test-control-flow-macros.clr tests/macros/control-flow-test.clr
mv test-cond-case.clr tests/macros/cond-case-test.clr
```

#### Collection Tests → `/tests/collections/`
```bash
mv test-collections.clr tests/collections/api-test.clr
```

#### Atom Tests → `/tests/atoms/`
```bash
mv test-atoms.clr tests/atoms/atoms-test.clr
mv test-atoms-simple.clr tests/atoms/atoms-simple-test.clr
```

#### Exception Tests → `/tests/exceptions/`
```bash
mv test-exceptions.clr tests/exceptions/exceptions-test.clr
mv test-simple-exception.clr tests/exceptions/simple-test.clr
mv test-try-catch.clr tests/exceptions/try-catch-test.clr
```

#### Lazy Sequence Tests → `/tests/` (keep at top level for now)
```bash
# These are already in good locations
# test-lazy.clr → Already moved to tests/lazy-test.clr ✅
# test-lazy-minimal.clr → Keep for examples or remove
```

### Phase 3: Move Documentation Files

#### Feature Documentation → `/docs/features/`
```bash
mv COLLECTIONS_API_COMPLETE.md docs/features/collections-api.md
mv CONTROL_FLOW_MACROS_COMPLETE.md docs/features/control-flow-macros.md
mv EXCEPTION_HANDLING_COMPLETE.md docs/features/exception-handling.md
mv ASYNC_INTEGRATION_PROVEN.md docs/features/async-integration.md
mv FFI_COMPLETE.md docs/features/ffi.md
mv AUTOMATIC_FFI_COMPLETE.md docs/features/automatic-ffi.md
```

#### Technical Documentation → `/docs/technical/`
```bash
mv LAZY_SEQUENCES_PURE_CLORUS.md docs/technical/lazy-sequences-implementation.md
mv LAZY_VS_POLYMORPHISM.md docs/technical/lazy-vs-polymorphism.md
```

#### Archive Old Status Docs → `/docs/archive/`
```bash
mv MAJOR_FEATURES_PROGRESS.md docs/archive/features-progress-jan27.md
mv TRY_CATCH_STATUS.md docs/archive/try-catch-status-jan27.md
```

### Phase 4: Cleanup Temporary Files

```bash
# Remove test scripts and temporary files
rm -f test_expr.sh
rm -f repl.sh
rm -f test_value_star.txt
rm -f test_value_system.rs
rm -f repl-test.txt

# Remove test directories
rm -rf test-fs-demo/

# Remove IDE files
rm -rf .idea/

# Clean build artifacts (if any)
rm -rf build/
```

---

## Detailed Migration Commands

### Execute All Migrations

```bash
#!/bin/bash
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
[ -f test-loop-recur.clr ] && mv test-loop-recur.clr tests/core/loop-recur-test.clr
[ -f test-multi-arity.clr ] && mv test-multi-arity.clr tests/core/multi-arity-test.clr
[ -f test-multi-simple.clr ] && mv test-multi-simple.clr tests/core/multi-arity-simple-test.clr
[ -f test-variadic.clr ] && mv test-variadic.clr tests/core/variadic-test.clr
[ -f test-destructuring.clr ] && mv test-destructuring.clr tests/core/destructuring-test.clr

# Macro tests
[ -f test-defmacro.clr ] && mv test-defmacro.clr tests/macros/defmacro-test.clr
[ -f test-macro-expansion.clr ] && mv test-macro-expansion.clr tests/macros/expansion-test.clr
[ -f test-control-flow-macros.clr ] && mv test-control-flow-macros.clr tests/macros/control-flow-test.clr
[ -f test-cond-case.clr ] && mv test-cond-case.clr tests/macros/cond-case-test.clr

# Collection tests
[ -f test-collections.clr ] && mv test-collections.clr tests/collections/api-test.clr

# Atom tests
[ -f test-atoms.clr ] && mv test-atoms.clr tests/atoms/atoms-test.clr
[ -f test-atoms-simple.clr ] && mv test-atoms-simple.clr tests/atoms/atoms-simple-test.clr

# Exception tests
[ -f test-exceptions.clr ] && mv test-exceptions.clr tests/exceptions/exceptions-test.clr
[ -f test-simple-exception.clr ] && mv test-simple-exception.clr tests/exceptions/simple-test.clr
[ -f test-try-catch.clr ] && mv test-try-catch.clr tests/exceptions/try-catch-test.clr

# Phase 3: Move documentation
echo "📚 Moving documentation files..."

# Feature docs
[ -f COLLECTIONS_API_COMPLETE.md ] && mv COLLECTIONS_API_COMPLETE.md docs/features/collections-api.md
[ -f CONTROL_FLOW_MACROS_COMPLETE.md ] && mv CONTROL_FLOW_MACROS_COMPLETE.md docs/features/control-flow-macros.md
[ -f EXCEPTION_HANDLING_COMPLETE.md ] && mv EXCEPTION_HANDLING_COMPLETE.md docs/features/exception-handling.md
[ -f ASYNC_INTEGRATION_PROVEN.md ] && mv ASYNC_INTEGRATION_PROVEN.md docs/features/async-integration.md
[ -f FFI_COMPLETE.md ] && mv FFI_COMPLETE.md docs/features/ffi.md
[ -f AUTOMATIC_FFI_COMPLETE.md ] && mv AUTOMATIC_FFI_COMPLETE.md docs/features/automatic-ffi.md

# Technical docs
[ -f LAZY_SEQUENCES_PURE_CLORUS.md ] && mv LAZY_SEQUENCES_PURE_CLORUS.md docs/technical/lazy-sequences-implementation.md
[ -f LAZY_VS_POLYMORPHISM.md ] && mv LAZY_VS_POLYMORPHISM.md docs/technical/lazy-vs-polymorphism.md

# Archive old docs
[ -f MAJOR_FEATURES_PROGRESS.md ] && mv MAJOR_FEATURES_PROGRESS.md docs/archive/features-progress-jan27.md
[ -f TRY_CATCH_STATUS.md ] && mv TRY_CATCH_STATUS.md docs/archive/try-catch-status-jan27.md

# Phase 4: Cleanup temporary files
echo "🗑️  Removing temporary files..."
rm -f test_expr.sh
rm -f repl.sh
rm -f test_value_star.txt
rm -f test_value_system.rs
rm -f repl-test.txt
rm -rf test-fs-demo/ 2>/dev/null || true

echo "✅ Cleanup and organization complete!"
echo ""
echo "📊 New structure:"
echo "  tests/core/        - Core language tests"
echo "  tests/macros/      - Macro system tests"
echo "  tests/collections/ - Collection API tests"
echo "  tests/atoms/       - Atom tests"
echo "  tests/exceptions/  - Exception handling tests"
echo "  docs/features/     - Feature documentation"
echo "  docs/technical/    - Technical specs"
echo "  docs/archive/      - Archived docs"
echo ""
echo "🎉 Codebase is now organized!"
```

---

## After Migration

### Update Documentation References

Update any references in documentation to reflect new locations:

1. **README.md** - Update test/example file paths
2. **LANGUAGE_PARITY.md** - Update feature doc references
3. **CONTRIBUTING.md** (if exists) - Update file location guidelines

### Create Index Files

```bash
# tests/README.md
echo "# Clorus Test Suite

Organization:
- core/ - Core language features
- macros/ - Macro system
- collections/ - Collection operations
- atoms/ - Mutable state
- exceptions/ - Exception handling
" > tests/README.md

# docs/features/README.md
echo "# Feature Documentation

Complete documentation for implemented Clorus features:
- collections-api.md - Collection operations
- control-flow-macros.md - Control flow macros
- exception-handling.md - Exception handling
- ffi.md - Foreign function interface
" > docs/features/README.md
```

### Update .gitignore

```bash
# Add to .gitignore
echo "
# Build artifacts
/build/
/target/

# IDE
/.idea/
*.swp

# Temporary files
test_*.sh
*-test.txt
test-fs-demo/
" >> .gitignore
```

---

## Verification

After migration, verify the structure:

```bash
# Check test organization
tree tests/

# Check docs organization
tree docs/

# Verify no test files in root
ls -la *.clr 2>&1 | grep test

# Verify no scattered docs in root
ls -la *.md | grep -E "(COMPLETE|STATUS|PROVEN)"
```

---

## Final Structure

```
clorus/
├── crates/                      # Runtime & compiler ✅
│   ├── clorus-syntax/
│   ├── clorus-codegen/
│   ├── clorus-runtime/
│   ├── clorus-cli/
│   └── clorus-repl/
│
├── stdlib/                      # Standard library ✅
│   └── lazy.clr
│
├── examples/                    # Examples ✅
│   └── lazy-examples.clr
│
├── tests/                       # Organized tests ✨
│   ├── lazy-test.clr
│   ├── core/
│   │   ├── loop-recur-test.clr
│   │   ├── multi-arity-test.clr
│   │   ├── variadic-test.clr
│   │   └── destructuring-test.clr
│   ├── macros/
│   │   ├── defmacro-test.clr
│   │   ├── expansion-test.clr
│   │   ├── control-flow-test.clr
│   │   └── cond-case-test.clr
│   ├── collections/
│   │   └── api-test.clr
│   ├── atoms/
│   │   ├── atoms-test.clr
│   │   └── atoms-simple-test.clr
│   └── exceptions/
│       ├── exceptions-test.clr
│       ├── simple-test.clr
│       └── try-catch-test.clr
│
├── docs/                        # Organized documentation ✨
│   ├── LANGUAGE_PARITY.md
│   ├── LAZY_SEQUENCES_GUIDE.md
│   ├── PROJECT_ORGANIZATION.md
│   ├── features/
│   │   ├── collections-api.md
│   │   ├── control-flow-macros.md
│   │   ├── exception-handling.md
│   │   ├── async-integration.md
│   │   └── ffi.md
│   ├── technical/
│   │   ├── lazy-sequences-implementation.md
│   │   └── lazy-vs-polymorphism.md
│   └── archive/
│       ├── features-progress-jan27.md
│       └── try-catch-status-jan27.md
│
├── Cargo.toml                   # Workspace manifest
├── Cargo.lock
├── README.md
├── .gitignore
└── LICENSE
```

---

## Execution

To execute this cleanup plan:

1. **Review** - Read this document carefully
2. **Backup** - Create a git commit: `git add -A && git commit -m "Before cleanup"`
3. **Run script** - Execute the cleanup script above
4. **Verify** - Check that all files are in correct locations
5. **Test** - Run tests to ensure nothing broke
6. **Commit** - `git add -A && git commit -m "Organize codebase structure"`

---

## Benefits

After cleanup:

✅ **Clear organization** - Easy to find files
✅ **Scalable structure** - Room for growth
✅ **Professional appearance** - Production-ready
✅ **Easy navigation** - Logical grouping
✅ **Better maintainability** - Easier to contribute

---

**Status:** Ready to Execute
**Estimated Time:** 10 minutes
**Risk:** Low (all moves, no deletions of important files)

---

**Last Updated:** January 27, 2026
