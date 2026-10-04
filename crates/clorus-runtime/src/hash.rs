use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::value::{Value, ValueTag};

/// Combine two hash values (order-dependent)
pub(crate) fn hash_combine(seed: u64, value: u64) -> u64 {
    let mut hash = seed;
    hash ^= value
        .wrapping_add(0x9e3779b97f4a7c15)
        .wrapping_add(hash << 6)
        .wrapping_add(hash >> 2);
    hash
}

fn hash_number(val: f64) -> u64 {
    let bits = if val.is_nan() {
        // Canonical NaN
        0x7ff8_0000_0000_0000u64
    } else {
        val.to_bits()
    };
    let mut hasher = DefaultHasher::new();
    bits.hash(&mut hasher);
    hasher.finish()
}

/// Compute structural hash for a Value*
#[no_mangle]
pub extern "C" fn clorus_hash(val: *mut Value) -> u64 {
    if val.is_null() {
        return 0;
    }

    unsafe {
        match (*val).header().tag() {
            ValueTag::Nil => 0,
            ValueTag::Bool => {
                if (*val).as_bool() { 1 } else { 0 }
            }
            ValueTag::Long => {
                hash_number((*val).as_long() as f64)
            }
            ValueTag::Double => {
                hash_number((*val).as_double())
            }
            ValueTag::String => {
                let mut hasher = DefaultHasher::new();
                (*val).as_string().hash(&mut hasher);
                hasher.finish()
            }
            ValueTag::Char => {
                let mut hasher = DefaultHasher::new();
                (*val).as_char().hash(&mut hasher);
                hasher.finish()
            }
            ValueTag::Keyword => {
                let mut hasher = DefaultHasher::new();
                (*val).as_keyword().hash(&mut hasher);
                hasher.finish()
            }
            ValueTag::Symbol => {
                let mut hasher = DefaultHasher::new();
                (*val).as_symbol().hash(&mut hasher);
                hasher.finish()
            }
            ValueTag::Vector => {
                let vec_ptr = (*val).as_ptr() as *mut crate::vector::PersistentVector;
                let count = (*vec_ptr).count();
                let mut state = 0u64;
                for i in 0..count {
                    let elem = crate::vector::PersistentVector::nth(vec_ptr, i);
                    let h = clorus_hash(elem);
                    state = hash_combine(state, h);
                    if !elem.is_null() {
                        crate::value::clorus_release(elem);
                    }
                }
                state
            }
            ValueTag::List => {
                let list_ptr = (*val).as_ptr() as *mut crate::list::PersistentList;
                crate::list::list_hash(list_ptr)
            }
            ValueTag::LazySeq | ValueTag::SeqNode | ValueTag::VectorSeq => hash_sequential(val),
            ValueTag::HashMap => {
                let map_ptr = (*val).as_ptr() as *mut crate::map::ClorusHashMap;
                let mut state = 0u64;
                for (key, value) in (*map_ptr).entries_iter() {
                    let h_key = clorus_hash(key);
                    let h_val = clorus_hash(value);
                    let entry_hash = h_key ^ h_val.rotate_left(1);
                    state ^= entry_hash;
                }
                state
            }
            ValueTag::HashSet => {
                let set_ptr = (*val).as_ptr() as *mut crate::set::ClorusHashSet;
                let mut state = 0u64;
                for value in (*set_ptr).values().iter() {
                    let h = clorus_hash(*value);
                    state ^= h;
                }
                state
            }
            ValueTag::Atom | ValueTag::Ref | ValueTag::Agent | ValueTag::Channel |
            ValueTag::Function | ValueTag::MultiArityFunction | ValueTag::Var |
            ValueTag::OpaquePointer | ValueTag::Socket => {
                (*val).as_ptr() as u64
            }
            ValueTag::Exception => {
                let payload = (*val).as_exception_payload();
                hash_combine(0xEC7E_0001, clorus_hash(payload))
            }
        }
    }
}

/// Exact ordered hash across general sequence values.
///
/// The absence of a bound is intentional: an arbitrary bound would give equal
/// finite sequences different hashes after the cut-off and violate hash-map
/// correctness. Hashing an unbounded sequence therefore has the same
/// non-termination contract as Clojure.
unsafe fn hash_sequential(value: *mut Value) -> u64 {
    let mut current = crate::collections::clorus_seq(value);
    let mut state = 0_u64;

    loop {
        if crate::value::clorus_is_exception(current) {
            let result = clorus_hash(current);
            crate::value::clorus_release(current);
            return result;
        }
        if current.is_null() || (*current).tag() == ValueTag::Nil {
            if !current.is_null() {
                crate::value::clorus_release(current);
            }
            return state;
        }

        let element = crate::collections::clorus_first(current);
        state = hash_combine(state, clorus_hash(element));
        crate::value::clorus_release(element);

        let rest = crate::collections::clorus_rest(current);
        crate::value::clorus_release(current);
        current = crate::collections::clorus_seq(rest);
        crate::value::clorus_release(rest);
    }
}
