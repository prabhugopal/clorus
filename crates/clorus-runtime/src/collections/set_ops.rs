use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::collections::{clorus_count, clorus_nth};

/// Remove duplicate elements from a collection (distinct)
#[no_mangle]
pub extern "C" fn clorus_distinct(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return crate::vector::clorus_vector_empty();
    }

    unsafe {
        let mut result = PersistentVector::empty();
        let mut seen: std::collections::HashMap<u64, Vec<*mut Value>> = std::collections::HashMap::new();
        let count = clorus_count(coll);

        for i in 0..count {
            let elem = clorus_nth(coll, i);

            let hash = crate::hash::clorus_hash(elem);
            let mut exists = false;
            if let Some(bucket) = seen.get(&hash) {
                for existing in bucket {
                    if crate::value::clorus_equals(*existing, elem) {
                        exists = true;
                        break;
                    }
                }
            }

            if !exists {
                result = PersistentVector::conj(result, elem);
                seen.entry(hash).or_default().push(elem);
            } else {
                crate::value::clorus_release(elem);
            }
        }

        for bucket in seen.values() {
            for val in bucket {
                crate::value::clorus_release(*val);
            }
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
        let mut prev_val: Option<*mut Value> = None;

        for i in 0..count {
            let elem = clorus_nth(coll, i);

            let should_add = match prev_val {
                None => true,
                Some(prev) => !crate::value::clorus_equals(prev, elem),
            };

            if should_add {
                result = PersistentVector::conj(result, elem);
                if let Some(prev) = prev_val {
                    crate::value::clorus_release(prev);
                }
                prev_val = Some(elem);
            } else {
                crate::value::clorus_release(elem);
            }
        }

        if let Some(prev) = prev_val {
            crate::value::clorus_release(prev);
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}
