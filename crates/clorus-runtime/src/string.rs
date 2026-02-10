/// String operations for Clorus
///
/// This module provides comprehensive string manipulation functions including
/// concatenation, extraction, splitting, joining, case conversion, and more.
///
/// All functions follow the Clorus memory model:
/// - Input strings are read-only
/// - New strings are created with refcount 1
/// - Caller owns the returned reference

use crate::value::{Value, ValueTag, Header, ValueData};
use crate::vector::PersistentVector;
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
        ValueTag::HashMap => {
            // Simple map representation
            format!("{{...}}")  // Simplified for now
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

/// Check if string starts with prefix
///
/// (starts-with? "hello world" "hello") => true
/// (starts-with? "hello" "hi") => false
#[no_mangle]
pub extern "C" fn clorus_starts_with(s: *mut Value, prefix: *mut Value) -> bool {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return false,
        };

        let prefix_str = match get_string_value(prefix) {
            Some(p) => p,
            None => return false,
        };

        string.starts_with(&prefix_str)
    }
}

/// Check if string ends with suffix
///
/// (ends-with? "hello.txt" ".txt") => true
/// (ends-with? "hello" ".txt") => false
#[no_mangle]
pub extern "C" fn clorus_ends_with(s: *mut Value, suffix: *mut Value) -> bool {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return false,
        };

        let suffix_str = match get_string_value(suffix) {
            Some(p) => p,
            None => return false,
        };

        string.ends_with(&suffix_str)
    }
}

/// Check if string contains substring
///
/// (includes? "hello world" "lo wo") => true
/// (includes? "hello" "x") => false
#[no_mangle]
pub extern "C" fn clorus_includes(s: *mut Value, substr: *mut Value) -> bool {
    unsafe {
        let string = match get_string_value(s) {
            Some(s) => s,
            None => return false,
        };

        let substring = match get_string_value(substr) {
            Some(p) => p,
            None => return false,
        };

        string.contains(&substring)
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
            "(...list...)".to_string() // TODO: Implement list printing
        }
        ValueTag::HashMap => {
            // Print map as {:key1 val1, :key2 val2}
            "{...map...}".to_string() // TODO: Implement map printing
        }
        ValueTag::HashSet => {
            // Print set as #{elem1 elem2}
            "#{...set...}".to_string() // TODO: Implement set printing
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
            "symbol".to_string() // TODO: Implement when Symbol type is added
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
