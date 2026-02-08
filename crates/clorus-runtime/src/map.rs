/// Basic HashMap implementation for Clorus
/// This is a simple initial implementation - will be replaced with
/// persistent HAMT (Hash Array Mapped Trie) in Phase C for full immutability

use crate::value::Value;
use std::collections::HashMap as StdHashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Simple wrapper for Clorus hash maps
/// TODO: Replace with persistent HAMT for immutability
pub struct ClorusHashMap {
    entries: StdHashMap<u64, (*mut Value, *mut Value)>,
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
        // Compute hash of key
        let hash = hash_value(key);

        // Retain the value
        crate::value::clorus_retain(key);
        crate::value::clorus_retain(val);

        // Insert into map
        if let Some((old_key, old_val)) = self.entries.insert(hash, (key, val)) {
            // Release old values
            crate::value::clorus_release(old_key);
            crate::value::clorus_release(old_val);
        }
    }

    /// Get a value by key, returns nil if not found
    /// IMPORTANT: This function retains the returned value before returning it,
    /// following the same pattern as PersistentVector::nth().
    /// The caller is responsible for releasing it when done.
    pub fn get(&self, key: *mut Value) -> *mut Value {
        let hash = hash_value(key);
        match self.entries.get(&hash) {
            Some((_, val)) => {
                // Retain the value before returning (same pattern as vector nth)
                unsafe {
                    if !val.is_null() {
                        (**val).header().retain();
                    }
                }
                *val
            },
            None => Value::nil(),
        }
    }

    pub fn count(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Get an iterator over entries (for dissoc, keys, vals)
    pub fn entries_iter(&self) -> impl Iterator<Item = (&u64, &(*mut Value, *mut Value))> {
        self.entries.iter()
    }
}

/// Hash a Value pointer for map lookups
/// For now, uses a simple hash - will be improved in Phase C
fn hash_value(val: *mut Value) -> u64 {
    if val.is_null() {
        return 0;
    }

    unsafe {
        match (*val).header().tag() {
            crate::value::ValueTag::Long => {
                let mut hasher = DefaultHasher::new();
                (*val).as_long().hash(&mut hasher);
                hasher.finish()
            }
            crate::value::ValueTag::Double => {
                let mut hasher = DefaultHasher::new();
                (*val).as_double().to_bits().hash(&mut hasher);
                hasher.finish()
            }
            crate::value::ValueTag::String => {
                let mut hasher = DefaultHasher::new();
                (*val).as_string().hash(&mut hasher);
                hasher.finish()
            }
            crate::value::ValueTag::Bool => {
                if (*val).as_bool() { 1 } else { 0 }
            }
            crate::value::ValueTag::Nil => 0,
            _ => {
                // For complex types, use pointer address as hash
                // This is temporary - proper structural hashing in Phase C
                val as u64
            }
        }
    }
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
    for (key, val) in (*map).entries.values() {
        crate::value::clorus_release(*key);
        crate::value::clorus_release(*val);
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
    if map_val.is_null() {
        return clorus_map_empty();
    }

    unsafe {
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;

        // For now, mutate in place
        // TODO: In Phase C, create a new persistent map
        (*map_ptr).assoc(key, val);

        // Return the same map (will be new persistent copy in Phase C)
        map_val
    }
}

#[no_mangle]
pub extern "C" fn clorus_map_get(map_val: *mut Value, key: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return Value::nil();
    }

    unsafe {
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        (*map_ptr).get(key)
    }
}

#[no_mangle]
pub extern "C" fn clorus_map_count(map_val: *mut Value) -> u64 {
    if map_val.is_null() {
        return 0;
    }

    unsafe {
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
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
