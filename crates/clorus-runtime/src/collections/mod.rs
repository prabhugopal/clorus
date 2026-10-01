/// Generic collection access functions
///
/// These functions work polymorphically across different collection types
/// (vectors, lists, maps) by checking the ValueTag and dispatching to
/// the appropriate type-specific implementation.

use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use crate::list::PersistentList;
use crate::map::ClorusHashMap;
use crate::set::ClorusHashSet;

pub mod map_ops;
pub mod concat;
pub mod set_ops;

pub use map_ops::*;
pub use concat::*;
pub use set_ops::*;

/// Materialize a string as a list of `Char` values. Strings are immutable and
/// do not carry a sequence cursor, so a real list is the smallest existing
/// sequence representation that keeps the public `seq` contract uniform.
unsafe fn string_as_char_list(string_value: *mut Value) -> *mut Value {
    let mut result = crate::list::clorus_list_empty();
    for ch in (*string_value).as_string().chars().rev() {
        let char_value = Value::char(ch);
        let next = crate::list::clorus_list_cons(result, char_value);
        crate::value::clorus_release(result);
        crate::value::clorus_release(char_value);
        result = next;
    }
    result
}

/// Materialize a HashMap's entries as a Vector of [k v] 2-element vectors,
/// in the map's iteration order, and a HashSet's elements as a Vector, in
/// the set's iteration order.
///
/// Clorus doesn't have a lazy map-entry-seq or set-seq type, so sequence
/// operations (first/rest/last/nth/take/drop) can't walk a HashMap/HashSet
/// directly the way they walk a Vector or List. Rather than duplicating
/// each of those operations' logic once per collection type, every
/// HashMap/HashSet arm of those operations converts through this one
/// shared helper and then delegates back to the same operation's existing,
/// already-correct Vector handling -- so there is exactly one
/// implementation of "what does first/rest/last/nth/take/drop mean", not
/// one per collection type it's been taught to understand.
///
/// Matches real Clojure semantics for these: (first {:a 1}) => [:a 1],
/// (first #{1 2 3}) => some element, etc. -- maps and sets are genuinely
/// seqable in Clojure, not just vectors and lists.
pub(crate) unsafe fn coll_as_seqable_vector(coll: *mut Value) -> Option<*mut Value> {
    match (*coll).header().tag() {
        ValueTag::HashMap => {
            let map_ptr = (*coll).as_ptr() as *mut ClorusHashMap;
            let mut result = crate::vector::clorus_vector_empty();
            for (k, v) in (*map_ptr).entries_iter() {
                let pair = crate::vector::clorus_vector_empty();
                let pair_k = crate::vector::clorus_vector_conj(pair, k);
                crate::value::clorus_release(pair);
                let pair_kv = crate::vector::clorus_vector_conj(pair_k, v);
                crate::value::clorus_release(pair_k);
                let next = crate::vector::clorus_vector_conj(result, pair_kv);
                crate::value::clorus_release(result);
                crate::value::clorus_release(pair_kv);
                result = next;
            }
            Some(result)
        }
        ValueTag::HashSet => {
            let set_ptr = (*coll).as_ptr() as *mut ClorusHashSet;
            let mut result = crate::vector::clorus_vector_empty();
            for elem in (*set_ptr).values() {
                let next = crate::vector::clorus_vector_conj(result, elem);
                crate::value::clorus_release(result);
                result = next;
            }
            Some(result)
        }
        _ => None,
    }
}

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
            ValueTag::String => {
                let index = match (*key).header().tag() {
                    ValueTag::Long if (*key).as_long() >= 0 => (*key).as_long() as usize,
                    ValueTag::Double if (*key).as_double().is_finite()
                        && (*key).as_double() >= 0.0
                        && (*key).as_double().fract() == 0.0 => (*key).as_double() as usize,
                    _ => return Value::nil(),
                };
                (*coll).as_string().chars().nth(index).map(Value::char).unwrap_or_else(Value::nil)
            }
            ValueTag::HashSet => {
                // (get set k) => k itself if present (equality is what
                // matters here, not object identity), else nil.
                let set_ptr = (*coll).as_ptr() as *mut ClorusHashSet;
                if (*set_ptr).contains(key) {
                    (*key).header().retain();
                    key
                } else {
                    Value::nil()
                }
            }
            _ => Value::nil(),
        }
    }
}

/// Check whether a collection contains a key/index/value.
///
/// Semantics:
/// - maps: key presence
/// - vectors: index presence (numeric key)
/// - sets: element presence
/// - others: false
#[no_mangle]
pub extern "C" fn clorus_contains(coll: *mut Value, key: *mut Value) -> *mut Value {
    if coll.is_null() || key.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::HashMap => {
                let map_ptr = (*coll).as_ptr() as *mut ClorusHashMap;
                Value::boolean((*map_ptr).get_entry(key).is_some())
            }
            ValueTag::Vector => {
                let idx_opt: Option<u64> = match (*key).header().tag() {
                    ValueTag::Long => {
                        let i = (*key).as_long();
                        if i < 0 { None } else { Some(i as u64) }
                    }
                    ValueTag::Double => {
                        let d = (*key).as_double();
                        if d.is_finite() && d >= 0.0 && d.fract() == 0.0 {
                            Some(d as u64)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(idx) = idx_opt {
                    let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                    Value::boolean(idx < (*vec_ptr).count())
                } else {
                    Value::boolean(false)
                }
            }
            ValueTag::HashSet => {
                let set_ptr = (*coll).as_ptr() as *mut ClorusHashSet;
                Value::boolean((*set_ptr).contains(key))
            }
            _ => Value::boolean(false),
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
            ValueTag::LazySeq => {
                let seq = crate::lazy_seq::clorus_lazy_seq_force(coll);
                if crate::value::clorus_is_exception(seq) { return seq; }
                let result = clorus_nth(seq, index);
                crate::value::clorus_release(seq);
                result
            }
            ValueTag::SeqNode => {
                if index == 0 {
                    crate::seq_node::seq_node_head(coll)
                } else {
                    let tail = crate::seq_node::seq_node_tail(coll);
                    let result = if tail.is_null() || (*tail).tag() == ValueTag::Nil {
                        Value::nil()
                    } else {
                        clorus_nth(tail, index - 1)
                    };
                    if !tail.is_null() {
                        crate::value::clorus_release(tail);
                    }
                    result
                }
            }
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                PersistentVector::nth(vec_ptr, index as u64)
            }
            ValueTag::List => {
                // For lists, walk to nth position (logic below)
                clorus_list_nth(coll, index)
            }
            ValueTag::String => (*coll)
                .as_string()
                .chars()
                .nth(index as usize)
                .map(Value::char)
                .unwrap_or_else(Value::nil),
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll).unwrap();
                let result = clorus_nth(seq_vec, index);
                crate::value::clorus_release(seq_vec);
                result
            }
            _ => Value::nil(),
        }
    }
}

/// Return a real sequence for a non-empty collection.
///
/// Finite collections materialize to `ValueTag::List`; native `SeqNode`
/// values keep their possibly-lazy tails intact. Strings materialize to a
/// finite list of `Char` values, just like Clojure's string `seq` contract.
#[no_mangle]
pub extern "C" fn clorus_seq(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*coll).header().tag() == ValueTag::LazySeq {
            return crate::lazy_seq::clorus_lazy_seq_force(coll);
        }
        if (*coll).header().tag() == ValueTag::SeqNode {
            (*coll).header().retain();
            return coll;
        }
        if clorus_count(coll) == 0 {
            return Value::nil();
        }
        match (*coll).header().tag() {
            ValueTag::List => {
                (*coll).header().retain();
                coll
            }
            ValueTag::Vector => crate::list::clorus_vector_to_list(coll),
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll)
                    .expect("map and set are sequenceable by construction");
                let result = crate::list::clorus_vector_to_list(seq_vec);
                crate::value::clorus_release(seq_vec);
                result
            }
            ValueTag::String => string_as_char_list(coll),
            ValueTag::LazySeq => unreachable!("lazy sequence handled before collection dispatch"),
            ValueTag::SeqNode => unreachable!("sequence node handled before collection dispatch"),
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
            ValueTag::LazySeq => {
                let seq = crate::lazy_seq::clorus_lazy_seq_force(coll);
                if crate::value::clorus_is_exception(seq) { return seq; }
                let result = clorus_first(seq);
                crate::value::clorus_release(seq);
                result
            }
            ValueTag::SeqNode => crate::seq_node::seq_node_head(coll),
            ValueTag::Vector => {
                let vec_ptr = (*coll).as_ptr() as *mut PersistentVector;
                if (*vec_ptr).is_empty() {
                    Value::nil()
                } else {
                    // nth retains the element before returning
                    PersistentVector::nth(vec_ptr, 0)
                }
            }
            ValueTag::List => {
                // Use list helper which retains before returning
                crate::list::clorus_list_first(coll)
            }
            ValueTag::String => (*coll).as_string().chars().next().map(Value::char).unwrap_or_else(Value::nil),
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll).unwrap();
                let result = clorus_first(seq_vec);
                crate::value::clorus_release(seq_vec);
                result
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
            ValueTag::LazySeq => {
                let seq = crate::lazy_seq::clorus_lazy_seq_force(coll);
                if crate::value::clorus_is_exception(seq) { return seq; }
                let result = clorus_rest(seq);
                crate::value::clorus_release(seq);
                result
            }
            ValueTag::SeqNode => {
                let tail = crate::seq_node::seq_node_tail(coll);
                if tail.is_null() || (*tail).tag() == ValueTag::Nil {
                    if !tail.is_null() {
                        crate::value::clorus_release(tail);
                    }
                    crate::list::clorus_list_empty()
                } else {
                    tail
                }
            }
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
            ValueTag::String => {
                let seq = string_as_char_list(coll);
                let result = crate::list::clorus_list_rest(seq);
                crate::value::clorus_release(seq);
                result
            }
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll).unwrap();
                let result = clorus_rest(seq_vec);
                crate::value::clorus_release(seq_vec);
                result
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
            ValueTag::LazySeq => {
                let seq = crate::lazy_seq::clorus_lazy_seq_force(coll);
                if crate::value::clorus_is_exception(seq) { return seq; }
                let result = clorus_last(seq);
                crate::value::clorus_release(seq);
                result
            }
            ValueTag::SeqNode => {
                clorus_last_sequence(coll)
            }
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
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll).unwrap();
                let result = clorus_last(seq_vec);
                crate::value::clorus_release(seq_vec);
                result
            }
            ValueTag::String => (*coll).as_string().chars().last().map(Value::char).unwrap_or_else(Value::nil),
            _ => Value::nil(),
        }
    }
}

/// Find the last value of a general sequence without growing the Rust stack.
/// As in Clojure, an unbounded sequence never yields a result; it remains in
/// this loop instead of eventually turning that semantic non-termination into
/// a stack overflow.
unsafe fn clorus_last_sequence(coll: *mut Value) -> *mut Value {
    let mut current = clorus_seq(coll);
    if crate::value::clorus_is_exception(current) {
        return current;
    }
    let mut last = Value::nil();

    loop {
        if current.is_null() || (*current).tag() == ValueTag::Nil {
            if !current.is_null() {
                crate::value::clorus_release(current);
            }
            return last;
        }

        match (*current).tag() {
            ValueTag::SeqNode => {
                let head = crate::seq_node::seq_node_head(current);
                crate::value::clorus_release(last);
                last = head;
                let next = crate::seq_node::seq_node_tail(current);
                crate::value::clorus_release(current);
                current = next;
            }
            ValueTag::LazySeq => {
                let next = crate::lazy_seq::clorus_lazy_seq_force(current);
                crate::value::clorus_release(current);
                if crate::value::clorus_is_exception(next) {
                    crate::value::clorus_release(last);
                    return next;
                }
                current = next;
            }
            _ => {
                let suffix_last = clorus_last(current);
                crate::value::clorus_release(current);
                if suffix_last.is_null() || (*suffix_last).tag() == ValueTag::Nil {
                    if !suffix_last.is_null() {
                        crate::value::clorus_release(suffix_last);
                    }
                    return last;
                }
                crate::value::clorus_release(last);
                return suffix_last;
            }
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
                (*coll).as_string().chars().count() as i64
            }
            _ => 0,
        }
    }
}

/// Count through the language-value ABI, preserving lazy-sequence exceptions.
///
/// `clorus_count` intentionally remains an i64 kernel primitive for runtime
/// loops. The public compiler lowering uses this Value-returning companion so
/// a failing lazy realization cannot be misreported as numeric zero.
#[no_mangle]
pub extern "C" fn clorus_count_value(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return Value::long(0);
    }

    unsafe {
        if matches!((*coll).header().tag(), ValueTag::LazySeq | ValueTag::SeqNode) {
            return clorus_count_sequence_value(coll);
        }
    }

    Value::long(clorus_count(coll))
}

/// Count a general sequence iteratively. This deliberately has the same
/// termination contract as Clojure's `count`: asking for the count of an
/// unbounded sequence does not terminate, but it never recurses through Rust
/// frames or converts a realization error into an ordinary number.
unsafe fn clorus_count_sequence_value(coll: *mut Value) -> *mut Value {
    let mut current = clorus_seq(coll);
    if crate::value::clorus_is_exception(current) {
        return current;
    }

    let mut count = 0_i64;
    loop {
        if current.is_null() || (*current).tag() == ValueTag::Nil {
            if !current.is_null() {
                crate::value::clorus_release(current);
            }
            return Value::long(count);
        }

        match (*current).tag() {
            ValueTag::List => {
                let suffix_count = clorus_count(current);
                crate::value::clorus_release(current);
                return Value::long(count.saturating_add(suffix_count));
            }
            ValueTag::SeqNode => {
                count = match count.checked_add(1) {
                    Some(next) => next,
                    None => {
                        crate::value::clorus_release(current);
                        let message = Value::string("count overflowed i64");
                        let exception = Value::exception(message);
                        crate::value::clorus_release(message);
                        return exception;
                    }
                };
                let next = crate::seq_node::seq_node_tail(current);
                crate::value::clorus_release(current);
                current = next;
            }
            ValueTag::LazySeq => {
                let next = crate::lazy_seq::clorus_lazy_seq_force(current);
                crate::value::clorus_release(current);
                if crate::value::clorus_is_exception(next) {
                    return next;
                }
                current = next;
            }
            _ => {
                crate::value::clorus_release(current);
                return Value::long(count);
            }
        }
    }
}

/// Return an empty collection of the same kind as coll.
///
/// (empty [1 2 3]) => []
/// (empty {:a 1}) => {}
/// (empty #{1 2}) => #{}
/// (empty '(1 2)) => ()
/// (empty "text") => nil
/// (empty nil) => nil
///
/// Clojure's `empty` only constructs empty persistent collections. Strings
/// remain countable and seqable for functions such as `not-empty`, but have
/// no persistent empty representation, so they return nil here. Values
/// without an empty representation use the same nil fallback.
#[no_mangle]
pub extern "C" fn clorus_empty(coll: *mut Value) -> *mut Value {
    if coll.is_null() {
        return Value::nil();
    }

    unsafe {
        match (*coll).header().tag() {
            ValueTag::Vector => crate::vector::clorus_vector_empty(),
            ValueTag::List => crate::list::clorus_list_empty(),
            ValueTag::HashMap => crate::map::clorus_map_empty(),
            ValueTag::HashSet => crate::set::clorus_set_empty(),
            ValueTag::String => Value::nil(),
            ValueTag::Nil => Value::nil(),
            _ => Value::nil(),
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
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll).unwrap();
                let result = clorus_take(seq_vec, n);
                crate::value::clorus_release(seq_vec);
                result
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
        unsafe {
            (*coll).header().retain();
        }
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
            ValueTag::HashMap | ValueTag::HashSet => {
                let seq_vec = coll_as_seqable_vector(coll).unwrap();
                let result = clorus_drop(seq_vec, n);
                crate::value::clorus_release(seq_vec);
                result
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
            ValueTag::HashMap => {
                if elem.is_null() {
                    crate::value::clorus_retain(coll);
                    return coll;
                }
                match (*elem).header().tag() {
                    // (conj m [k v]) -- a 2-element vector is a key/value
                    // entry to add, matching Clojure's conj-onto-map.
                    ValueTag::Vector if crate::vector::clorus_vector_count(elem) == 2 => {
                        let k = crate::vector::clorus_vector_nth(elem, 0);
                        let v = crate::vector::clorus_vector_nth(elem, 1);
                        let result = crate::map::clorus_map_assoc(coll, k, v);
                        crate::value::clorus_release(k);
                        crate::value::clorus_release(v);
                        result
                    }
                    // (conj m1 m2) -- merge m2's entries into m1, matching
                    // Clojure's conj-a-map-onto-a-map.
                    ValueTag::HashMap => {
                        let src_ptr = (*elem).as_ptr() as *mut ClorusHashMap;
                        let mut result = coll;
                        crate::value::clorus_retain(result);
                        for (k, v) in (*src_ptr).entries_iter() {
                            let next = crate::map::clorus_map_assoc(result, k, v);
                            crate::value::clorus_release(result);
                            result = next;
                        }
                        result
                    }
                    _ => Value::nil(),
                }
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
    fn test_seq_materializes_non_empty_vector_as_list() {
        unsafe {
            let vec = crate::vector::clorus_vector_empty();
            let vec1 = crate::vector::clorus_vector_conj(vec, Value::double(1.0));
            let vec2 = crate::vector::clorus_vector_conj(vec1, Value::double(2.0));

            let seq = clorus_seq(vec2);
            assert_eq!((*seq).header().tag(), ValueTag::List);
            assert_eq!(clorus_count(seq), 2);

            let first = clorus_first(seq);
            assert_eq!((*first).as_double(), 1.0);
            crate::value::clorus_release(first);
            crate::value::clorus_release(seq);

            let empty_seq = clorus_seq(vec);
            assert_eq!((*empty_seq).header().tag(), ValueTag::Nil);
            crate::value::clorus_release(empty_seq);
            crate::value::clorus_release(vec);
            crate::value::clorus_release(vec1);
            crate::value::clorus_release(vec2);
        }
    }

    #[test]
    fn test_string_sequence_uses_unicode_scalar_characters() {
        unsafe {
            let string = Value::string("aλ");
            assert_eq!(clorus_count(string), 2);

            let first = clorus_first(string);
            assert_eq!((*first).tag(), ValueTag::Char);
            assert_eq!((*first).as_char(), 'a');
            crate::value::clorus_release(first);

            let second = clorus_nth(string, 1);
            assert_eq!((*second).as_char(), 'λ');
            crate::value::clorus_release(second);

            let seq = clorus_seq(string);
            assert_eq!((*seq).tag(), ValueTag::List);
            let seq_second = clorus_nth(seq, 1);
            assert_eq!((*seq_second).as_char(), 'λ');
            crate::value::clorus_release(seq_second);
            crate::value::clorus_release(seq);
            crate::value::clorus_release(string);
        }
    }

    #[test]
    fn test_empty_preserves_persistent_collection_kind() {
        unsafe {
            let string = Value::string("clorus");
            let empty_string = clorus_empty(string);
            assert_eq!((*empty_string).header().tag(), ValueTag::Nil);

            let vector = crate::vector::clorus_vector_empty();
            let empty_vector = clorus_empty(vector);
            assert_eq!((*empty_vector).header().tag(), ValueTag::Vector);
            assert_eq!(clorus_count(empty_vector), 0);

            crate::value::clorus_release(string);
            crate::value::clorus_release(empty_string);
            crate::value::clorus_release(vector);
            crate::value::clorus_release(empty_vector);
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
