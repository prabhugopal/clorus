# Clorus Improvements Roadmap

This document tracks potential improvements and enhancements to the Clorus language and toolchain.

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

---

## P3 - Nice to Have

### 7. Better Progress Indicators
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

### 8. Package Registry Support (clorus install)
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

### 9. Dev Workflow Commands
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

### 10. REPL Enhancements
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

### 11. Performance Optimizations
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
