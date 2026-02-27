# Clorus Improvements Roadmap

This document tracks potential improvements and enhancements to the Clorus language and toolchain.

---

## P1 - Critical (High Priority)

### 1. Repository Cleanup and Organization
**Status:** Not started
**Effort:** Medium (2-3 hours)

The repository root is cluttered with 73+ files including scattered docs, tests, and scripts.

**Current Mess:**
```bash
$ ls | wc -l
73  # Way too many files in root!

Root directory contains:
- 21 markdown files (should be in docs/)
- 32 test files (should be in tests/)
- 7 shell scripts (should be in scripts/)
- Various temporary files
```

**Files to Organize:**

1. **Documentation → docs/**
   ```bash
   # Move to docs/
   IMPROVEMENTS.md
   CLIP_SPECIFICATION.md
   CLIP_USER_GUIDE.md
   FFI_MIGRATION_GUIDE.md
   LIBRARY_DISTRIBUTION_DESIGN.md
   TEST_INFRASTRUCTURE.md
   TEST_ORGANIZATION.md
   TEST_QUALITY.md

   # Move to docs/archive/ (completed sessions)
   BUGS_FIXED.md
   SESSION_COMPLETE.md
   SESSION_DESTRUCTURING.md
   DESTRUCTURING_COMPLETE.md
   FFI_IMPLEMENTATION_COMPLETE.md
   FFI_FINAL_STATUS.md

   # Move to docs/issues/
   KNOWN_ISSUES.md
   KNOWN_LIMITATIONS.md
   LANGUAGE_ISSUES.md
   FIX_PLAN.md

   # Move to docs/comparisons/
   CLORUS_VS_JANK.md
   ```

2. **Test Files → tests/**
   ```bash
   # Move to tests/unit/
   test-numeric.clr
   test-keyword-equality.clr
   test-comparisons.clr
   test-vectors.clr
   test-functions-in-data.clr
   test-arithmetic.clr
   ... (all test-*.clr files)

   # Move to tests/stdlib/
   test-stdlib-simple.clr
   test-loop-fix.clr
   test-loop-recur.clr

   # Move to tests/integration/
   test-keyboard-sim.clr
   test-repl-*.clr (if exist)
   ```

3. **Scripts → scripts/**
   ```bash
   # Move to scripts/
   install.sh
   cleanup-and-organize.sh
   cleanup-repo.sh
   test_metrics.sh
   test-agent-minimal.sh
   test-repl-formats.sh
   test-repl-output.sh
   ```

4. **Examples → examples/**
   ```bash
   # Check if any demo/example files in root
   # Move to examples/
   ```

**Target Structure:**
```
clorus/
├── README.md                 ← Keep in root
├── BUILD.md                  ← Keep in root
├── Cargo.toml               ← Keep in root
├── .gitignore               ← Keep in root
│
├── docs/                     ← All documentation
│   ├── README.md
│   ├── improvements/
│   │   └── IMPROVEMENTS.md
│   ├── guides/
│   │   ├── CLIP_USER_GUIDE.md
│   │   ├── CLIP_SPECIFICATION.md
│   │   └── FFI_MIGRATION_GUIDE.md
│   ├── design/
│   │   └── LIBRARY_DISTRIBUTION_DESIGN.md
│   ├── issues/
│   │   ├── KNOWN_ISSUES.md
│   │   ├── KNOWN_LIMITATIONS.md
│   │   └── LANGUAGE_ISSUES.md
│   ├── archive/              ← Completed sessions
│   │   ├── BUGS_FIXED.md
│   │   ├── SESSION_*.md
│   │   └── *_COMPLETE.md
│   └── comparisons/
│       └── CLORUS_VS_JANK.md
│
├── tests/                    ← All test files
│   ├── README.md
│   ├── unit/
│   │   ├── test-numeric.clr
│   │   ├── test-vectors.clr
│   │   └── ... (all unit tests)
│   ├── stdlib/
│   │   ├── test-stdlib-simple.clr
│   │   └── test-loop-*.clr
│   ├── integration/
│   │   └── test-keyboard-sim.clr
│   └── compiler/             ← Already exists
│
├── scripts/                  ← All automation
│   ├── README.md
│   ├── build/
│   │   ├── build-all.sh
│   │   └── build-compiler.sh
│   ├── test/
│   │   ├── test_metrics.sh
│   │   ├── test-repl-*.sh
│   │   └── run_all_tests.sh
│   └── install/
│       └── install.sh
│
├── examples/                 ← Example projects
│   ├── factorial-demo/
│   └── json-lib/
│
├── crates/                   ← Rust source (keep as-is)
│   ├── clorus-cli/
│   ├── clorus-runtime/
│   └── ...
│
├── stdlib/                   ← Standard library (keep as-is)
│   ├── core.clr
│   └── transducers.clr
│
└── target/                   ← Build artifacts
```

**Implementation Script:**

```bash
#!/bin/bash
# scripts/organize-repo.sh

# Create directory structure
mkdir -p docs/{guides,design,issues,archive,comparisons,improvements}
mkdir -p tests/{unit,stdlib,integration}
mkdir -p scripts/{build,test,install}

# Move documentation
mv IMPROVEMENTS.md docs/improvements/
mv CLIP_*.md docs/guides/
mv FFI_MIGRATION_GUIDE.md LIBRARY_DISTRIBUTION_DESIGN.md docs/guides/
mv TEST_*.md docs/guides/
mv *_COMPLETE.md SESSION_*.md BUGS_FIXED.md docs/archive/
mv KNOWN_*.md LANGUAGE_ISSUES.md FIX_PLAN.md docs/issues/
mv CLORUS_VS_JANK.md docs/comparisons/

# Move test files
mv test-numeric.clr test-keyword*.clr test-comparisons.clr tests/unit/
mv test-vectors.clr test-functions*.clr test-arithmetic.clr tests/unit/
mv test-stdlib*.clr test-loop*.clr tests/stdlib/
mv test-keyboard*.clr test-repl*.clr tests/integration/ 2>/dev/null || true

# Move scripts
mv cleanup*.sh test_metrics.sh test-*.sh scripts/test/
mv install.sh scripts/install/
mv build*.sh scripts/build/ 2>/dev/null || true

# Create README files
cat > docs/README.md <<EOF
# Clorus Documentation

## Structure
- guides/ - User guides and tutorials
- design/ - Design documents and specs
- issues/ - Known issues and limitations
- archive/ - Completed session notes
- comparisons/ - Language comparisons

## Main Docs
- [User Guide](guides/CLIP_USER_GUIDE.md)
- [Improvements Roadmap](improvements/IMPROVEMENTS.md)
- [Known Issues](issues/KNOWN_ISSUES.md)
EOF

cat > tests/README.md <<EOF
# Clorus Test Suite

## Structure
- unit/ - Unit tests for language features
- stdlib/ - Standard library tests
- integration/ - Integration and system tests
- compiler/ - Compiler-specific tests

## Running Tests
\`\`\`bash
# All tests
./scripts/test/run-all.sh

# Specific category
clorus build && ./target/test-name
\`\`\`
EOF

cat > scripts/README.md <<EOF
# Clorus Scripts

## Structure
- build/ - Build and compilation scripts
- test/ - Testing and validation scripts
- install/ - Installation scripts

## Usage
See individual script files for details.
EOF

echo "✅ Repository organized!"
```

**Benefits:**
- Clean root directory (only essential files)
- Easy to find docs, tests, scripts
- Professional repo structure
- Follows Cargo/Rust conventions
- Better for contributors
- Easier maintenance

**Backward Compatibility:**
- Update CI/CD paths if needed
- Update internal doc links
- Add redirects in root README if needed

---

## P2 - Important (Not Urgent)

### 1. API Exports Extraction
**Status:** Not started
**Effort:** Medium (4-6 hours)

Currently `exports.json` in .clip packages is a placeholder. We should extract actual function signatures.

**Benefits:**
- Type checking for .clip functions
- Better error messages
- Autocomplete support in REPL
- Documentation generation

**Implementation Options:**
- **Option A (Easier):** Parse source .clrs during `clorus pack`, extract `(defn ...)` forms
- **Option B (Robust):** Extract symbols from .o file using nm/objdump, demangle names

**Recommended:** Start with Option A, add Option B later for validation.

---

### 2. Better Error Messages for .clip Functions
**Status:** Not started
**Effort:** Small (2-3 hours)

When a .clip function fails or is not found, provide helpful context.

**Examples:**
```
Error: Unknown function: json/encode
  → Did you mean json/json-object-3?
  → Available in package: json-lib v0.1.0
  → Loaded .clip packages: json-lib, http-client

Error: Function json/encode not found
  → Package json-lib is loaded but doesn't export this function
  → Available functions: json-value, json-object, json-object-2, json-object-3
```

**Implementation:**
- Read exports.json when loading .clip
- Store in REPL engine and CodeGen
- Use for better error messages and suggestions

---

### 3. .repl/ Cache Management
**Status:** Not started
**Effort:** Small (2-3 hours)

The `.repl/` folder grows but never cleans up old .dylib files.

**Features to add:**
- `clorus repl --clean` - Clear entire .repl cache
- Auto-detect stale .dylib (older than source .clip)
- Rebuild .dylib if .clip package was updated
- Show cache size in `clorus repl --info`

**Implementation:**
```rust
// Check if cached .dylib is stale
fn is_cache_stale(dylib_path: &Path, clip_path: &Path) -> bool {
    let dylib_mtime = fs::metadata(dylib_path)?.modified()?;
    let clip_mtime = fs::metadata(clip_path)?.modified()?;
    clip_mtime > dylib_mtime
}
```

---

### 4. Version Conflict Detection
**Status:** Not started
**Effort:** Medium (3-4 hours)

Detect when multiple versions of the same .clip package are loaded.

**Example:**
```
⚠️  Warning: Version conflict detected
   json-lib v0.1.0 (from factorial-demo)
   json-lib v0.2.0 (from http-server)
   → Using: json-lib v0.2.0

   To resolve:
   - Update factorial-demo to use json-lib v0.2.0
   - Or pin http-server to use json-lib v0.1.0
```

**Implementation:**
- Track loaded packages with versions
- Detect conflicts during dependency resolution
- Use semantic versioning for compatibility checks

---

### 5. Build Caching (Incremental Compilation)
**Status:** Not started
**Effort:** Large (1-2 weeks)

Don't recompile unchanged source files. Cache intermediate artifacts.

**Benefits:**
- Much faster rebuild times
- Similar to cargo's incremental compilation
- Only recompile changed modules

**Implementation Strategy:**
1. Hash each source file content
2. Cache .o files in target/incremental/
3. Track dependency graph
4. Only recompile if:
   - Source file changed
   - Dependencies changed
   - Compiler version changed
5. Relink only if any .o changed

**Example:**
```
$ clorus build
   Compiling my-app v0.1.0
   Fresh json-lib v0.1.0 (cached)
   Fresh http-client v0.2.1 (cached)
   Compiling src/main.clrs (changed)
   Finished dev [unoptimized] in 0.5s
```

---

### 6. Rust FFI Dependencies in .clip Packages
**Status:** Not started
**Effort:** Large (1-2 weeks)

Currently .clip packages don't include Rust FFI dependencies, causing link errors when used.

**Problem:**
```toml
# crypto-lib uses Rust FFI
[rust-dependencies]
sha2 = "0.10"

# After packing, crypto-lib.clip is missing libsha2.a
# Apps that use crypto-lib.clip fail to link
```

**Solution Options:**

**Option A: Bundle Rust Static Libraries** (Recommended)
- Package Rust .a files inside .clip archive
- Platform-specific variants (macos-aarch64, linux-x86_64, etc.)
- Extract and link during build

**Option B: Transitive Rust Dependencies**
- Store rust-dependencies in clip.toml
- Apps rebuild Rust deps on-the-fly
- Requires Rust toolchain on user machines

**Implementation (Option A):**

1. **Update pack.rs** to bundle Rust libraries:
```rust
// After building Rust FFI deps, copy them into .clip
clip_archive/
├── lib/
│   ├── mylib.o
│   ├── mylib.bc
│   └── rust/              # NEW
│       ├── libserde.a
│       └── libsha2.a
```

2. **Update clip.toml format**:
```toml
[rust-dependencies]
serde = "1.0"
sha2 = "0.10"

[bundled-libs]
# Platform-specific artifacts
platform = "macos-aarch64"
libs = ["rust/libserde.a", "rust/libsha2.a"]
```

3. **Update extract_clip()** to extract Rust libs

4. **Update build/link** to include bundled Rust libraries:
```rust
// In commands.rs link phase
for rust_lib in package.rust_libs {
    link_cmd.arg(rust_lib.path);
}
```

**Benefits:**
- Self-contained .clip packages
- No Rust toolchain required for apps
- Faster builds (no Rust recompilation)
- Works like other language ecosystems

**Challenges:**
- Larger .clip files
- Need platform detection/selection
- License compliance for bundled Rust crates
- Multiple platform support in single .clip

**Alternative: Hybrid Approach**
- Bundle Rust libs by default
- Fall back to building if platform mismatch
- Best of both worlds

**Configuration Option:**

Add `bundle` field to control Rust dependency packaging:

```toml
[rust-dependencies]
# Option 1: Bundle in .clip (default for libraries)
serde_json = { version = "1.0", bundle = true }

# Option 2: Transitive (user rebuilds, smaller .clip)
sha2 = { version = "0.10", bundle = false }

# Option 3: Auto-detect based on package type
tokio = "1.0"  # bundle = true for lib, false for bin
```

Implementation:
```rust
// In pack.rs
if rust_dep.bundle.unwrap_or(true) {  // Default to bundling
    copy_rust_static_lib(&rust_dep, &temp_dir)?;
} else {
    // Just store metadata, user rebuilds
    store_rust_dep_metadata(&rust_dep, clip_toml)?;
}
```

Benefits:
- Library authors choose bundling strategy
- Large deps (17MB) can be transitive
- Small critical deps can be bundled
- Flexibility for different use cases

---

### 7. Multi-Module and Workspace Support
**Status:** Not started
**Effort:** Large (1-2 weeks)

Currently, .clip packages only support single-entry compilation. Need support for multi-module libraries and workspace projects.

**Problem 1: Multi-Module Libraries**

Today, only the entry file is compiled:
```toml
[build]
entry = "src/core.clrs"  # Only this file

# src/utils.clrs is ignored!
# src/parser.clrs is ignored!
```

**Problem 2: Monorepos / Workspaces**

No support for multi-package projects:
```
my-project/
├── lib-a/
│   └── Clorus.toml
├── lib-b/
│   └── Clorus.toml
└── app/
    └── Clorus.toml

# Must cd to each directory and pack individually
```

**Solution 1: Multi-Module Libraries**

```toml
[build]
# Option A: Explicit list
modules = [
    "src/json/core.clrs",
    "src/json/parser.clrs",
    "src/json/encoder.clrs",
]

# Option B: Glob pattern (simpler)
src = ["src/**/*.clrs"]
exclude = ["src/tests/**"]
```

Result:
```
json-lib.clip → Contains:
  - json.core
  - json.parser
  - json.encoder
  (All namespaces in one package)
```

**Solution 2: Workspace Support (Like Cargo)**

```toml
# Workspace.toml (at repo root)
[workspace]
members = [
    "coral-gfx",
    "coral-ui",
    "coral-layout",
]

[workspace.dependencies]
# Shared dependencies across workspace
serde = "1.0"
```

Commands:
```bash
# Pack all workspace members
$ clorus pack --workspace
   Packaging coral-gfx v1.0.0
   Packaging coral-ui v2.0.0
   Packaging coral-layout v1.5.0
   Created 3 packages in target/package/

# Build workspace member
$ cd coral-ui
$ clorus build
   Using workspace dependencies...
```

**Benefits:**

Multi-Module:
- Package entire libraries (all namespaces)
- No need to list every file
- Automatic dependency ordering
- Better modularization

Workspace:
- Manage multiple packages in one repo
- Shared dependency versions
- Single command to pack all
- Consistent tooling (like Cargo, Lerna)

**Implementation:**

1. **Multi-Module:**
   - Extend Manifest to support `modules` or `src` fields
   - Walk directory tree and collect .clrs files
   - Detect namespaces and dependencies
   - Compile in dependency order
   - Package all .o files in one .clip

2. **Workspace:**
   - New `Workspace.toml` format
   - `clorus pack --workspace` command
   - Resolve inter-package dependencies
   - Build in topological order
   - Shared target/package/ directory

**Example Use Cases:**

Multi-Module:
```
coral-gfx.clip → 35 functions across 3 namespaces
  - coral-gfx.core     (window, drawing)
  - coral-gfx.events   (mouse, keyboard)
  - coral-gfx.utils    (string helpers)
```

Workspace:
```
coral-workspace/
├── target/package/
│   ├── coral-gfx-1.0.0.clip
│   ├── coral-ui-2.0.0.clip      (depends on coral-gfx)
│   └── coral-layout-1.5.0.clip  (depends on coral-ui)
└── Workspace.toml

$ clorus pack --workspace
  → Packages all 3 in correct order
  → Shared dependency resolution
```

**Workspace REPL Behavior:**

```bash
# Option 1: Run REPL from workspace root
$ clorus repl
   Error: Must specify workspace member
   Available members: coral-gfx, coral-ui, coral-layout

   Usage:
     clorus repl -p coral-ui
     clorus repl --package coral-gfx

# Option 2: Run REPL from member directory
$ cd coral-ui
$ clorus repl
   Loading workspace dependencies...
      ✓ coral-gfx v1.0.0 (workspace member)

   Clorus REPL v0.1.0
   coral-ui λ> (require '[coral-gfx.core :as gfx])
   coral-ui λ> (require '[coral-layout.flex :as layout])
   coral-ui λ> (gfx/create-window "Test" 800 600)
   => #<Window 0x...>

# Option 3: Multi-member REPL (advanced)
$ clorus repl --workspace
   Loading all workspace members...
      ✓ coral-gfx v1.0.0
      ✓ coral-ui v2.0.0
      ✓ coral-layout v1.5.0

   workspace λ> (require '[coral-ui.button :as btn])
   workspace λ> (require '[coral-gfx.core :as gfx])
   # All workspace namespaces available!
```

**REPL Hot Reload with Workspace:**

```bash
# Terminal 1: Edit coral-gfx source
$ cd coral-gfx
$ # Edit src/core.clrs

# Terminal 2: REPL running coral-ui
coral-ui λ> (reload-workspace-member 'coral-gfx)
   Rebuilding coral-gfx v1.0.0...
   ✓ Recompiled
   Reloading dependent namespaces...
     ✓ coral-ui.window
     ✓ coral-ui.button
   ✅ Hot reload complete

coral-ui λ> (gfx/create-window "Test" 800 600)
   # Now uses updated coral-gfx code!
```

This enables:
- Interactive development across multiple packages
- Test workspace member integration in REPL
- Hot reload workspace dependencies
- Explore workspace APIs interactively

Similar to:
- Cargo: `cargo run -p member-name`
- Lerna: `lerna run --scope package-name`
- Nx: `nx serve app-name`

---

## P3 - Nice to Have

### 8. Output .clip Packages to target/ Directory
**Status:** Not started
**Effort:** Trivial (15-30 minutes)

Currently `clorus pack` creates .clip files in the project root, cluttering the workspace.

**Problem:**
```bash
$ clorus pack
✅ Created coral-gfx-1.0.0.clip  # In project root

$ ls
coral-gfx-1.0.0.clip  ← Clutter!
Clorus.toml
src/
target/
```

**Solution:**
Output to `target/package/` like Cargo:
```bash
$ clorus pack
✅ Created target/package/coral-gfx-1.0.0.clip

$ ls target/
coral-gfx.o
coral-gfx.bc
package/
  └── coral-gfx-1.0.0.clip  ← Clean!
```

**Implementation:**
```rust
// In pack.rs, line 262
let output_filename = output.unwrap_or_else(|| {
    // Create target/package/ if it doesn't exist
    let package_dir = PathBuf::from("target/package");
    fs::create_dir_all(&package_dir).ok();

    format!("target/package/{}-{}.clip", package_name, package_version)
});
```

**Benefits:**
- Cleaner project root
- Consistent with Cargo/Maven/Gradle conventions
- Easy to add to .gitignore: `target/`
- Natural location for build artifacts

**Backward compatibility:**
- Keep `--output` flag for custom paths
- Update error messages and docs

---

### 9. Better Progress Indicators
**Status:** Not started
**Effort:** Small (1-2 hours)

Replace `[PROGRESS]` logs with better UX.

**Options:**
- Spinner: `⠋ Compiling... 300/700 expressions`
- Progress bar: `████████░░ 45% (300/700 expressions)`
- Single-line update (overwrite previous line)

**Implementation:**
- Use `indicatif` crate for spinners/progress bars
- Or simple ANSI escape codes for line overwrite

---

### 10. Package Registry Support (clorus install)
**Status:** Not started
**Effort:** Large (2-3 weeks)

Support centralized package registry like crates.io.

**Features:**
- `clorus install json-lib` - Install from registry
- `clorus publish` - Publish package to registry
- Version range resolution: `json-lib = "^0.1.0"`
- Transitive dependency resolution
- Lock file (Clorus.lock) for reproducible builds

**Architecture:**
- HTTP API for registry
- Package index (Git repository)
- Authentication for publishing
- Checksum verification

---

### 11. Dev Workflow Commands
**Status:** Not started
**Effort:** Medium (1-2 weeks)

Add common development commands.

**Commands to add:**

#### `clorus test`
Run test files in `test/` directory:
```clojure
; test/factorial_test.clrs
(ns factorial-test
  (:require [factorial :as f]))

(deftest test-factorial-base
  (assert (= (f/factorial 0) 1))
  (assert (= (f/factorial 1) 1)))

(deftest test-factorial-recursive
  (assert (= (f/factorial 5) 120)))
```

#### `clorus doc`
Generate HTML documentation from docstrings:
```clojure
(defn factorial
  "Calculate the factorial of n.

   Examples:
     (factorial 5) => 120
     (factorial 0) => 1"
  [n]
  ...)
```

#### `clorus fmt`
Format code according to style guide:
- Consistent indentation
- Align closing brackets
- Standard spacing rules

#### `clorus clean`
Clean build artifacts:
```bash
$ clorus clean
   Removing target/
   Removing .repl/
```

---

### 12. REPL Enhancements
**Status:** Not started
**Effort:** Medium (1 week)

**Features:**

#### Hot Reload
Watch source files and auto-reload on change:
```
examples.factorialλ> :watch
Watching src/ for changes...
[File changed: src/examples/factorial.clrs]
Reloading...
✓ Reloaded (3 forms)
```

#### Better History Search
- Ctrl-R: Reverse search through history
- Persistent history across sessions
- Search by substring or regex

#### Multi-line Editing
Support editing multi-line expressions:
```
userλ> (defn long-function [x y z]
     |   (let [a (+ x y)
     |         b (* y z)]
     |     (+ a b)))
#'user/long-function
```

#### Autocomplete for .clip Functions
Use exports.json to suggest .clip functions:
```
userλ> (json/<TAB>
json/json-value
json/json-object
json/json-object-2
json/json-object-3
```

---

### 13. Performance Optimizations
**Status:** Not started
**Effort:** Large (2-3 weeks)

**Optimizations:**

#### Incremental Compilation
See P2 #5 above - cache unchanged modules.

#### Parallel Compilation
Compile independent modules in parallel:
```rust
// Use rayon to compile modules in parallel
use rayon::prelude::*;

modules.par_iter().for_each(|module| {
    compile_module(module);
});
```

#### Link-Time Optimization (LTO)
Enable LTO for release builds:
```toml
[profile.release]
lto = true           # Enable LTO
codegen-units = 1    # Single codegen unit for max optimization
opt-level = 3        # Maximum optimization
```

Expected improvements:
- 10-30% smaller binaries
- 5-15% faster execution
- Longer compile times (acceptable for release)

#### JIT Optimization Levels
Allow configuring JIT optimization:
```bash
clorus repl --opt=2  # O2 optimization
clorus run --jit --opt=3  # O3 for max speed
```

---

## Completed Features

### ✅ Phase 1-4: .clip Library System
- Create .clip packages with `clorus pack`
- Install with `clorus install`
- Static linking in `clorus build`
- Support in `clorus run`
- Dynamic loading in `clorus repl` with `.repl/` cache
- Multiple .clip package support
- Auto-declaration of .clip functions

### ✅ Clean Output
- Removed [PROGRESS] spam from normal builds
- Clean REPL startup
- Professional error messages

### ✅ Proper Entry Point Behavior
- -main only executes when explicitly called
- No duplicate execution in build/run/REPL

---

## Contributing

To work on any of these improvements:

1. Check this document for status
2. Create a branch: `git checkout -b feature/api-exports`
3. Update status to "In Progress" in this doc
4. Implement the feature
5. Add tests
6. Update documentation
7. Submit PR

For questions or discussion, open an issue on GitHub.
