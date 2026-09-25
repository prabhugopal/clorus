/// Keyword support for Clorus
/// Keywords are interned symbols that start with :
/// Same keyword always has the same pointer address for fast equality

use crate::value::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::os::raw::c_char;
use std::ffi::CStr;

/// Global keyword intern table
/// Keywords are never deallocated - they live for the program lifetime
/// Store as usize to make it Send-safe for Mutex
static KEYWORD_TABLE: Mutex<Option<HashMap<String, usize>>> = Mutex::new(None);

/// Initialize the keyword table
fn get_keyword_table() -> std::sync::MutexGuard<'static, Option<HashMap<String, usize>>> {
    let mut table = KEYWORD_TABLE.lock().unwrap();
    if table.is_none() {
        *table = Some(HashMap::new());
    }
    table
}

/// Intern a keyword - returns the same pointer for the same keyword string
/// Keywords are stored globally and never deallocated
pub fn intern_keyword(name: &str) -> *mut Value {
    let mut table_guard = get_keyword_table();
    let table = table_guard.as_mut().unwrap();

    if let Some(&existing) = table.get(name) {
        // Keyword already interned, return existing pointer
        existing as *mut Value
    } else {
        // Create new keyword value
        let keyword_val = Value::keyword(name);

        // Store in intern table (as usize for Send safety)
        table.insert(name.to_string(), keyword_val as usize);

        keyword_val
    }
}

/// FFI function to create a keyword from a C string
#[no_mangle]
pub extern "C" fn clorus_keyword(name_ptr: *const c_char) -> *mut Value {
    if name_ptr.is_null() {
        return Value::nil();
    }

    unsafe {
        let c_str = CStr::from_ptr(name_ptr);
        let name = c_str.to_str().unwrap_or("");
        intern_keyword(name)
    }
}

/// Check if a Value is a keyword
#[no_mangle]
pub extern "C" fn clorus_is_keyword(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }

    unsafe {
        (*val).header().tag() == crate::value::ValueTag::Keyword
    }
}

#[no_mangle]
pub extern "C" fn clorus_is_keyword_i32(val: *mut Value) -> i32 {
    if clorus_is_keyword(val) { 1 } else { 0 }
}

/// Get the keyword name as a C string
/// The returned string must be freed with clorus_free_cstring
#[no_mangle]
pub extern "C" fn clorus_keyword_name(val: *mut Value) -> *mut c_char {
    if val.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        if (*val).header().tag() == crate::value::ValueTag::Keyword {
            let name = (*val).as_keyword();
            match std::ffi::CString::new(name) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    }
}

/// Get the "name" of a keyword, symbol, or string, matching Clojure's
/// (name x): the keyword/symbol name with no leading `:` or namespace
/// separator, or the string itself unchanged. Returns null for any other
/// type (real Clojure throws ClassCastException there).
/// The returned string must be freed with clorus_free_cstring.
#[no_mangle]
pub extern "C" fn clorus_name(val: *mut Value) -> *mut c_char {
    if val.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let name = match (*val).header().tag() {
            crate::value::ValueTag::Keyword => (*val).as_keyword(),
            crate::value::ValueTag::Symbol => (*val).as_symbol(),
            crate::value::ValueTag::String => (*val).as_string(),
            _ => return std::ptr::null_mut(),
        };
        match std::ffi::CString::new(name) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_interning() {
        let k1 = intern_keyword("name");
        let k2 = intern_keyword("name");
        let k3 = intern_keyword("age");

        // Same keyword = same pointer
        assert_eq!(k1, k2);

        // Different keywords = different pointers
        assert_ne!(k1, k3);

        unsafe {
            assert_eq!((*k1).as_keyword(), "name");
            assert_eq!((*k3).as_keyword(), "age");
        }
    }

    #[test]
    fn test_keyword_ffi() {
        let c_name = std::ffi::CString::new("test").unwrap();
        let k1 = clorus_keyword(c_name.as_ptr());
        let k2 = clorus_keyword(c_name.as_ptr());

        assert_eq!(k1, k2);
        assert!(clorus_is_keyword(k1));

        let name_ptr = clorus_keyword_name(k1);
        unsafe {
            let name = CStr::from_ptr(name_ptr).to_str().unwrap();
            assert_eq!(name, "test");
            crate::value::clorus_free_cstring(name_ptr);
        }
    }
}
