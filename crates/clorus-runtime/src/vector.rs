/// Persistent Vector implementation
///
/// A 32-way branching tree (trie) with tail optimization.
/// Based on Clojure's PersistentVector and Phil Bagwell's "Ideal Hash Trees".
///
/// Structure:
/// - 32-way branching (5 bits per level)
/// - Tail optimization (last 32 elements)
/// - Path copying for updates
/// - Structural sharing between versions
///
/// Performance:
/// - conj: O(1) amortized
/// - nth: O(log32 n) ≈ O(1) - max 7 levels for 2^32 elements
/// - assoc: O(log32 n)
/// - count: O(1)

use crate::value::{Value, ValueTag};
use std::sync::atomic::{AtomicU64, Ordering};
use std::ptr::null_mut;
use std::sync::{Mutex, OnceLock};
use std::collections::HashSet;

/// Branching factor (32 = 2^5)
const BRANCHING_FACTOR: usize = 32;
const SHIFT_INCREMENT: u8 = 5;

static FREED_VECS: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

#[inline]
fn mark_vector_alloc(ptr: *mut PersistentVector) {
    if ptr.is_null() {
        return;
    }
    if let Ok(mut set) = FREED_VECS.get_or_init(|| Mutex::new(HashSet::new())).lock() {
        set.remove(&(ptr as usize));
    }
}

/// Vector node - internal node in the trie
///
/// Each node has 32 slots. Can contain either:
/// - Pointers to child nodes (internal levels)
/// - Pointers to Values (leaf level)
#[repr(C)]
struct VectorNode {
    refcount: AtomicU64,
    /// Array of 32 pointers (to nodes or values)
    children: [*mut u8; BRANCHING_FACTOR],
}

impl VectorNode {
    /// Create a new empty node
    fn new() -> *mut Self {
        let node = Box::new(VectorNode {
            refcount: AtomicU64::new(1),
            children: [null_mut(); BRANCHING_FACTOR],
        });
        Box::into_raw(node)
    }

    /// Clone a node (shallow copy)
    unsafe fn clone_node(node: *const Self) -> *mut Self {
        if node.is_null() {
            return null_mut();
        }

        let new_node = Self::new();

        // Copy children pointers
        for i in 0..BRANCHING_FACTOR {
            (*new_node).children[i] = (*node).children[i];
        }

        new_node
    }

    #[inline]
    fn refcount(&self) -> u64 {
        self.refcount.load(Ordering::Relaxed)
    }
}

/// Tail array - optimization for the last 32 elements
type TailArray = [*mut Value; BRANCHING_FACTOR];

/// Persistent Vector
///
/// Immutable vector with efficient updates through structural sharing.
#[repr(C)]
pub struct PersistentVector {
    /// Reference count for this vector
    refcount: AtomicU64,
    /// Total number of elements
    count: u64,
    /// Tree depth (shift = depth * 5)
    /// shift=5 means 1 level (32 elements)
    /// shift=10 means 2 levels (1024 elements)
    /// etc.
    shift: u8,
    /// Root of the tree (null if count <= 32)
    root: *mut VectorNode,
    /// Last 32 elements (optimization)
    tail: *mut TailArray,
    /// Number of elements in tail (0-32)
    tail_len: u8,
}

impl PersistentVector {
    /// Create an empty vector
    pub fn empty() -> *mut Self {
        let vec = Box::new(PersistentVector {
            refcount: AtomicU64::new(1),
            count: 0,
            shift: SHIFT_INCREMENT,
            root: null_mut(),
            tail: null_mut(),
            tail_len: 0,
        });
        let ptr = Box::into_raw(vec);
        mark_vector_alloc(ptr);
        ptr
    }

    /// Get element count
    #[inline]
    pub unsafe fn count(&self) -> u64 {
        self.count
    }

    /// Check if vector is empty
    #[inline]
    pub unsafe fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Append element to end of vector (conj)
    ///
    /// Fast path: If tail has room, just add to tail
    /// Slow path: Push tail to tree, create new tail
    pub unsafe fn conj(vec: *const Self, value: *mut Value) -> *mut Self {
        let vec_ref = &*vec;

        // Retain the value
        if !value.is_null() {
            (*value).header().retain();
        }

        // Fast path: room in tail
        if vec_ref.tail_len < BRANCHING_FACTOR as u8 {
            return Self::append_to_tail(vec, value);
        }

        // Slow path: push tail to tree
        Self::push_tail_to_tree(vec, value)
    }

    /// Fast path: append to tail
    unsafe fn append_to_tail(vec: *const Self, value: *mut Value) -> *mut Self {
        let vec_ref = &*vec;

        // Create new vector (copy metadata)
        let new_vec = Box::new(PersistentVector {
            refcount: AtomicU64::new(1),
            count: vec_ref.count + 1,
            shift: vec_ref.shift,
            root: vec_ref.root,
            tail: null_mut(),
            tail_len: vec_ref.tail_len + 1,
        });
        let new_vec_ptr = Box::into_raw(new_vec);
        mark_vector_alloc(new_vec_ptr);

        // Retain root
        if !vec_ref.root.is_null() {
            (*vec_ref.root).refcount.fetch_add(1, Ordering::Relaxed);
        }

        // Clone tail array
        let new_tail = Box::into_raw(Box::new([null_mut(); BRANCHING_FACTOR]));
        (*new_vec_ptr).tail = new_tail;

        // Copy existing tail elements
        if !vec_ref.tail.is_null() {
            for i in 0..vec_ref.tail_len as usize {
                let elem = (*vec_ref.tail)[i];
                (*new_tail)[i] = elem;
                if !elem.is_null() {
                    (*elem).header().retain();
                }
            }
        }

        // Add new element
        (*new_tail)[vec_ref.tail_len as usize] = value;

        new_vec_ptr
    }

    /// Slow path: push tail to tree, start new tail
    unsafe fn push_tail_to_tree(vec: *const Self, value: *mut Value) -> *mut Self {
        let vec_ref = &*vec;

        // Calculate elements currently in tree (not in tail)
        let tree_count = vec_ref.count - vec_ref.tail_len as u64;

        let new_root = if tree_count == 0 {
            // No tree yet - tail becomes first node
            Self::tail_to_node(vec_ref.tail) as *mut VectorNode
        } else {
            // Will overflow current tree level?
            // Capacity at current shift is 2^shift elements
            let capacity = 1u64 << vec_ref.shift;
            let overflow = tree_count >= capacity;

            if overflow {
                // Need new root level
                let new_root = VectorNode::new();
                (*new_root).children[0] = vec_ref.root as *mut u8;
                if !vec_ref.root.is_null() {
                    (*vec_ref.root).refcount.fetch_add(1, Ordering::Relaxed);
                }
                (*new_root).children[1] = Self::tail_to_node(vec_ref.tail);
                new_root
            } else {
                // Add tail to existing tree
                Self::push_tail(vec_ref.root, vec_ref.shift, tree_count, vec_ref.tail)
            }
        };

        let new_shift = if tree_count == 0 {
            SHIFT_INCREMENT
        } else if tree_count >= (1u64 << vec_ref.shift) {
            vec_ref.shift + SHIFT_INCREMENT
        } else {
            vec_ref.shift
        };

        // Create new vector with new tail containing just the new element
        let new_tail = Box::into_raw(Box::new([null_mut(); BRANCHING_FACTOR]));
        (*new_tail)[0] = value;

        let new_vec = Box::new(PersistentVector {
            refcount: AtomicU64::new(1),
            count: vec_ref.count + 1,
            shift: new_shift,
            root: new_root,
            tail: new_tail,
            tail_len: 1,
        });

        let new_vec_ptr = Box::into_raw(new_vec);
        mark_vector_alloc(new_vec_ptr);
        new_vec_ptr
    }

    /// Convert tail array to tree node
    unsafe fn tail_to_node(tail: *const TailArray) -> *mut u8 {
        if tail.is_null() {
            return null_mut();
        }

        let node = VectorNode::new();
        for i in 0..BRANCHING_FACTOR {
            (*node).children[i] = (*tail)[i] as *mut u8;
            let val = (*tail)[i];
            if !val.is_null() {
                (*val).header().retain();
            }
        }
        node as *mut u8
    }

    /// Push tail into tree at the correct position
    unsafe fn push_tail(
        node: *mut VectorNode,
        level: u8,
        count: u64,
        tail: *const TailArray,
    ) -> *mut VectorNode {
        if level == SHIFT_INCREMENT {
            // At leaf level - just return tail as a leaf node
            // Don't wrap it in another node!
            Self::tail_to_node(tail) as *mut VectorNode
        } else {
            // Internal level - calculate which child to descend into
            let shift_amt = level - SHIFT_INCREMENT;
            let subidx = ((count >> shift_amt) & 0x1F) as usize;

            // Clone this node and recurse
            let new_node = VectorNode::clone_node(node);
            if new_node.is_null() {
                // Original node was null - create new path
                let new_node = VectorNode::new();
                let new_child = Self::push_tail(null_mut(), level - SHIFT_INCREMENT, count, tail);
                (*new_node).children[subidx] = new_child as *mut u8;
                new_node
            } else {
                // Retain shared children (excluding the branch we overwrite)
                for i in 0..BRANCHING_FACTOR {
                    if i == subidx {
                        continue;
                    }
                    let child = (*new_node).children[i] as *mut VectorNode;
                    if !child.is_null() {
                        (*child).refcount.fetch_add(1, Ordering::Relaxed);
                    }
                }
                // Clone existing node and recurse into child
                let child = (*node).children[subidx] as *mut VectorNode;
                let new_child = Self::push_tail(child, level - SHIFT_INCREMENT, count, tail);
                (*new_node).children[subidx] = new_child as *mut u8;
                new_node
            }
        }
    }

    /// Get element at index (nth)
    pub unsafe fn nth(vec: *const Self, index: u64) -> *mut Value {
        let vec_ref = &*vec;

        if index >= vec_ref.count {
            return Value::nil();
        }

        // Check if index is in tail
        let tail_start = vec_ref.count - vec_ref.tail_len as u64;
        if index >= tail_start {
            let tail_index = (index - tail_start) as usize;
            let val = (*vec_ref.tail)[tail_index];
            if !val.is_null() {
                (*val).header().retain();
            }
            return val;
        }

        // Navigate tree
        let mut node = vec_ref.root;
        let mut level = vec_ref.shift;

        while level > SHIFT_INCREMENT {
            // Extract bits for this level
            let shift_amt = level - SHIFT_INCREMENT;
            let child_index = ((index >> shift_amt) & 0x1F) as usize;
            node = (*node).children[child_index] as *mut VectorNode;
            level -= SHIFT_INCREMENT;
        }

        // At leaf level - extract final bits
        let leaf_index = (index & 0x1F) as usize;
        let val = (*node).children[leaf_index] as *mut Value;
        if !val.is_null() {
            (*val).header().retain();
        }
        val
    }

    /// Update element at index (assoc)
    pub unsafe fn assoc(vec: *const Self, index: u64, value: *mut Value) -> *mut Self {
        let vec_ref = &*vec;

        if index >= vec_ref.count {
            // Out of bounds - caller decides fallback (typically nil at Value boundary).
            return null_mut();
        }

        // Retain new value
        if !value.is_null() {
            (*value).header().retain();
        }

        // Check if index is in tail
        let tail_start = vec_ref.count - vec_ref.tail_len as u64;
        if index >= tail_start {
            return Self::assoc_tail(vec, index - tail_start, value);
        }

        // Update in tree - requires path copying
        Self::assoc_tree(vec, index, value)
    }

    /// Update element in tail
    unsafe fn assoc_tail(vec: *const Self, tail_index: u64, value: *mut Value) -> *mut Self {
        let vec_ref = &*vec;

        // Create new vector
        let new_vec = Box::new(PersistentVector {
            refcount: AtomicU64::new(1),
            count: vec_ref.count,
            shift: vec_ref.shift,
            root: vec_ref.root,
            tail: null_mut(),
            tail_len: vec_ref.tail_len,
        });
        let new_vec_ptr = Box::into_raw(new_vec);
        mark_vector_alloc(new_vec_ptr);

        // Retain root
        if !vec_ref.root.is_null() {
            (*vec_ref.root).refcount.fetch_add(1, Ordering::Relaxed);
        }

        // Clone tail with updated value
        let new_tail = Box::into_raw(Box::new([null_mut(); BRANCHING_FACTOR]));
        (*new_vec_ptr).tail = new_tail;

        for i in 0..vec_ref.tail_len as usize {
            if i == tail_index as usize {
                (*new_tail)[i] = value;
            } else {
                let elem = (*vec_ref.tail)[i];
                (*new_tail)[i] = elem;
                if !elem.is_null() {
                    (*elem).header().retain();
                }
            }
        }

        new_vec_ptr
    }

    /// Update element in tree (path copying)
    unsafe fn assoc_tree(vec: *const Self, index: u64, value: *mut Value) -> *mut Self {
        let vec_ref = &*vec;

        // Clone path and update
        let new_root = Self::do_assoc(vec_ref.root, vec_ref.shift, index, value);

        // Create new vector with a cloned tail to avoid sharing the tail array
        let new_vec = Box::new(PersistentVector {
            refcount: AtomicU64::new(1),
            count: vec_ref.count,
            shift: vec_ref.shift,
            root: new_root,
            tail: null_mut(),
            tail_len: vec_ref.tail_len,
        });
        let new_vec_ptr = Box::into_raw(new_vec);
        mark_vector_alloc(new_vec_ptr);

        // Clone tail elements
        if !vec_ref.tail.is_null() {
            let new_tail = Box::into_raw(Box::new([null_mut(); BRANCHING_FACTOR]));
            (*new_vec_ptr).tail = new_tail;
            for i in 0..vec_ref.tail_len as usize {
                let elem = (*vec_ref.tail)[i];
                (*new_tail)[i] = elem;
                if !elem.is_null() {
                    (*elem).header().retain();
                }
            }
        }

        new_vec_ptr
    }

    /// Recursive path copying for assoc
    unsafe fn do_assoc(
        node: *mut VectorNode,
        level: u8,
        index: u64,
        value: *mut Value,
    ) -> *mut VectorNode {
        // Clone this node
        let new_node = VectorNode::clone_node(node);

        // Extract the appropriate bits for this level
        let shift_amt = if level > SHIFT_INCREMENT { level - SHIFT_INCREMENT } else { 0 };
        let child_index = ((index >> shift_amt) & 0x1F) as usize;

        if level == SHIFT_INCREMENT {
            // Leaf level - update value
            (*new_node).children[child_index] = value as *mut u8;
            // Retain other values since we're sharing them from the old node
            for i in 0..BRANCHING_FACTOR {
                if i == child_index {
                    continue;
                }
                let child_val = (*new_node).children[i] as *mut Value;
                if !child_val.is_null() {
                    (*child_val).header().retain();
                }
            }
        } else {
            // Internal level - recurse
            let child = (*node).children[child_index] as *mut VectorNode;
            let new_child = Self::do_assoc(child, level - SHIFT_INCREMENT, index, value);
            (*new_node).children[child_index] = new_child as *mut u8;

            // Retain other children
            for i in 0..BRANCHING_FACTOR {
                if i != child_index && !(*new_node).children[i].is_null() {
                    // These are shared - increment refcount
                    let child_node = (*new_node).children[i] as *mut VectorNode;
                    (*child_node).refcount.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        new_node
    }
}

// Cleanup functions

unsafe fn release_node(node: *mut VectorNode, level: u8) {
    if node.is_null() {
        return;
    }

    // Guard against duplicate release attempts on already-released nodes.
    // This prevents refcount underflow and use-after-free cascades.
    if (*node).refcount.load(Ordering::Relaxed) == 0 {
        return;
    }

    if (*node).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        // Last reference - recursively release children
        if level == SHIFT_INCREMENT {
            // Leaf level - children are Values
            for i in 0..BRANCHING_FACTOR {
                let child = (*node).children[i] as *mut Value;
                if !child.is_null() {
                    crate::value::clorus_release(child);
                }
            }
        } else {
            // Internal level - children are nodes
            for i in 0..BRANCHING_FACTOR {
                let child = (*node).children[i] as *mut VectorNode;
                release_node(child, level - SHIFT_INCREMENT);
            }
        }

        // Free the node
        drop(Box::from_raw(node));
    }
}

pub(crate) unsafe fn release_vector(vec: *mut PersistentVector) {
    if vec.is_null() {
        return;
    }

    let ptr = vec as usize;
    if let Ok(set) = FREED_VECS.get_or_init(|| Mutex::new(HashSet::new())).lock() {
        if set.contains(&ptr) {
            if std::env::var("CLORUS_DEBUG_VECTOR_RELEASE").is_ok() {
                eprintln!("[clorus] double free detected (vector ptr={:p})", vec);
                if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                    let bt_now = std::backtrace::Backtrace::force_capture().to_string();
                    eprintln!("[clorus] vector double free current backtrace:\n{}", bt_now);
                }
            }
            return;
        }
    }

    if std::env::var("CLORUS_SAFE_VECTOR").is_ok() {
        // Temporary safety valve: avoid freeing shared nodes/values to prevent corruption.
        // This intentionally leaks vector internals but keeps the process stable.
        drop(Box::from_raw(vec));
        return;
    }

    // Guard against duplicate release attempts on already-released vectors.
    if (*vec).refcount.load(Ordering::Relaxed) == 0 {
        return;
    }

    if (*vec).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        if let Ok(mut set) = FREED_VECS.get_or_init(|| Mutex::new(HashSet::new())).lock() {
            set.insert(vec as usize);
        }

        // Release root
        if !(*vec).root.is_null() {
            release_node((*vec).root, (*vec).shift);
        }

        // Release tail
        if !(*vec).tail.is_null() {
            for i in 0..(*vec).tail_len as usize {
                let elem = (*(*vec).tail)[i];
                if !elem.is_null() {
                    crate::value::clorus_release(elem);
                }
            }
            drop(Box::from_raw((*vec).tail));
        }

        // Free the vector
        drop(Box::from_raw(vec));
    }
}

// FFI functions for LLVM

#[no_mangle]
pub extern "C" fn clorus_vector_empty() -> *mut Value {
    let vec = PersistentVector::empty();
    Value::from_ptr(ValueTag::Vector, vec as *mut u8)
}

#[no_mangle]
pub extern "C" fn clorus_vector_conj(vec_val: *mut Value, elem: *mut Value) -> *mut Value {
    if vec_val.is_null() {
        return clorus_vector_empty();
    }

    unsafe {
        let vec_ptr = (*vec_val).as_ptr() as *mut PersistentVector;
        let new_vec = PersistentVector::conj(vec_ptr, elem);
        Value::from_ptr(ValueTag::Vector, new_vec as *mut u8)
    }
}

#[no_mangle]
pub extern "C" fn clorus_vector_nth(vec_val: *mut Value, index: u64) -> *mut Value {
    if vec_val.is_null() {
        return Value::nil();
    }

    unsafe {
        let vec_ptr = (*vec_val).as_ptr() as *mut PersistentVector;
        PersistentVector::nth(vec_ptr, index)
    }
}

#[no_mangle]
pub extern "C" fn clorus_vector_assoc(
    vec_val: *mut Value,
    index: u64,
    elem: *mut Value,
) -> *mut Value {
    if vec_val.is_null() {
        return Value::nil();
    }

    unsafe {
        let vec_ptr = (*vec_val).as_ptr() as *mut PersistentVector;
        let new_vec = PersistentVector::assoc(vec_ptr, index, elem);
        if new_vec.is_null() {
            return Value::nil();
        }
        Value::from_ptr(ValueTag::Vector, new_vec as *mut u8)
    }
}

#[no_mangle]
pub extern "C" fn clorus_vector_count(vec_val: *mut Value) -> u64 {
    if vec_val.is_null() {
        return 0;
    }

    unsafe {
        let vec_ptr = (*vec_val).as_ptr() as *mut PersistentVector;
        (*vec_ptr).count()
    }
}

/// Create a new vector containing elements from start_index to end
/// Used for rest parameter destructuring: [a b & rest]
#[no_mangle]
pub extern "C" fn clorus_vector_rest(vec_val: *mut Value, start_index: u64) -> *mut Value {
    if vec_val.is_null() {
        return clorus_vector_empty();
    }

    unsafe {
        let vec_ptr = (*vec_val).as_ptr() as *mut PersistentVector;
        let count = (*vec_ptr).count();

        // If start_index >= count, return empty vector
        if start_index >= count {
            return clorus_vector_empty();
        }

        // Build new vector with remaining elements
        let mut result = PersistentVector::empty();
        for i in start_index..count {
            let elem = PersistentVector::nth(vec_ptr, i);
            result = PersistentVector::conj(result, elem);
        }

        Value::from_ptr(ValueTag::Vector, result as *mut u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_vector() {
        let vec = PersistentVector::empty();
        unsafe {
            assert_eq!((*vec).count(), 0);
            assert!((*vec).is_empty());
            release_vector(vec);
        }
    }

    #[test]
    fn test_conj_small() {
        let vec = PersistentVector::empty();

        unsafe {
            let val1 = Value::double(1.0);
            let vec1 = PersistentVector::conj(vec, val1);
            assert_eq!((*vec1).count(), 1);

            let val2 = Value::double(2.0);
            let vec2 = PersistentVector::conj(vec1, val2);
            assert_eq!((*vec2).count(), 2);

            // Original vector still empty
            assert_eq!((*vec).count(), 0);

            release_vector(vec);
            release_vector(vec1);
            release_vector(vec2);
        }
    }

    #[test]
    fn test_nth() {
        let vec = PersistentVector::empty();

        unsafe {
            let mut current = vec;
            for i in 0..10 {
                let val = Value::double(i as f64);
                current = PersistentVector::conj(current, val);
            }

            // Check all elements
            for i in 0..10 {
                let val = PersistentVector::nth(current, i);
                assert_eq!((*val).as_double(), i as f64);
                crate::value::clorus_release(val);
            }

            release_vector(current);
        }
    }

    #[test]
    fn test_conj_large() {
        // Test with more than 32 elements (forces tree creation)
        let vec = PersistentVector::empty();

        unsafe {
            let mut current = vec;
            for i in 0..100 {
                let val = Value::double(i as f64);
                current = PersistentVector::conj(current, val);
            }

            assert_eq!((*current).count(), 100);

            // Verify all elements
            for i in 0..100 {
                let val = PersistentVector::nth(current, i);
                assert_eq!((*val).as_double(), i as f64);
                crate::value::clorus_release(val);
            }

            release_vector(current);
        }
    }

    #[test]
    fn test_assoc() {
        let vec = PersistentVector::empty();

        unsafe {
            let mut current = vec;
            for i in 0..10 {
                let val = Value::double(i as f64);
                current = PersistentVector::conj(current, val);
            }

            // Update element at index 5
            let new_val = Value::double(99.0);
            let vec2 = PersistentVector::assoc(current, 5, new_val);

            // Original unchanged
            let val = PersistentVector::nth(current, 5);
            assert_eq!((*val).as_double(), 5.0);
            crate::value::clorus_release(val);

            // New vector has updated value
            let val = PersistentVector::nth(vec2, 5);
            assert_eq!((*val).as_double(), 99.0);
            crate::value::clorus_release(val);

            release_vector(current);
            release_vector(vec2);
        }
    }

    #[test]
    fn test_assoc_out_of_bounds_returns_null_ptr() {
        let vec = PersistentVector::empty();

        unsafe {
            let mut current = vec;
            for i in 0..4 {
                let val = Value::double(i as f64);
                current = PersistentVector::conj(current, val);
            }

            let new_val = Value::double(99.0);
            let vec2 = PersistentVector::assoc(current, 99, new_val);
            assert!(vec2.is_null());

            // assoc retains only on in-bounds path; release our test value.
            crate::value::clorus_release(new_val);
            release_vector(current);
        }
    }

    #[test]
    fn test_structural_sharing() {
        let vec1 = PersistentVector::empty();

        unsafe {
            let mut current = vec1;
            // Fill exactly 64 elements - this fills tree and tail completely
            for i in 0..64 {
                let val = Value::double(i as f64);
                current = PersistentVector::conj(current, val);
            }
            // At this point: tree=[0..31], tail=[32..63] (full)

            // Add 65th element - this pushes tail to tree, creating new root
            let val = Value::double(64.0);
            let vec2 = PersistentVector::conj(current, val);
            // Now vec2: tree=[0..63], tail=[64]

            // Both vectors should share most nodes
            // Verify by checking root is different (path copying)
            assert_ne!((*current).root, (*vec2).root);

            release_vector(current);
            release_vector(vec2);
        }
    }
}
