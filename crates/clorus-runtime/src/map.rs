/// Basic HashMap implementation for Clorus
/// This is a simple initial implementation - will be replaced with
/// persistent HAMT (Hash Array Mapped Trie) in Phase C for full immutability

use crate::value::Value;
use std::collections::HashMap as StdHashMap;

/// Simple wrapper for Clorus hash maps
/// TODO: Replace with persistent HAMT for immutability
pub struct ClorusHashMap {
    entries: StdHashMap<u64, Vec<(*mut Value, *mut Value)>>,
}

impl ClorusHashMap {
    pub fn empty() -> *mut Self {
        Box::into_raw(Box::new(ClorusHashMap {
            entries: StdHashMap::new(),
        }))
    }

    /// Create a new map by adding/updating a key-value pair
    /// For now, this mutates - will be persistent in Phase C
    pub unsafe fn assoc(&mut self, key: *mut Value, val: *mut Value) {
        let hash = hash_value(key);

        if let Some(bucket) = self.entries.get_mut(&hash) {
            for (existing_key, existing_val) in bucket.iter_mut() {
                if crate::value::clorus_equals(*existing_key, key) {
                    // Replace existing entry
                    crate::value::clorus_retain(key);
                    crate::value::clorus_retain(val);
                    crate::value::clorus_release(*existing_key);
                    crate::value::clorus_release(*existing_val);
                    *existing_key = key;
                    *existing_val = val;
                    return;
                }
            }

            // Not found in bucket - insert new
            crate::value::clorus_retain(key);
            crate::value::clorus_retain(val);
            bucket.push((key, val));
            return;
        }

        // No bucket yet - create one
        crate::value::clorus_retain(key);
        crate::value::clorus_retain(val);
        self.entries.insert(hash, vec![(key, val)]);
    }

    /// Get a value by key, returns nil if not found
    /// IMPORTANT: This function retains the returned value before returning it,
    /// following the same pattern as PersistentVector::nth().
    /// The caller is responsible for releasing it when done.
    pub fn get(&self, key: *mut Value) -> *mut Value {
        let hash = hash_value(key);
        if let Some(bucket) = self.entries.get(&hash) {
            for (bucket_key, bucket_val) in bucket {
                if crate::value::clorus_equals(*bucket_key, key) {
                    unsafe {
                        if !bucket_val.is_null() {
                            (**bucket_val).header().retain();
                        }
                    }
                    return *bucket_val;
                }
            }
        }

        Value::nil()
    }

    /// Get a value by key without retaining (internal use)
    pub fn get_entry(&self, key: *mut Value) -> Option<*mut Value> {
        let hash = hash_value(key);
        if let Some(bucket) = self.entries.get(&hash) {
            for (bucket_key, bucket_val) in bucket {
                if crate::value::clorus_equals(*bucket_key, key) {
                    return Some(*bucket_val);
                }
            }
        }
        None
    }

    pub fn count(&self) -> u64 {
        self.entries.values().map(|bucket| bucket.len() as u64).sum()
    }

    /// Get an iterator over entries (for dissoc, keys, vals)
    pub fn entries_iter(&self) -> impl Iterator<Item = &(*mut Value, *mut Value)> {
        self.entries.values().flat_map(|bucket| bucket.iter())
    }
}

/// Hash a Value pointer for map lookups
/// For now, uses a simple hash - will be improved in Phase C
fn hash_value(val: *mut Value) -> u64 {
    crate::hash::clorus_hash(val)
}

/// Public version of hash_value for use in collections module
pub fn hash_value_pub(val: *mut Value) -> u64 {
    hash_value(val)
}

/// Release a hash map and all its contents
pub unsafe fn release_map(map: *mut ClorusHashMap) {
    if map.is_null() {
        return;
    }

    // Release all keys and values
    for bucket in (*map).entries.values() {
        for (key, val) in bucket {
            crate::value::clorus_release(*key);
            crate::value::clorus_release(*val);
        }
    }

    // Free the map itself
    drop(Box::from_raw(map));
}

// FFI functions for LLVM

#[no_mangle]
pub extern "C" fn clorus_map_empty() -> *mut Value {
    let map = ClorusHashMap::empty();
    Value::from_ptr(crate::value::ValueTag::HashMap, map as *mut u8)
}

#[no_mangle]
pub extern "C" fn clorus_map_assoc(
    map_val: *mut Value,
    key: *mut Value,
    val: *mut Value
) -> *mut Value {
    unsafe {
        let target_map = if map_val.is_null() {
            let map = ClorusHashMap::empty();
            Value::from_ptr(crate::value::ValueTag::HashMap, map as *mut u8)
        } else if (*map_val).header().tag() != crate::value::ValueTag::HashMap {
            let map = ClorusHashMap::empty();
            Value::from_ptr(crate::value::ValueTag::HashMap, map as *mut u8)
        } else if (*map_val).as_ptr().is_null() {
            let map = ClorusHashMap::empty();
            Value::from_ptr(crate::value::ValueTag::HashMap, map as *mut u8)
        } else {
            map_val
        };
        let map_ptr = (*target_map).as_ptr() as *mut ClorusHashMap;
        if map_ptr.is_null() {
            crate::value::clorus_retain(target_map);
            return target_map;
        }
        if key.is_null() {
            crate::value::clorus_retain(target_map);
            return target_map;
        }
        let value_to_store = if val.is_null() { Value::nil() } else { val };

        // For now, mutate in place
        // TODO: In Phase C, create a new persistent map
        (*map_ptr).assoc(key, value_to_store);

        // Return the same map (will be new persistent copy in Phase C).
        // Retain to allow caller to release both old and "new" maps safely.
        crate::value::clorus_retain(target_map);
        target_map
    }
}

#[no_mangle]
pub extern "C" fn clorus_map_get(map_val: *mut Value, key: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*map_val).header().tag() != crate::value::ValueTag::HashMap {
            return Value::nil();
        }
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        if map_ptr.is_null() {
            return Value::nil();
        }
        (*map_ptr).get(key)
    }
}

#[no_mangle]
pub extern "C" fn clorus_map_count(map_val: *mut Value) -> u64 {
    if map_val.is_null() {
        return 0;
    }

    unsafe {
        if (*map_val).header().tag() != crate::value::ValueTag::HashMap {
            return 0;
        }
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        if map_ptr.is_null() {
            return 0;
        }
        (*map_ptr).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_map() {
        let map_val = clorus_map_empty();
        unsafe {
            assert_eq!((*map_val).header().tag(), crate::value::ValueTag::HashMap);
        }
        unsafe { crate::value::clorus_release(map_val); }
    }

    #[test]
    fn test_map_assoc_get() {
        let map = clorus_map_empty();
        let key = Value::double(42.0);
        let val = Value::string("hello");

        let map2 = clorus_map_assoc(map, key, val);
        let result = clorus_map_get(map2, key);

        unsafe {
            assert_eq!((*result).as_string(), "hello");
            crate::value::clorus_release(map);
            crate::value::clorus_release(key);
            crate::value::clorus_release(val);
        }
    }
}
