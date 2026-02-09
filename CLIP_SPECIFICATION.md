# Clorus Library Package (.clip) Specification

## Overview

`.clip` files are Clorus Library Packages - distributable, pre-compiled libraries
that can be linked into Clorus applications without source code.

Similar to:
- Java: `.jar` files
- Rust: `.rlib` / `.crate` files
- Ruby: `.gem` files
- Python: `.whl` files

## Format

A `.clip` file is a ZIP archive containing:

```
mylib.clip/
├── clip.toml          # Package metadata
├── lib/
│   ├── mylib.bc       # LLVM bitcode (portable)
│   └── mylib.o        # Native object file (platform-specific)
├── api/
│   └── exports.json   # Public API definitions
└── docs/
    └── README.md      # Optional documentation
```

## Metadata Format (clip.toml)

```toml
[package]
name = "my-library"
version = "1.0.0"
authors = ["Author Name <email@example.com>"]
description = "A useful Clorus library"
license = "MIT"
homepage = "https://github.com/user/my-library"

[dependencies]
# Other .clip files this library depends on
clorus-json = "0.5.0"
clorus-http = "1.2.0"

[api]
# Public functions and namespaces
exports = [
    "my-library.core/add",
    "my-library.core/subtract",
    "my-library.utils/*",  # Export entire namespace
]

[build]
# Which source files were included
entry = "src/my_library/core.clrs"
include = ["src/**/*.clrs"]
```

## API Exports Format (api/exports.json)

```json
{
  "version": "1.0.0",
  "exports": {
    "my-library.core/add": {
      "type": "function",
      "arities": [2],
      "doc": "Adds two numbers",
      "signature": "(add x y)"
    },
    "my-library.core/subtract": {
      "type": "function",
      "arities": [2],
      "doc": "Subtracts y from x",
      "signature": "(subtract x y)"
    },
    "my-library.utils/format-date": {
      "type": "function",
      "arities": [1, 2],
      "doc": "Formats a date",
      "signatures": [
        "(format-date date)",
        "(format-date date format-str)"
      ]
    }
  }
}
```

## Usage

### Creating a .clip Package

```bash
# From source directory
$ clorus pack --entry src/core.clrs --output mylib.clip

# Or use Clorus.toml
$ clorus pack
```

### Installing a .clip Package

```bash
# Install globally
$ clorus install mylib.clip

# Install to project
$ clorus install --local mylib.clip
```

### Using in Code

```clojure
;; In Clorus.toml
[dependencies]
mylib = { path = "./libs/mylib.clip" }

;; In code
(ns my-app.main
  (:require [my-library.core :as lib]))

(defn -main []
  (println (lib/add 10 20)))
```

### Linking Process

1. Compiler reads `Clorus.toml` dependencies
2. Locates `.clip` files (global or local)
3. Extracts `.bc` or `.o` files
4. Links them with LLVM linker
5. Resolves symbols using `api/exports.json`

## Distribution

### Package Registry (Future)

```bash
# Publish to registry
$ clorus publish mylib.clip

# Install from registry
$ clorus add my-library@1.0.0
```

### Local Distribution

```bash
# Copy .clip file to project
$ cp mylib.clip libs/

# Reference in Clorus.toml
[dependencies]
mylib = { path = "libs/mylib.clip" }
```

## Benefits

1. **No Source Required**: Distribute compiled libraries
2. **Fast Compilation**: No need to recompile dependencies
3. **Versioning**: Track library versions easily
4. **Portability**: LLVM bitcode works across platforms
5. **Security**: Only expose public API, hide implementation

## Implementation Phases

### Phase 1: Basic Packaging (This PR)
- Create .clip archive from compiled .o/.bc files
- Package metadata (clip.toml)
- API exports extraction

### Phase 2: Linking Support
- Read .clip files during compilation
- Extract and link object files
- Symbol resolution

### Phase 3: Dependency Management
- Install/uninstall commands
- Version resolution
- Dependency tree management

### Phase 4: Registry
- Central package repository
- Publishing workflow
- Package discovery

## Example: Creating a Library

```bash
# 1. Write library code
$ mkdir clorus-json && cd clorus-json

# 2. Create source files
$ cat > src/clorus_json/core.clrs <<EOF
(ns clorus-json.core)

(defn parse [json-str]
  ;; Implementation...
  )

(defn stringify [data]
  ;; Implementation...
  )
EOF

# 3. Create package metadata
$ cat > Clorus.toml <<EOF
[package]
name = "clorus-json"
version = "0.1.0"

[build]
entry = "src/clorus_json/core.clrs"
EOF

# 4. Build and package
$ clorus build
$ clorus pack --output clorus-json-0.1.0.clip

# 5. Distribute
$ cp clorus-json-0.1.0.clip ~/clorus-packages/
```

## Advantages Over Source Distribution

| Aspect | Source (`.clrs`) | Package (`.clip`) |
|--------|------------------|-------------------|
| Compilation Speed | Slow (recompile every time) | Fast (pre-compiled) |
| IP Protection | Source visible | Source hidden |
| Versioning | Manual | Automated |
| Dependencies | Manual | Automatic resolution |
| Distribution | Multiple files | Single file |
| API Documentation | Scattered | Centralized in exports.json |

## Future Enhancements

1. **Digital Signatures**: Verify package authenticity
2. **Platform Variants**: Different .o files per platform in same .clip
3. **Source Maps**: Optional debug info
4. **Incremental Compilation**: Only recompile changed modules
5. **Tree Shaking**: Remove unused exports

## File Extension

`.clip` stands for **Clorus Library Package**
- Memorable and related to Clojure (clip/clj)
- Unique file extension
- Easy to associate with Clorus ecosystem
