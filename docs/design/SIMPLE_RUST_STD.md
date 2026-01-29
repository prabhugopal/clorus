# Simple Rust Std Library Import for Clorus

## Philosophy: Clojure Simplicity + Rust Power

Keep Clojure's elegance while accessing Rust's std library:

```clojure
; Simple, clean imports - like Clojure
(use rust.fs)
(use rust.path)
(use rust.env)

; Call like Clojure functions
(def content (fs/read "data.txt"))
(def exists? (path/exists? "file.txt"))
(def home (env/var "HOME"))

; No complexity, no boilerplate!
```

## What We Import: Rust Std Lib Only

Focus on the most useful parts of Rust's standard library:

### File System
```clojure
(use rust.fs)

; Reading
(def text (fs/read "file.txt"))           ; read to string
(def bytes (fs/read-bytes "data.bin"))    ; read bytes

; Writing
(fs/write "output.txt" "Hello")           ; write string
(fs/append "log.txt" "New entry\n")       ; append

; Info
(def size (fs/size "file.txt"))           ; file size
(def modified (fs/modified "data.json"))  ; last modified

; Operations
(fs/copy "src.txt" "dst.txt")             ; copy file
(fs/rename "old.txt" "new.txt")           ; rename
(fs/remove "temp.txt")                    ; delete
(fs/create-dir "new-folder")              ; mkdir
```

### Path Operations
```clojure
(use rust.path)

(def exists? (path/exists? "/tmp/file"))
(def is-file? (path/file? "data.txt"))
(def is-dir? (path/dir? "folder"))
(def parent (path/parent "/home/user/file.txt"))  ; => "/home/user"
(def filename (path/filename "/path/to/file.txt")) ; => "file.txt"
(def ext (path/extension "doc.pdf"))               ; => "pdf"
```

### Environment
```clojure
(use rust.env)

(def home (env/var "HOME"))
(def path (env/var "PATH"))
(env/set-var "DEBUG" "true")
(def cwd (env/current-dir))
(def args (env/args))  ; command line args
```

### Process
```clojure
(use rust.process)

; Run command
(def output (process/run "ls" ["-la"]))
(def status (process/status "git" ["status"]))

; With output
(let [result (process/output "echo" ["hello"])]
  (println result.stdout))
```

### Collections
```clojure
(use rust.vec)
(use rust.map)

; Vectors (in addition to our persistent ones)
(def v (vec/new))
(vec/push! v 1)
(vec/push! v 2)
(def item (vec/get v 0))

; HashMaps
(def m (map/new))
(map/insert! m "key" "value")
(def val (map/get m "key"))
```

### Strings
```clojure
(use rust.str)

(def trimmed (str/trim "  hello  "))
(def upper (str/upper "hello"))
(def lower (str/lower "HELLO"))
(def parts (str/split "a,b,c" ","))
(def joined (str/join ["a" "b" "c"] ","))
(def contains? (str/contains? "hello world" "world"))
(def starts? (str/starts-with? "hello" "hel"))
```

### Time
```clojure
(use rust.time)

(def now (time/now))
(def unix (time/unix-timestamp))
(def formatted (time/format now "%Y-%m-%d"))
(time/sleep 1000)  ; sleep 1000ms
```

## Implementation: Pre-Generated Wrappers

Instead of auto-generating for any crate, we **ship pre-made wrappers** for std lib:

```
clorus-std/
├── src/
│   ├── fs.rs        # File system wrappers
│   ├── path.rs      # Path wrappers
│   ├── env.rs       # Environment wrappers
│   ├── str.rs       # String wrappers
│   └── time.rs      # Time wrappers
└── bindings.clrs    # Clorus declarations
```

### Example: fs.rs
```rust
// Pre-written, tested, documented wrapper
use std::fs;

#[no_mangle]
pub extern "C" fn clorus_fs_read(
    path_ptr: *const u8,
    path_len: usize,
) -> *mut String {
    let path = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(path_ptr, path_len))
    };

    match fs::read_to_string(path) {
        Ok(content) => Box::into_raw(Box::new(content)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn clorus_fs_write(
    path_ptr: *const u8,
    path_len: usize,
    content_ptr: *const u8,
    content_len: usize,
) -> bool {
    let path = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(path_ptr, path_len))
    };
    let content = unsafe {
        std::slice::from_raw_parts(content_ptr, content_len)
    };

    fs::write(path, content).is_ok()
}
```

### Example: bindings.clrs
```clojure
; These are built-in, no need to write!
(module rust.fs
  (defn read [path] ...)
  (defn write [path content] ...)
  (defn exists? [path] ...)
  ; ... etc
)
```

## Simple Syntax

### Importing
```clojure
; Just the module name
(use rust.fs)
(use rust.path)

; Import specific functions
(use rust.fs [read write])

; Import with alias
(use rust.fs as file)
(file/read "data.txt")
```

### Calling
```clojure
; Module/function style (like Clojure namespaces)
(fs/read "file.txt")
(path/exists? "data.json")
(env/var "HOME")

; Or direct if imported
(use rust.fs [read write])
(read "file.txt")
(write "output.txt" "content")
```

### Error Handling
```clojure
; Returns nil on error by default (Clojure style)
(def content (fs/read "missing.txt"))  ; => nil

; Or use try-catch
(try
  (fs/read "file.txt")
  (catch err
    (println "Error:" err)
    "default content"))
```

## Real Examples

### Example 1: Read Config File
```clojure
(use rust.fs)
(use rust.path)

(defn load-config []
  (if (path/exists? "config.json")
    (fs/read "config.json")
    "{}"))  ; default empty config

(def config (load-config))
```

### Example 2: Process Directory
```clojure
(use rust.fs)
(use rust.path)

(defn process-files [dir]
  (let [files (fs/read-dir dir)]
    (map (fn [file]
           (if (path/file? file)
             (fs/read file)
             nil))
         files)))

(def contents (process-files "data/"))
```

### Example 3: Write Log
```clojure
(use rust.fs)
(use rust.time)

(defn log-message [msg]
  (let [timestamp (time/now)
        entry (str timestamp " - " msg "\n")]
    (fs/append "app.log" entry)))

(log-message "Application started")
```

### Example 4: Environment Setup
```clojure
(use rust.env)
(use rust.path)

(defn setup []
  (let [home (env/var "HOME")
        config-dir (path/join home ".myapp")]
    (when (not (path/exists? config-dir))
      (fs/create-dir config-dir))
    config-dir))
```

## Standard Library Coverage

### Tier 1: Essential (Week 1)
- ✅ `rust.fs` - File operations
- ✅ `rust.path` - Path manipulation
- ✅ `rust.env` - Environment variables
- ✅ `rust.str` - String operations

### Tier 2: Common (Week 2)
- ✅ `rust.time` - Time and duration
- ✅ `rust.process` - Running commands
- ✅ `rust.io` - Input/output
- ✅ `rust.net` - Basic networking

### Tier 3: Advanced (Week 3)
- ✅ `rust.thread` - Threading
- ✅ `rust.sync` - Synchronization
- ✅ `rust.collections` - Additional collections
- ✅ `rust.iter` - Iterators

## Implementation Plan (Simple!)

### Week 1: Core Wrappers
```bash
Day 1-2: Write Rust wrappers for fs, path, env
Day 3: Parser support for (use rust.module)
Day 4: Codegen to call wrappers
Day 5: Test & docs
```

### Files to Create
```
crates/clorus-std/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Re-exports
│   ├── fs.rs           # ~100 lines
│   ├── path.rs         # ~80 lines
│   ├── env.rs          # ~60 lines
│   └── str.rs          # ~100 lines
└── README.md

crates/clorus-syntax/src/
└── parser.rs           # Add (use ...) form

crates/clorus-codegen/src/
└── codegen.rs          # Add std function calls
```

### Build Integration
```bash
# Automatic - no configuration needed!
$ clorus run

# Clorus CLI detects (use rust.*)
# Links clorus-std automatically
# Just works!
```

## Clojure-Style API Design

Follow Clojure naming conventions:

```clojure
; Predicates end with ?
(path/exists? "file.txt")
(path/file? "data.txt")
(str/empty? "")

; Mutating operations end with !
(vec/push! v item)
(map/insert! m key val)

; Conversions with ->
(str->int "42")
(vec->list [1 2 3])

; Short, clear names
(fs/read "file.txt")      ; not read_to_string
(fs/write "file.txt" s)   ; not write_all
```

## Error Messages
```clojure
; Clear, helpful errors
(fs/read "missing.txt")

; Shows:
error: File not found
  --> src/main.clrs:5:10
   Could not read file: missing.txt

 5 | (fs/read "missing.txt")
     ^~~~~~

   help: Check that the file exists
         Use (path/exists? "missing.txt") to check first
```

## Documentation

Built-in help:
```clojure
(doc fs/read)
; fs/read - Read file to string
;
; Usage:
;   (fs/read path)
;
; Returns: String or nil on error
;
; Example:
;   (def content (fs/read "data.txt"))
```

## Benefits

1. **Simple** - No boilerplate, just import and use
2. **Familiar** - Feels like Clojure
3. **Powerful** - Full Rust std lib
4. **Safe** - Error handling built-in
5. **Fast** - Direct Rust calls
6. **Predictable** - Pre-made wrappers, well-tested

## Comparison

### Other Languages
```python
# Python - similar simplicity
import os
content = os.read("file.txt")
```

```clojure
; Clojure
(require '[clojure.java.io :as io])
(def content (slurp "file.txt"))
```

### Clorus - Best of both!
```clojure
(use rust.fs)
(def content (fs/read "file.txt"))
```

- Simple as Python
- Functional as Clojure
- Fast as Rust!

## Priority: Start Small

**Phase 1: Just fs module** (3-4 days)
- `fs/read`, `fs/write`, `fs/exists?`
- Get it working end-to-end
- Then expand

**Example:**
```clojure
; main.clrs
(use rust.fs)

(defn main []
  (let [content (fs/read "input.txt")
        upper (str/upper content)]
    (fs/write "output.txt" upper)))

(main)
```

```bash
$ clorus run
# Just works! No configuration!
```

---

**This is the right approach!**
- ✅ Simple (Clojure philosophy)
- ✅ Powerful (Rust std lib)
- ✅ Practical (focus on what's useful)
- ✅ Easy to implement (pre-made wrappers)

Want me to start with **just `rust.fs` module** to prove it out? That's the foundation - file I/O is essential! 🚀

Should take 3-4 days to get:
- `(use rust.fs)` working
- Read/write files
- Error handling
- Full example program

Then we expand from there!
