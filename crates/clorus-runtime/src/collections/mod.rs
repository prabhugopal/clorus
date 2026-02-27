/// Generic collection access functions
///
/// These functions work polymorphically across different collection types
/// (vectors, lists, maps) by checking the ValueTag and dispatching to
/// the appropriate type-specific implementation.

use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::list::PersistentList;
use crate::map::ClorusHashMap;

pub mod map_ops;
pub mod concat;
pub mod set_ops;

pub use map_ops::*;
pub use concat::*;
pub use set_ops::*;

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

/// Generic conj - add element to collection
/// Works with vectors, lists, and sets
/// For vectors: adds to end
/// For lists: adds to front
/// For sets: adds element (if not present)
#[no_mangle]
pub extern "C" fn clorus_conj(coll: *mut Value, elem: *mut Value) -> *mut Value {
    if coll.is_null() {
        return Value::nil();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => {
                crate::vector::clorus_vector_conj(coll, elem)
            }
            ValueTag::List => {
                crate::list::clorus_list_cons(coll, elem)
            }
            ValueTag::HashSet => {
                crate::set::clorus_set_conj(coll, elem)
            }
            _ => Value::nil(),
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
