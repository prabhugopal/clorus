/// Generic collection access functions
///
/// These functions work polymorphically across different collection types
/// (vectors, lists, maps) by checking the ValueTag and dispatching to
/// the appropriate type-specific implementation.

use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::list::PersistentList;
use crate::map::ClorusHashMap;

/// Get element from collection
///
/// - For maps: get value by key
/// - For vectors: get by index (if key is a number)
/// - For lists: not supported (use nth)
#[no_mangle]
pub extern "C" fn clorus_get(coll: *mut Value, key: *mut Value) -> *mut Value {
    if coll.is_null() || key.is_null() {
        return Value::nil();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::HashMap => {
                crate::map::clorus_map_get(coll, key)
            }
            ValueTag::Vector => {
                // For vectors, key must be a number (Long or Double)
                let index = match (*key).header().tag() {
                    ValueTag::Long => (*key).as_long() as u64,
                    ValueTag::Double => (*key).as_double() as u64,
                    _ => return Value::nil(),
                };
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                PersistentVector::nth(vec_ptr, index)
            }
            _ => Value::nil(),
        }
    }
}

/// Get element at index from a sequential collection
///
/// Works with vectors and lists
#[no_mangle]
pub extern "C" fn clorus_nth(coll: *mut Value, index: i64) -> *mut Value {
    if coll.is_null() || index < 0 {
        return Value::nil();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                PersistentVector::nth(vec_ptr, index as u64)
            }
            ValueTag::List => {
                // For lists, walk to nth position (logic below)
                clorus_list_nth(coll, index)
            }
            _ => Value::nil(),
        }
    }
}

// Helper function for nth on lists (add to list.rs if not present)
#[no_mangle]
pub extern "C" fn clorus_list_nth(list_val: *mut Value, index: i64) -> *mut Value {
    if list_val.is_null() || index < 0 {
        return Value::nil();
    }

    unsafe {
        let list_ptr = (*list_val).as_ptr() as *mut PersistentList;
        let list = &*list_ptr;

        let mut current = list.clone();
        let mut i = 0;

        while i < index {
            if current.is_empty() {
                return Value::nil();
            }
            current = current.rest();
            i += 1;
        }

        if current.is_empty() {
            Value::nil()
        } else {
            let first = current.first();
            if !first.is_null() {
                (*first).header().retain();
            }
            first
        }
    }
}

/// Get first element of a collection
///
/// Works with vectors and lists
#[no_mangle]
pub extern "C" fn clorus_first(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return Value::nil();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                if (*vec_ptr).is_empty() {
                    Value::nil()
                } else {
                    PersistentVector::nth(vec_ptr, 0)
                }
            }
            ValueTag::List => {
                let list_ptr = (*coll).as_ptr() as *mut PersistentList;
                (*list_ptr).first()
            }
            _ => Value::nil(),
        }
    }
}

/// Get rest of collection (all but first)
///
/// Works with vectors and lists
#[no_mangle]
pub extern "C" fn clorus_rest(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return crate::list::clorus_list_empty();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                let count = (*vec_ptr).count();

                if count <= 1 {
                    // Empty vector or single element - return empty list
                    return crate::list::clorus_list_empty();
                }

                // Create a new vector with elements [1..]
                let mut new_vec = PersistentVector::empty();
                for i in 1..count {
                    let elem = PersistentVector::nth(vec_ptr, i);
                    new_vec = PersistentVector::conj(new_vec, elem);
                    if !elem.is_null() {
                        crate::value::clorus_release(elem);
                    }
                }

                Value::from_ptr(ValueTag::Vector, new_vec as *mut u8)
            }
            ValueTag::List => {
                crate::list::clorus_list_rest(coll)
            }
            _ => crate::list::clorus_list_empty(),
        }
    }
}

/// Get last element of a collection
///
/// Works with vectors and lists
#[no_mangle]
pub extern "C" fn clorus_last(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return Value::nil();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                let count = (*vec_ptr).count();

                if count == 0 {
                    Value::nil()
                } else {
                    PersistentVector::nth(vec_ptr, count - 1)
                }
            }
            ValueTag::List => {
                let list_ptr = (*coll).as_ptr() as *mut PersistentList;
                let list = &*list_ptr;

                if list.is_empty() {
                    return Value::nil();
                }

                // Walk to the end
                let mut current = list.clone();
                let mut last_val = Value::nil();

                while !current.is_empty() {
                    if !last_val.is_null() {
                        crate::value::clorus_release(last_val);
                    }
                    last_val = current.first();
                    if !last_val.is_null() {
                        (*last_val).header().retain();
                    }
                    current = current.rest();
                }

                last_val
            }
            _ => Value::nil(),
        }
    }
}

/// Get count of elements in collection
///
/// Works with vectors, lists, maps, and sets
#[no_mangle]
pub extern "C" fn clorus_count(coll: *mut Value) -> i64 {
    if coll.is_null() {
        return 0;
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                (*vec_ptr).count() as i64
            }
            ValueTag::List => {
                crate::list::clorus_list_count(coll) as i64
            }
            ValueTag::HashMap => {
                let map_ptr = (*coll).as_ptr() as *mut ClorusHashMap;
                (*map_ptr).count() as i64
            }
            ValueTag::HashSet => {
                let set_ptr = (*coll).as_ptr() as *mut crate::set::ClorusHashSet;
                (*set_ptr).count() as i64
            }
            ValueTag::String => {
                // Get string length
                let s = (*coll).as_string();
                s.len() as i64
            }
            _ => 0,
        }
    }
}

/// Remove a key from a map (dissoc)
#[no_mangle]
pub extern "C" fn clorus_map_dissoc(map_val: *mut Value, key: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return crate::map::clorus_map_empty();
    }

    unsafe {
        if (*map_val).header().tag() != ValueTag::HashMap {
            return map_val;
        }

        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;

        // For now, create a new map without the key
        // TODO: Use persistent data structure in Phase C
        let new_map = ClorusHashMap::empty();
        let hash_to_remove = crate::map::hash_value_pub(key);

        // Copy all entries except the one to remove
        for (hash, (k, v)) in (*map_ptr).entries_iter() {
            if *hash != hash_to_remove {
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

        for (_, (k, _)) in (*map_ptr).entries_iter() {
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

        for (_, (_, v)) in (*map_ptr).entries_iter() {
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
                for (_, (k, v)) in (*map_ptr).entries_iter() {
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
        return map_val;
    }

    unsafe {
        // Keys should be a vector
        if (*keys).header().tag() != ValueTag::Vector {
            return map_val;
        }

        let vec_ptr = (*keys).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        if count == 0 {
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
        return map_val;
    }

    let _current_val = clorus_get(map_val, key);

    // Call the function with the current value
    // This will be implemented when we have function calling in runtime
    // For now, just return the map unchanged
    // TODO: Implement clorus_call_function in runtime
    map_val
}

/// Take first n elements from a collection
#[no_mangle]
pub extern "C" fn clorus_take(coll: *mut Value, n: i64) -> *mut Value {
    if coll.is_null() || n <= 0 {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                let count = (*vec_ptr).count().min(n as u64);
                let mut result = PersistentVector::empty();

                for i in 0..count {
                    let elem = PersistentVector::nth(vec_ptr, i);
                    result = PersistentVector::conj(result, elem);
                    if !elem.is_null() {
                        crate::value::clorus_release(elem);
                    }
                }

                Value::from_ptr(ValueTag::Vector, result as *mut u8)
            }
            ValueTag::List => {
                let list_ptr = (*coll).as_ptr() as *mut PersistentList;
                let mut current = (*list_ptr).clone();
                let mut result = PersistentVector::empty();
                let mut i = 0;

                while i < n && !current.is_empty() {
                    let elem = current.first();
                    result = PersistentVector::conj(result, elem);
                    current = current.rest();
                    i += 1;
                }

                Value::from_ptr(ValueTag::Vector, result as *mut u8)
            }
            _ => crate::vector::clorus_vector_empty(),
        }
    }
}

/// Drop first n elements from a collection
#[no_mangle]
pub extern "C" fn clorus_drop(coll: *mut Value, n: i64) -> *mut Value {
    if coll.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    if n <= 0 {
        return coll;
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                let count = (*vec_ptr).count();
                let start = (n as u64).min(count);
                let mut result = PersistentVector::empty();

                for i in start..count {
                    let elem = PersistentVector::nth(vec_ptr, i);
                    result = PersistentVector::conj(result, elem);
                    if !elem.is_null() {
                        crate::value::clorus_release(elem);
                    }
                }

                Value::from_ptr(ValueTag::Vector, result as *mut u8)
            }
            ValueTag::List => {
                let list_ptr = (*coll).as_ptr() as *mut PersistentList;
                let mut current = (*list_ptr).clone();
                let mut i = 0;

                while i < n && !current.is_empty() {
                    current = current.rest();
                    i += 1;
                }

                let mut result = PersistentVector::empty();
                while !current.is_empty() {
                    let elem = current.first();
                    result = PersistentVector::conj(result, elem);
                    current = current.rest();
                }

                Value::from_ptr(ValueTag::Vector, result as *mut u8)
            }
            _ => crate::vector::clorus_vector_empty(),
        }
    }
}

/// Concatenate multiple collections into one vector
#[no_mangle]
pub extern "C" fn clorus_concat(colls: *mut Value) -> *mut Value {
    if colls.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        let mut result = PersistentVector::empty();

        // colls should be a vector of collections
        if (*colls).header().tag() != ValueTag::Vector {
            return crate::vector::clorus_vector_empty();
        }

        let vec_ptr = (*colls).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        for i in 0..count {
            let coll = PersistentVector::nth(vec_ptr, i);

            match (*coll).header().tag() {
                ValueTag::Vector => {
                    let inner_vec = (*coll).as_ptr() as *mut PersistentVector;
                    let inner_count = (*inner_vec).count();

                    for j in 0..inner_count {
                        let elem = PersistentVector::nth(inner_vec, j);
                        result = PersistentVector::conj(result, elem);
                        if !elem.is_null() {
                            crate::value::clorus_release(elem);
                        }
                    }
                }
                ValueTag::List => {
                    let list_ptr = (*coll).as_ptr() as *mut PersistentList;
                    let mut current = (*list_ptr).clone();

                    while !current.is_empty() {
                        let elem = current.first();
                        result = PersistentVector::conj(result, elem);
                        current = current.rest();
                    }
                }
                _ => {}
            }
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

/// Interleave multiple collections (alternate elements from each)
#[no_mangle]
pub extern "C" fn clorus_interleave(colls: *mut Value) -> *mut Value {
    if colls.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        // colls should be a vector of collections
        if (*colls).header().tag() != ValueTag::Vector {
            return crate::vector::clorus_vector_empty();
        }

        let vec_ptr = (*colls).as_ptr() as *mut PersistentVector;
        let num_colls = (*vec_ptr).count();

        if num_colls == 0 {
            return crate::vector::clorus_vector_empty();
        }

        let mut result = PersistentVector::empty();
        let mut index = 0;

        // Interleave until any collection is exhausted
        loop {
            let mut added_any = false;

            for i in 0..num_colls {
                let coll = PersistentVector::nth(vec_ptr, i);
                let elem = clorus_nth(coll, index);

                if !elem.is_null() && (*elem).header().tag() != ValueTag::Nil {
                    result = PersistentVector::conj(result, elem);
                    crate::value::clorus_release(elem);
                    added_any = true;
                }
            }

            if !added_any {
                break;
            }

            index += 1;
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

/// Insert a separator between elements of a collection
#[no_mangle]
pub extern "C" fn clorus_interpose(coll: *mut Value, sep: *mut Value) -> *mut Value {
    if coll.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        let mut result = PersistentVector::empty();
        let count = clorus_count(coll);

        if count == 0 {
            return crate::vector::clorus_vector_empty();
        }

        for i in 0..count {
            let elem = clorus_nth(coll, i);
            result = PersistentVector::conj(result, elem);
            crate::value::clorus_release(elem);

            // Add separator except after last element
            if i < count - 1 {
                result = PersistentVector::conj(result, sep);
            }
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

/// Remove duplicate elements from a collection (distinct)
#[no_mangle]
pub extern "C" fn clorus_distinct(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        let mut result = PersistentVector::empty();
        let mut seen = std::collections::HashSet::new();
        let count = clorus_count(coll);

        for i in 0..count {
            let elem = clorus_nth(coll, i);

            // Use simple hash for deduplication
            let hash = crate::map::hash_value_pub(elem);

            if !seen.contains(&hash) {
                seen.insert(hash);
                result = PersistentVector::conj(result, elem);
            }

            crate::value::clorus_release(elem);
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

/// Remove consecutive duplicate elements (dedupe)
#[no_mangle]
pub extern "C" fn clorus_dedupe(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        let count = clorus_count(coll);

        if count == 0 {
            return crate::vector::clorus_vector_empty();
        }

        let mut result = PersistentVector::empty();
        let mut prev_hash: Option<u64> = None;

        for i in 0..count {
            let elem = clorus_nth(coll, i);
            let hash = crate::map::hash_value_pub(elem);

            if prev_hash.is_none() || prev_hash.unwrap() != hash {
                result = PersistentVector::conj(result, elem);
                prev_hash = Some(hash);
            }

            crate::value::clorus_release(elem);
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

/// Flatten nested collections into a single vector
#[no_mangle]
pub extern "C" fn clorus_flatten(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        let mut result = PersistentVector::empty();
        flatten_into(&mut result, coll);
        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

unsafe fn flatten_into(result: &mut *mut PersistentVector, coll: *mut Value) {
    if coll.is_null() {
        return;
    }

    match (*coll).header().tag() {
        ValueTag::Vector | ValueTag::List => {
            let count = clorus_count(coll);
            for i in 0..count {
                let elem = clorus_nth(coll, i);
                flatten_into(result, elem);
                crate::value::clorus_release(elem);
            }
        }
        _ => {
            // Not a collection - add as leaf
            *result = PersistentVector::conj(*result, coll);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nth_vector() {
        unsafe {
            let vec = crate::vector::clorus_vector_empty();
            let val1 = Value::double(1.0);
            let val2 = Value::double(2.0);
            let val3 = Value::double(3.0);

            let vec1 = crate::vector::clorus_vector_conj(vec, val1);
            let vec2 = crate::vector::clorus_vector_conj(vec1, val2);
            let vec3 = crate::vector::clorus_vector_conj(vec2, val3);

            let result = clorus_nth(vec3, 1);
            assert_eq!((*result).as_double(), 2.0);

            crate::value::clorus_release(result);
            crate::value::clorus_release(vec);
            crate::value::clorus_release(vec1);
            crate::value::clorus_release(vec2);
            crate::value::clorus_release(vec3);
        }
    }

    #[test]
    fn test_first_vector() {
        unsafe {
            let vec = crate::vector::clorus_vector_empty();
            let val1 = Value::double(42.0);
            let vec1 = crate::vector::clorus_vector_conj(vec, val1);

            let result = clorus_first(vec1);
            assert_eq!((*result).as_double(), 42.0);

            crate::value::clorus_release(result);
            crate::value::clorus_release(vec);
            crate::value::clorus_release(vec1);
        }
    }

    #[test]
    fn test_last_vector() {
        unsafe {
            let vec = crate::vector::clorus_vector_empty();
            let val1 = Value::double(1.0);
            let val2 = Value::double(2.0);
            let val3 = Value::double(99.0);

            let vec1 = crate::vector::clorus_vector_conj(vec, val1);
            let vec2 = crate::vector::clorus_vector_conj(vec1, val2);
            let vec3 = crate::vector::clorus_vector_conj(vec2, val3);

            let result = clorus_last(vec3);
            assert_eq!((*result).as_double(), 99.0);

            crate::value::clorus_release(result);
            crate::value::clorus_release(vec);
            crate::value::clorus_release(vec1);
            crate::value::clorus_release(vec2);
            crate::value::clorus_release(vec3);
        }
    }

    #[test]
    fn test_count_vector() {
        unsafe {
            let vec = crate::vector::clorus_vector_empty();
            let val1 = Value::double(1.0);
            let val2 = Value::double(2.0);

            assert_eq!(clorus_count(vec), 0);

            let vec1 = crate::vector::clorus_vector_conj(vec, val1);
            assert_eq!(clorus_count(vec1), 1);

            let vec2 = crate::vector::clorus_vector_conj(vec1, val2);
            assert_eq!(clorus_count(vec2), 2);

            crate::value::clorus_release(vec);
            crate::value::clorus_release(vec1);
            crate::value::clorus_release(vec2);
        }
    }

    #[test]
    fn test_get_map() {
        unsafe {
            let map = crate::map::clorus_map_empty();
            let key = crate::keyword::clorus_keyword(b"name\0".as_ptr() as *const i8);
            let val = Value::string("Alice");

            let map1 = crate::map::clorus_map_assoc(map, key, val);

            let result = clorus_get(map1, key);
            assert!(!result.is_null());
            assert_eq!((*result).header().tag(), ValueTag::String);

            crate::value::clorus_release(result);
            crate::value::clorus_release(key);
            crate::value::clorus_release(map);
            crate::value::clorus_release(map1);
        }
    }
}
