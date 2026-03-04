use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::list::PersistentList;
use crate::map::ClorusHashMap;
use crate::collections::{clorus_contains, clorus_get, clorus_nth};

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
    clorus_map_get_in_or(map_val, keys, Value::nil())
}

/// Get nested value from a map using a path of keys, with explicit not-found.
pub extern "C" fn clorus_map_get_in_or(
    map_val: *mut Value,
    keys: *mut Value,
    not_found: *mut Value,
) -> *mut Value {
    if map_val.is_null() {
        if !not_found.is_null() {
            unsafe { (*not_found).header().retain(); }
            return not_found;
        }
        return Value::nil();
    }

    let not_found = if not_found.is_null() {
        Value::nil()
    } else {
        not_found
    };

    unsafe {
        let count = match path_count(keys) {
            Some(c) => c,
            None => {
                (*not_found).header().retain();
                return not_found;
            }
        };

        // Empty key path returns the original value.
        if count == 0 {
            (*map_val).header().retain();
            return map_val;
        }

        let mut current = map_val;
        let mut owns_current = false;

        for i in 0..count {
            let key = clorus_nth(keys, i as i64);
            let contains = clorus_contains(current, key);
            let key_present = !contains.is_null()
                && (*contains).header().tag() == ValueTag::Bool
                && (*contains).as_bool();
            crate::value::clorus_release(contains);

            if !key_present {
                crate::value::clorus_release(key);
                if owns_current {
                    crate::value::clorus_release(current);
                }
                (*not_found).header().retain();
                return not_found;
            }

            let next = clorus_get(current, key);
            crate::value::clorus_release(key);

            if owns_current {
                crate::value::clorus_release(current);
            }
            current = next;
            owns_current = true;
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
    unsafe {
        let count = match path_count(keys) {
            Some(c) => c,
            None => {
                if map_val.is_null() {
                    return crate::map::clorus_map_empty();
                }
                (*map_val).header().retain();
                return map_val;
            }
        };

        // assoc-in on nil/non-map should create nested maps.
        let base_map = if map_val.is_null() || (*map_val).header().tag() != ValueTag::HashMap {
            crate::map::clorus_map_empty()
        } else {
            map_val
        };

        if count == 0 {
            // Clojure parity: (assoc-in m [] v) => (assoc m nil v)
            return crate::map::clorus_map_assoc(base_map, Value::nil(), value);
        }

        if count == 1 {
            // Base case: single key
            let key = clorus_nth(keys, 0);
            let result = crate::map::clorus_map_assoc(base_map, key, value);
            crate::value::clorus_release(key);
            return result;
        }

        // Recursive case: assoc-in on nested map
        let first_key = clorus_nth(keys, 0);
        let nested_map = clorus_get(base_map, first_key);
        let nested_base = if nested_map.is_null() || (*nested_map).header().tag() != ValueTag::HashMap {
            crate::map::clorus_map_empty()
        } else {
            nested_map
        };

        // Create rest of keys vector
        let mut rest_keys = PersistentVector::empty();
        for i in 1..count {
            let key = clorus_nth(keys, i as i64);
            rest_keys = PersistentVector::conj(rest_keys, key);
            crate::value::clorus_release(key);
        }
        let rest_keys_val = Value::from_ptr(ValueTag::Vector, rest_keys as *mut u8);

        // Recursively update nested map
        let updated_nested = clorus_map_assoc_in(nested_base, rest_keys_val, value);

        // Assoc the updated nested map back into the original map
        let result = crate::map::clorus_map_assoc(base_map, first_key, updated_nested);

        crate::value::clorus_release(first_key);
        crate::value::clorus_release(rest_keys_val);
        crate::value::clorus_release(updated_nested);
        if !nested_map.is_null() {
            crate::value::clorus_release(nested_map);
        }
        if nested_base != nested_map {
            crate::value::clorus_release(nested_base);
        }
        result
    }
}

unsafe fn path_count(keys: *mut Value) -> Option<u64> {
    if keys.is_null() {
        return Some(0);
    }

    match (*keys).header().tag() {
        ValueTag::Nil => Some(0),
        ValueTag::Vector => {
            let vec_ptr = (*keys).as_ptr() as *mut PersistentVector;
            Some((*vec_ptr).count())
        }
        ValueTag::List => {
            let list_ptr = (*keys).as_ptr() as *mut PersistentList;
            Some((*list_ptr).count())
        }
        _ => None,
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
            if map_val.is_null() {
                return crate::map::clorus_map_empty();
            }
            (*map_val).header().retain();
            return map_val;
        }
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
