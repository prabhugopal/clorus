//! Transducer support for Clorus runtime
//!
//! Provides reduced value support for early termination in transducers.
//! Reduced values are represented as maps: {:type :reduced :value val}

use crate::value::Value;
use crate::keyword::clorus_keyword;
use crate::map::{clorus_map_empty, clorus_map_assoc};

/// Create a reduced value wrapper
/// Returns a map {:type :reduced :value val}
#[no_mangle]
pub extern "C" fn clorus_reduced(val: *mut Value) -> *mut Value {
    // Create empty map
    let map = clorus_map_empty();

    // Create :type keyword
    let type_kw = clorus_keyword(b":type\0".as_ptr() as *const i8);

    // Create :reduced keyword (the value for :type)
    let reduced_kw = clorus_keyword(b":reduced\0".as_ptr() as *const i8);

    // Assoc :type :reduced
    let map = clorus_map_assoc(map, type_kw, reduced_kw);

    // Create :value keyword
    let value_kw = clorus_keyword(b":value\0".as_ptr() as *const i8);

    // Assoc :value val
    let map = clorus_map_assoc(map, value_kw, val);

    map
}

/// Check if a value is reduced
/// Returns true if value is a map with {:type :reduced}
#[no_mangle]
pub extern "C" fn clorus_is_reduced(val: *mut Value) -> bool {
    unsafe {
        if val.is_null() {
            return false;
        }

        // Check if it's a map
        if (*val).tag() != crate::value::ValueTag::HashMap {
            return false;
        }

        // Get :type key
        let type_kw = clorus_keyword(b":type\0".as_ptr() as *const i8);

        // Get value for :type
        let type_val = crate::map::clorus_map_get(val, type_kw);

        if type_val.is_null() {
            return false;
        }

        // Check if :type value is :reduced keyword
        if (*type_val).tag() != crate::value::ValueTag::Keyword {
            return false;
        }

        let type_str = (*type_val).as_keyword();
        type_str == "reduced"
    }
}

/// Extract the value from a reduced wrapper
/// Returns the :value field from the map
#[no_mangle]
pub extern "C" fn clorus_deref_reduced(val: *mut Value) -> *mut Value {
    if val.is_null() {
        return std::ptr::null_mut();
    }

    // Get :value key
    let value_kw = clorus_keyword(b":value\0".as_ptr() as *const i8);

    // Get and return the value
    crate::map::clorus_map_get(val, value_kw)
}

/// Ensure a value is reduced
/// If already reduced, returns it unchanged
/// Otherwise wraps it with clorus_reduced
#[no_mangle]
pub extern "C" fn clorus_ensure_reduced(val: *mut Value) -> *mut Value {
    if clorus_is_reduced(val) {
        val
    } else {
        clorus_reduced(val)
    }
}
