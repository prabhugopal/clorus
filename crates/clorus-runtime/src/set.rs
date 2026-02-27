/// HashSet implementation for Clorus
/// This is a simple initial implementation - will be replaced with
/// persistent hash set in future for full immutability

use crate::value::Value;
use std::collections::HashMap as StdHashMap;

/// Simple wrapper for Clorus hash sets
/// TODO: Replace with persistent hash set for immutability
pub struct ClorusHashSet {
    entries: StdHashMap<u64, Vec<*mut Value>>,
    // Store actual values separately for iteration/reference counting
    values: Vec<*mut Value>,
}

impl ClorusHashSet {
    pub fn empty() -> *mut Self {
        Box::into_raw(Box::new(ClorusHashSet {
            entries: StdHashMap::new(),
            values: Vec::new(),
        }))
    }

    /// Add a value to the set
    /// For now, this mutates - will be persistent in future
    pub unsafe fn conj(&mut self, val: *mut Value) {
        let hash = hash_value(val);

        // Only add if not already present
        if let Some(bucket) = self.entries.get_mut(&hash) {
            for existing in bucket.iter() {
                if crate::value::clorus_equals(*existing, val) {
                    return;
                }
            }
            crate::value::clorus_retain(val);
            bucket.push(val);
            self.values.push(val);
            return;
        }

        crate::value::clorus_retain(val);
        self.entries.insert(hash, vec![val]);
        self.values.push(val);
    }

    /// Remove a value from the set
    pub unsafe fn disj(&mut self, val: *mut Value) {
        let hash = hash_value(val);

        if let Some(bucket) = self.entries.get_mut(&hash) {
            if let Some(pos) = bucket.iter().position(|&v| crate::value::clorus_equals(v, val)) {
                let removed = bucket.remove(pos);
                if bucket.is_empty() {
                    self.entries.remove(&hash);
                }

                if let Some(index) = self.values.iter().position(|&v| crate::value::clorus_equals(v, removed)) {
                    let old_val = self.values.remove(index);
                    crate::value::clorus_release(old_val);
                }
            }
        }
    }

    /// Check if a value is in the set
    pub fn contains(&self, val: *mut Value) -> bool {
        let hash = hash_value(val);
        if let Some(bucket) = self.entries.get(&hash) {
            for existing in bucket.iter() {
                if crate::value::clorus_equals(*existing, val) {
                    return true;
                }
            }
        }
        false
    }

    pub fn count(&self) -> u64 {
        self.values.len() as u64
    }

    /// Get all values in the set (for iteration)
    pub fn values(&self) -> &[*mut Value] {
        &self.values
    }
}

/// Hash a Value pointer for set lookups
/// Reuses logic from map.rs
fn hash_value(val: *mut Value) -> u64 {
    crate::hash::clorus_hash(val)
}

/// Release a hash set and all its contents
pub unsafe fn release_set(set: *mut ClorusHashSet) {
    if set.is_null() {
        return;
    }

    // Release all values
    for val in (*set).values.iter() {
        crate::value::clorus_release(*val);
    }

    // Drop the set itself
    drop(Box::from_raw(set));
}

// ========================================
// FFI exports for LLVM
// ========================================

use crate::value::ValueTag;

/// Create an empty set
#[no_mangle]
pub extern "C" fn clorus_set_empty() -> *mut Value {
    let set_ptr = ClorusHashSet::empty();
    Value::from_ptr(ValueTag::HashSet, set_ptr as *mut u8)
}

/// Add a value to a set, returning a new set
/// For now this mutates, will be persistent in future
#[no_mangle]
pub extern "C" fn clorus_set_conj(set_val: *mut Value, val: *mut Value) -> *mut Value {
    if set_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*set_val).header().tag() != ValueTag::HashSet {
            return Value::nil();
        }

        let set_ptr = (*set_val).as_ptr() as *mut ClorusHashSet;
        (*set_ptr).conj(val);

        // For now, return the same set (mutation)
        // In future, create a new set (persistence)
        (*set_val).header().retain();
        set_val
    }
}

/// Remove a value from a set, returning a new set
#[no_mangle]
pub extern "C" fn clorus_set_disj(set_val: *mut Value, val: *mut Value) -> *mut Value {
    if set_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*set_val).header().tag() != ValueTag::HashSet {
            return Value::nil();
        }

        let set_ptr = (*set_val).as_ptr() as *mut ClorusHashSet;
        (*set_ptr).disj(val);

        // For now, return the same set (mutation)
        (*set_val).header().retain();
        set_val
    }
}

/// Check if a set contains a value
#[no_mangle]
pub extern "C" fn clorus_set_contains(set_val: *mut Value, val: *mut Value) -> *mut Value {
    if set_val.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        if (*set_val).header().tag() != ValueTag::HashSet {
            return Value::boolean(false);
        }

        let set_ptr = (*set_val).as_ptr() as *mut ClorusHashSet;
        if (*set_ptr).contains(val) {
            Value::boolean(true)
        } else {
            Value::boolean(false)
        }
    }
}

/// Get the count of elements in a set
#[no_mangle]
pub extern "C" fn clorus_set_count(set_val: *mut Value) -> u64 {
    if set_val.is_null() {
        return 0;
    }

    unsafe {
        if (*set_val).header().tag() != ValueTag::HashSet {
            return 0;
        }

        let set_ptr = (*set_val).as_ptr() as *mut ClorusHashSet;
        (*set_ptr).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_set() {
        let set = clorus_set_empty();
        unsafe {
            assert_eq!((*set).header().tag(), ValueTag::HashSet);
            let set_ptr = (*set).as_ptr() as *mut ClorusHashSet;
            assert_eq!((*set_ptr).count(), 0);
        }
        unsafe { crate::value::clorus_release(set); }
    }

    #[test]
    fn test_set_conj() {
        let set = clorus_set_empty();
        let val1 = Value::double(42.0);
        let val2 = Value::double(43.0);

        let set2 = clorus_set_conj(set, val1);
        let set3 = clorus_set_conj(set2, val2);

        unsafe {
            let set_ptr = (*set3).as_ptr() as *mut ClorusHashSet;
            assert_eq!((*set_ptr).count(), 2);
            assert!((*set_ptr).contains(val1));
            assert!((*set_ptr).contains(val2));
        }

        unsafe {
            crate::value::clorus_release(set3);
            crate::value::clorus_release(val1);
            crate::value::clorus_release(val2);
        }
    }

    #[test]
    fn test_set_contains() {
        let set = clorus_set_empty();
        let val = Value::double(42.0);

        // Before adding
        let contains_before = clorus_set_contains(set, val);
        unsafe {
            assert!(!(*contains_before).as_bool());
        }

        // After adding
        let set2 = clorus_set_conj(set, val);
        let contains_after = clorus_set_contains(set2, val);
        unsafe {
            assert!((*contains_after).as_bool());
        }

        unsafe {
            crate::value::clorus_release(set2);
            crate::value::clorus_release(val);
            crate::value::clorus_release(contains_before);
            crate::value::clorus_release(contains_after);
        }
    }
}
