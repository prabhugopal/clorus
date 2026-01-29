# Phase 2a: Interface Files - COMPLETE

**Status:** ✅ Complete
**Completion Date:** January 26, 2025
**Duration:** 2-3 hours

---

## Summary

Interface files (.clorus-ffi) allow explicit FFI declarations without modifying Rust library source code. This enables:
- Using crates.io libraries directly
- Community-shareable interface definitions
- Better API documentation
- **Backward compatibility:** Auto-parse (Phase 1) still works

---

## What Was Implemented

### 1. Interface File Format

```clojure
;; library-name.clorus-ffi
(interface library-name
  (fn function-name [param1 :type1 param2 :type2] :return-type
    "Optional docstring")

  (fn another-function [] :return-type
    "Documentation"))
```

**Key Design Decision:** Use `(fn ...)` instead of `(defn ...)` to avoid conflicts with Clorus's parser treating defn as a special form.

### 2. Parser Implementation

**File:** `crates/clorus-cli/src/interface.rs`

- `parse_interface_file()` - Main entry point
- Handles Clorus parser's Call expressions (treats `(interface ...)` as function call)
- Converts hyphen-case to snake_case for Rust compatibility
- Extracts function signatures, parameters, return types, and docs

### 3. Manifest Support

**File:** `crates/clorus-cli/src/manifest.rs`

Updated `RustDependency` enum to support interface files:
```rust
pub enum RustDependency {
    WithInterface {
        path: String,
        interface: String,
    },
    Path {
        path: String,
    },
    Simple(String),
}
```

Usage in Clorus.toml:
```toml
[rust-dependencies]
egui-hello = { path = "../egui-hello", interface = "egui-hello.clorus-ffi" }
```

### 4. FFI Processor Integration

**File:** `crates/clorus-cli/src/rust_ffi.rs`

- Added `create_wrapper_from_interface()` method
- Modified `process_dependencies()` to check for interface files
- Converts interface function signatures to `FunctionInfo`
- Generates same FFI wrappers as auto-parse

**Branch logic:**
```rust
if let Some(interface_path) = dep.get_interface() {
    // Phase 2a: Use interface file
    Self::create_wrapper_from_interface(...)
} else {
    // Phase 1: Auto-parse (backward compat)
    Self::create_and_compile_wrapper(...)
}
```

---

## Test Case: egui-hello

### Interface File
**Location:** `gui-demo/egui-hello.clorus-ffi`

```clojure
;; Interface file for egui-hello library
(interface egui-hello
  (fn show-gui [message :string] :f64 "Show a GUI window")
  (fn get-gui-version [] :string "Get the egui version"))
```

### Clorus Code (unchanged)
```clojure
(ns gui.demo
  (:rust [egui-hello :as gui]))

(gui/show-gui "Hello from interface files!")
5
```

### Result
✅ Compiles successfully
✅ GUI displays correctly
✅ No source modification needed
✅ Clean output (no debug messages)

---

## Type Mapping

| Clorus Type | Rust Type | LLVM Type | Usage |
|-------------|-----------|-----------|-------|
| `:f64` | `f64` | `double` | Numbers |
| `:i32` | `i32` | `i32` | Integers |
| `:bool` | `bool` | `i1` | Booleans |
| `:string` | `String` | `i8*` | Null-terminated C strings |
| `:unit` | `()` | `void` | No return value |

---

## Key Technical Solutions

### 1. Parser Challenge
**Problem:** Clorus parser treats `(interface ...)` as a Call expression, not a List.

**Solution:** Handle both Call and List expressions in parser:
```rust
match &exprs[0] {
    Expr::Call { func, args } => {
        if func == "interface" {
            return parse_interface_from_call(args);
        }
    }
    Expr::List(items) => {
        // Handle list syntax too
    }
    _ => Err(...)
}
```

### 2. Naming Convention
**Problem:** Clorus uses hyphen-case (`show-gui`), Rust uses snake_case (`show_gui`).

**Solution:** Convert in FFI processor:
```rust
let rust_name = f.name.replace('-', "_");
```

### 3. Special Form Conflict
**Problem:** Using `(defn ...)` caused parser errors because defn is a special form.

**Solution:** Use `(fn ...)` for interface files instead.

---

## Backward Compatibility

**100% Maintained:**

### Auto-Parse Still Works
```toml
[rust-dependencies]
simple-lib = { path = "../simple-lib" }  # No interface = auto-parse
```

### Mixed Approach
```toml
[rust-dependencies]
# Simple library: auto-parse
math-lib = { path = "../math-lib" }

# Complex library: interface file
http-client = { path = "../http-client", interface = "http.clorus-ffi" }
```

Both work in the same project!

---

## Files Created/Modified

### Created
- `crates/clorus-cli/src/interface.rs` (267 lines)
- `gui-demo/egui-hello.clorus-ffi` (interface file example)
- `docs/PHASE2A_INTERFACE_FILES.md` (design document)
- `docs/PHASE2A_COMPLETE.md` (this document)

### Modified
- `crates/clorus-cli/src/main.rs` - Added interface module
- `crates/clorus-cli/src/manifest.rs` - Added WithInterface variant
- `crates/clorus-cli/src/rust_ffi.rs` - Added create_wrapper_from_interface()
- `gui-demo/Clorus.toml` - Updated to use interface file

---

## Benefits

### For Users
- **No source modification** - Use crates.io directly
- **Clear documentation** - Interfaces document the API
- **Flexibility** - Choose auto-parse OR interface per-library
- **Backward compatible** - Existing code keeps working

### For Community
- **Shareable interfaces** - Community can maintain interfaces
- **Professional FFI** - Matches OCaml .mli pattern
- **Future: Registry** - Can build interface registry

---

## Future Enhancements

### 1. Complex Types
```clojure
;; Option<T>
(fn maybe-find [key :string] [:option :string])

;; Result<T, E>
(fn try-parse [s :string] [:result :f64 :string])

;; Vec<T>
(fn get-items [] [:vec :string])

;; Custom structs
(deftype HttpResponse
  {:status :i32
   :headers [:map :string :string]
   :body :string})
```

### 2. Interface Registry
```bash
clorus install serde-json-interface
clorus install reqwest-interface
```

### 3. Interface Validation
- Check that Rust library exports declared functions
- Verify type signatures match
- Warn about breaking changes

---

## Success Criteria (All Met)

1. ✅ Parse .clorus-ffi files correctly
2. ✅ Convert to RustLibrary format
3. ✅ Generate same FFI wrappers as auto-parse
4. ✅ gui-demo works with interface file
5. ✅ Auto-parse still works (backward compat)
6. ✅ Mixed projects work (some auto, some interface)
7. ✅ Clean output (no debug messages)

---

## Performance

- **Parse time:** < 1ms (interface files are small)
- **Compile time:** Same as auto-parse
- **Runtime:** Zero overhead (same generated code)

---

## Next: Phase B - Vector/Map Literals

Now that FFI is complete, we can move to language features:
- `[1 2 3]` - Vector literals
- `{:key "value"}` - Map literals
- Collections work with Value* system

---

**Document Version:** 1.0
**Last Updated:** January 26, 2025
**Status:** Phase 2a Complete, Moving to Phase B
