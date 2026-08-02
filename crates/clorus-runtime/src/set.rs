/// Persistent HashSet implementation for Clorus
///
/// Same hash array mapped trie (HAMT) shape as `ClorusHashMap`, storing a
/// single value per slot instead of a key/value pair. `conj`/`disj` never
/// mutate an existing set -- every update returns a new set, with
/// structural sharing of unrelated subtrees.

use crate::value::Value;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU64, Ordering};

const BITS: u32 = 5;
const BRANCHING: usize = 32;
const MASK: u64 = 0x1F;
const MAX_LEVELS: u32 = 13;

#[inline]
fn level_index(hash: u64, level: u32) -> usize {
    let shift = level * BITS;
    if shift >= 64 {
        return 0;
    }
    ((hash >> shift) & MASK) as usize
}

#[derive(Clone, Copy)]
enum Child {
    Empty,
    Entry(*mut Value),
    Node(*mut SetNode),
    Collision(*mut CollisionNode),
}

#[inline]
unsafe fn retain_child(child: Child) {
    match child {
        Child::Empty => {}
        Child::Entry(v) => crate::value::clorus_retain(v),
        Child::Node(n) => {
            (*n).refcount.fetch_add(1, Ordering::Relaxed);
        }
        Child::Collision(c) => {
            (*c).refcount.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[inline]
unsafe fn release_child(child: Child) {
    match child {
        Child::Empty => {}
        Child::Entry(v) => crate::value::clorus_release(v),
        Child::Node(n) => release_node(n),
        Child::Collision(c) => release_collision(c),
    }
}

unsafe fn collect_child(child: Child, out: &mut Vec<*mut Value>) {
    match child {
        Child::Empty => {}
        Child::Entry(v) => out.push(v),
        Child::Node(n) => collect_node(n, out),
        Child::Collision(c) => out.extend((*c).entries.iter().copied()),
    }
}

unsafe fn contains_child(child: Child, hash: u64, val: *mut Value, level: u32) -> bool {
    match child {
        Child::Empty => false,
        Child::Entry(v) => crate::value::clorus_equals(v, val),
        Child::Node(n) => contains(n, level + 1, hash, val),
        Child::Collision(c) => (*c)
            .entries
            .iter()
            .any(|v| crate::value::clorus_equals(*v, val)),
    }
}

struct SetNode {
    refcount: AtomicU64,
    children: [Child; BRANCHING],
}

impl SetNode {
    fn new() -> *mut Self {
        Box::into_raw(Box::new(SetNode {
            refcount: AtomicU64::new(1),
            children: [Child::Empty; BRANCHING],
        }))
    }

    unsafe fn clone_node(node: *const Self) -> *mut Self {
        let new_node = Self::new();
        for i in 0..BRANCHING {
            (*new_node).children[i] = (*node).children[i];
        }
        new_node
    }
}

struct CollisionNode {
    refcount: AtomicU64,
    hash: u64,
    entries: Vec<*mut Value>,
}

impl CollisionNode {
    fn new(hash: u64, entries: Vec<*mut Value>) -> *mut Self {
        Box::into_raw(Box::new(CollisionNode {
            refcount: AtomicU64::new(1),
            hash,
            entries,
        }))
    }
}

unsafe fn release_node(node: *mut SetNode) {
    if node.is_null() {
        return;
    }
    if (*node).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        for i in 0..BRANCHING {
            release_child((*node).children[i]);
        }
        drop(Box::from_raw(node));
    }
}

unsafe fn release_collision(node: *mut CollisionNode) {
    if node.is_null() {
        return;
    }
    if (*node).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        for v in (*node).entries.iter() {
            crate::value::clorus_release(*v);
        }
        drop(Box::from_raw(node));
    }
}

unsafe fn collect_node(node: *mut SetNode, out: &mut Vec<*mut Value>) {
    if node.is_null() {
        return;
    }
    for i in 0..BRANCHING {
        collect_child((*node).children[i], out);
    }
}

unsafe fn contains(node: *mut SetNode, level: u32, hash: u64, val: *mut Value) -> bool {
    if node.is_null() {
        return false;
    }
    let idx = level_index(hash, level);
    contains_child((*node).children[idx], hash, val, level)
}

unsafe fn build_branch_for_two(
    v1: *mut Value,
    hash1: u64,
    v2: *mut Value,
    hash2: u64,
    level: u32,
) -> Child {
    if level >= MAX_LEVELS {
        crate::value::clorus_retain(v2);
        let bucket = CollisionNode::new(hash1, vec![v1, v2]);
        return Child::Collision(bucket);
    }

    let idx1 = level_index(hash1, level);
    let idx2 = level_index(hash2, level);
    let node = SetNode::new();

    if idx1 == idx2 {
        let child = build_branch_for_two(v1, hash1, v2, hash2, level + 1);
        (*node).children[idx1] = child;
    } else {
        crate::value::clorus_retain(v2);
        (*node).children[idx1] = Child::Entry(v1);
        (*node).children[idx2] = Child::Entry(v2);
    }

    Child::Node(node)
}

/// Persistently add `val` starting at `node` (may be null for an empty
/// subtree). Returns the new subtree root; `is_new` is set to true iff
/// this insert grew the set's count.
unsafe fn insert(
    node: *mut SetNode,
    level: u32,
    hash: u64,
    val: *mut Value,
    is_new: &mut bool,
) -> *mut SetNode {
    let idx = level_index(hash, level);
    let new_node = if node.is_null() {
        SetNode::new()
    } else {
        SetNode::clone_node(node)
    };
    let existing = if node.is_null() {
        Child::Empty
    } else {
        (*node).children[idx]
    };

    match existing {
        Child::Empty => {
            crate::value::clorus_retain(val);
            (*new_node).children[idx] = Child::Entry(val);
            *is_new = true;
        }
        Child::Entry(v) => {
            if crate::value::clorus_equals(v, val) {
                // v is being duplicated from node's slot into new_node's
                // slot (excluded from the sibling-retain loop below since
                // it's the slot being written here) -- node still owns its
                // own reference, so this needs its own retain.
                crate::value::clorus_retain(v);
                (*new_node).children[idx] = Child::Entry(v);
                *is_new = false;
            } else {
                // Same reasoning: v is duplicated into a second, independent
                // branch reachable from new_node (build_branch_for_two
                // already retains val).
                crate::value::clorus_retain(v);
                let existing_hash = crate::hash::clorus_hash(v);
                (*new_node).children[idx] =
                    build_branch_for_two(v, existing_hash, val, hash, level + 1);
                *is_new = true;
            }
        }
        Child::Node(child) => {
            let new_child = insert(child, level + 1, hash, val, is_new);
            (*new_node).children[idx] = Child::Node(new_child);
        }
        Child::Collision(bucket) => {
            let bucket_ref = &*bucket;
            let already_present = bucket_ref
                .entries
                .iter()
                .any(|v| crate::value::clorus_equals(*v, val));
            let mut new_entries = bucket_ref.entries.clone();
            for v in new_entries.iter() {
                crate::value::clorus_retain(*v);
            }
            if !already_present {
                crate::value::clorus_retain(val);
                new_entries.push(val);
            }
            (*new_node).children[idx] = Child::Collision(CollisionNode::new(bucket_ref.hash, new_entries));
            *is_new = !already_present;
        }
    }

    if !node.is_null() {
        for i in 0..BRANCHING {
            if i == idx {
                continue;
            }
            retain_child((*new_node).children[i]);
        }
    }

    new_node
}

/// Persistent hash set. Every write returns a new set; existing sets are
/// never mutated.
#[repr(C)]
pub struct ClorusHashSet {
    refcount: AtomicU64,
    count: u64,
    root: *mut SetNode,
}

impl ClorusHashSet {
    pub fn empty() -> *mut Self {
        Box::into_raw(Box::new(ClorusHashSet {
            refcount: AtomicU64::new(1),
            count: 0,
            root: null_mut(),
        }))
    }

    /// Return a new set with `val` added. Does not modify `set`.
    pub unsafe fn conj(set: *const Self, val: *mut Value) -> *mut Self {
        let set_ref = &*set;
        let hash = crate::hash::clorus_hash(val);
        let mut is_new = false;
        let new_root = insert(set_ref.root, 0, hash, val, &mut is_new);
        let new_count = if is_new { set_ref.count + 1 } else { set_ref.count };
        Box::into_raw(Box::new(ClorusHashSet {
            refcount: AtomicU64::new(1),
            count: new_count,
            root: new_root,
        }))
    }

    /// Return a new set without `val`. Does not modify `set`. Rebuilds via
    /// entries_iter + conj (mirrors how map dissoc is built above the
    /// trie); a dedicated trie-removal isn't needed for correctness.
    pub unsafe fn disj(set: *const Self, val: *mut Value) -> *mut Self {
        let set_ref = &*set;
        if !contains(set_ref.root, 0, crate::hash::clorus_hash(val), val) {
            let mut out = Vec::with_capacity(set_ref.count as usize);
            collect_node(set_ref.root, &mut out);
            let mut new_root: *mut SetNode = null_mut();
            let mut count = 0u64;
            for v in out {
                let h = crate::hash::clorus_hash(v);
                let mut is_new = false;
                new_root = insert(new_root, 0, h, v, &mut is_new);
                if is_new {
                    count += 1;
                }
            }
            return Box::into_raw(Box::new(ClorusHashSet {
                refcount: AtomicU64::new(1),
                count,
                root: new_root,
            }));
        }

        let mut out = Vec::with_capacity(set_ref.count as usize);
        collect_node(set_ref.root, &mut out);
        let mut new_root: *mut SetNode = null_mut();
        let mut count = 0u64;
        for v in out {
            if crate::value::clorus_equals(v, val) {
                continue;
            }
            let h = crate::hash::clorus_hash(v);
            let mut is_new = false;
            new_root = insert(new_root, 0, h, v, &mut is_new);
            if is_new {
                count += 1;
            }
        }
        Box::into_raw(Box::new(ClorusHashSet {
            refcount: AtomicU64::new(1),
            count,
            root: new_root,
        }))
    }

    pub fn contains(&self, val: *mut Value) -> bool {
        unsafe { contains(self.root, 0, crate::hash::clorus_hash(val), val) }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    /// All values in the set, in unspecified order.
    pub fn values(&self) -> Vec<*mut Value> {
        let mut out = Vec::with_capacity(self.count as usize);
        unsafe {
            collect_node(self.root, &mut out);
        }
        out
    }
}

/// Release a hash set and all its contents.
pub unsafe fn release_set(set: *mut ClorusHashSet) {
    if set.is_null() {
        return;
    }
    if (*set).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        release_node((*set).root);
        drop(Box::from_raw(set));
    }
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

/// Add a value to a set, returning a new set. `set_val` is left untouched.
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
        let new_set = ClorusHashSet::conj(set_ptr, val);
        Value::from_ptr(ValueTag::HashSet, new_set as *mut u8)
    }
}

/// Remove a value from a set, returning a new set. `set_val` is left
/// untouched.
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
        let new_set = ClorusHashSet::disj(set_ptr, val);
        Value::from_ptr(ValueTag::HashSet, new_set as *mut u8)
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
        Value::boolean((*set_ptr).contains(val))
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
        unsafe {
            crate::value::clorus_release(set);
        }
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
            crate::value::clorus_release(set);
            crate::value::clorus_release(set2);
            crate::value::clorus_release(set3);
            crate::value::clorus_release(val1);
            crate::value::clorus_release(val2);
        }
    }

    #[test]
    fn test_set_contains() {
        let set = clorus_set_empty();
        let val = Value::double(42.0);

        let contains_before = clorus_set_contains(set, val);
        unsafe {
            assert!(!(*contains_before).as_bool());
        }

        let set2 = clorus_set_conj(set, val);
        let contains_after = clorus_set_contains(set2, val);
        unsafe {
            assert!((*contains_after).as_bool());
        }

        unsafe {
            crate::value::clorus_release(set);
            crate::value::clorus_release(set2);
            crate::value::clorus_release(val);
            crate::value::clorus_release(contains_before);
            crate::value::clorus_release(contains_after);
        }
    }

    #[test]
    fn test_conj_does_not_mutate_original() {
        let set1 = clorus_set_empty();
        let val = Value::double(1.0);

        let set2 = clorus_set_conj(set1, val);

        unsafe {
            let set1_ptr = (*set1).as_ptr() as *mut ClorusHashSet;
            let set2_ptr = (*set2).as_ptr() as *mut ClorusHashSet;
            assert_eq!((*set1_ptr).count(), 0);
            assert_eq!((*set2_ptr).count(), 1);
            assert!(!(*set1_ptr).contains(val));
            assert!((*set2_ptr).contains(val));

            crate::value::clorus_release(set1);
            crate::value::clorus_release(set2);
            crate::value::clorus_release(val);
        }
    }

    #[test]
    fn test_disj_does_not_mutate_original() {
        let set = clorus_set_empty();
        let val1 = Value::double(1.0);
        let val2 = Value::double(2.0);

        let set1 = clorus_set_conj(set, val1);
        let set2 = clorus_set_conj(set1, val2);
        let set3 = clorus_set_disj(set2, val1);

        unsafe {
            let set2_ptr = (*set2).as_ptr() as *mut ClorusHashSet;
            let set3_ptr = (*set3).as_ptr() as *mut ClorusHashSet;
            assert_eq!((*set2_ptr).count(), 2);
            assert!((*set2_ptr).contains(val1));
            assert_eq!((*set3_ptr).count(), 1);
            assert!(!(*set3_ptr).contains(val1));
            assert!((*set3_ptr).contains(val2));

            crate::value::clorus_release(set);
            crate::value::clorus_release(set1);
            crate::value::clorus_release(set2);
            crate::value::clorus_release(set3);
            crate::value::clorus_release(val1);
            crate::value::clorus_release(val2);
        }
    }
}
