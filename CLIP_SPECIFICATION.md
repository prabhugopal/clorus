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

### REPL Integration

The REPL fully supports .clip packages through dynamic library loading:

```bash
$ cd my-app  # Project with .clip dependencies
$ clorus repl

Clorus REPL v0.1.0
📦 Loaded 2 .clip package(s) for REPL
   ✓ json-parser v0.5.0
   ✓ http-client v1.2.0

my-app.mainλ> (lib/add 10 20)
=> 30
```

**How REPL Loading Works:**

1. **Extract**: .clip packages extracted to temp directories
2. **Build**: .o files linked as .dylib libraries
3. **Cache**: Built .dylib cached in `.repl/{package}/` for reuse
4. **Load**: Dynamic libraries loaded with RTLD_GLOBAL
5. **Register**: Namespaces registered with CodeGen
6. **Ready**: All functions immediately available

**Cache Benefits:**
- First REPL session: ~2-3 seconds to build .dylib
- Subsequent sessions: ~100ms to load cached .dylib
- Per-project isolation (each project has own `.repl/` folder)
- Automatically rebuilds if .clip file changes

### Linking Process

When you run `clorus build` or `clorus run` with .clip dependencies:

1. Compiler reads `Clorus.toml` dependencies
2. For each .clip dependency:
   - Extracts .clip ZIP to temp directory
   - Reads `api/exports.json` to discover available functions
   - Registers external function declarations in CodeGen
3. Compiles your source code
   - When code calls `(my-library.core/add 1 2)`
   - Compiler knows function exists (from exports.json)
   - Generates LLVM call to mangled symbol: `clorus_my_library_core_add_2`
4. LLVM linking phase:
   - Links your compiled bitcode
   - WITH library bitcode from .clip packages
   - LLVM linker resolves all symbols automatically
5. Generates final executable

**Example:**

```toml
# Your app's Clorus.toml
[package]
name = "my-app"
version = "1.0.0"

[dependencies]
json-parser = { path = "./libs/json-parser-0.5.0.clip" }
http-client = { path = "./libs/http-client-1.2.0.clip" }

[build]
entry = "src/main.clrs"
```

```clojure
;; Your app code (src/main.clrs)
(ns my-app.main
  (:require [json-parser.core :as json]
            [http-client.request :as http]))

(defn -main []
  (let [response (http/get "https://api.example.com/data")
        data (json/parse (:body response))]
    (println "Received:" data)))
```

When you run `clorus build`, the compiler:
1. Extracts json-parser-0.5.0.clip and http-client-1.2.0.clip
2. Registers `json-parser.core/parse` and `http-client.request/get` as external functions
3. Compiles your main.clrs
4. Links everything together automatically
5. You get a working executable with zero manual linking!

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

### Phase 1: Basic Packaging ✅ COMPLETE
- Create .clip archive from compiled .o/.bc files
- Package metadata (clip.toml)
- API exports extraction
- Install command for local .clip files

### Phase 2: Extraction & Parsing ✅ COMPLETE
- Read .clip files during compilation
- Extract and parse metadata
- Load dependencies from Clorus.toml
- ClipPackage data structures

### Phase 3: Automatic Linking ✅ COMPLETE
- Static linking (.o files) in `clorus build`
- Dynamic linking (.dylib files) in `clorus repl`
- Namespace registration with CodeGen
- Symbol resolution and LLVM linking
- Multiple .clip package support
- `.repl/` cache for fast REPL startup

### Phase 4: Registry & Advanced ⏳ FUTURE
- Central package repository
- Publishing workflow (clorus publish)
- Package discovery and download
- Version conflict detection
- Hot reload in REPL
- Dependency tree visualization

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
