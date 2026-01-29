# String Operations Implementation Plan

## Overview

Implement comprehensive string operations for Clorus, enabling real-world application development. Currently at **10% string support**, target is **80%+** (production-ready).

**Status:** Planning Phase
**Timeline:** 1-2 weeks
**Priority:** 🔥 **HIGHEST** - Blocking real application development

---

## Current State

### What We Have ✅
- String literals: `"hello world"`
- String values (ValueTag::String)
- Basic string storage in runtime
- Print/println (output only)

### What We're Missing ❌
- String concatenation (`str`)
- Substring extraction (`subs`)
- String splitting (`split`)
- String joining (`join`)
- Case conversion (`upper-case`, `lower-case`)
- Trimming (`trim`)
- String replacement (`replace`)
- String predicates (`string?`, `blank?`, `starts-with?`, `ends-with?`)
- String comparison

---

## Design Decisions

### Runtime (Rust FFI) vs Stdlib (Pure Clorus)

#### Runtime FFI (Performance-Critical)
Implement in `/crates/clorus-runtime/src/string.rs`:
- ✅ Core operations (str, subs, count)
- ✅ Splitting/joining (involves allocation)
- ✅ Case conversion (Unicode-aware)
- ✅ Trimming (char iteration)
- ✅ Search/replace (complex string algorithms)

#### Stdlib (Pure Clorus)
Could implement later in `/stdlib/string.clr`:
- String formatting/templates (uses str)
- String validation (uses predicates)
- Complex string processing (uses primitives)

**Decision:** Start with runtime FFI for all core operations, optimize later if needed.

---

## API Design

### 1. String Construction

#### `str` - String concatenation
```clojure
(str "hello")                    ; => "hello"
(str "hello" " " "world")        ; => "hello world"
(str "count: " 42)               ; => "count: 42"
(str nil)                        ; => ""
(str [1 2 3])                    ; => "[1 2 3]"
```

**Signature:** `(str & args) -> String`
**FFI:** `clorus_str(args: *mut Value) -> *mut Value`

**Behavior:**
- Takes variable number of arguments
- Converts each argument to string representation
- Concatenates all strings
- nil becomes empty string
- Numbers, keywords, symbols converted to strings
- Collections use print representation

#### `count` - String length (already exists for collections)
```clojure
(count "hello")                  ; => 5
(count "")                       ; => 0
```

**Note:** This already works via `clorus_count` - just verify string support.

---

### 2. String Extraction

#### `subs` - Substring extraction
```clojure
(subs "hello world" 0 5)         ; => "hello"
(subs "hello world" 6)           ; => "world"
(subs "hello" 1 4)               ; => "ell"
```

**Signature:**
- `(subs string start) -> String`
- `(subs string start end) -> String`

**FFI:**
- `clorus_subs2(s: *mut Value, start: i64) -> *mut Value`
- `clorus_subs3(s: *mut Value, start: i64, end: i64) -> *mut Value`

**Behavior:**
- 0-indexed (like Clojure)
- Returns substring from start (inclusive) to end (exclusive)
- If end omitted, goes to end of string
- Negative indices not supported (for simplicity)
- Out of bounds returns empty string

#### `first` - First character (could use existing first?)
```clojure
(first "hello")                  ; => "h"
```

#### `last` - Last character (could use existing last?)
```clojure
(last "hello")                   ; => "o"
```

---

### 3. String Splitting & Joining

#### `split` - Split string by delimiter
```clojure
(split "a,b,c" ",")              ; => ["a" "b" "c"]
(split "hello world" " ")        ; => ["hello" "world"]
(split "a::b::c" "::")           ; => ["a" "b" "c"]
(split "" ",")                   ; => [""]
```

**Signature:** `(split string delimiter) -> Vector<String>`
**FFI:** `clorus_split(s: *mut Value, delim: *mut Value) -> *mut Value`

**Behavior:**
- Returns vector of strings
- Delimiter is a string (not regex for MVP)
- Empty string returns vector with one empty string
- Trailing delimiters create empty strings

#### `join` - Join strings with separator
```clojure
(join "," ["a" "b" "c"])         ; => "a,b,c"
(join " " ["hello" "world"])     ; => "hello world"
(join ", " [1 2 3])              ; => "1, 2, 3"
(join "" ["a" "b" "c"])          ; => "abc"
```

**Signature:** `(join separator collection) -> String`
**FFI:** `clorus_join(sep: *mut Value, coll: *mut Value) -> *mut Value`

**Behavior:**
- Takes separator and collection
- Converts each element to string
- Joins with separator

---

### 4. Case Conversion

#### `upper-case` - Convert to uppercase
```clojure
(upper-case "hello")             ; => "HELLO"
(upper-case "Hello World")       ; => "HELLO WORLD"
(upper-case "")                  ; => ""
```

**Signature:** `(upper-case string) -> String`
**FFI:** `clorus_upper_case(s: *mut Value) -> *mut Value`

#### `lower-case` - Convert to lowercase
```clojure
(lower-case "HELLO")             ; => "hello"
(lower-case "Hello World")       ; => "hello world"
(lower-case "")                  ; => ""
```

**Signature:** `(lower-case string) -> String`
**FFI:** `clorus_lower_case(s: *mut Value) -> *mut Value`

**Implementation:** Use Rust's `.to_uppercase()` and `.to_lowercase()` (Unicode-aware)

---

### 5. String Trimming

#### `trim` - Remove leading/trailing whitespace
```clojure
(trim "  hello  ")               ; => "hello"
(trim "\t\nhello\n\t")           ; => "hello"
(trim "hello")                   ; => "hello"
```

**Signature:** `(trim string) -> String`
**FFI:** `clorus_trim(s: *mut Value) -> *mut Value`

#### `trim-left` / `trim-right` - One-sided trimming
```clojure
(trim-left "  hello  ")          ; => "hello  "
(trim-right "  hello  ")         ; => "  hello"
```

**Signature:** Same as trim
**FFI:**
- `clorus_trim_left(s: *mut Value) -> *mut Value`
- `clorus_trim_right(s: *mut Value) -> *mut Value`

---

### 6. String Search & Replace

#### `replace` - Replace occurrences
```clojure
(replace "hello world" "l" "L")  ; => "heLLo worLd"
(replace "hello" "ll" "yy")      ; => "heyyo"
(replace "hello" "x" "y")        ; => "hello"
```

**Signature:** `(replace string match replacement) -> String`
**FFI:** `clorus_replace(s: *mut Value, match: *mut Value, repl: *mut Value) -> *mut Value`

**Behavior:**
- Replaces all occurrences (not just first)
- Match is literal string (not regex for MVP)
- Returns new string

#### `replace-first` - Replace first occurrence only
```clojure
(replace-first "hello hello" "ll" "yy")  ; => "heyyo hello"
```

**Signature:** Same as replace
**FFI:** `clorus_replace_first(...) -> *mut Value`

---

### 7. String Predicates

#### `string?` - Check if value is string
```clojure
(string? "hello")                ; => true
(string? 42)                     ; => false
(string? nil)                    ; => false
```

**Signature:** `(string? x) -> Boolean`
**FFI:** `clorus_is_string(val: *mut Value) -> bool`

#### `blank?` - Check if string is empty or whitespace
```clojure
(blank? "")                      ; => true
(blank? "   ")                   ; => true
(blank? "hello")                 ; => false
(blank? nil)                     ; => true
```

**Signature:** `(blank? string) -> Boolean`
**Implementation:** Can be pure Clorus: `(or (nil? s) (= (trim s) ""))`

#### `starts-with?` - Check prefix
```clojure
(starts-with? "hello world" "hello")  ; => true
(starts-with? "hello" "hi")           ; => false
```

**Signature:** `(starts-with? string prefix) -> Boolean`
**FFI:** `clorus_starts_with(s: *mut Value, prefix: *mut Value) -> bool`

#### `ends-with?` - Check suffix
```clojure
(ends-with? "hello.txt" ".txt")  ; => true
(ends-with? "hello" ".txt")      ; => false
```

**Signature:** `(ends-with? string suffix) -> Boolean`
**FFI:** `clorus_ends_with(s: *mut Value, suffix: *mut Value) -> bool`

#### `includes?` / `contains?` - Check substring
```clojure
(includes? "hello world" "lo wo")  ; => true
(includes? "hello" "x")            ; => false
```

**Signature:** `(includes? string substring) -> Boolean`
**FFI:** `clorus_includes(s: *mut Value, substr: *mut Value) -> bool`

---

### 8. String Comparison

#### `=` - String equality (already works)
```clojure
(= "hello" "hello")              ; => true
(= "hello" "world")              ; => false
```

**Note:** Already implemented in value equality

#### `compare` - Lexicographic comparison
```clojure
(compare "apple" "banana")       ; => -1 (negative if <)
(compare "hello" "hello")        ; => 0  (equal)
(compare "zebra" "apple")        ; => 1  (positive if >)
```

**Signature:** `(compare string1 string2) -> i64`
**FFI:** `clorus_compare_strings(s1: *mut Value, s2: *mut Value) -> i64`

---

## Implementation Plan

### Phase 1: Runtime String Module (Week 1)

#### Step 1: Create string.rs module
**File:** `/crates/clorus-runtime/src/string.rs`

```rust
use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use std::ffi::CStr;

/// String concatenation - convert all args to strings and concatenate
#[no_mangle]
pub extern "C" fn clorus_str(args: *mut Value) -> *mut Value {
    // Implementation
}

/// Substring extraction
#[no_mangle]
pub extern "C" fn clorus_subs2(s: *mut Value, start: i64) -> *mut Value {
    // Implementation
}

#[no_mangle]
pub extern "C" fn clorus_subs3(s: *mut Value, start: i64, end: i64) -> *mut Value {
    // Implementation
}

/// Split string by delimiter
#[no_mangle]
pub extern "C" fn clorus_split(s: *mut Value, delim: *mut Value) -> *mut Value {
    // Implementation
}

/// Join collection with separator
#[no_mangle]
pub extern "C" fn clorus_join(sep: *mut Value, coll: *mut Value) -> *mut Value {
    // Implementation
}

// ... more functions
```

#### Step 2: Add module to lib.rs
```rust
// In /crates/clorus-runtime/src/lib.rs
pub mod string;
```

#### Step 3: Helper Functions
```rust
/// Convert Value* to Rust String for manipulation
unsafe fn value_to_rust_string(val: *mut Value) -> Result<String, &'static str> {
    if val.is_null() {
        return Ok(String::new());
    }

    match (*val).header().tag() {
        ValueTag::String => {
            // Extract C string and convert to Rust String
            let c_str_ptr = (*val).as_ptr() as *const i8;
            let c_str = CStr::from_ptr(c_str_ptr);
            Ok(c_str.to_string_lossy().into_owned())
        }
        ValueTag::Number => {
            Ok((*val).as_number().to_string())
        }
        ValueTag::Keyword => {
            // Format as :keyword
            let c_str_ptr = (*val).as_ptr() as *const i8;
            let c_str = CStr::from_ptr(c_str_ptr);
            Ok(format!(":{}", c_str.to_string_lossy()))
        }
        ValueTag::Nil => Ok(String::new()),
        // ... handle other types
        _ => Ok(format!("{:?}", val)) // Fallback
    }
}

/// Create String Value* from Rust String
unsafe fn rust_string_to_value(s: String) -> *mut Value {
    Value::string(&s)
}
```

### Phase 2: FFI Declarations (Week 1)

#### Update codegen.rs
**File:** `/crates/clorus-codegen/src/codegen.rs`

Add to `declare_runtime_functions()`:

```rust
// String operations
let str_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], true); // variadic
self.module.add_function("clorus_str", str_type, None);

let subs2_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
self.module.add_function("clorus_subs2", subs2_type, None);

let subs3_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i64_type.into()], false);
self.module.add_function("clorus_subs3", subs3_type, None);

let split_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
self.module.add_function("clorus_split", split_type, None);

let join_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
self.module.add_function("clorus_join", join_type, None);

// ... more declarations
```

### Phase 3: Builtin Dispatch (Week 1)

Add cases to `compile_builtin()`:

```rust
"str" => {
    // Variable arguments - pack into vector
    let mut arg_vec = PersistentVector::empty();
    for arg in args {
        let val = self.compile_expr(arg)?;
        arg_vec = PersistentVector::conj(arg_vec, val);
    }
    let vec_val = Value::from_ptr(ValueTag::Vector, arg_vec as *mut u8);

    let str_fn = self.module.get_function("clorus_str")?;
    let result = self.builder.build_call(str_fn, &[vec_val.into()], "str_call")?;
    Ok(result.try_as_basic_value().left()?.into_pointer_value())
}

"subs" => {
    if args.len() < 2 || args.len() > 3 {
        return Err("subs requires 2 or 3 arguments".to_string());
    }

    let s_val = self.compile_expr(&args[0])?;
    let start = self.compile_expr(&args[1])?;
    let start_i64 = self.unbox_number(start);
    let start_i64 = self.builder.build_float_to_signed_int(start_i64, self.context.i64_type(), "start")?;

    if args.len() == 2 {
        let subs_fn = self.module.get_function("clorus_subs2")?;
        let result = self.builder.build_call(subs_fn, &[s_val.into(), start_i64.into()], "subs2")?;
        Ok(result.try_as_basic_value().left()?.into_pointer_value())
    } else {
        let end = self.compile_expr(&args[2])?;
        let end_i64 = self.unbox_number(end);
        let end_i64 = self.builder.build_float_to_signed_int(end_i64, self.context.i64_type(), "end")?;

        let subs_fn = self.module.get_function("clorus_subs3")?;
        let result = self.builder.build_call(subs_fn, &[s_val.into(), start_i64.into(), end_i64.into()], "subs3")?;
        Ok(result.try_as_basic_value().left()?.into_pointer_value())
    }
}

// Similar for split, join, etc.
```

### Phase 4: Testing (Week 2)

#### Create test file
**File:** `/tests/strings/string-operations-test.clr`

```clojure
(println "=== String Operations Tests ===\n")

;; Test str concatenation
(println "--- String Concatenation ---")
(def result1 (str "hello" " " "world"))
(println "str concat:" result1)
;; Expected: "hello world"

(def result2 (str "count: " 42))
(println "str with number:" result2)
;; Expected: "count: 42"

;; Test subs
(println "\n--- Substring ---")
(def result3 (subs "hello world" 0 5))
(println "subs [0,5):" result3)
;; Expected: "hello"

(def result4 (subs "hello world" 6))
(println "subs from 6:" result4)
;; Expected: "world"

;; Test split
(println "\n--- Split ---")
(def result5 (split "a,b,c" ","))
(println "split by comma:" result5)
;; Expected: ["a" "b" "c"]

;; Test join
(println "\n--- Join ---")
(def result6 (join "," ["a" "b" "c"]))
(println "join with comma:" result6)
;; Expected: "a,b,c"

;; Test case conversion
(println "\n--- Case Conversion ---")
(def result7 (upper-case "hello"))
(println "uppercase:" result7)
;; Expected: "HELLO"

(def result8 (lower-case "WORLD"))
(println "lowercase:" result8)
;; Expected: "world"

;; Test trim
(println "\n--- Trim ---")
(def result9 (trim "  hello  "))
(println "trim:" result9)
;; Expected: "hello"

;; Test replace
(println "\n--- Replace ---")
(def result10 (replace "hello" "l" "L"))
(println "replace:" result10)
;; Expected: "heLLo"

(println "\n✅ All string operations tested!")
```

---

## Success Criteria

### Functional Completeness ✅
- [ ] String concatenation works
- [ ] Substring extraction works
- [ ] Split/join works
- [ ] Case conversion works
- [ ] Trimming works
- [ ] Replace works
- [ ] Predicates work
- [ ] String comparison works

### Integration ✅
- [ ] All FFI functions declared in codegen
- [ ] All builtins dispatch correctly
- [ ] Memory management correct (no leaks)
- [ ] Works with existing collection operations

### Testing ✅
- [ ] Unit tests in Rust
- [ ] Integration tests in Clorus
- [ ] Edge cases covered (empty, nil, Unicode)
- [ ] Performance acceptable

### Documentation ✅
- [ ] API reference
- [ ] Usage examples
- [ ] Common patterns
- [ ] Migration from current state

---

## Timeline

### Week 1: Core Implementation
- **Day 1-2:** Create string.rs module, implement str/subs/split/join
- **Day 3:** Implement case conversion and trimming
- **Day 4:** Implement replace and predicates
- **Day 5:** Add FFI declarations and builtin dispatch

### Week 2: Testing & Polish
- **Day 1-2:** Write comprehensive tests
- **Day 3:** Fix bugs and edge cases
- **Day 4:** Optimize performance
- **Day 5:** Write documentation and examples

---

## Future Enhancements (Post-MVP)

### Phase 2 Features:
- Regular expressions (match, replace with regex)
- String interpolation/formatting
- Character operations
- String builder for performance
- Encoding/decoding (UTF-8, base64, etc.)

### Phase 3 Features:
- Locale-aware operations
- Unicode normalization
- String streams

---

## Risk Assessment

### Low Risk ✅
- Core operations (str, subs) - straightforward
- Case conversion - Rust stdlib handles it
- Basic predicates - simple checks

### Medium Risk ⚠️
- Memory management - must properly retain/release strings
- Unicode handling - need to decide on char vs byte indexing
- Variadic str function - needs special handling

### High Risk ❌
- Performance with large strings - may need optimization
- Regex support (if added) - complex integration

---

## Dependencies

### Required:
- ✅ Value system (already have)
- ✅ String storage (already have)
- ✅ Vector operations (for split results)
- ✅ Memory management (retain/release)

### Optional:
- Regex crate (for future regex support)
- Unicode crate (for advanced Unicode operations)

---

## Comparison with Clojure

| Operation | Clojure | Clorus (Planned) | Status |
|-----------|---------|------------------|--------|
| str | ✅ | 🚧 | Planned |
| subs | ✅ | 🚧 | Planned |
| split | ✅ (regex) | 🚧 (literal) | Simplified MVP |
| join | ✅ | 🚧 | Planned |
| upper-case | ✅ | 🚧 | Planned |
| lower-case | ✅ | 🚧 | Planned |
| trim | ✅ | 🚧 | Planned |
| replace | ✅ (regex) | 🚧 (literal) | Simplified MVP |
| re-find | ✅ | ❌ | Future |
| re-seq | ✅ | ❌ | Future |

**MVP Coverage:** ~70% of common string operations
**Production Ready:** Yes, for most use cases

---

**Status:** 📋 Plan Complete - Ready to Implement
**Next Step:** Create `/crates/clorus-runtime/src/string.rs` and start implementing core functions

---

*Last Updated: January 27, 2026*
*Contributors: Prabhu Gopal + Claude Code*
