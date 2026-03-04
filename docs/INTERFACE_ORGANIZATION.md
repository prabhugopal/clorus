# Interface File Organization - Best Practices

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/FFI_ROADMAP.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Status:** ✅ Complete
**Date:** January 26, 2025

---

## Standard Directory Structure

```
my-project/
├── Clorus.toml
├── src/
│   └── main.clrs
└── interfaces/          ← All FFI interface files go here
    ├── egui-hello.clorus-ffi
    ├── serde-json.clorus-ffi
    └── reqwest.clorus-ffi
```

---

## Three FFI Approaches (All Supported)

### 1. Auto-Parse (Phase 1) - Quick Iteration
```toml
[rust-dependencies]
simple-lib = { path = "../simple-lib" }
```
- ✅ Zero configuration
- ✅ Perfect for local development
- ⚠️ Modifies library source (adds ffi.rs)

### 2. Auto-Discovery (Phase 2a) - Recommended
```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello", interface = true }
```
- ✅ Clean and simple
- ✅ Auto-discovers: `interfaces/egui-hello.clorus-ffi`
- ✅ No source modification
- ✅ Production-ready

### 3. Explicit Path - Full Control
```toml
[rust-dependencies]
custom-lib = { path = "../custom-lib", interface = "custom/location.clorus-ffi" }
```
- ✅ Custom organization
- ✅ Multiple interfaces per library
- ✅ Shared interface files

---

## Interface File Format

```clojure
;; interfaces/library-name.clorus-ffi
(interface library-name
  (fn function-name [param1 :type1 param2 :type2] :return-type
    "Optional documentation string")

  (fn another-function [] :return-type
    "More documentation"))
```

**Supported Types:**
- `:f64` - Numbers
- `:i32` - 32-bit integers
- `:i64` - 64-bit integers
- `:bool` - Booleans
- `:string` - Strings
- `:unit` - No return value (void)

---

## Usage Example

### 1. Create Interface File

**`interfaces/egui-hello.clorus-ffi`:**
```clojure
(interface egui-hello
  (fn show-gui [message :string] :f64
    "Show a GUI window with the given message")

  (fn get-gui-version [] :string
    "Get the egui version string"))
```

### 2. Reference in Clorus.toml

**Simple auto-discovery:**
```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello", interface = true }
```

**Or explicit path:**
```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello", interface = "interfaces/egui-hello.clorus-ffi" }
```

### 3. Use in Code

```clojure
(ns gui.demo
  (:rust [egui-hello :as gui]))

(gui/show-gui "Hello World!")
```

---

## Naming Conventions

### Function Names
- **Clorus style:** Use kebab-case: `show-gui`, `get-version`
- **Rust conversion:** Automatically converts to snake_case: `show_gui`, `get_version`

### File Names
- **Pattern:** `<library-name>.clorus-ffi`
- **Example:** `egui-hello.clorus-ffi`, `serde-json.clorus-ffi`
- **Location:** Always in `interfaces/` directory

### Library Names
- **Clorus.toml:** Use the library's Cargo.toml name with hyphens
- **Example:** `egui-hello`, `serde-json`, `tokio`

---

## Migration Guide

### From Phase 1 (Auto-parse) to Phase 2a (Interface Files)

**Step 1:** Create interfaces directory
```bash
mkdir interfaces
```

**Step 2:** Create interface file from library's public API
```clojure
;; interfaces/mylib.clorus-ffi
(interface mylib
  (fn public-function [param :type] :return-type))
```

**Step 3:** Update Clorus.toml
```toml
# Before
[rust-dependencies]
mylib = { path = "../mylib" }

# After
[rust-dependencies]
mylib = { path = "../mylib", interface = true }
```

**Step 4:** Remove generated ffi.rs (optional cleanup)
```bash
rm ../mylib/src/ffi.rs  # No longer needed
```

---

## Best Practices

### ✅ DO:
- Put all interface files in `interfaces/` directory
- Use `interface = true` for auto-discovery
- Document functions with docstrings
- Follow kebab-case naming in Clorus
- Version control interface files (commit to git)

### ❌ DON'T:
- Scatter interface files across the project
- Hardcode absolute paths
- Mix auto-parse and interface for same library
- Forget to update interface when library API changes

---

## Community Sharing

### Publishing Interface Files

**Option 1: Include in project**
```
your-repo/
└── interfaces/
    └── popular-crate.clorus-ffi  ← Share with users
```

**Option 2: Separate interface repository**
```
clorus-interfaces/
├── serde-json.clorus-ffi
├── reqwest.clorus-ffi
├── tokio.clorus-ffi
└── README.md
```

Users can reference:
```toml
[rust-dependencies]
serde-json = {
  crates-io = "1.0",
  interface = "~/.clorus/interfaces/serde-json.clorus-ffi"
}
```

### Future: Interface Registry
```bash
# Install community interfaces
clorus install reqwest-interface
clorus install tokio-interface

# Automatically uses: ~/.clorus/interfaces/
```

---

## Troubleshooting

### Error: "Interface file not found"
- Check file exists: `interfaces/<library-name>.clorus-ffi`
- Verify name matches Clorus.toml exactly
- Try explicit path instead of `interface = true`

### Error: "Function not found in library"
- Ensure Rust library exports the function
- Check function name matches (kebab vs snake case)
- Verify Rust library is compiled and linked

### Error: "Parameter type not supported"
- Only use supported types: f64, i32, i64, bool, string, unit
- Complex types (Vec, Option, etc.) not yet supported
- Consider breaking into simpler types

---

## Advantages Over Auto-Parse

| Feature | Auto-Parse | Interface Files |
|---------|-----------|----------------|
| Setup | ✅ Zero | ⚠️ Manual creation |
| Source Modification | ❌ Yes (ffi.rs) | ✅ None |
| crates.io Support | ❌ No | ✅ Yes |
| Documentation | ⚠️ From source | ✅ In interface |
| Community Sharing | ❌ No | ✅ Yes |
| API Stability | ⚠️ Couples to impl | ✅ Decoupled |
| Version Control | ⚠️ Generated code | ✅ Clean interfaces |

**Recommendation:**
- Development: Use auto-parse
- Production: Use interface files

---

## Example: Complete Project

```
my-gui-app/
├── Clorus.toml
├── src/
│   └── main.clrs
└── interfaces/
    ├── egui-hello.clorus-ffi
    ├── serde-json.clorus-ffi
    └── reqwest.clorus-ffi
```

**Clorus.toml:**
```toml
[package]
name = "my-gui-app"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
# Simple libraries: auto-parse
math-utils = { path = "../math-utils" }

# Production libraries: interface files
egui-hello = { path = "../egui-hello", interface = true }
serde-json = { crates-io = "1.0", interface = true }
reqwest = { crates-io = "0.11", interface = true }

[link]
frameworks = ["AppKit", "CoreGraphics", "Metal"]
```

**src/main.clrs:**
```clojure
(ns my-gui-app.core
  (:rust [egui-hello :as gui]
         [serde-json :as json]
         [reqwest :as http]))

(def config (json/parse "{\"title\": \"My App\"}"))
(gui/show-gui "Welcome!")
```

---

## Summary

✅ **Organization:** Standard `interfaces/` directory
✅ **Auto-discovery:** `interface = true`
✅ **Flexibility:** Explicit paths supported
✅ **Professional:** Industry-standard pattern
✅ **Community-ready:** Easy to share

**The Clorus FFI system is now production-ready!**

---

**Document Version:** 1.0
**Last Updated:** January 26, 2025
**See Also:**
- `PHASE2A_INTERFACE_FILES.md` - Interface file design
- `PHASE2A_COMPLETE.md` - Implementation details
- `FFI_ROADMAP.md` - Complete FFI strategy
