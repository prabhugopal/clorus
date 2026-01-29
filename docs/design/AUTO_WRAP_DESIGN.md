# Auto-Wrapping Rust APIs for Clorus

## The Problem

**Current approach**: Manually wrap each function
```rust
// Must write this for EVERY function
#[no_mangle]
pub extern "C" fn clorus_fs_read(path: *const c_char) -> *mut c_char {
    unsafe {
        let path_str = CStr::from_ptr(path).to_str().unwrap();
        let content = std::fs::read_to_string(path_str).unwrap();
        CString::new(content).unwrap().into_raw()
    }
}
```

**Limitation**:
- Very tedious - must wrap every function manually
- Only 12/50+ fs functions available
- Can't use other crates without more manual work

## Solution: Auto-Wrapping with Macros

### Approach 1: Declarative Macro

**Define once, generate all wrappers**:

```rust
// In clorus-std/src/lib.rs
use clorus_macros::wrap_module;

wrap_module! {
    rust.fs => std::fs {
        // Just list the functions you want!
        read_to_string(path: String) -> Result<String>,
        write(path: String, contents: String) -> Result<()>,
        create_dir_all(path: String) -> Result<()>,
        metadata(path: String) -> Result<Metadata>,
        read_dir(path: String) -> Result<ReadDir>,
        remove_file(path: String) -> Result<()>,
        remove_dir_all(path: String) -> Result<()>,
        canonicalize(path: String) -> Result<PathBuf>,
        // ... add any function you want
    }
}
```

**This auto-generates**:
```rust
// Generated automatically:
#[no_mangle]
pub extern "C" fn clorus_fs_read_to_string(path: *const c_char) -> *mut Value { ... }

#[no_mangle]
pub extern "C" fn clorus_fs_write(path: *const c_char, contents: *const c_char) -> *mut Value { ... }

#[no_mangle]
pub extern "C" fn clorus_fs_create_dir_all(path: *const c_char) -> *mut Value { ... }
// ... etc for all listed functions
```

### Approach 2: Procedural Macro (More Powerful)

```rust
// In clorus-std/src/fs.rs
use clorus_macros::export_rust_api;

#[export_rust_api(module = "rust.fs")]
impl RustFs {
    // Just write normal Rust!
    pub fn read_to_string(path: String) -> Result<String, Error> {
        std::fs::read_to_string(path)
    }

    pub fn write(path: String, contents: String) -> Result<(), Error> {
        std::fs::write(path, contents)
    }

    pub fn metadata(path: String) -> Result<Metadata, Error> {
        std::fs::metadata(path)
    }

    // The macro handles ALL the FFI boilerplate!
}
```

**Macro generates**:
- C-compatible function signatures
- Automatic type conversions (String ↔ *const c_char)
- Error handling (Result → Value)
- All LLVM declarations in codegen

## Using ANY Rust Crate

### Example: Using `serde_json` crate

```rust
// In clorus-std/src/json.rs
use clorus_macros::wrap_module;

wrap_module! {
    rust.json => serde_json {
        from_str(s: String) -> Result<Value>,
        to_string(value: &Value) -> Result<String>,
        to_string_pretty(value: &Value) -> Result<String>,
    }
}
```

**In Clorus**:
```clojure
(use rust.json)

(def json-str "{\"name\":\"Alice\",\"age\":30}")
(def data (json/from-str json-str))
; data => {:name "Alice" :age 30}

(def output (json/to-string-pretty data))
; output => "{\n  \"name\": \"Alice\",\n  \"age\": 30\n}"
```

### Example: Using `reqwest` for HTTP

```rust
// In clorus-std/src/http.rs
wrap_module! {
    rust.http => reqwest::blocking {
        get(url: String) -> Result<Response>,
    }
}

// Custom wrapper for Response
#[export_rust_api(module = "rust.http")]
impl Response {
    pub fn text(self) -> Result<String> {
        self.text()
    }

    pub fn status(&self) -> u16 {
        self.status().as_u16()
    }
}
```

**In Clorus**:
```clojure
(use rust.http)

(def response (http/get "https://api.github.com"))
(def status (http/status response))
(def body (http/text response))
```

### Example: Using `regex` crate

```rust
wrap_module! {
    rust.regex => regex {
        new(pattern: String) -> Result<Regex>,
    }
}

#[export_rust_api(module = "rust.regex")]
impl Regex {
    pub fn is_match(&self, text: String) -> bool {
        self.is_match(&text)
    }

    pub fn find_all(&self, text: String) -> Vec<String> {
        self.find_iter(&text)
            .map(|m| m.as_str().to_string())
            .collect()
    }
}
```

**In Clorus**:
```clojure
(use rust.regex)

(def pattern (regex/new "\\d+"))
(def text "foo 123 bar 456")
(def numbers (regex/find-all pattern text))
; numbers => ["123" "456"]
```

## Implementation Plan

### Phase 1: Basic Macro (1 week)

Create `clorus-macros` crate with simple wrapping:

```rust
// crates/clorus-macros/src/lib.rs
use proc_macro::TokenStream;

#[proc_macro]
pub fn wrap_module(input: TokenStream) -> TokenStream {
    // Parse the input DSL
    // Generate C-compatible wrappers
    // Generate type conversions
}
```

### Phase 2: Type System Integration (2 weeks)

Handle complex types automatically:
- `String` ↔ `*const c_char`
- `Vec<T>` ↔ `*mut Value` (Vector)
- `HashMap<K,V>` ↔ `*mut Value` (Map)
- `Result<T, E>` ↔ `*mut Value` (with error handling)
- Custom structs ↔ `*mut Value` (Map with fields)

### Phase 3: Full Crate Support (3 weeks)

- Parse Rust documentation
- Auto-discover available functions
- Generate bindings for entire crates
- Smart type inference

## Example: Complete std::fs Access

```rust
// Just declare what you want from std::fs
wrap_module! {
    rust.fs => std::fs {
        // File operations
        read(path: String) -> Result<Vec<u8>>,
        read_to_string(path: String) -> Result<String>,
        write(path: String, contents: String) -> Result<()>,

        // Directory operations
        read_dir(path: String) -> Result<ReadDir>,
        create_dir(path: String) -> Result<()>,
        create_dir_all(path: String) -> Result<()>,
        remove_dir(path: String) -> Result<()>,
        remove_dir_all(path: String) -> Result<()>,

        // Metadata
        metadata(path: String) -> Result<Metadata>,
        symlink_metadata(path: String) -> Result<Metadata>,

        // Path operations
        canonicalize(path: String) -> Result<PathBuf>,
        copy(from: String, to: String) -> Result<u64>,
        rename(from: String, to: String) -> Result<()>,

        // Permissions
        set_permissions(path: String, perm: Permissions) -> Result<()>,

        // Links
        hard_link(src: String, dst: String) -> Result<()>,
        read_link(path: String) -> Result<PathBuf>,

        // And 30+ more functions...
    }
}
```

**All 50+ std::fs functions available in Clorus!**

## Comparison: Manual vs Auto

### Manual Wrapping (Current)
```rust
// Must write ~50 lines per function
#[no_mangle]
pub extern "C" fn clorus_fs_read(path: *const c_char) -> *mut c_char {
    unsafe {
        let path_str = CStr::from_ptr(path).to_str().unwrap();
        match std::fs::read_to_string(path_str) {
            Ok(content) => CString::new(content).unwrap().into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
}
// 12 functions × 50 lines = 600 lines
// And only 12/50+ functions available!
```

### Auto Wrapping (Proposed)
```rust
// Single declaration
wrap_module! {
    rust.fs => std::fs {
        read_to_string(path: String) -> Result<String>,
        // Add 50+ more with just one line each
    }
}
// All 50+ functions with ~100 lines total!
```

## Real-World Example: Using Popular Crates

### CSV Processing
```rust
wrap_module! {
    rust.csv => csv {
        Reader::from_path(path: String) -> Result<Reader>,
    }
}
```

```clojure
(use rust.csv)
(def reader (csv/Reader/from-path "data.csv"))
(def rows (csv/read-all reader))
```

### Database Access (SQLite)
```rust
wrap_module! {
    rust.sqlite => rusqlite {
        Connection::open(path: String) -> Result<Connection>,
    }
}
```

```clojure
(use rust.sqlite)
(def db (sqlite/Connection/open "app.db"))
(def rows (sqlite/query db "SELECT * FROM users"))
```

### Image Processing
```rust
wrap_module! {
    rust.image => image {
        open(path: String) -> Result<DynamicImage>,
    }
}
```

```clojure
(use rust.image)
(def img (image/open "photo.jpg"))
(def resized (image/resize img 800 600))
(image/save resized "thumb.jpg")
```

## Benefits

✅ **Access entire Rust ecosystem** - 100,000+ crates!
✅ **No manual wrapping** - just declare what you want
✅ **Type-safe** - macro handles conversions automatically
✅ **Maintainable** - changes to Rust APIs auto-propagate
✅ **Fast** - direct Rust calls, no overhead
✅ **Flexible** - use any crate, any version

## Next Steps

1. **Create clorus-macros crate** - procedural macro infrastructure
2. **Implement wrap_module! macro** - basic function wrapping
3. **Add type conversions** - handle String, Vec, Result, etc.
4. **Test with std::fs** - prove it works with all 50+ functions
5. **Document pattern** - show users how to add any crate

**Estimated time**: 2-3 weeks for full implementation

Would you like me to start implementing this? It would make Clorus **infinitely more powerful** - access to the entire Rust ecosystem with minimal code!
