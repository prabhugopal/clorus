# Rust-Clorus Data Structure Interop

## The Challenge

When calling Rust functions from Clorus, we need to convert between:
- **Clorus types**: Lists `(1 2 3)`, Vectors `[1 2 3]`, Maps `{:a 1}`, Keywords `:name`
- **Rust types**: `Vec<T>`, `HashMap<K,V>`, structs, enums, etc.

## Current Approach: Primitive Wrappers (rust.fs)

### What we have now:
```clojure
(use rust.fs)
(fs/write "file.txt" "content")  ; String → *const c_char → Rust String
(fs/exists? "file.txt")          ; Returns i32 → converted to f64 (1.0/0.0)
```

**Conversion**:
- Clorus strings → C strings (`*const c_char`)
- Rust returns → primitives (i32, f64)
- **Limitation**: Can only pass/return simple types

## Solution 1: Manual Wrappers (Current)

For each Rust function, write a C-compatible wrapper:

```rust
// In clorus-std/src/fs.rs
#[no_mangle]
pub extern "C" fn clorus_fs_read(path: *const c_char) -> *mut c_char {
    unsafe {
        // Convert C string → Rust String
        let path_str = CStr::from_ptr(path).to_str().unwrap();

        // Call Rust std::fs
        let content = std::fs::read_to_string(path_str).unwrap();

        // Convert Rust String → C string
        CString::new(content).unwrap().into_raw()
    }
}
```

**Pros**: Simple, explicit, works now
**Cons**: Must write wrapper for every function

## Solution 2: Value* Type System (Planned)

Create a universal Clorus Value type that can represent anything:

```rust
// In clorus-runtime
pub enum Value {
    Number(f64),
    String(Rc<String>),
    Vector(Rc<Vector>),
    Map(Rc<HashMap<Value, Value>>),
    // ... etc
}

// Conversion traits
impl From<Vec<f64>> for Value {
    fn from(vec: Vec<f64>) -> Self {
        Value::Vector(vec.into_iter().map(Value::Number).collect())
    }
}

impl TryInto<Vec<f64>> for Value {
    fn try_into(self) -> Result<Vec<f64>> {
        // Extract numbers from vector
    }
}
```

### Example with Value*:
```clojure
(use rust.collections)

; Rust HashMap<String, i32> → Clorus map
(def ages (hashmap/new))
(hashmap/insert ages "Alice" 30)
(hashmap/get ages "Alice")  ; => 30

; Rust Vec<String> → Clorus vector
(def names (vec/from-iter ["Alice" "Bob" "Charlie"]))
(vec/nth names 1)  ; => "Bob"
```

**Conversion happens automatically**:
```rust
// In wrapper
#[no_mangle]
pub extern "C" fn clorus_vec_from_rust(rust_vec: Vec<String>) -> *mut Value {
    let clorus_vec: Value = rust_vec.into_iter()
        .map(|s| Value::String(Rc::new(s)))
        .collect::<Vec<Value>>()
        .into();

    Box::into_raw(Box::new(clorus_vec))
}
```

## Solution 3: Type Mapping (Future)

Define explicit mappings between Rust and Clorus types:

```clojure
; In Clorus
(deftype rust-vec [T]
  "Wraps a Rust Vec<T>"
  :rust-type "Vec<T>"
  :to-clorus (fn [v] (vec (rust/collect v)))
  :from-clorus (fn [v] (rust/from-iter v)))

(use rust.std.vec)

; Automatic conversion
(def numbers [1 2 3])  ; Clorus vector
(def rust-numbers (rust-vec/from-clorus numbers))
(rust-vec/push rust-numbers 4)
(def back-to-clorus (rust-vec/to-clorus rust-numbers))
```

## Real-World Examples

### Example 1: File System (Current - Works!)
```clojure
(use rust.fs)

; Simple types only
(fs/write "data.txt" "Hello")  ; String → String
(fs/exists? "data.txt")        ; String → bool (as f64)
```

**No complex types needed!** ✅

### Example 2: Collections (Planned)
```clojure
(use rust.std.vec)

; Create Rust Vec
(def v (vec/new))
(vec/push v 10)
(vec/push v 20)
(vec/len v)  ; => 2

; Convert to Clorus vector
(def clorus-v (vec/to-vector v))  ; => [10 20]
```

### Example 3: JSON (Future)
```clojure
(use rust.serde-json)

; Parse JSON → Clorus map
(def data (json/parse "{\"name\":\"Alice\",\"age\":30}"))
; data => {:name "Alice" :age 30}

; Convert Clorus map → JSON string
(def json-str (json/stringify {:x 10 :y 20}))
; json-str => "{\"x\":10,\"y\":20}"
```

## Implementation Approaches

### Approach A: Shallow Wrappers (rust.fs style)
**Best for**: Simple I/O, system calls, primitives

```clojure
(use rust.fs)
(fs/write path content)  ; Just calls Rust, no complex types
```

**Interop**: None needed - uses primitives only

### Approach B: Deep Integration (serde style)
**Best for**: Complex data structures, JSON, serialization

```rust
// Automatic conversion via serde
impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer { ... }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where D: Deserializer<'de> { ... }
}
```

```clojure
; Now serde-json just works!
(use rust.serde-json)
(json/parse string)     ; JSON → Value (automatic!)
(json/stringify value)  ; Value → JSON (automatic!)
```

### Approach C: Manual Conversion (explicit)
**Best for**: Performance-critical code, custom types

```clojure
(use rust.custom)

; Explicit conversion
(def rust-data (to-rust clorus-data))
(def result (rust/process rust-data))
(def clorus-result (from-rust result))
```

## Recommended Path

### Phase 1: Primitives Only (Current - DONE ✅)
- Strings, numbers, booleans
- Simple wrappers like rust.fs
- No complex types needed

### Phase 2: Value* Type System (Next - 2-3 weeks)
- Universal Value type
- Basic conversions (Vec → Vector, HashMap → Map)
- Core collection operations

### Phase 3: Automatic Interop (1-2 months)
- Type inference
- Automatic conversions
- Serde integration

### Phase 4: Zero-Copy (Advanced)
- Shared memory between Rust and Clorus
- No copying for large data structures
- Performance optimization

## Comparison with Other Languages

### **Clojure (JVM)**:
```clojure
; Direct Java interop
(.toString (java.util.ArrayList. [1 2 3]))
; => "[1, 2, 3]"

; Automatic conversion
(into [] (java.util.ArrayList. [1 2 3]))
; => [1 2 3]
```

**Clorus approach**: Similar automatic conversion via Value*

### **LuaJIT FFI**:
```lua
ffi = require("ffi")
ffi.cdef[[
    int add(int a, int b);
]]
C = ffi.load("mylib")
result = C.add(10, 20)  -- Direct call, no conversion
```

**Clorus approach**: Similar C FFI, but with Value* wrapper

### **Python ctypes**:
```python
from ctypes import *
lib = CDLL("mylib.so")
lib.add.argtypes = [c_int, c_int]
lib.add.restype = c_int
result = lib.add(10, 20)
```

**Clorus approach**: Simpler syntax, no manual type declarations

## Practical Examples

### Example: Using rust.regex (Future)
```clojure
(use rust.regex)

; Regex is compiled in Rust
(def pattern (regex/new "\\d+"))

; Returns Clorus vector of matches
(def matches (regex/find-all pattern "foo 123 bar 456"))
; matches => ["123" "456"]

; Behind the scenes:
; 1. String literal → C string → Rust String
; 2. Regex::find_all returns Vec<String>
; 3. Vec<String> → Value::Vector → [String]
```

### Example: Using rust.http (Future)
```clojure
(use rust.http)

; Make HTTP request
(def response (http/get "https://api.github.com"))

; Response is a Clorus map (converted from Rust struct)
(def status (:status response))   ; => 200
(def body (:body response))       ; => JSON string
(def headers (:headers response)) ; => {:content-type "application/json"}
```

## Key Decisions

### 1. Immutability
**Clojure**: All collections immutable
**Rust**: Mutable by default

**Solution**: Wrap Rust collections in immutable facade:
```clojure
(def v (rust-vec/new))
(def v2 (rust-vec/conj v 10))  ; Creates new vector
; v unchanged, v2 has 10 added
```

### 2. Ownership
**Rust**: Strict ownership rules
**Clorus**: Garbage collected (Rc/Arc)

**Solution**: Use Rc for shared data:
```rust
pub struct ClorusVec {
    inner: Rc<Vec<Value>>
}

impl ClorusVec {
    pub fn conj(&self, item: Value) -> Self {
        let mut new_vec = (*self.inner).clone();
        new_vec.push(item);
        ClorusVec { inner: Rc::new(new_vec) }
    }
}
```

### 3. Error Handling
**Rust**: Result<T, E>
**Clorus**: Exceptions or special values

**Solution**: Convert Result to Value:
```rust
match rust_fn() {
    Ok(val) => val.into_value(),
    Err(e) => Value::Error(e.to_string())
}
```

## Summary

**Current Status** (rust.fs):
- ✅ Primitives work (strings, numbers)
- ✅ Simple wrappers
- ❌ Complex types not yet supported

**Next Steps**:
1. Implement Value* type system
2. Add basic conversions (Vec, HashMap)
3. Test with real Rust libraries
4. Add automatic conversion traits

**Long-term Goal**: Seamless interop where Clorus users don't think about conversions:

```clojure
(use rust.std)
(use rust.serde-json)

; Just works - types convert automatically
(def data (json/parse (fs/read "data.json")))
(def names (map :name (:users data)))
(fs/write "names.txt" (str/join ", " names))
```

The key is **gradual enhancement**: start simple (primitives), add complexity as needed (collections), eventually achieve seamless interop (automatic conversion).
