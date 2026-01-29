# Code Organization: Interop vs Clojure-Style APIs

## Architecture Overview

We'll organize code in **3 layers**:

```
┌─────────────────────────────────────────┐
│  clorus-core (Clojure-style)           │  ← User-friendly layer
│  slurp, spit, str, println             │
└──────────────┬──────────────────────────┘
               │ calls
               v
┌─────────────────────────────────────────┐
│  clorus-std (Rust interop)             │  ← Low-level wrappers
│  rust.fs, rust.path, rust.io           │
└──────────────┬──────────────────────────┘
               │ wraps
               v
┌─────────────────────────────────────────┐
│  Rust std library                       │  ← Native Rust
│  std::fs, std::path, std::io           │
└─────────────────────────────────────────┘
```

## Layer 1: clorus-std (Low-Level Interop)

**Location**: `/crates/clorus-std/`

**Purpose**: Direct Rust API wrappers - matches Rust API exactly

**Example**:
```rust
// src/fs.rs - Auto-generated wrappers
#[no_mangle]
pub extern "C" fn clorus_fs_read_to_string(path: *const c_char) -> *mut Value {
    // Direct call to std::fs::read_to_string
}

#[no_mangle]
pub extern "C" fn clorus_fs_write(path: *const c_char, content: *const c_char) -> i32 {
    // Direct call to std::fs::write
}
```

**Usage in Clorus**:
```clojure
(use rust.fs)

; Rust-style API - exact match
(fs/read-to-string "file.txt")
(fs/write "file.txt" "content")
(fs/metadata "file.txt")
```

**Characteristics**:
- ✅ Matches Rust API exactly
- ✅ All Rust features available
- ✅ Auto-generated from macros
- ❌ Verbose for simple tasks
- ❌ Rust-style naming (snake_case)

## Layer 2: clorus-core (Clojure-Style)

**Location**: `/crates/clorus-core/` (new crate!)

**Purpose**: Clojure-style convenience functions

**Example**:
```rust
// src/io.rs - Clojure-style wrappers
use clorus_std::fs;

#[no_mangle]
pub extern "C" fn clorus_slurp(path: *const c_char) -> *mut Value {
    // Calls clorus_fs_read_to_string internally
    fs::clorus_fs_read_to_string(path)
}

#[no_mangle]
pub extern "C" fn clorus_spit(path: *const c_char, content: *const c_char) -> *mut Value {
    // Calls clorus_fs_write internally
    fs::clorus_fs_write(path, content);
    // Return nil (like Clojure)
    Value::Nil
}
```

**Usage in Clorus**:
```clojure
(use clorus.core)

; Clojure-style - simple and clean
(slurp "file.txt")
(spit "file.txt" "content")
(println "Hello!")
(str "foo" "bar")
```

**Characteristics**:
- ✅ Simple, clean API
- ✅ Matches Clojure conventions
- ✅ Short function names
- ✅ Built on top of clorus-std
- ✅ lisp-case naming

## Layer 3: User's Code

**Location**: Your `.clrs` files

**Choice**: Use either or both!

```clojure
; Choice 1: Clojure-style (recommended for beginners)
(use clorus.core)

(defn process-file [input output]
  (let [content (slurp input)
        upper (str-upper content)]
    (spit output upper)))

; Choice 2: Rust-style (for advanced users who want full control)
(use rust.fs)

(defn process-file [input output]
  (let [content (fs/read-to-string input)
        upper (str-upper content)]
    (fs/write output upper)))

; Choice 3: Mix both!
(use clorus.core)
(use rust.fs)

(defn process-file [input]
  (let [content (slurp input)           ; Clojure-style
        meta (fs/metadata input)]       ; Rust-style
    (println "Size:" (meta/len meta))))
```

## Directory Structure

```
clorus/
├── crates/
│   ├── clorus-macros/          # Macro infrastructure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs          # wrap_module! macro
│   │
│   ├── clorus-std/             # LAYER 1: Rust interop
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── fs.rs           # rust.fs
│   │       ├── path.rs         # rust.path
│   │       ├── io.rs           # rust.io
│   │       ├── env.rs          # rust.env
│   │       └── ...
│   │
│   ├── clorus-core/            # LAYER 2: Clojure-style (NEW!)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── io.rs           # slurp, spit
│   │       ├── string.rs       # str, str-upper, str-lower
│   │       ├── print.rs        # println, print
│   │       └── ...
│   │
│   └── ...
```

## Implementation Example

### clorus-std/src/fs.rs (Rust-style)

```rust
use clorus_macros::wrap_module;

wrap_module! {
    rust.fs => std::fs {
        // Expose ALL std::fs functions
        read_to_string(path: String) -> Result<String>,
        write(path: String, contents: String) -> Result<()>,
        read(path: String) -> Result<Vec<u8>>,
        create_dir_all(path: String) -> Result<()>,
        metadata(path: String) -> Result<Metadata>,
        // ... 50+ more
    }
}
```

### clorus-core/src/io.rs (Clojure-style)

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// slurp - Read entire file to string (Clojure-style)
#[no_mangle]
pub extern "C" fn clorus_slurp(path: *const c_char) -> *mut c_char {
    unsafe {
        let path_str = CStr::from_ptr(path).to_str().unwrap();
        match std::fs::read_to_string(path_str) {
            Ok(content) => CString::new(content).unwrap().into_raw(),
            Err(e) => {
                eprintln!("Error reading file: {}", e);
                std::ptr::null_mut()
            }
        }
    }
}

/// spit - Write string to file (Clojure-style)
#[no_mangle]
pub extern "C" fn clorus_spit(path: *const c_char, content: *const c_char) -> i32 {
    unsafe {
        let path_str = CStr::from_ptr(path).to_str().unwrap();
        let content_str = CStr::from_ptr(content).to_str().unwrap();
        match std::fs::write(path_str, content_str) {
            Ok(_) => 1,
            Err(e) => {
                eprintln!("Error writing file: {}", e);
                0
            }
        }
    }
}
```

## Comparison: Rust-style vs Clojure-style

### Reading a File

**Rust-style (clorus-std)**:
```clojure
(use rust.fs)

(let [content (fs/read-to-string "data.txt")
      lines (str/split content "\n")]
  (println (count lines)))
```

**Clojure-style (clorus-core)**:
```clojure
(use clorus.core)

(let [content (slurp "data.txt")
      lines (str/split content "\n")]
  (println (count lines)))
```

### Writing a File

**Rust-style**:
```clojure
(use rust.fs)

(fs/write "output.txt" "Hello, World!")
```

**Clojure-style**:
```clojure
(use clorus.core)

(spit "output.txt" "Hello, World!")
```

### Advanced Operations

**Rust-style** (full control):
```clojure
(use rust.fs)

; Get detailed file metadata
(let [meta (fs/metadata "big-file.dat")
      size (meta/len meta)
      modified (meta/modified meta)
      perms (meta/permissions meta)]
  (if (> size 1000000)
    (println "Large file!")
    (println "Small file")))
```

**Clojure-style** (simple):
```clojure
(use clorus.core)

; Just read it
(let [content (slurp "big-file.dat")]
  (if (> (count content) 1000000)
    (println "Large file!")
    (println "Small file")))
```

## Clojure Core Functions to Implement

### I/O Functions
- `slurp` - Read entire file
- `spit` - Write to file
- `line-seq` - Lazy sequence of lines

### String Functions
- `str` - Concatenate strings
- `str/upper-case` - Uppercase
- `str/lower-case` - Lowercase
- `str/split` - Split string
- `str/join` - Join strings
- `str/trim` - Trim whitespace

### Print Functions
- `println` - Print with newline
- `print` - Print without newline
- `pr` - Print readable
- `prn` - Print readable with newline

### Collection Functions
- `map` - Map function over collection
- `filter` - Filter collection
- `reduce` - Reduce collection
- `conj` - Add to collection
- `first`, `rest`, `nth` - Access elements

## Benefits of This Organization

✅ **Separation of Concerns**
- clorus-std = Low-level, complete Rust access
- clorus-core = High-level, simple Clojure-style

✅ **Choice for Users**
- Beginners: Use clorus.core (simple)
- Advanced: Use rust.fs (full control)
- Mix both as needed!

✅ **Maintainability**
- clorus-std is auto-generated
- clorus-core is hand-crafted for simplicity
- Changes to Rust don't break Clojure-style API

✅ **Discoverability**
- Rust users know rust.fs
- Clojure users know slurp/spit
- Both groups feel at home!

## Implementation Priority

### Phase 1: Complete clorus-std (This Week)
1. ✅ Basic fs functions (done!)
2. Implement wrap_module! macro
3. Expose all std::fs
4. Test auto-wrapping

### Phase 2: Create clorus-core (Next Week)
1. Create clorus-core crate
2. Implement slurp, spit
3. Add string functions
4. Add print functions

### Phase 3: Expand Both (Ongoing)
1. clorus-std: Add more rust.* modules
2. clorus-core: Add more Clojure functions
3. Documentation and examples

---

**Summary**: Two libraries, two styles, one language!
- **clorus-std**: For Rust enthusiasts who want full control
- **clorus-core**: For Clojure enthusiasts who want simplicity

Both built on the same foundation, both equally powerful! 🚀
