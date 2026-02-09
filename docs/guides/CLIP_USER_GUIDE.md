# Clorus Library Packages (.clip) - Complete User Guide

**A comprehensive guide to creating, distributing, and using Clorus libraries**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Why .clip Files?](#why-clip-files)
3. [How .clip Works](#how-clip-works)
4. [Creating a Library](#creating-a-library)
5. [Using Libraries in Your App](#using-libraries-in-your-app)
6. [Distribution Workflows](#distribution-workflows)
7. [REPL Integration](#repl-integration)
8. [Implementation Status](#implementation-status)
9. [Comparison with Other Languages](#comparison-with-other-languages)

---

## Introduction

**.clip files** are Clorus Library Packages - the standard way to distribute and reuse compiled Clorus code. Think of them like:

- **Rust:** `.rlib` / `.crate` files
- **Java:** `.jar` files
- **Ruby:** `.gem` files
- **Python:** `.whl` files

A `.clip` file is a self-contained package that includes everything needed to use a library:
- ✅ Pre-compiled code (no recompilation needed!)
- ✅ Metadata (name, version, dependencies)
- ✅ API definitions (exported functions)
- ✅ Documentation (optional)

---

## Why .clip Files?

### **Problem: Distributing Clorus Libraries**

Without .clip, sharing libraries is painful:

```
❌ BEFORE .clip:
1. Share source code (.clrs files)
2. User must recompile every time
3. No versioning
4. No dependency management
5. Exposes implementation details
```

### **Solution: .clip Packages**

With .clip, distribution is easy:

```
✅ WITH .clip:
1. Package once, use anywhere
2. Pre-compiled (fast builds!)
3. Built-in versioning
4. Automatic dependency resolution
5. Hide implementation, expose API only
```

### **Real-World Benefits**

| Scenario | Without .clip | With .clip |
|----------|---------------|------------|
| **Build Time** | 30s (recompiles stdlib) | 2s (pre-compiled) |
| **Distribution** | Share 50 source files | Share 1 .clip file |
| **Versioning** | Manual tracking | Automatic (clip.toml) |
| **IP Protection** | Source exposed | Binary only |
| **Dependencies** | Manual management | Auto-resolved |

---

## How .clip Works

### **Architecture Overview**

```
┌─────────────────────────────────────────────────────────────┐
│                    .clip Package Structure                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  mylib.clip (ZIP archive)                                    │
│  ├── clip.toml          ← Package metadata                   │
│  ├── lib/                                                     │
│  │   ├── mylib.bc       ← LLVM bitcode (portable)           │
│  │   └── mylib.o        ← Native object (platform-specific) │
│  ├── api/                                                     │
│  │   └── exports.json   ← Public API definitions            │
│  └── docs/                                                    │
│      └── README.md      ← Optional documentation            │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### **Compilation & Linking Flow**

```
LIBRARY CREATION:
┌──────────────┐
│ Source Code  │  mylib/src/core.clrs
│  (.clrs)     │
└──────┬───────┘
       │ clorus build
       ▼
┌──────────────┐
│  Compiled    │  mylib.o, mylib.bc
│   Code       │
└──────┬───────┘
       │ clorus pack
       ▼
┌──────────────┐
│  .clip File  │  mylib-1.0.0.clip
│  (Package)   │
└──────────────┘


APP BUILDING:
┌──────────────┐
│  Your App    │  src/main.clrs
│  (.clrs)     │
└──────┬───────┘
       │
       ├─────────────┐
       │             │
       ▼             ▼
┌──────────────┐  ┌──────────────┐
│  Compile     │  │  Extract     │
│  Your Code   │  │  .clip Deps  │
└──────┬───────┘  └──────┬───────┘
       │                 │
       │                 │ mylib.o
       │                 │
       └────────┬────────┘
                │
                ▼
         ┌──────────────┐
         │  LLVM Linker │
         │    Links     │
         │  Everything  │
         └──────┬───────┘
                │
                ▼
         ┌──────────────┐
         │  Executable  │
         │    (app)     │
         └──────────────┘
```

### **Dependency Resolution Flow**

```
Phase 1: Parse Clorus.toml
─────────────────────────────────────────
[dependencies]
json-parser = { path = "libs/json-parser.clip" }
http-client = "1.2.0"  ← Future: from registry


Phase 2: Resolve & Extract
─────────────────────────────────────────
json-parser.clip  → Extract to /tmp/clorus-clip-json-parser/
                  → Read clip.toml metadata
                  → Read api/exports.json
                  → Locate lib/json-parser.o

http-client.clip  → Download from registry (Phase 4)
                  → Extract & parse


Phase 3: Register External Functions
─────────────────────────────────────────
From api/exports.json:
  - json-parser.core/parse     → Declare in CodeGen
  - json-parser.core/stringify → Declare in CodeGen
  - http-client.request/get    → Declare in CodeGen


Phase 4: Compile Your Code
─────────────────────────────────────────
src/main.clrs:
  (json-parser.core/parse "{}")  → Generates call to declared function
  (http-client.request/get url)  → Generates call to declared function


Phase 5: Link Everything
─────────────────────────────────────────
LLVM Linker:
  main.o + json-parser.o + http-client.o → final executable
  All symbols resolved automatically!
```

---

## Creating a Library

### **Step 1: Write Your Library Code**

```bash
$ clorus new json-parser
$ cd json-parser
```

```clojure
;; src/json.clrs
(ns json-parser.core)

(defn parse [json-str]
  "Parse JSON string into Clorus data structures"
  ;; Implementation...
  {:type :object :data json-str})

(defn stringify [data]
  "Convert Clorus data to JSON string"
  ;; Implementation...
  (str data))

(defn validate [json-str]
  "Check if JSON string is valid"
  ;; Implementation...
  true)
```

### **Step 2: Configure Package Metadata**

```toml
# Clorus.toml
[package]
name = "json-parser"
version = "0.5.0"
authors = ["Your Name <you@example.com>"]
description = "JSON parsing utilities for Clorus"
license = "MIT"
homepage = "https://github.com/you/json-parser"

[dependencies]
# Other .clip libraries this one depends on
string-utils = "1.0.0"

[build]
entry = "src/json.clrs"
src = ["src"]  # Multiple source directories (optional)
```

### **Step 3: Build Your Library**

```bash
$ clorus build
   Compiling json-parser v0.5.0
   [DEBUG] Total expressions to compile: 42
    Generated object file: target/json-parser.o
    Finished dev [unoptimized] target(s) in 0.5s
```

This creates:
- `target/json-parser.o` - Native object file
- `target/json-parser.bc` - LLVM bitcode (if configured)

### **Step 4: Package as .clip**

```bash
$ clorus pack --output json-parser-0.5.0.clip

📦 Packaging Clorus library...
   📝 Package: json-parser v0.5.0
   📄 Output: json-parser-0.5.0.clip
   🔨 Building project...
   📋 Creating metadata...
   📦 Packaging compiled artifacts...
      ✓ Packaged json-parser.o
      ✓ Packaged json-parser.bc
   🔍 Extracting API exports...
   🗜️  Creating .clip archive...
✅ Successfully created json-parser-0.5.0.clip
```

### **Step 5: Distribute**

```bash
# Option 1: Share the file directly
$ cp json-parser-0.5.0.clip ~/shared-libraries/

# Option 2: Publish to registry (Phase 4 - Future)
$ clorus publish json-parser-0.5.0.clip
   Uploading to https://packages.clorus.dev...
   ✅ Published json-parser v0.5.0
```

---

## Using Libraries in Your App

### **Workflow: Automatic Dependency Resolution (Phase 3)**

This is how it **SHOULD** work (like Cargo/deps.edn):

```
┌─────────────────────────────────────────────────────────────┐
│  Step 1: Add Dependency to Clorus.toml                      │
└─────────────────────────────────────────────────────────────┘

# Clorus.toml
[package]
name = "my-app"
version = "1.0.0"

[dependencies]
json-parser = "0.5.0"          # From registry (Phase 4)
http-client = { path = "../http-lib" }  # Local source
math-utils = { path = "libs/math-utils.clip" }  # Local .clip

[build]
entry = "src/main.clrs"


┌─────────────────────────────────────────────────────────────┐
│  Step 2: Use in Your Code                                   │
└─────────────────────────────────────────────────────────────┘

;; src/main.clrs
(ns my-app.main
  (:require [json-parser.core :as json]
            [http-client.request :as http]
            [math-utils.calc :as math]))

(defn -main []
  (let [response (http/get "https://api.example.com/data")
        data (json/parse (:body response))
        result (math/calculate data)]
    (println "Result:" result)))

(-main)


┌─────────────────────────────────────────────────────────────┐
│  Step 3: Just Build - Dependencies Auto-Resolved!           │
└─────────────────────────────────────────────────────────────┘

$ clorus build

   Resolving dependencies...
   ├── json-parser v0.5.0
   │   └── string-utils v1.0.0
   ├── http-client v1.2.0 (../http-lib)
   └── math-utils v2.0.0 (libs/math-utils.clip)

   Extracting json-parser-0.5.0.clip...
   Extracting math-utils-2.0.0.clip...
   Compiling http-client v1.2.0
   Compiling my-app v1.0.0

   [PROGRESS] Compiled 500 expressions (1.2s elapsed)

   Linking libraries...
   ├── json-parser.o
   ├── string-utils.o
   ├── http-client.o
   └── math-utils.o

   Finished dev [unoptimized] target(s) in 1.8s
   Executable: target/my-app


┌─────────────────────────────────────────────────────────────┐
│  Step 4: Run Your App                                       │
└─────────────────────────────────────────────────────────────┘

$ ./target/my-app
Result: 42
```

**NO `clorus install` NEEDED!** Just like Cargo and deps.edn! 🎉

---

## Distribution Workflows

### **Workflow 1: Local Development**

```
Developer's Machine:
────────────────────────────────────────────────
~/projects/
├── json-parser/          ← Your library
│   ├── Clorus.toml
│   └── src/
└── my-app/               ← Your app
    ├── Clorus.toml
    │   [dependencies]
    │   json-parser = { path = "../json-parser" }
    └── src/

$ cd my-app
$ clorus build  # Automatically compiles json-parser
```

### **Workflow 2: Pre-compiled Library**

```
Library Author:
────────────────────────────────────────────────
$ cd json-parser
$ clorus pack
✅ Created json-parser-0.5.0.clip

$ cp json-parser-0.5.0.clip ~/shared/


App Developer:
────────────────────────────────────────────────
$ cd my-app
$ mkdir libs
$ cp ~/shared/json-parser-0.5.0.clip libs/

# Clorus.toml
[dependencies]
json-parser = { path = "libs/json-parser-0.5.0.clip" }

$ clorus build  # Extracts and links automatically
```

### **Workflow 3: Package Registry (Phase 4 - Future)**

```
Library Author:
────────────────────────────────────────────────
$ cd json-parser
$ clorus pack
$ clorus publish
   Uploading to https://packages.clorus.dev...
   ✅ Published json-parser v0.5.0


App Developer:
────────────────────────────────────────────────
# Clorus.toml
[dependencies]
json-parser = "0.5.0"  # Fetched from registry!

$ clorus build
   Downloading json-parser v0.5.0...
   ✅ Downloaded
   Compiling...
```

### **Workflow 4: Team Collaboration**

```
Team Repository Structure:
────────────────────────────────────────────────
company-project/
├── libs/                    ← Shared libraries
│   ├── company-auth.clip
│   ├── company-db.clip
│   └── company-ui.clip
├── services/
│   ├── api-gateway/
│   │   ├── Clorus.toml      [dependencies]
│   │   │                    company-auth = { path = "../../libs/company-auth.clip" }
│   │   └── src/
│   └── user-service/
│       ├── Clorus.toml      [dependencies]
│       │                    company-auth = { path = "../../libs/company-auth.clip" }
│       │                    company-db = { path = "../../libs/company-db.clip" }
│       └── src/
└── README.md

All services share the same pre-compiled libraries!
No need to rebuild common code for each service!
```

---

## REPL Integration

### **Can REPL Use .clip Libraries?**

**YES! ✅** The REPL fully supports .clip libraries using dynamic loading:

#### **Phase 3: REPL Support (WORKING NOW)**
```bash
$ cd factorial-demo  # Project with json-lib dependency
$ clorus repl

Clorus REPL v0.1.0
Type :help for help, :quit to exit

📦 Loaded 1 .clip package(s) for REPL
   ✓ json-lib v0.1.0

examples.factorialλ> (json/json-object-2 "name" "Clorus" "version" "0.2.0")
=> "{\"name\":\"Clorus\",\"version\":\"0.2.0\"}"

examples.factorialλ> (factorial 10)
=> 3628800
```

**How it works:**
1. REPL reads `Clorus.toml` dependencies on startup
2. Extracts .clip packages to temp directories
3. Links .o files as .dylib in `.repl/{package}/` cache
4. Loads dynamic libraries with RTLD_GLOBAL
5. Registers namespaces with CodeGen
6. Functions available immediately - no manual loading needed!

**Cache Benefits:**
- `.repl/` folder caches built .dylib files
- Subsequent REPL sessions reuse cached libraries
- Much faster startup after first run
- Per-project cache isolation

#### **Phase 4: Advanced REPL (FUTURE)**
```bash
$ clorus repl

> (add-deps '{json-parser {:mvn/version "0.5.0"}})
Downloading json-parser v0.5.0...
✅ Loaded

> (require 'json-parser.core :as 'json)
✅ Ready

> (json/parse "{}")
{}
```

**Advanced REPL features:**
- Hot reload .clip files during development
- Load libraries without restarting REPL
- Inspect library metadata
- View API documentation

### **REPL + .clip Workflow**

```
Interactive Development:
────────────────────────────────────────────────

Terminal 1 (Library):          Terminal 2 (REPL):
──────────────────            ──────────────────
$ cd json-parser              $ clorus repl
$ # Edit code...              > (require 'json-parser.core)
$ clorus pack                 ✅ Loaded (old version)
✅ Created json-parser.clip
                              > (reload 'json-parser.core)
                              ✅ Reloaded (new version!)

                              > (json/parse "...")
                              ;; Tests new code instantly!
```

### **REPL-Specific Commands (Phase 4)**

```clojure
;; List loaded libraries
> (loaded-libs)
[:json-parser "0.5.0"
 :http-client "1.2.0"
 :math-utils "2.0.0"]

;; Show library info
> (lib-info 'json-parser)
{:name "json-parser"
 :version "0.5.0"
 :exports [json-parser.core/parse
           json-parser.core/stringify
           json-parser.core/validate]
 :dependencies [:string-utils "1.0.0"]}

;; Reload library
> (reload 'json-parser)
Reloading json-parser v0.5.0...
✅ Reloaded

;; Add library at runtime
> (add-lib 'new-library "1.0.0")
Downloading new-library v1.0.0...
✅ Added

;; Remove library
> (remove-lib 'old-library)
✅ Removed old-library
```

---

## Implementation Status

### **Current Status (February 2026)**

```
Phase 1: Basic Packaging ✅ COMPLETE
├── clorus pack         ✅ Creates .clip archives
├── clorus install      ✅ Installs .clip packages
├── clip.toml format    ✅ Package metadata
├── exports.json        ✅ API definitions (stub)
└── Documentation       ✅ This guide!

Phase 2: Extraction & Parsing ✅ COMPLETE
├── extract_clip()             ✅ Unzip and parse
├── load_clip_dependencies()   ✅ Load from Clorus.toml
├── ClipPackage struct         ✅ Data structure
└── Metadata parsing           ✅ clip.toml + exports.json

Phase 3: Automatic Linking ✅ COMPLETE
├── Integrate in build()       ✅ Auto-loads from Clorus.toml
├── Register external fns      ✅ Namespace registration
├── Static linking (.o files)  ✅ Build and run commands
├── Dynamic linking (.dylib)   ✅ REPL support with .repl/ cache
└── Multiple packages          ✅ Full support

Phase 4: Registry & Advanced ⏳ FUTURE
├── Package registry           ⏳ Planned
├── clorus publish            ⏳ Planned
├── Version resolution        ⏳ Planned
├── REPL hot reload           ⏳ Planned
└── Dependency tree display   ⏳ Planned
```

### **What Works Right Now**

✅ **You can:**
- Create .clip packages: `clorus pack`
- Install .clip packages: `clorus install mylib.clip --local`
- Auto-load dependencies from Clorus.toml in `clorus build`
- Static linking of .clip packages in executables
- Use .clip functions in your code via namespaces
- Load .clip packages in REPL (dynamic loading with .repl/ cache)
- Run programs with .clip dependencies: `clorus run`
- Multiple .clip packages work simultaneously
- Clean output without progress spam

❌ **What doesn't work yet:**
- Package registry (download from internet)
- Version conflict detection
- Incremental compilation / build caching
- Hot reload in REPL

### **Timeline**

| Phase | Features | Status | ETA |
|-------|----------|--------|-----|
| Phase 1 | pack, install, format | ✅ Done | Released |
| Phase 2 | Extract, parse, load | ✅ Done | Released |
| Phase 3 | Auto-linking, REPL | ✅ Done | Released |
| Phase 4 | Registry, hot reload | ⏳ Future | TBD |

---

## Comparison with Other Languages

### **How .clip Compares**

| Feature | Rust Cargo | Java Maven | Ruby Gems | Clorus .clip |
|---------|------------|------------|-----------|--------------|
| **Package Format** | .crate (tar.gz) | .jar (ZIP) | .gem (tar.gz) | .clip (ZIP) |
| **Binary Distribution** | ✅ Yes | ✅ Yes | ❌ No | ✅ Yes |
| **Source Distribution** | ✅ Yes | ✅ Yes | ✅ Yes | ⏳ Future |
| **Auto-dependency** | ✅ Yes | ✅ Yes | ✅ Yes | ⏳ Phase 3 |
| **Registry** | crates.io | Maven Central | rubygems.org | ⏳ Phase 4 |
| **Versioning** | ✅ Semantic | ✅ Semantic | ✅ Semantic | ✅ Semantic |
| **REPL Integration** | N/A | N/A | ✅ Yes | ⏳ Phase 4 |
| **Documentation** | ✅ docs.rs | ✅ JavaDoc | ✅ RDoc | ⏳ Future |

### **Workflow Comparison**

#### **Rust Cargo**
```bash
# Add dependency
$ cargo add serde

# Build (auto-fetches)
$ cargo build
```

#### **Java Maven**
```xml
<!-- Add dependency -->
<dependency>
  <groupId>com.google.gson</groupId>
  <artifactId>gson</artifactId>
  <version>2.8.9</version>
</dependency>

<!-- Build (auto-fetches) -->
$ mvn package
```

#### **Clorus .clip**
```toml
# Add dependency
[dependencies]
json-parser = "0.5.0"

# Build (auto-fetches after Phase 3)
$ clorus build
```

**Result:** Same developer experience across languages! 🎉

---

## Advanced Topics

### **Multi-Platform Support**

.clip packages can include binaries for multiple platforms:

```
mylib.clip/
├── clip.toml
├── lib/
│   ├── mylib-linux-x86_64.o
│   ├── mylib-macos-aarch64.o
│   ├── mylib-windows-x86_64.o
│   └── mylib.bc              ← Portable LLVM bitcode
└── api/
    └── exports.json
```

The compiler automatically selects the correct binary for your platform.

### **Dependency Versioning**

```toml
[dependencies]
# Exact version
json-parser = "0.5.0"

# Version range
http-client = "^1.2.0"  # >= 1.2.0, < 2.0.0

# Wildcard
utils = "1.*"           # Any 1.x version

# Git repository (Phase 4)
special-lib = { git = "https://github.com/user/lib", tag = "v1.0.0" }

# Local path
my-lib = { path = "../my-lib" }
```

### **Private Registries**

```toml
# Use company private registry
[registry]
url = "https://packages.company.com"
token = "${COMPANY_REGISTRY_TOKEN}"

[dependencies]
company-auth = "2.1.0"  # Fetched from private registry
```

---

## Best Practices

### **Library Authors**

✅ **DO:**
- Use semantic versioning (0.5.0, 1.0.0, 2.1.3)
- Document your public API
- Include examples and tests
- Keep .clip packages small (only necessary code)
- Specify dependencies precisely

❌ **DON'T:**
- Break backward compatibility in patch versions
- Include debug symbols in release .clip files
- Expose internal implementation functions
- Create circular dependencies

### **App Developers**

✅ **DO:**
- Pin dependency versions in production
- Use local paths for in-development libraries
- Test with exact versions before releasing
- Document which .clip versions you support

❌ **DON'T:**
- Use `*` wildcards in production
- Mix source and .clip dependencies carelessly
- Commit .clip files to version control (use registry)
- Ignore dependency version conflicts

---

## Troubleshooting

### **Common Issues**

#### **Problem: "Package not found"**
```bash
Error: Could not find package json-parser v0.5.0
```

**Solution:**
- Check package name spelling in Clorus.toml
- Verify .clip file exists at specified path
- For registry packages: check internet connection

#### **Problem: "Symbol not found during linking"**
```bash
Error: Undefined symbol: json_parser_core_parse_2
```

**Solution:**
- .clip package may be outdated
- Recompile the library: `clorus pack`
- Check exports.json matches actual functions

#### **Problem: "Version conflict"**
```bash
Error: Dependency version conflict
  json-parser requires string-utils ^1.0.0
  http-client requires string-utils ^2.0.0
```

**Solution:**
- Update dependencies to compatible versions
- Use `clorus tree` to visualize dependency graph (Phase 4)
- Consider forking one library to use compatible version

---

## FAQ

**Q: Do I need to recompile .clip packages when I update my code?**
A: Only if you're the library author. App developers use pre-compiled .clip files.

**Q: Can I inspect the contents of a .clip file?**
A: Yes! It's a ZIP file:
```bash
$ unzip -l mylib.clip
$ unzip -p mylib.clip clip.toml
```

**Q: How do .clip files handle different CPU architectures?**
A: .clip packages can include multiple binaries or use portable LLVM bitcode.

**Q: Can I use source and .clip for the same library?**
A: Yes! Use `path = "../lib-src"` for source, `path = "libs/lib.clip"` for .clip.

**Q: What happens if two libraries export the same function name?**
A: Namespaces prevent conflicts: `lib1.core/add` vs `lib2.math/add`

**Q: Can REPL hot-reload .clip files?**
A: After Phase 4, yes! Use `(reload 'library-name)` in REPL.

---

## Conclusion

.clip files bring modern package management to Clorus:
- ✅ Fast compilation (pre-compiled libraries)
- ✅ Easy distribution (single file)
- ✅ Version management (built-in)
- ✅ Dependency resolution (automatic)
- ✅ REPL integration (dynamic loading)
- ✅ Multiple packages (works seamlessly)

**Current Status:** Phases 1, 2, and 3 complete. Phase 4 planned.

**Next Steps:**
1. Implement Phase 4 (registry + package download)
2. Add hot reload support in REPL
3. Build community package registry
4. Implement improvements from IMPROVEMENTS.md (P2/P3)

Join us in making Clorus a great systems programming language with first-class library support!

---

**Documentation Version:** 1.1
**Last Updated:** February 2026
**Status:** Phases 1-3 Complete, Phase 4 Planned
