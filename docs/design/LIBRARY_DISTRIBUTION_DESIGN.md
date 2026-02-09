# Clorus Library Distribution System Design

## Overview

Proposal for a comprehensive library distribution system for the Clorus language, inspired by Cargo (Rust), npm (JavaScript), and Maven (Java).

## Goals

1. **Easy to use** - Simple library creation and consumption
2. **Fast compilation** - Cached compiled artifacts
3. **Version management** - Semantic versioning, dependency resolution
4. **Multiple distribution methods** - Source, compiled, hybrid
5. **Backwards compatible** - Existing projects continue to work

---

## Clorus File Formats

| Extension | Type | Description |
|-----------|------|-------------|
| `.clrs` | Source | Clorus source file |
| `.clrsp` | Package | Clorus source package (compressed archive) |
| `.clro` | Object | Compiled LLVM bitcode + metadata (platform-independent) |
| `.cla` | Archive | Static library (for static linking) |
| `.cldylib` | Library | Dynamic library (macOS) |
| `.clso` | Library | Shared object (Linux) |
| `.cldll` | Library | Dynamic library (Windows) |

**Design Philosophy:**
- Clear namespace: All formats start with `cl` (Clorus)
- Familiar patterns: `.clro` like `.o`, `.cla` like `.a`
- Cross-platform: Platform-specific extensions only for binaries

---

## 1. Library Project Structure

### Clorus.toml Extensions

```toml
[package]
name = "coral"
version = "1.0.0"
authors = ["You <you@example.com>"]
description = "CORAL GUI library for Clorus"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourname/coral"
keywords = ["gui", "ui", "reactive"]

# NEW: Specify project type
type = "library"           # or "executable" (default)

# NEW: Library configuration
[lib]
entry = "src/coral/core.clrs"      # Main library entry point
public = ["src/coral/*.clrs"]       # Public API files
internal = ["src/coral/internal/"]  # Private implementation
compile = true                      # Produce compiled artifact

# NEW: Dependencies
[dependencies]
clorus-std = "0.1.0"               # From registry
other-lib = { path = "../other" }   # Local path
remote-lib = { git = "https://github.com/user/lib.git", tag = "v1.0" }

# Rust dependencies (existing)
[rust-dependencies]
coral-gfx = { path = "rust", interface = false }

# NEW: Dev dependencies (for tests)
[dev-dependencies]
test-lib = "1.0.0"

# Build configuration
[build]
entry = "examples/gallery.clrs"  # For library, this runs examples
```

---

## 2. Library Compilation Formats

### Option A: Source Distribution (v1.0 - Simple)

**What:** Distribute Clorus source files

**Structure:**
```
coral-1.0.0/
├── Clorus.toml
├── src/
│   └── coral/
│       ├── core.clrs
│       ├── widgets.clrs
│       └── ...
├── rust/
│   └── libcoral_gfx.dylib
└── README.md
```

**Pros:**
- ✅ Simple to implement
- ✅ Works today (no compiler changes)
- ✅ Source code visible for debugging
- ✅ Cross-platform (source is universal)

**Cons:**
- ❌ Slower (recompile every time)
- ❌ No IP protection
- ❌ Larger download (includes all code)

### Option B: Compiled Artifact (v1.5 - Better)

**What:** Distribute compiled LLVM bitcode or native libraries

**Structure:**
```
coral-1.0.0/
├── Clorus.toml
├── lib/
│   ├── libcoral.clro       # Compiled Clorus code (LLVM bitcode)
│   ├── libcoral.cla        # Static library
│   └── libcoral.cldylib    # Dynamic library
├── include/
│   └── coral.h             # C header for FFI
└── README.md
```

**Formats:**
- `.clro` - Clorus Object (LLVM bitcode + metadata, platform-independent)
- `.cla` - Clorus Archive (static library)
- `.cldylib` / `.clso` / `.cldll` - Clorus dynamic library (macOS/Linux/Windows)

**Pros:**
- ✅ Fast (pre-compiled)
- ✅ Smaller download
- ✅ IP protection (compiled code)

**Cons:**
- ❌ Platform-specific (need macOS, Linux, Windows builds)
- ❌ Requires compiler changes
- ❌ Harder to debug

### Option C: Hybrid Distribution (v2.0 - Best)

**What:** Distribute both source and compiled artifacts

**Structure:**
```
coral-1.0.0/
├── Clorus.toml
├── src/              # Source code
├── lib/              # Pre-compiled for common platforms
│   ├── darwin-aarch64/
│   │   └── libcoral.cldylib
│   ├── darwin-x86_64/
│   │   └── libcoral.cldylib
│   ├── linux-x86_64/
│   │   └── libcoral.clso
│   └── windows-x86_64/
│       └── coral.cldll
└── README.md
```

**Pros:**
- ✅ Fast (use pre-compiled if available)
- ✅ Fallback to source compilation
- ✅ Cross-platform

**Cons:**
- ❌ Larger download
- ❌ Complex to manage

---

## 3. Dependency Resolution

### Clorus.lock (Lock File)

```toml
# Clorus.lock - Generated automatically, commit to git
[[package]]
name = "coral"
version = "1.0.0"
source = "registry+https://clorus-packages.dev"
checksum = "a1b2c3d4e5f6..."

dependencies = [
    "clorus-std 0.1.0 (registry+https://clorus-packages.dev)",
]

[[package]]
name = "clorus-std"
version = "0.1.0"
source = "registry+https://clorus-packages.dev"
checksum = "1a2b3c4d5e6f..."
```

### Resolution Algorithm

1. Parse `Clorus.toml` dependencies
2. Fetch package metadata from registry
3. Resolve version constraints (semver)
4. Check `Clorus.lock` for existing resolutions
5. Download packages to `~/.clorus/registry/`
6. Compile if needed, cache artifacts

---

## 4. Package Registry

### Directory Structure

```
https://clorus-packages.dev/
├── api/
│   ├── v1/packages/              # Package metadata
│   ├── v1/download/              # Package downloads
│   └── v1/search/                # Search API
└── index/
    └── coral/
        ├── 1.0.0/
        │   ├── Clorus.toml
        │   ├── coral-1.0.0.clrsp    # Source package
        │   └── checksums.txt
        └── index.json           # All versions
```

### Publishing

```bash
# Publish to registry
clorus publish

# What it does:
# 1. Build library (compile + test)
# 2. Package: .clrsp with source + compiled artifacts
# 3. Upload to registry
# 4. Update index
```

### Local Cache

```
~/.clorus/
├── registry/
│   └── clorus-packages.dev/
│       └── coral/
│           └── 1.0.0/
│               ├── src/
│               └── lib/
│                   ├── libcoral.clro    # Compiled object
│                   └── libcoral.cla     # Static archive
├── git/
│   └── github.com-user-repo-hash/
└── build-cache/
    └── coral-1.0.0-release.cla
```

---

## 5. Usage Examples

### Creating a Library

```bash
# Create new library project
clorus new --lib coral

# Generated Clorus.toml:
[package]
name = "coral"
version = "0.1.0"
type = "library"

[lib]
entry = "src/lib.clrs"
```

### Using a Library

```toml
# In your app's Clorus.toml
[dependencies]
coral = "1.0.0"
```

```clojure
;; In your code
(ns my-app
  (:require [coral.core :as c]
            [coral.widgets :as w]))

(defn -main []
  (c/mount "My App" 800 600 [w/text "Hello!"]))
```

```bash
# Build automatically downloads coral
clorus build

# What happens:
# 1. Reads Clorus.toml
# 2. Resolves dependencies (coral 1.0.0)
# 3. Downloads from registry to ~/.clorus/registry/
# 4. Compiles if needed
# 5. Links into your app
```

---

## 6. Clorus CLI Commands

### New Commands

```bash
# Library management
clorus new --lib mylib           # Create library project
clorus publish                   # Publish to registry
clorus yank 1.0.0                # Remove version from registry

# Dependency management
clorus add coral                 # Add dependency
clorus add coral@1.0.0           # Add specific version
clorus remove coral              # Remove dependency
clorus update                    # Update dependencies
clorus tree                      # Show dependency tree

# Registry
clorus search gui                # Search registry
clorus info coral                # Show package info
clorus login                     # Authenticate to registry
```

---

## 7. Implementation Phases

### Phase 1: Source Distribution (v1.0) - Immediate

**Changes needed:**
- Add `type = "library"` to `Clorus.toml`
- Add `[dependencies]` section
- Simple path-based resolution
- No registry yet (manual git clone/copy)

**Example:**
```toml
[dependencies]
coral = { path = "../coral" }
```

### Phase 2: Compiled Artifacts (v1.5)

**Changes needed:**
- Define `.clr` format (LLVM bitcode + metadata)
- Compiler produces library artifacts
- Link compiled libraries into executables
- Cache compiled artifacts

### Phase 3: Package Registry (v2.0)

**Changes needed:**
- Build package registry infrastructure
- `clorus publish` command
- Version resolution algorithm
- Download and cache management

### Phase 4: Advanced Features (v2.5+)

- Workspaces (monorepo support)
- Private registries
- Build scripts
- Feature flags
- Platform-specific dependencies

---

## 8. Comparison with Other Languages

| Feature | Rust (Cargo) | JavaScript (npm) | Clorus (Proposed) |
|---------|--------------|------------------|-------------------|
| **Format** | Source + compiled | Source only | Hybrid (both) |
| **Registry** | crates.io | npmjs.com | clorus-packages.dev |
| **Lock file** | Cargo.lock | package-lock.json | Clorus.lock |
| **Cache** | ~/.cargo | ~/.npm | ~/.clorus |
| **Versioning** | Semver | Semver | Semver |
| **Workspaces** | ✅ | ✅ | 🔜 Phase 4 |

---

## 9. CORAL Example

### CORAL as a Library

```toml
# coral/Clorus.toml
[package]
name = "coral"
version = "1.0.0"
type = "library"

[lib]
entry = "src/coral/core.clrs"
public = ["src/coral/*.clrs"]
compile = true

[rust-dependencies]
coral-gfx = { path = "rust" }
```

### User Project

```toml
# my-app/Clorus.toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
coral = "1.0.0"

[build]
entry = "src/main.clrs"
```

```clojure
;; my-app/src/main.clrs
(ns my-app
  (:require [coral.core :as c]
            [coral.widgets :as w]))

(defn app []
  [w/column
    [w/text "Hello from CORAL library!"]])

(defn -main []
  (c/mount "My App" 400 300 [app]))
```

```bash
# Build (automatically downloads coral)
cd my-app
clorus build
./target/my-app
```

---

## 10. Next Steps

### For Clorus Language Team

1. **Phase 1 (Essential)**:
   - Add `type = "library"` support
   - Implement path-based dependencies
   - Create `Clorus.lock` format

2. **Phase 2 (Important)**:
   - Define `.clr` compiled format
   - Implement library compilation
   - Add dependency caching

3. **Phase 3 (Future)**:
   - Build package registry
   - Implement `clorus publish`
   - Version resolution

### For CORAL (Now)

**We can distribute CORAL today using git:**

```bash
# Install CORAL
git clone https://github.com/yourname/coral.git ~/.clorus/libs/coral

# Use in project
[dependencies]
coral = { path = "~/.clorus/libs/coral" }
```

Or as a git submodule:
```bash
git submodule add https://github.com/yourname/coral.git libs/coral
```

---

## Summary

**Immediate (Phase 1)**: CORAL distributes as source via git
**Near-term (Phase 2)**: Clorus adds compiled library support
**Long-term (Phase 3)**: Full package registry like crates.io

**The design is ready - we just need to implement it!**
