# 🧹 Repository Cleanup & Reorganization Plan

**Current State:** Files scattered across root, tests/, docs/, stdlib/
**Goal:** Clean, professional organization by feature/component

---

## 📊 Current Issues

```
Problems Identified:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. 96 test files in root directory (test-*.clr)
   ❌ Makes root messy and unprofessional
   ❌ Hard to find specific tests
   ❌ No organization by feature

2. tests/ has good structure BUT:
   ✅ Has subdirectories (atoms/, core/, strings/, etc.)
   ⚠️  Naming inconsistent (some test-*.clr, some *-test.clr)
   ⚠️  Some duplicate tests with root

3. stdlib/ has test files mixed in:
   ❌ test.clr, test-partial.clr, test-simple.clr
   ❌ Should only contain library code

4. docs/ organization needs improvement:
   ✅ Has subdirectories (design/, features/, sessions/)
   ⚠️  Some docs in root (CMDLINE_ARGS_COMPLETE.md)
   ⚠️  Could be better categorized

5. Backup files in codebase:
   ❌ *.bak, *.bak2, *.bak3 files
   ❌ Should be in .gitignore or removed
```

---

## 🎯 Proposed Structure

```
clorus/
├── crates/                     # Rust source code (unchanged)
│   ├── clorus-cli/
│   ├── clorus-codegen/
│   ├── clorus-runtime/
│   ├── clorus-syntax/
│   └── ...
│
├── stdlib/                     # Standard library (CLEAN - code only)
│   ├── core.clr               # Core utilities
│   ├── lazy.clr               # Lazy sequences
│   ├── transducers.clr        # Transducers
│   ├── math.clr               # Math library (NEW)
│   ├── string.clr             # String operations (NEW)
│   └── io.clr                 # I/O operations (NEW)
│
├── tests/                      # All tests organized by component
│   ├── README.md              # Test organization guide
│   ├── lang/                  # Language feature tests
│   │   ├── core/              # Core language
│   │   │   ├── functions.clr
│   │   │   ├── closures.clr
│   │   │   ├── multi-arity.clr
│   │   │   ├── variadic.clr
│   │   │   ├── loop-recur.clr
│   │   │   └── destructuring.clr
│   │   ├── macros/            # Macro system
│   │   │   ├── defmacro.clr
│   │   │   ├── expansion.clr
│   │   │   ├── control-flow.clr
│   │   │   └── cond-case.clr
│   │   ├── collections/       # Collection operations
│   │   │   ├── vectors.clr
│   │   │   ├── maps.clr
│   │   │   ├── sets.clr
│   │   │   └── api.clr
│   │   └── special-forms/     # Special forms
│   │       ├── def.clr
│   │       ├── if.clr
│   │       ├── let.clr
│   │       └── do.clr
│   │
│   ├── stdlib/                # Standard library tests
│   │   ├── core/
│   │   │   ├── predicates.clr
│   │   │   ├── sequences.clr
│   │   │   └── utilities.clr
│   │   ├── lazy/
│   │   │   ├── lazy-seq.clr
│   │   │   └── realization.clr
│   │   ├── transducers/
│   │   │   └── basic.clr
│   │   ├── math/              # Math library tests (NEW)
│   │   │   ├── trig.clr
│   │   │   ├── vectors.clr
│   │   │   └── interpolation.clr
│   │   └── string/            # String tests
│   │       └── operations.clr
│   │
│   ├── concurrency/           # Concurrency primitives
│   │   ├── atoms/
│   │   │   ├── basic.clr
│   │   │   ├── swap.clr
│   │   │   └── reset.clr
│   │   ├── refs/
│   │   │   ├── stm.clr
│   │   │   ├── dosync.clr
│   │   │   └── alter.clr
│   │   ├── agents/
│   │   │   └── send.clr
│   │   └── channels/
│   │       └── go-blocks.clr
│   │
│   ├── polymorphism/          # Protocols, records, multimethods
│   │   ├── protocols.clr
│   │   ├── records.clr
│   │   └── multimethods.clr
│   │
│   ├── types/                 # Type checking
│   │   └── predicates.clr
│   │
│   ├── exceptions/            # Error handling
│   │   ├── try-catch.clr
│   │   └── throw.clr
│   │
│   ├── repl/                  # REPL-specific tests
│   │   ├── autocomplete.clr
│   │   ├── binding.clr
│   │   └── evaluation.clr
│   │
│   └── integration/           # End-to-end tests
│       ├── simple-app.clr
│       └── data-pipeline.clr
│
├── docs/                       # Documentation (reorganized)
│   ├── README.md              # Main documentation index
│   │
│   ├── guide/                 # User guides
│   │   ├── GETTING_STARTED.md
│   │   ├── LANGUAGE_GUIDE.md
│   │   ├── STDLIB_GUIDE.md
│   │   └── RUST_FFI_GUIDE.md
│   │
│   ├── reference/             # Reference documentation
│   │   ├── LANGUAGE_SPEC.md
│   │   ├── STDLIB_API.md
│   │   ├── FFI_REFERENCE.md
│   │   └── LINKING.md
│   │
│   ├── design/                # Design documents (keep)
│   │   ├── ARCHITECTURE.md
│   │   ├── MEMORY_MODEL.md
│   │   ├── MODULAR_ARCHITECTURE.md
│   │   └── ...
│   │
│   ├── features/              # Feature documentation (reorganize)
│   │   ├── concurrency/
│   │   │   ├── ATOMS.md
│   │   │   ├── REFS_STM.md
│   │   │   ├── AGENTS.md
│   │   │   └── CHANNELS.md
│   │   ├── collections/
│   │   │   ├── PERSISTENT_DATA_STRUCTURES.md
│   │   │   └── COLLECTIONS_API.md
│   │   ├── macros/
│   │   │   └── CONTROL_FLOW_MACROS.md
│   │   ├── repl/
│   │   │   ├── REPL_FEATURES.md
│   │   │   └── AUTOCOMPLETE.md
│   │   └── exceptions/
│   │       └── EXCEPTION_HANDLING.md
│   │
│   ├── implementation/        # Implementation status (NEW)
│   │   ├── STDLIB_ENHANCEMENT_PLAN.md
│   │   ├── FEATURE_MATRIX.md
│   │   ├── COVERAGE_ASSESSMENT.md
│   │   └── ROADMAP.md
│   │
│   ├── sessions/              # Development session notes
│   │   ├── README.md          # Index of sessions
│   │   ├── 2025-01/
│   │   │   ├── PHASE1_COMPLETE.md
│   │   │   ├── PHASE5_COMPLETE.md
│   │   │   └── ...
│   │   └── archive/
│   │
│   └── archive/               # Old/deprecated docs
│       └── ...
│
├── examples/                   # Example programs (unchanged)
│   └── ...
│
├── target/                     # Build output (unchanged)
│
├── .gitignore                 # Updated to ignore *.bak, *.clr~ etc.
├── Cargo.toml
├── Clorus.toml
└── README.md
```

---

## 🔧 Migration Steps

### Step 1: Create New Test Structure (5 min)

```bash
# Create new test directories
mkdir -p tests/lang/{core,macros,collections,special-forms}
mkdir -p tests/stdlib/{core,lazy,transducers,math,string}
mkdir -p tests/concurrency/{atoms,refs,agents,channels}
mkdir -p tests/polymorphism
mkdir -p tests/types
mkdir -p tests/exceptions
mkdir -p tests/repl
mkdir -p tests/integration
```

### Step 2: Move Root Tests to Organized Structure (10 min)

**Automated with script:**

```bash
#!/bin/bash
# migrate-tests.sh

# Core language tests
mv test-*-closure*.clr tests/lang/core/closures.clr  # Combine closure tests
mv test-multi-arity*.clr tests/lang/core/multi-arity.clr
mv test-*-fn*.clr tests/lang/core/functions.clr
mv test-*recur*.clr tests/lang/core/loop-recur.clr

# Atoms
mv test-atom*.clr tests/concurrency/atoms/
mv test-swap*.clr tests/concurrency/atoms/

# Refs
mv test-ref*.clr tests/concurrency/refs/
mv test-dosync*.clr tests/concurrency/refs/
mv test-alter*.clr tests/concurrency/refs/

# Binding (REPL)
mv test-binding*.clr tests/repl/

# Macros
mv test-macro*.clr tests/lang/macros/

# Collections
mv test-map*.clr tests/lang/collections/
mv test-vector*.clr tests/lang/collections/

# Stdlib
mv test-partition*.clr tests/stdlib/core/
mv test-transducer*.clr tests/stdlib/transducers/

# Simple/minimal tests → integration
mv test-simple*.clr tests/integration/
mv test-minimal*.clr tests/integration/
mv test-hello*.clr tests/integration/

# IO tests
mv test-io*.clr tests/stdlib/io/ || mkdir -p tests/stdlib/io && mv test-io*.clr tests/stdlib/io/

# Clean up remaining
mv test-*.clr tests/lang/core/ || true  # Anything left goes to core
```

### Step 3: Clean stdlib/ Directory (2 min)

```bash
# Move stdlib test files to tests/stdlib/
mv stdlib/test*.clr tests/stdlib/core/
mv stdlib/minimal.clr tests/integration/stdlib-minimal.clr
mv stdlib/core-working.clr stdlib/core.clr.bak  # Backup if needed
```

### Step 4: Reorganize Documentation (10 min)

```bash
# Create new doc structure
mkdir -p docs/guide
mkdir -p docs/reference
mkdir -p docs/implementation
mkdir -p docs/features/{concurrency,collections,macros,repl,exceptions}
mkdir -p docs/sessions/2025-01

# Move guides
mv docs/guides/RUST_FFI_GUIDE.md docs/guide/

# Move reference docs
mv docs/reference/LANGUAGE_SPEC.md docs/reference/
mv docs/reference/LINKING.md docs/reference/  # Keep

# Move implementation docs
mv docs/STDLIB_ENHANCEMENT_PLAN.md docs/implementation/
mv docs/FEATURE_MATRIX.md docs/implementation/
mv docs/COVERAGE_ASSESSMENT.md docs/implementation/

# Organize feature docs
mv docs/features/ATOMS*.md docs/features/concurrency/
mv docs/features/REFS*.md docs/features/concurrency/
mv docs/features/AGENTS*.md docs/features/concurrency/
mv docs/features/CHANNELS*.md docs/features/concurrency/
mv docs/features/PERSISTENT_DATA_STRUCTURES.md docs/features/collections/
mv docs/features/collections-api.md docs/features/collections/
mv docs/features/control-flow-macros.md docs/features/macros/
mv docs/features/REPL*.md docs/features/repl/
mv docs/features/AUTOCOMPLETE*.md docs/features/repl/
mv docs/features/exception-handling.md docs/features/exceptions/

# Move session docs
mv docs/sessions/PHASE*.md docs/sessions/2025-01/
mv docs/sessions/*_COMPLETE.md docs/sessions/2025-01/

# Move root docs to docs/
mv CMDLINE_ARGS_COMPLETE.md docs/sessions/2025-01/
```

### Step 5: Remove Backup Files (1 min)

```bash
# Remove .bak files (after backing up to safe location if needed)
find . -name "*.bak*" -type f -delete

# Update .gitignore
cat >> .gitignore << 'EOF'

# Backup files
*.bak
*.bak2
*.bak3
*.bak4
*~
*.swp
*.swo

# Test artifacts
test-*.clr.out
*.temp
EOF
```

### Step 6: Update README Files (5 min)

```bash
# Create test README
cat > tests/README.md << 'EOF'
# Clorus Test Suite

## Organization

- `lang/` - Language feature tests (core, macros, collections)
- `stdlib/` - Standard library tests
- `concurrency/` - Concurrency primitives (atoms, refs, agents, channels)
- `polymorphism/` - Protocols, records, multimethods
- `types/` - Type checking and predicates
- `exceptions/` - Error handling
- `repl/` - REPL-specific functionality
- `integration/` - End-to-end integration tests

## Naming Convention

- Test files should be descriptive: `feature-name.clr`
- Group related tests in subdirectories
- Keep test files focused on a single feature

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test category
cargo test lang
cargo test stdlib
cargo test concurrency
```
EOF

# Create docs README
cat > docs/README.md << 'EOF'
# Clorus Documentation

## User Documentation

- **[Getting Started](guide/GETTING_STARTED.md)** - Quick start guide
- **[Language Guide](guide/LANGUAGE_GUIDE.md)** - Complete language tutorial
- **[Standard Library Guide](guide/STDLIB_GUIDE.md)** - Stdlib reference
- **[Rust FFI Guide](guide/RUST_FFI_GUIDE.md)** - Calling Rust from Clorus

## Reference

- **[Language Specification](reference/LANGUAGE_SPEC.md)** - Formal spec
- **[Standard Library API](reference/STDLIB_API.md)** - Complete API
- **[FFI Reference](reference/FFI_REFERENCE.md)** - FFI details

## Implementation

- **[Stdlib Enhancement Plan](implementation/STDLIB_ENHANCEMENT_PLAN.md)** - Roadmap
- **[Feature Matrix](implementation/FEATURE_MATRIX.md)** - Feature coverage
- **[Coverage Assessment](implementation/COVERAGE_ASSESSMENT.md)** - Gaps

## Design Documents

See [design/](design/) for architecture and design decisions.

## Features

Detailed feature documentation organized by category in [features/](features/).

## Development Sessions

Development notes and session summaries in [sessions/](sessions/).
EOF
```

---

## ✅ Validation Checklist

After migration:

```
[ ] All 96 root test files moved to tests/
[ ] Root directory clean (only project files)
[ ] tests/ has clear organization
[ ] stdlib/ contains only .clr library code
[ ] docs/ has clear structure with READMEs
[ ] All backup files removed or .gitignored
[ ] Tests still pass: cargo test
[ ] No broken links in documentation
[ ] Git status clean (no accidental deletions)
```

---

## 🎯 Benefits

After cleanup:

```
✅ Professional repository structure
✅ Easy to find specific tests
✅ Clear separation: code vs tests vs docs
✅ Consistent naming conventions
✅ Better for new contributors
✅ Easier to navigate in IDE
✅ Clean git status
✅ Ready for stdlib enhancement work
```

---

## 🚀 Execution Plan

**Estimated Time: ~30 minutes**

```
Phase 1 (10 min): Create structure + test migration
Phase 2 (10 min): Doc reorganization
Phase 3 (5 min):  Cleanup (backups, gitignore)
Phase 4 (5 min):  Validation + git commit
```

**When to do this:**
- Now, before starting stdlib work
- Creates clean foundation for future development
- Prevents accumulating more mess

---

Ready to execute? 🧹
