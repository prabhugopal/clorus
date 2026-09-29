use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::list::PersistentList;
use crate::collections::{clorus_count, clorus_nth};

/// Concatenate multiple finite collections into one sequence list.
///
/// The runtime materializes eagerly because Clorus does not yet have a lazy
/// sequence runtime. Returning a List, rather than leaking the vector used
/// while building it, preserves the public `seq?` contract of clojure.core/
/// concat and keeps the representation boundary in one place.
#[no_mangle]
pub extern "C" fn clorus_concat(colls: *mut Value) -> *mut Value {
    if colls.is_null() {
        return crate::list::clorus_list_empty();
    }

    unsafe {
        let mut result = PersistentVector::empty();

        // colls should be a vector of collections
        if (*colls).header().tag() != ValueTag::Vector {
            return crate::list::clorus_list_empty();
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
                ValueTag::HashMap | ValueTag::HashSet => {
                    // Matches Clojure: (concat [1 2] {:a 1} #{3}) => (1 2 [:a 1] 3)
                    // -- maps contribute [k v] entries, sets contribute their
                    // elements. Reuses the same seqable-vector helper collections
                    // uses for first/rest/last/nth/take/drop instead of a third
                    // copy of "how do I walk a HashMap/HashSet" logic.
                    if let Some(seq_vec) = crate::collections::coll_as_seqable_vector(coll) {
                        let inner_vec = (*seq_vec).as_ptr() as *mut PersistentVector;
                        let inner_count = (*inner_vec).count();
                        for j in 0..inner_count {
                            let elem = PersistentVector::nth(inner_vec, j);
                            result = PersistentVector::conj(result, elem);
                            if !elem.is_null() {
                                crate::value::clorus_release(elem);
                            }
                        }
                        crate::value::clorus_release(seq_vec);
                    }
                }
                _ => {}
            }
        }

        let vector_result = Value::from_ptr(ValueTag::Vector, result as *mut u8);
        let list_result = crate::list::clorus_vector_to_list(vector_result);
        crate::value::clorus_release(vector_result);
        list_result
    }
}

/// Interleave multiple finite collections until the shortest is exhausted.
///
/// Like `clorus_concat`, this materializes eagerly but exposes its result as
/// Clorus's canonical List sequence. Counting before indexing is deliberate:
/// a nil element is valid data and must not be mistaken for an exhausted
/// collection.
#[no_mangle]
pub extern "C" fn clorus_interleave(colls: *mut Value) -> *mut Value {
    if colls.is_null() {
        return crate::list::clorus_list_empty();
    }

    unsafe {
        // colls should be a vector of collections
        if (*colls).header().tag() != ValueTag::Vector {
            return crate::list::clorus_list_empty();
        }

        let vec_ptr = (*colls).as_ptr() as *mut PersistentVector;
        let num_colls = (*vec_ptr).count();

        if num_colls == 0 {
            return crate::list::clorus_list_empty();
        }

        let mut result = PersistentVector::empty();
        let mut index = 0;

        // Interleave until *any* collection is exhausted. Checking counts
        // first preserves nil values, which clorus_nth also uses as its
        // out-of-range sentinel.
        loop {
            for i in 0..num_colls {
                let coll = PersistentVector::nth(vec_ptr, i);
                if clorus_count(coll) <= index {
                    let vector_result = Value::from_ptr(ValueTag::Vector, result as *mut u8);
                    let list_result = crate::list::clorus_vector_to_list(vector_result);
                    crate::value::clorus_release(vector_result);
                    return list_result;
                }
            }

            for i in 0..num_colls {
                let coll = PersistentVector::nth(vec_ptr, i);
                let elem = clorus_nth(coll, index);
                result = PersistentVector::conj(result, elem);
                if !elem.is_null() {
                    crate::value::clorus_release(elem);
                }
            }

            index += 1;
        }
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
