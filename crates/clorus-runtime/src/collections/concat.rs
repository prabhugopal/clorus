use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::list::PersistentList;
use crate::collections::{clorus_count, clorus_nth};

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
