use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::map::ClorusHashMap;
use crate::collections::clorus_get;

/// Remove a key from a map (dissoc)
#[no_mangle]
pub extern "C" fn clorus_map_dissoc(map_val: *mut Value, key: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return crate::map::clorus_map_empty();
    }

    unsafe {
        if (*map_val).header().tag() != ValueTag::HashMap {
            (*map_val).header().retain();
            return map_val;
        }

        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;

        // For now, create a new map without the key
        // TODO: Use persistent data structure in Phase C
        let new_map = ClorusHashMap::empty();
        // Copy all entries except the one to remove
        for (k, v) in (*map_ptr).entries_iter() {
            if !crate::value::clorus_equals(*k, key) {
                (*new_map).assoc(*k, *v);
            }
        }

        Value::from_ptr(ValueTag::HashMap, new_map as *mut u8)
    }
}

/// Get all keys from a map as a vector
#[no_mangle]
pub extern "C" fn clorus_map_keys(map_val: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        if (*map_val).header().tag() != ValueTag::HashMap {
            return crate::vector::clorus_vector_empty();
        }

        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        let mut vec = PersistentVector::empty();

        for (k, _) in (*map_ptr).entries_iter() {
            vec = PersistentVector::conj(vec, *k);
            (*(*k)).header().retain();
        }

        Value::from_ptr(ValueTag::Vector, vec as *mut u8)
    }
}

/// Get all values from a map as a vector
#[no_mangle]
pub extern "C" fn clorus_map_vals(map_val: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        if (*map_val).header().tag() != ValueTag::HashMap {
            return crate::vector::clorus_vector_empty();
        }

        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        let mut vec = PersistentVector::empty();

        for (_, v) in (*map_ptr).entries_iter() {
            vec = PersistentVector::conj(vec, *v);
            (*(*v)).header().retain();
        }

        Value::from_ptr(ValueTag::Vector, vec as *mut u8)
    }
}

/// Merge multiple maps together
#[no_mangle]
pub extern "C" fn clorus_map_merge(maps: *mut Value) -> *mut Value {
    if maps.is_null() {
        return crate::map::clorus_map_empty();
    }

    unsafe {
        let result = ClorusHashMap::empty();

        // Maps should be a vector of maps
        if (*maps).header().tag() != ValueTag::Vector {
            return crate::map::clorus_map_empty();
        }

        let vec_ptr = (*maps).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        for i in 0..count {
            let map_val = PersistentVector::nth(vec_ptr, i);
            if !map_val.is_null() && (*map_val).header().tag() == ValueTag::HashMap {
                let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;

                // Copy all entries from this map
                for (k, v) in (*map_ptr).entries_iter() {
                    (*result).assoc(*k, *v);
                }
            }
        }

        Value::from_ptr(ValueTag::HashMap, result as *mut u8)
    }
}

/// Get nested value from a map using a path of keys (get-in)
#[no_mangle]
pub extern "C" fn clorus_map_get_in(map_val: *mut Value, keys: *mut Value) -> *mut Value {
    if map_val.is_null() || keys.is_null() {
        return Value::nil();
    }

    unsafe {
        let mut current = map_val;

        // Keys should be a vector
        if (*keys).header().tag() != ValueTag::Vector {
            return Value::nil();
        }

        let vec_ptr = (*keys).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        for i in 0..count {
            let key = PersistentVector::nth(vec_ptr, i);
            current = clorus_get(current, key);

            if current.is_null() || (*current).header().tag() == ValueTag::Nil {
                return Value::nil();
            }
        }

        current
    }
}

/// Update nested value in a map using a path of keys (assoc-in)
#[no_mangle]
pub extern "C" fn clorus_map_assoc_in(
    map_val: *mut Value,
    keys: *mut Value,
    value: *mut Value
) -> *mut Value {
    if keys.is_null() {
        unsafe {
            if !map_val.is_null() {
                (*map_val).header().retain();
            }
        }
        return map_val;
    }

    unsafe {
        // Keys should be a vector
        if (*keys).header().tag() != ValueTag::Vector {
            (*map_val).header().retain();
            return map_val;
        }

        let vec_ptr = (*keys).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        if count == 0 {
            (*map_val).header().retain();
            return map_val;
        }

        if count == 1 {
            // Base case: single key
            let key = PersistentVector::nth(vec_ptr, 0);
            return crate::map::clorus_map_assoc(map_val, key, value);
        }

        // Recursive case: assoc-in on nested map
        let first_key = PersistentVector::nth(vec_ptr, 0);
        let nested_map = clorus_get(map_val, first_key);

        // Create rest of keys vector
        let mut rest_keys = PersistentVector::empty();
        for i in 1..count {
            let key = PersistentVector::nth(vec_ptr, i);
            rest_keys = PersistentVector::conj(rest_keys, key);
        }
        let rest_keys_val = Value::from_ptr(ValueTag::Vector, rest_keys as *mut u8);

        // Recursively update nested map
        let updated_nested = clorus_map_assoc_in(nested_map, rest_keys_val, value);

        // Assoc the updated nested map back into the original map
        let result = crate::map::clorus_map_assoc(map_val, first_key, updated_nested);

        crate::value::clorus_release(rest_keys_val);
        result
    }
}

/// Update a value in a map by applying a function (update)
/// Function should be a callable Value
#[no_mangle]
pub extern "C" fn clorus_map_update(
    map_val: *mut Value,
    key: *mut Value,
    func: *mut Value
) -> *mut Value {
    if map_val.is_null() || func.is_null() {
        unsafe {
            if !map_val.is_null() {
                (*map_val).header().retain();
            }
        }
        return map_val;
    }

    let _current_val = clorus_get(map_val, key);

    // Call the function with the current value
    // This will be implemented when we have function calling in runtime
    // For now, just return the map unchanged
    // TODO: Implement clorus_call_function in runtime
    unsafe {
        if !map_val.is_null() {
            (*map_val).header().retain();
        }
    }
    map_val
}
