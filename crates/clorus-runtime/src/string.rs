/// String operations for Clorus
///
/// This module provides comprehensive string manipulation functions including
/// concatenation, extraction, splitting, joining, case conversion, and more.
///
/// All functions follow the Clorus memory model:
/// - Input strings are read-only
/// - New strings are created with refcount 1
/// - Caller owns the returned reference

use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use regex_lite::Regex;
use std::ffi::CStr;
use std::os::raw::c_char;

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a Value* to a Rust String for manipulation
///
/// Handles various value types:
/// - String: Direct extraction
/// - Number: Convert to string representation
/// - Keyword: Format as :keyword
/// - Boolean: "true" or "false"
/// - Nil: Empty string
/// - Collections: Print representation
unsafe fn value_to_rust_string(val: *mut Value) -> String {
    if val.is_null() {
        return String::new();
    }

    match (*val).header().tag() {
        ValueTag::String => {
            // Value* strings store Rust String pointers, not C strings
            (*val).as_string().to_string()
        }
        ValueTag::Long => {
            format!("{}", (*val).as_long())
        }
        ValueTag::Double => {
            let num = (*val).as_double();
            // Format nicely (no unnecessary decimals for integer-valued doubles)
            if num.fract() == 0.0 && num.is_finite() {
                format!("{:.0}", num)
            } else {
                format!("{}", num)
            }
        }
        ValueTag::Keyword => {
            // Value* keywords also store Rust String pointers
            format!(":{}", (*val).as_keyword())
        }
        ValueTag::Symbol => {
            // Value* symbols also store Rust String pointers
            (*val).as_string().to_string()
        }
        ValueTag::Bool => {
            if (*val).as_bool() {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        ValueTag::Nil => String::new(),
        ValueTag::Vector => {
            // Simple vector representation
            let vec_ptr = (*val).as_ptr() as *mut PersistentVector;
            let count = (*vec_ptr).count();
            let mut parts = Vec::new();
            for i in 0..count {
                let elem = PersistentVector::nth(vec_ptr, i);
                parts.push(value_to_rust_string(elem));
                crate::value::clorus_release(elem);
            }
            format!("[{}]", parts.join(" "))
        }
        ValueTag::List => {
            let mut parts = Vec::new();
            let mut current = val;
            loop {
                let count = crate::list::clorus_list_count(current);
                if count == 0 {
                    break;
                }
                let first = crate::list::clorus_list_first(current);
                parts.push(value_to_rust_string(first));
                crate::value::clorus_release(first);
                let next = crate::list::clorus_list_rest(current);
                if current != val {
                    crate::value::clorus_release(current);
                }
                current = next;
            }
            if current != val {
                crate::value::clorus_release(current);
            }
            format!("({})", parts.join(" "))
        }
        ValueTag::HashMap => {
            let map_ptr = (*val).as_ptr() as *mut crate::map::ClorusHashMap;
            let mut entries: Vec<(String, String)> = Vec::new();
            for (k, v) in (*map_ptr).entries_iter() {
                entries.push((value_to_rust_string(k), value_to_rust_string(v)));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let rendered: Vec<String> = entries
                .into_iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .collect();
            format!("{{{}}}", rendered.join(" "))
        }
        ValueTag::HashSet => {
            let set_ptr = (*val).as_ptr() as *mut crate::set::ClorusHashSet;
            let mut elems: Vec<String> = (*set_ptr)
                .values()
                .iter()
                .map(|e| value_to_rust_string(*e))
                .collect();
            elems.sort();
            format!("#{{{}}}", elems.join(" "))
        }
        _ => format!("#<{:?}>", (*val).header().tag()),
    }
}

/// Create a String Value* from a Rust String
/// FIXED: Box the string to keep it alive before creating Value
unsafe fn rust_string_to_value(s: String) -> *mut Value {
    // The key is that Value::string expects a &str that will be copied.
    // We create a Box to give the String a stable heap location,
    // then leak it so Value::string's internal Box::new(s.to_string())
    // gets a stable reference. The Value's refcount will manage cleanup.
    let stable_ref: &'static str = Box::leak(Box::new(s));
    Value::string(stable_ref)
}

/// Get string from Value*, returns None if not a string
unsafe fn get_string_value(val: *mut Value) -> Option<String> {
    if val.is_null() {
        return None;
    }

    if (*val).header().tag() == ValueTag::String {
        Some(value_to_rust_string(val))
    } else {
        None
    }
}

unsafe fn captures_to_value(caps: &regex_lite::Captures<'_>) -> *mut Value {
    if caps.len() <= 1 {
        return match caps.get(0) {
            Some(full) => rust_string_to_value(full.as_str().to_string()),
            None => Value::nil(),
        };
    }

    let mut out = crate::vector::clorus_vector_empty();
    for idx in 0..caps.len() {
        let capture_val = match caps.get(idx) {
            Some(m) => rust_string_to_value(m.as_str().to_string()),
            None => Value::nil(),
        };
        let next = crate::vector::clorus_vector_conj(out, capture_val);
        crate::value::clorus_release(out);
        crate::value::clorus_release(capture_val);
        out = next;
    }
    out
}

// ============================================================================
// Core String Operations
// ============================================================================

/// String concatenation - convert all arguments to strings and concatenate
///
/// (str "hello" " " "world") => "hello world"
/// (str "count: " 42) => "count: 42"
/// (str nil) => ""
///
/// Takes a vector of values as argument
#[no_mangle]
pub extern "C" fn clorus_str(args: *mut Value) -> *mut Value {
    if args.is_null() {
        return unsafe { rust_string_to_value(String::new()) };
    }

    unsafe {
        // Args should be a vector
        if (*args).header().tag() != ValueTag::Vector {
            return rust_string_to_value(String::new());
        }

        let vec_ptr = (*args).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        let mut result = String::new();

        for i in 0..count {
            let elem = PersistentVector::nth(vec_ptr, i);
            result.push_str(&value_to_rust_string(elem));
            crate::value::clorus_release(elem);
        }

        rust_string_to_value(result)
    }
}

/// Substring extraction from start to end of string
///
/// (subs "hello world" 6) => "world"
/// (subs "hello" 1) => "ello"
#[no_mangle]
pub extern "C" fn clorus_subs2(s: *mut Value, start: i64) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        let start_idx = start.max(0) as usize;

        if start_idx >= string.len() {
            return rust_string_to_value(String::new());
        }

        // Substring from start to end
        let result = string[start_idx..].to_string();
        rust_string_to_value(result)
    }
}

/// Substring extraction from start to end (exclusive)
///
/// (subs "hello world" 0 5) => "hello"
/// (subs "hello" 1 4) => "ell"
#[no_mangle]
pub extern "C" fn clorus_subs3(s: *mut Value, start: i64, end: i64) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        let start_idx = start.max(0) as usize;
        let end_idx = end.max(0) as usize;

        // Clamp to string bounds
        let start_idx = start_idx.min(string.len());
        let end_idx = end_idx.min(string.len());

        if start_idx >= end_idx {
            return rust_string_to_value(String::new());
        }

        let result = string[start_idx..end_idx].to_string();
        rust_string_to_value(result)
    }
}

/// Split string by delimiter, returning a vector of strings
///
/// (split "a,b,c" ",") => ["a" "b" "c"]
/// (split "hello world" " ") => ["hello" "world"]
#[no_mangle]
pub extern "C" fn clorus_split(s: *mut Value, delim: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return crate::vector::clorus_vector_empty(),
        };

        let delimiter = match get_string_value(delim) {
            Some(d) => d,
            None => return crate::vector::clorus_vector_empty(),
        };

        // Split by delimiter
        let parts: Vec<&str> = if delimiter.is_empty() {
            // Empty delimiter - split into chars
            string.chars().map(|_| "").collect()
        } else {
            string.split(&delimiter).collect()
        };

        // Create vector of string values
        let mut result = PersistentVector::empty();

        for part in parts {
            let part_val = rust_string_to_value(part.to_string());
            result = PersistentVector::conj(result, part_val);
            crate::value::clorus_release(part_val);
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

/// Join collection elements with separator
///
/// (join "," ["a" "b" "c"]) => "a,b,c"
/// (join " " [1 2 3]) => "1 2 3"
#[no_mangle]
pub extern "C" fn clorus_join(sep: *mut Value, coll: *mut Value) -> *mut Value {
    unsafe {
        let separator = match get_string_value(sep) {
            Some(s) => s,
            None => String::new(),
        };

        if coll.is_null() {
            return rust_string_to_value(String::new());
        }

        // Handle vectors and lists
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                let count = (*vec_ptr).count();

                let mut parts = Vec::new();
                for i in 0..count {
                    let elem = PersistentVector::nth(vec_ptr, i);
                    parts.push(value_to_rust_string(elem));
                    crate::value::clorus_release(elem);
                }

                rust_string_to_value(parts.join(&separator))
            }
            ValueTag::List => {
                let list_ptr = (*coll).as_ptr() as *mut crate::list::PersistentList;
                let mut current = (*list_ptr).clone();
                let mut parts = Vec::new();

                while !current.is_empty() {
                    let elem = current.first();
                    parts.push(value_to_rust_string(elem));
                    current = current.rest();
                }

                rust_string_to_value(parts.join(&separator))
            }
            _ => rust_string_to_value(String::new()),
        }
    }
}

// ============================================================================
// Case Conversion
// ============================================================================

/// Convert string to uppercase
///
/// (upper-case "hello") => "HELLO"
/// (upper-case "Hello World") => "HELLO WORLD"
#[no_mangle]
pub extern "C" fn clorus_upper_case(s: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        rust_string_to_value(string.to_uppercase())
    }
}

/// Convert string to lowercase
///
/// (lower-case "HELLO") => "hello"
/// (lower-case "Hello World") => "hello world"
#[no_mangle]
pub extern "C" fn clorus_lower_case(s: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        rust_string_to_value(string.to_lowercase())
    }
}

// ============================================================================
// String Trimming
// ============================================================================

/// Remove leading and trailing whitespace
///
/// (trim "  hello  ") => "hello"
/// (trim "\t\nhello\n\t") => "hello"
#[no_mangle]
pub extern "C" fn clorus_trim(s: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        rust_string_to_value(string.trim().to_string())
    }
}

/// Remove leading whitespace
///
/// (trim-left "  hello  ") => "hello  "
#[no_mangle]
pub extern "C" fn clorus_trim_left(s: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        rust_string_to_value(string.trim_start().to_string())
    }
}

/// Remove trailing whitespace
///
/// (trim-right "  hello  ") => "  hello"
#[no_mangle]
pub extern "C" fn clorus_trim_right(s: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        rust_string_to_value(string.trim_end().to_string())
    }
}

// ============================================================================
// String Search and Replace
// ============================================================================

/// Replace all occurrences of match string with replacement
///
/// (replace "hello world" "l" "L") => "heLLo worLd"
/// (replace "hello" "ll" "yy") => "heyyo"
#[no_mangle]
pub extern "C" fn clorus_replace(s: *mut Value, match_str: *mut Value, replacement: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        let match_pattern = match get_string_value(match_str) {
            Some(m) => m,
            None => return rust_string_to_value(string),
        };

        let repl = match get_string_value(replacement) {
            Some(r) => r,
            None => String::new(),
        };

        rust_string_to_value(string.replace(&match_pattern, &repl))
    }
}

/// Replace first occurrence of match string with replacement
///
/// (replace-first "hello hello" "ll" "yy") => "heyyo hello"
#[no_mangle]
pub extern "C" fn clorus_replace_first(s: *mut Value, match_str: *mut Value, replacement: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return rust_string_to_value(String::new()),
        };

        let match_pattern = match get_string_value(match_str) {
            Some(m) => m,
            None => return rust_string_to_value(string),
        };

        let repl = match get_string_value(replacement) {
            Some(r) => r,
            None => String::new(),
        };

        rust_string_to_value(string.replacen(&match_pattern, &repl, 1))
    }
}

/// Regex find: returns first match as string or nil.
///
/// (re-find #\"[0-9]+\" \"abc123def\") => \"123\"
#[no_mangle]
pub extern "C" fn clorus_re_find(pattern: *mut Value, s: *mut Value) -> *mut Value {
    unsafe {
        let pattern = match get_string_value(pattern) {
            Some(p) => p,
            None => return Value::nil(),
        };
        let input = match get_string_value(s) {
            Some(v) => v,
            None => return Value::nil(),
        };

        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => return Value::nil(),
        };

        match re.captures(&input) {
            Some(caps) => captures_to_value(&caps),
            None => Value::nil(),
        }
    }
}

/// Regex matches: returns full match string only when regex matches entire input.
///
/// (re-matches #\"[0-9]+\" \"123\") => \"123\"
/// (re-matches #\"[0-9]+\" \"a123\") => nil
#[no_mangle]
pub extern "C" fn clorus_re_matches(pattern: *mut Value, s: *mut Value) -> *mut Value {
    unsafe {
        let pattern = match get_string_value(pattern) {
            Some(p) => p,
            None => return Value::nil(),
        };
        let input = match get_string_value(s) {
            Some(v) => v,
            None => return Value::nil(),
        };

        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => return Value::nil(),
        };

        if let Some(caps) = re.captures(&input) {
            if let Some(full) = caps.get(0) {
                if full.start() == 0 && full.end() == input.len() {
                    return captures_to_value(&caps);
                }
            }
        }
        Value::nil()
    }
}

/// Regex seq: returns vector of all match strings (or empty vector).
#[no_mangle]
pub extern "C" fn clorus_re_seq(pattern: *mut Value, s: *mut Value) -> *mut Value {
    unsafe {
        let pattern = match get_string_value(pattern) {
            Some(p) => p,
            None => return Value::nil(),
        };
        let input = match get_string_value(s) {
            Some(v) => v,
            None => return Value::nil(),
        };

        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => return Value::nil(),
        };

        let mut result = crate::vector::clorus_vector_empty();
        for caps in re.captures_iter(&input) {
            let mv = captures_to_value(&caps);
            let next = crate::vector::clorus_vector_conj(result, mv);
            crate::value::clorus_release(result);
            crate::value::clorus_release(mv);
            result = next;
        }
        if crate::vector::clorus_vector_count(result) == 0 {
            crate::value::clorus_release(result);
            Value::nil()
        } else {
            result
        }
    }
}

/// Regex replace all.
#[no_mangle]
pub extern "C" fn clorus_re_replace(s: *mut Value, pattern: *mut Value, replacement: *mut Value) -> *mut Value {
    unsafe {
        let input = match get_string_value(s) {
            Some(v) => v,
            None => return rust_string_to_value(String::new()),
        };
        let pattern = match get_string_value(pattern) {
            Some(p) => p,
            None => return rust_string_to_value(input),
        };
        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => return rust_string_to_value(input),
        };

        if crate::value::clorus_is_fn(replacement) {
            let replaced = re.replace_all(&input, |caps: &regex_lite::Captures<'_>| unsafe {
                let match_val = captures_to_value(caps);
                let args = [match_val];
                let ret = crate::function::clorus_function_call(replacement, args.as_ptr(), 1);
                crate::value::clorus_release(match_val);

                let out = match get_string_value(ret) {
                    Some(s) => s,
                    None => value_to_pr_string(ret),
                };
                crate::value::clorus_release(ret);
                out
            });
            return rust_string_to_value(replaced.to_string());
        }

        let replacement = match get_string_value(replacement) {
            Some(r) => r,
            None => String::new(),
        };
        rust_string_to_value(re.replace_all(&input, replacement.as_str()).to_string())
    }
}

/// Regex replace first.
#[no_mangle]
pub extern "C" fn clorus_re_replace_first(
    s: *mut Value,
    pattern: *mut Value,
    replacement: *mut Value,
) -> *mut Value {
    unsafe {
        let input = match get_string_value(s) {
            Some(v) => v,
            None => return rust_string_to_value(String::new()),
        };
        let pattern = match get_string_value(pattern) {
            Some(p) => p,
            None => return rust_string_to_value(input),
        };
        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => return rust_string_to_value(input),
        };

        if crate::value::clorus_is_fn(replacement) {
            let replaced = re.replace(&input, |caps: &regex_lite::Captures<'_>| unsafe {
                let match_val = captures_to_value(caps);
                let args = [match_val];
                let ret = crate::function::clorus_function_call(replacement, args.as_ptr(), 1);
                crate::value::clorus_release(match_val);

                let out = match get_string_value(ret) {
                    Some(s) => s,
                    None => value_to_pr_string(ret),
                };
                crate::value::clorus_release(ret);
                out
            });
            return rust_string_to_value(replaced.to_string());
        }

        let replacement = match get_string_value(replacement) {
            Some(r) => r,
            None => String::new(),
        };
        rust_string_to_value(re.replace(&input, replacement.as_str()).to_string())
    }
}

/// Regex validity check for stdlib-level re-pattern parity.
/// Returns 1 when `pattern` is a valid regex string, 0 otherwise.
#[no_mangle]
pub extern "C" fn clorus_regex_valid_i32(pattern: *mut Value) -> i32 {
    unsafe {
        let Some(pattern) = get_string_value(pattern) else {
            return 0;
        };
        if Regex::new(&pattern).is_ok() {
            1
        } else {
            0
        }
    }
}

// ============================================================================
// String Predicates
// ============================================================================

/// Check if value is a string
///
/// (string? "hello") => true
/// (string? 42) => false
#[no_mangle]
pub extern "C" fn clorus_is_string(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }

    unsafe {
        (*val).header().tag() == ValueTag::String
    }
}

#[no_mangle]
pub extern "C" fn clorus_is_string_i32(val: *mut Value) -> i32 {
    if clorus_is_string(val) { 1 } else { 0 }
}

/// Check if string starts with prefix
///
/// (starts-with? "hello world" "hello") => true
/// (starts-with? "hello" "hi") => false
///
/// Returns i32 (0/1), not bool: the LLVM declaration for this function
/// (declare_value2_to_i32_fn) expects a 4-byte i32 return value. A Rust
/// `bool` return is only 1 byte at the C ABI level, and the C ABI does not
/// guarantee the unused upper bytes of the return register are zeroed --
/// so the caller reading all 4 bytes as i32 could see garbage in them, or
/// (in the specific case of `false`, which is byte 0x00) still register as
/// truthy whenever those garbage upper bytes happen to be non-zero. This
/// silently broke every caller's false case. Every other function declared
/// via declare_value2_to_i32_fn / declare_value_to_i32_fn is already named
/// with an `_i32` suffix and already returns i32 correctly; these three
/// were the only ones that didn't follow that convention.
#[no_mangle]
pub extern "C" fn clorus_starts_with(s: *mut Value, prefix: *mut Value) -> i32 {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return 0,
        };

        let prefix_str = match get_string_value(prefix) {
            Some(p) => p,
            None => return 0,
        };

        string.starts_with(&prefix_str) as i32
    }
}

/// Check if string ends with suffix
///
/// (ends-with? "hello.txt" ".txt") => true
/// (ends-with? "hello" ".txt") => false
/// Returns i32 (0/1) -- see clorus_starts_with's doc comment for why.
#[no_mangle]
pub extern "C" fn clorus_ends_with(s: *mut Value, suffix: *mut Value) -> i32 {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return 0,
        };

        let suffix_str = match get_string_value(suffix) {
            Some(p) => p,
            None => return 0,
        };

        string.ends_with(&suffix_str) as i32
    }
}

/// Check if string contains substring
///
/// (includes? "hello world" "lo wo") => true
/// (includes? "hello" "x") => false
/// Returns i32 (0/1) -- see clorus_starts_with's doc comment for why.
#[no_mangle]
pub extern "C" fn clorus_includes(s: *mut Value, substr: *mut Value) -> i32 {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return 0,
        };

        let substring = match get_string_value(substr) {
            Some(p) => p,
            None => return 0,
        };

        string.contains(&substring) as i32
    }
}

/// Get character at index
///
/// (char-at "hello" 1) => "e"
/// (char-at "hello" 10) => nil (out of bounds)
#[no_mangle]
pub extern "C" fn clorus_char_at(s: *mut Value, index: i64) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return Value::nil(),
        };

        if index < 0 || index as usize >= string.len() {
            return Value::nil();
        }

        // Get character at byte index
        // Note: This is byte-based indexing, not Unicode scalar index
        match string.chars().nth(index as usize) {
            Some(ch) => rust_string_to_value(ch.to_string()),
            None => Value::nil(),
        }
    }
}

/// Find index of substring
///
/// (index-of "hello world" "world") => 6
/// (index-of "hello" "x") => nil
#[no_mangle]
pub extern "C" fn clorus_index_of(s: *mut Value, substr: *mut Value) -> *mut Value {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return Value::nil(),
        };

        let substring = match get_string_value(substr) {
            Some(p) => p,
            None => return Value::nil(),
        };

        match string.find(&substring) {
            Some(idx) => Value::long(idx as i64),
            None => Value::nil(),
        }
    }
}

// ============================================================================
// String Comparison
// ============================================================================

/// Lexicographic string comparison
///
/// Returns: -1 if s1 < s2, 0 if equal, 1 if s1 > s2
///
/// (compare "apple" "banana") => -1
/// (compare "hello" "hello") => 0
/// (compare "zebra" "apple") => 1
#[no_mangle]
pub extern "C" fn clorus_compare_strings(s1: *mut Value, s2: *mut Value) -> i64 {
    unsafe {
        let string1 = match get_string_value(s1) {
            Some(s) => s,
            None => String::new(),
        };

        let string2 = match get_string_value(s2) {
            Some(s) => s,
            None => String::new(),
        };

        match string1.cmp(&string2) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }
}

/// Compare two values for ordering.
/// Returns negative/zero/positive like clojure.core/compare.
///
/// Current contract:
/// - numbers compare numerically (Long/Double cross-compatible)
/// - strings/keywords/symbols compare lexicographically
/// - bool compares false < true
/// - nil compares before non-nil
/// - mixed remaining types fall back to tag ordering for deterministic behavior
#[no_mangle]
pub extern "C" fn clorus_compare_values(a: *mut Value, b: *mut Value) -> i64 {
    unsafe {
        if a.is_null() && b.is_null() {
            return 0;
        }
        if a.is_null() {
            return -1;
        }
        if b.is_null() {
            return 1;
        }

        let at = (*a).header().tag();
        let bt = (*b).header().tag();

        if at == ValueTag::Nil && bt == ValueTag::Nil {
            return 0;
        }
        if at == ValueTag::Nil {
            return -1;
        }
        if bt == ValueTag::Nil {
            return 1;
        }

        if (at == ValueTag::Long || at == ValueTag::Double) && (bt == ValueTag::Long || bt == ValueTag::Double) {
            let av = match at {
                ValueTag::Long => (*a).as_long() as f64,
                ValueTag::Double => (*a).as_double(),
                _ => 0.0,
            };
            let bv = match bt {
                ValueTag::Long => (*b).as_long() as f64,
                ValueTag::Double => (*b).as_double(),
                _ => 0.0,
            };
            return if av < bv {
                -1
            } else if av > bv {
                1
            } else {
                0
            };
        }

        if at == ValueTag::String && bt == ValueTag::String {
            let as_ = (*a).as_string();
            let bs_ = (*b).as_string();
            return if as_ < bs_ {
                -1
            } else if as_ > bs_ {
                1
            } else {
                0
            };
        }

        if at == ValueTag::Keyword && bt == ValueTag::Keyword {
            let as_ = (*a).as_keyword();
            let bs_ = (*b).as_keyword();
            return if as_ < bs_ {
                -1
            } else if as_ > bs_ {
                1
            } else {
                0
            };
        }

        if at == ValueTag::Symbol && bt == ValueTag::Symbol {
            let as_ = (*a).as_symbol();
            let bs_ = (*b).as_symbol();
            return if as_ < bs_ {
                -1
            } else if as_ > bs_ {
                1
            } else {
                0
            };
        }

        if at == ValueTag::Bool && bt == ValueTag::Bool {
            let av = (*a).as_bool();
            let bv = (*b).as_bool();
            return if av == bv {
                0
            } else if !av && bv {
                -1
            } else {
                1
            };
        }

        let at_num = at as i32;
        let bt_num = bt as i32;
        if at_num < bt_num {
            -1
        } else if at_num > bt_num {
            1
        } else {
            0
        }
    }
}

/// Create a string value from a C string pointer
/// Used for map destructuring with :strs
#[no_mangle]
pub extern "C" fn clorus_string(s_ptr: *const c_char) -> *mut Value {
    if s_ptr.is_null() {
        return Value::nil();
    }

    unsafe {
        let c_str = CStr::from_ptr(s_ptr);
        let s = c_str.to_str().unwrap_or("");
        rust_string_to_value(s.to_string())
    }
}

/// pr-str - Print to string with readable representation
///
/// Converts a value to a readable string representation:
/// - Strings are quoted and escaped
/// - Keywords include the colon
/// - Other values use their display format
///
/// # Arguments
/// * `val` - The value to convert to string
///
/// # Returns
/// A new String Value with the readable representation
#[no_mangle]
pub extern "C" fn clorus_string_data(val: *mut Value) -> *const c_char {
    unsafe {
        if val.is_null() {
            return std::ptr::null();
        }

        // Extract string from Value*
        if (*val).header().tag() != ValueTag::String {
            return std::ptr::null();
        }

        let rust_str = (*val).as_string();

        // Convert to C string and leak (caller must manage)
        // This is safe for protocol dispatch where we only read during dispatch
        let c_string = std::ffi::CString::new(rust_str).unwrap();
        c_string.into_raw() as *const c_char
    }
}

/// pr-str - Print to string with readable representation
///
/// Converts a value to a readable string representation:
/// - Strings are quoted and escaped
/// - Keywords include the colon
/// - Other values use their display format
///
/// # Arguments
/// * `val` - The value to convert to string
///
/// # Returns
/// A new String Value with the readable representation
#[no_mangle]
pub extern "C" fn clorus_pr_str(val: *mut Value) -> *mut Value {
    unsafe {
        let result = value_to_pr_string(val);
        rust_string_to_value(result)
    }
}

/// Convert a value to a readable (pr-str) representation
unsafe fn value_to_pr_string(val: *mut Value) -> String {
    if val.is_null() {
        return "nil".to_string();
    }

    match (*val).header().tag() {
        ValueTag::String => {
            // Strings should be quoted and escaped
            let s = (*val).as_string();
            format!("\"{}\"", s.replace('\\', "\\\\")
                               .replace('"', "\\\"")
                               .replace('\n', "\\n")
                               .replace('\t', "\\t")
                               .replace('\r', "\\r"))
        }
        ValueTag::Long => {
            format!("{}", (*val).as_long())
        }
        ValueTag::Double => {
            let num = (*val).as_double();
            if num.fract() == 0.0 && num.is_finite() {
                format!("{:.0}", num)
            } else {
                format!("{}", num)
            }
        }
        ValueTag::Keyword => {
            format!(":{}", (*val).as_keyword())
        }
        ValueTag::Bool => {
            if (*val).as_bool() {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        ValueTag::Nil => "nil".to_string(),
        ValueTag::Exception => {
            let payload = (*val).as_exception_payload();
            format!("(exception {})", value_to_pr_string(payload))
        }
        ValueTag::Vector => {
            // Print vector as [elem1 elem2 ...]
            let vec_ptr = (*val).as_ptr() as *const PersistentVector;
            if vec_ptr.is_null() {
                return "[]".to_string();
            }

            // Call clorus_vector_count to get count
            let count = crate::vector::clorus_vector_count(val);
            if count == 0 {
                return "[]".to_string();
            }

            let mut result = String::from("[");
            for i in 0..count {
                if i > 0 {
                    result.push(' ');
                }
                // Call clorus_vector_nth to get element
                let elem = crate::vector::clorus_vector_nth(val, i);
                result.push_str(&value_to_pr_string(elem));
            }
            result.push(']');
            result
        }
        ValueTag::List => {
            // Print list as (elem1 elem2 ...)
            let mut parts: Vec<String> = Vec::new();
            let mut current = val;
            loop {
                let count = crate::list::clorus_list_count(current);
                if count == 0 {
                    break;
                }
                let first = crate::list::clorus_list_first(current);
                parts.push(value_to_pr_string(first));
                crate::value::clorus_release(first);
                let next = crate::list::clorus_list_rest(current);
                if current != val {
                    crate::value::clorus_release(current);
                }
                current = next;
            }
            if current != val {
                crate::value::clorus_release(current);
            }
            format!("({})", parts.join(" "))
        }
        ValueTag::HashMap => {
            // Print map as {:k v ...}. Ordering is not guaranteed by runtime map.
            let map_ptr = (*val).as_ptr() as *mut crate::map::ClorusHashMap;
            let mut entries: Vec<(String, String)> = Vec::new();
            for (k, v) in (*map_ptr).entries_iter() {
                entries.push((value_to_pr_string(k), value_to_pr_string(v)));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let rendered: Vec<String> = entries
                .into_iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .collect();
            format!("{{{}}}", rendered.join(" "))
        }
        ValueTag::HashSet => {
            // Print set as #{elem1 elem2}. Ordering is normalized for deterministic output.
            let set_ptr = (*val).as_ptr() as *mut crate::set::ClorusHashSet;
            let mut elems: Vec<String> = (*set_ptr)
                .values()
                .iter()
                .map(|e| value_to_pr_string(*e))
                .collect();
            elems.sort();
            format!("#{{{}}}", elems.join(" "))
        }
        ValueTag::Function => {
            "#<function>".to_string()
        }
        ValueTag::MultiArityFunction => {
            "#<function>".to_string()
        }
        ValueTag::Atom => {
            "#<atom>".to_string()
        }
        ValueTag::Ref => {
            "#<ref>".to_string()
        }
        ValueTag::Agent => {
            "#<agent>".to_string()
        }
        ValueTag::Channel => {
            "#<channel>".to_string()
        }
        ValueTag::Var => {
            // Print var name: #'<var: *var-name*>
            let var_ptr = (*val).as_var();
            if var_ptr.is_null() {
                "#<var>".to_string()
            } else {
                let name_val = crate::var::clorus_var_name(var_ptr);
                if name_val.is_null() {
                    "#<var>".to_string()
                } else {
                    let name = (*name_val).as_string();
                    format!("#<var: {}>", name)
                }
            }
        }
        ValueTag::Symbol => {
            (*val).as_symbol().to_string()
        }
        ValueTag::OpaquePointer => {
            "#<opaque-pointer>".to_string()
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_concatenation() {
        unsafe {
            let mut vec = PersistentVector::empty();
            let s1 = rust_string_to_value("hello".to_string());
            let s2 = rust_string_to_value(" ".to_string());
            let s3 = rust_string_to_value("world".to_string());

            vec = PersistentVector::conj(vec, s1);
            vec = PersistentVector::conj(vec, s2);
            vec = PersistentVector::conj(vec, s3);

            let vec_val = Value::from_ptr(ValueTag::Vector, vec as *mut u8);
            let result = clorus_str(vec_val);

            let result_str = value_to_rust_string(result);
            assert_eq!(result_str, "hello world");

            crate::value::clorus_release(result);
            crate::value::clorus_release(vec_val);
        }
    }

    #[test]
    fn test_subs() {
        unsafe {
            let s = rust_string_to_value("hello world".to_string());

            let result = clorus_subs3(s, 0, 5);
            let result_str = value_to_rust_string(result);
            assert_eq!(result_str, "hello");

            crate::value::clorus_release(result);

            let result2 = clorus_subs2(s, 6);
            let result2_str = value_to_rust_string(result2);
            assert_eq!(result2_str, "world");

            crate::value::clorus_release(result2);
            crate::value::clorus_release(s);
        }
    }

    #[test]
    fn test_split() {
        unsafe {
            let s = rust_string_to_value("a,b,c".to_string());
            let delim = rust_string_to_value(",".to_string());

            let result = clorus_split(s, delim);

            // Check it's a vector
            assert_eq!((*result).header().tag(), ValueTag::Vector);

            let vec_ptr = (*result).as_ptr() as *mut PersistentVector;
            assert_eq!((*vec_ptr).count(), 3);

            crate::value::clorus_release(result);
            crate::value::clorus_release(s);
            crate::value::clorus_release(delim);
        }
    }

    #[test]
    fn test_case_conversion() {
        unsafe {
            let s = rust_string_to_value("hello".to_string());

            let upper = clorus_upper_case(s);
            let upper_str = value_to_rust_string(upper);
            assert_eq!(upper_str, "HELLO");

            let lower = clorus_lower_case(upper);
            let lower_str = value_to_rust_string(lower);
            assert_eq!(lower_str, "hello");

            crate::value::clorus_release(upper);
            crate::value::clorus_release(lower);
            crate::value::clorus_release(s);
        }
    }

    #[test]
    fn test_trim() {
        unsafe {
            let s = rust_string_to_value("  hello  ".to_string());

            let result = clorus_trim(s);
            let result_str = value_to_rust_string(result);
            assert_eq!(result_str, "hello");

            crate::value::clorus_release(result);
            crate::value::clorus_release(s);
        }
    }
}
