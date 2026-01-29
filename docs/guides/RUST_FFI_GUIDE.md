# Rust FFI Integration for Clorus

## Can I Use Rust Functions from Clorus?

**YES!** Clorus can call Rust functions through FFI (Foreign Function Interface). This lets you:
- Use Rust std library
- Import any Rust crate
- Call system libraries
- Write performance-critical code in Rust

## How It Works

### Architecture

```
┌─────────────┐         ┌─────────────┐
│   Clorus    │ calls   │    Rust     │
│   Code      ├────────>│  Functions  │
│ (.clrs)     │   FFI   │   (.rs)     │
└─────────────┘         └─────────────┘
       │                       │
       │                       │
       v                       v
┌─────────────────────────────────┐
│         LLVM IR                 │
│  (compiled together)            │
└─────────────────────────────────┘
```

## Current State

### ✅ What Works Now

The infrastructure is ready:
- LLVM can link external functions
- Runtime library already uses FFI (`clorus_vector_conj`, etc.)
- JIT execution can load dynamic libraries

### ⏳ What's Needed

**Parser Support** for `extern` declarations:
```clojure
; Declare an external C function
(extern "C" function_name [arg-types...] return-type)
```

**Codegen Support** to call external functions:
- Declare external functions in LLVM
- Generate proper calling conventions
- Handle type conversions

## Implementation Guide

### Phase 1: Simple C Functions (1-2 hours)

#### 1. Parser: Add `extern` Form

```rust
// In clorus-syntax/src/parser.rs
pub enum Expr {
    // ... existing variants
    Extern {
        abi: String,         // "C", "Rust", etc.
        name: String,        // Function name
        params: Vec<Type>,   // Parameter types
        return_type: Type,   // Return type
    },
}
```

#### 2. Codegen: Declare External Functions

```rust
// In clorus-codegen/src/codegen.rs
Expr::Extern { abi, name, params, return_type } => {
    // Convert types to LLVM types
    let llvm_params: Vec<_> = params.iter()
        .map(|t| self.convert_type(t))
        .collect();

    let llvm_return = self.convert_type(return_type);

    // Declare the function
    let fn_type = llvm_return.fn_type(&llvm_params, false);
    self.module.add_function(name, fn_type, None);

    Ok(self.context.f64_type().const_float(0.0))
}
```

#### 3. Usage in Clorus

```clojure
; Simple math function
(extern "C" sqrt [f64] f64)
(def x (sqrt 16.0))  ; => 4.0

; String length
(extern "C" strlen [ptr] i64)
```

### Phase 2: Rust Functions with Types (2-3 hours)

#### Example: Using Rust regex crate

**1. Create a Rust library:**
```rust
// rust-utils/src/lib.rs
use regex::Regex;

#[no_mangle]
pub extern "C" fn rust_regex_match(pattern: *const i8, text: *const i8) -> bool {
    unsafe {
        let pattern_str = CStr::from_ptr(pattern).to_str().unwrap();
        let text_str = CStr::from_ptr(text).to_str().unwrap();

        let re = Regex::new(pattern_str).unwrap();
        re.is_match(text_str)
    }
}
```

**2. Use from Clorus:**
```clojure
; Load the library
(load-library "librust_utils.so")

; Declare the function
(extern "C" rust_regex_match [ptr ptr] bool)

; Use it
(def matches (rust_regex_match "^hello" "hello world"))  ; => true
```

### Phase 3: High-Level Rust Integration (1 week)

#### Auto-generate FFI Bindings

**Rust Side** - Use `cbindgen` or custom macro:
```rust
// Macro to auto-export
#[clorus_export]
pub fn process_data(input: Vec<i32>) -> Vec<i32> {
    input.iter().map(|x| x * 2).collect()
}

// Generates:
// 1. C-compatible wrapper
// 2. Clorus binding declaration
```

**Clorus Side** - Auto-import:
```clojure
; Import entire Rust module
(import rust.utils
  [process-data
   validate-input
   format-output])

; Use with Clojure-like syntax
(def result (process-data [1 2 3 4]))  ; => [2 4 6 8]
```

## Real-World Examples

### Example 1: Using Rust's Regex

**Rust library:**
```rust
// crates/clorus-rust-utils/src/lib.rs
use regex::Regex;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn rust_regex_find(
    pattern: *const c_char,
    text: *const c_char,
) -> *mut c_char {
    unsafe {
        let pattern_str = CStr::from_ptr(pattern).to_str().unwrap();
        let text_str = CStr::from_ptr(text).to_str().unwrap();

        let re = Regex::new(pattern_str).unwrap();

        if let Some(mat) = re.find(text_str) {
            CString::new(mat.as_str()).unwrap().into_raw()
        } else {
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_string_free(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}
```

**Clorus usage:**
```clojure
; my-app.clrs
(extern "C" rust_regex_find [ptr ptr] ptr)
(extern "C" rust_string_free [ptr] void)

(defn find-pattern [pattern text]
  (let [result (rust_regex_find pattern text)]
    (if (null? result)
      nil
      (let [str (ptr->string result)]
        (rust_string_free result)
        str))))

; Use it
(def email (find-pattern "[a-z]+@[a-z]+\\.com"
                         "Contact: john@example.com"))
; => "john@example.com"
```

### Example 2: Using HTTP Client

**Rust library:**
```rust
use reqwest::blocking::Client;

#[no_mangle]
pub extern "C" fn rust_http_get(url: *const c_char) -> *mut c_char {
    unsafe {
        let url_str = CStr::from_ptr(url).to_str().unwrap();

        let client = Client::new();
        match client.get(url_str).send() {
            Ok(response) => match response.text() {
                Ok(text) => CString::new(text).unwrap().into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    }
}
```

**Clorus usage:**
```clojure
(extern "C" rust_http_get [ptr] ptr)
(extern "C" rust_string_free [ptr] void)

(defn http-get [url]
  (let [result (rust_http_get url)]
    (if (null? result)
      nil
      (let [body (ptr->string result)]
        (rust_string_free result)
        body))))

; Fetch data
(def body (http-get "https://api.github.com/users/octocat"))
(println body)
```

### Example 3: Using File System

**Use Rust std directly:**
```clojure
; Declare libc functions
(extern "C" fopen [ptr ptr] ptr)
(extern "C" fread [ptr i64 i64 ptr] i64)
(extern "C" fclose [ptr] i32)

; Or use Rust wrapper
(extern "C" rust_read_file [ptr] ptr)

(defn read-file [path]
  (let [content (rust_read_file path)]
    (ptr->string content)))
```

## Type Mapping

| Clorus Type | Rust Type | LLVM Type | Notes |
|-------------|-----------|-----------|-------|
| `f64` | `f64` | `double` | Numbers |
| `i32` | `i32` | `i32` | Integers |
| `i64` | `i64` | `i64` | Large integers |
| `bool` | `bool` | `i1` | Booleans |
| `ptr` | `*mut T` | `i8*` | Pointers |
| `string` | `*const c_char` | `i8*` | C strings |
| `[f64]` | `*const f64` | `double*` | Arrays |

## Safety Considerations

### Unsafe Operations

FFI is inherently unsafe! Need to:
1. **Validate pointers** - Check for null before dereferencing
2. **Memory management** - Who owns the memory?
3. **String encoding** - UTF-8 vs C strings
4. **Error handling** - C functions don't panic

### Safe Wrapper Pattern

```clojure
; Low-level unsafe FFI
(extern "C" rust_unsafe_operation [ptr i32] i32)

; High-level safe wrapper
(defn safe-operation [data count]
  (if (and (valid? data) (> count 0))
    (rust_unsafe_operation data count)
    (error "Invalid arguments")))
```

## Build Integration

### Cargo.toml Setup

```toml
# Root workspace
[workspace]
members = [
    "crates/clorus",
    "crates/clorus-rust-utils",  # NEW: Rust FFI library
]

# In clorus-rust-utils/Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]  # Build as dynamic library

[dependencies]
regex = "1.0"
reqwest = { version = "0.11", features = ["blocking"] }
```

### Loading Libraries

**Option 1: Static linking (compile time)**
```bash
cargo build
# Libraries linked automatically
clorus run
```

**Option 2: Dynamic loading (runtime)**
```clojure
; Load at runtime
(load-library "target/debug/libclorus_rust_utils.dylib")
(extern "C" rust_function [args] return)
```

## Implementation Roadmap

### Week 1: Basic FFI
- [ ] Add `extern` syntax to parser
- [ ] Generate LLVM external declarations
- [ ] Test with simple C math functions
- [ ] Test with libc (printf, malloc, etc.)

### Week 2: Rust Integration
- [ ] Create `clorus-rust-utils` crate
- [ ] Add string handling helpers
- [ ] Build system integration
- [ ] Test with regex, HTTP, file I/O

### Week 3: High-Level API
- [ ] Auto-generate bindings from Rust
- [ ] Type-safe wrappers
- [ ] Error handling integration
- [ ] Documentation

### Week 4: Standard Library
- [ ] Core functions (map, filter, reduce in Rust)
- [ ] String operations
- [ ] File I/O
- [ ] HTTP client
- [ ] JSON parsing

## Example Project Structure

```
clorus/
├── crates/
│   ├── clorus/           # Core Clorus
│   ├── clorus-runtime/   # Persistent data structures
│   ├── clorus-rust-std/  # NEW: Rust std library bindings
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── string.rs    # String operations
│   │   │   ├── fs.rs        # File system
│   │   │   ├── http.rs      # HTTP client
│   │   │   └── json.rs      # JSON parsing
│   │   └── Cargo.toml
│   └── clorus-cli/
└── examples/
    └── rust-ffi-demo/
        ├── Clorus.toml
        └── src/
            └── main.clrs    # Uses Rust functions
```

## Quick Start: Minimal Example

Want to try it now? Here's a minimal working example:

**1. Create Rust function:**
```rust
// Add to clorus-runtime/src/lib.rs
#[no_mangle]
pub extern "C" fn hello_from_rust() -> f64 {
    println!("Hello from Rust!");
    42.0
}
```

**2. Declare in LLVM:**
```rust
// In codegen, add to declare_runtime_functions():
let hello_type = self.context.f64_type().fn_type(&[], false);
self.module.add_function("hello_from_rust", hello_type, None);
```

**3. Call from Clorus:**
```clojure
; If we had extern support:
(extern "C" hello_from_rust [] f64)
(def result (hello_from_rust))  ; Prints: Hello from Rust!
```

## Benefits

1. **Performance** - Critical code in Rust
2. **Ecosystem** - Use any Rust crate (regex, HTTP, crypto, etc.)
3. **Safety** - Rust's safety guarantees
4. **Gradual** - Start with Clorus, optimize with Rust
5. **Interop** - Best of both worlds

## Questions?

**Q: Can I use Rust std lib functions directly?**
A: Yes! Just declare them with `extern`.

**Q: Can I use any Rust crate?**
A: Yes, if it has C-compatible exports.

**Q: Is it safe?**
A: FFI is unsafe, but you can wrap it safely.

**Q: Performance cost?**
A: Minimal - it's just a function call.

---

**Status**: Infrastructure ready, `extern` syntax needed (1-2 days work)

**Want this feature?** Let me know and I can implement the `extern` support! 🚀
