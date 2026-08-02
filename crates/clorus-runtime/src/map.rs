/// Persistent HashMap implementation for Clorus
///
/// A hash array mapped trie (HAMT): 32-way branching (5 bits of the key's
/// hash per level), path copying for updates, structural sharing between
/// versions. Same shape as `PersistentVector`'s trie, keyed by hash instead
/// of index. `assoc`/`dissoc` never mutate an existing map -- every update
/// returns a new map, and unrelated subtrees are shared (refcounted) rather
/// than copied.
///
/// Two keys whose hashes collide at a level split into a two-level branch;
/// if their hashes still collide once all 64 hash bits are exhausted (13
/// levels), they fall back to a small linear collision bucket.

use crate::value::Value;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU64, Ordering};

const BITS: u32 = 5;
const BRANCHING: usize = 32;
const MASK: u64 = 0x1F;
/// ceil(64 / 5): the trie can address all 64 hash bits in this many levels
/// (level 12 shifts by 60, covering the remaining 4 bits) before it must
/// fall back to a collision bucket.
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
    Entry(*mut Value, *mut Value),
    Node(*mut MapNode),
    Collision(*mut CollisionNode),
}

#[inline]
unsafe fn retain_child(child: Child) {
    match child {
        Child::Empty => {}
        Child::Entry(k, v) => {
            crate::value::clorus_retain(k);
            crate::value::clorus_retain(v);
        }
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
        Child::Entry(k, v) => {
            crate::value::clorus_release(k);
            crate::value::clorus_release(v);
        }
        Child::Node(n) => release_node(n),
        Child::Collision(c) => release_collision(c),
    }
}

unsafe fn collect_child(child: Child, out: &mut Vec<(*mut Value, *mut Value)>) {
    match child {
        Child::Empty => {}
        Child::Entry(k, v) => out.push((k, v)),
        Child::Node(n) => collect_node(n, out),
        Child::Collision(c) => out.extend((*c).entries.iter().copied()),
    }
}

unsafe fn lookup_child(child: Child, hash: u64, key: *mut Value, level: u32) -> Option<*mut Value> {
    match child {
        Child::Empty => None,
        Child::Entry(k, v) => {
            if crate::value::clorus_equals(k, key) {
                Some(v)
            } else {
                None
            }
        }
        Child::Node(n) => lookup(n, level + 1, hash, key),
        Child::Collision(c) => (*c)
            .entries
            .iter()
            .find(|(k, _)| crate::value::clorus_equals(*k, key))
            .map(|(_, v)| *v),
    }
}

/// Internal trie node: 32 slots, each empty / a single entry / a child node
/// / a collision bucket.
struct MapNode {
    refcount: AtomicU64,
    children: [Child; BRANCHING],
}

impl MapNode {
    fn new() -> *mut Self {
        Box::into_raw(Box::new(MapNode {
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

/// Small linear bucket for keys whose hashes are identical across all 64
/// bits (either a true collision or, vanishingly rarely, exhausted trie
/// depth). Always rebuilt fresh on write, same persistence discipline as
/// the trie nodes above it.
struct CollisionNode {
    refcount: AtomicU64,
    hash: u64,
    entries: Vec<(*mut Value, *mut Value)>,
}

impl CollisionNode {
    fn new(hash: u64, entries: Vec<(*mut Value, *mut Value)>) -> *mut Self {
        Box::into_raw(Box::new(CollisionNode {
            refcount: AtomicU64::new(1),
            hash,
            entries,
        }))
    }
}

unsafe fn release_node(node: *mut MapNode) {
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
        for (k, v) in (*node).entries.iter() {
            crate::value::clorus_release(*k);
            crate::value::clorus_release(*v);
        }
        drop(Box::from_raw(node));
    }
}

unsafe fn collect_node(node: *mut MapNode, out: &mut Vec<(*mut Value, *mut Value)>) {
    if node.is_null() {
        return;
    }
    for i in 0..BRANCHING {
        collect_child((*node).children[i], out);
    }
}

unsafe fn lookup(node: *mut MapNode, level: u32, hash: u64, key: *mut Value) -> Option<*mut Value> {
    if node.is_null() {
        return None;
    }
    let idx = level_index(hash, level);
    lookup_child((*node).children[idx], hash, key, level)
}

/// Build the subtree distinguishing two different keys that collided at
/// the parent's level. `k1`/`v1` are moved in (already owned by the
/// caller, no retain); `k2`/`v2` are a new insert and must be retained.
unsafe fn build_branch_for_two(
    k1: *mut Value,
    v1: *mut Value,
    hash1: u64,
    k2: *mut Value,
    v2: *mut Value,
    hash2: u64,
    level: u32,
) -> Child {
    if level >= MAX_LEVELS {
        crate::value::clorus_retain(k2);
        crate::value::clorus_retain(v2);
        let bucket = CollisionNode::new(hash1, vec![(k1, v1), (k2, v2)]);
        return Child::Collision(bucket);
    }

    let idx1 = level_index(hash1, level);
    let idx2 = level_index(hash2, level);
    let node = MapNode::new();

    if idx1 == idx2 {
        let child = build_branch_for_two(k1, v1, hash1, k2, v2, hash2, level + 1);
        (*node).children[idx1] = child;
    } else {
        crate::value::clorus_retain(k2);
        crate::value::clorus_retain(v2);
        (*node).children[idx1] = Child::Entry(k1, v1);
        (*node).children[idx2] = Child::Entry(k2, v2);
    }

    Child::Node(node)
}

/// Persistently insert/update `key` -> `val` starting at `node` (may be
/// null for an empty subtree). Returns the new subtree root; `is_new_key`
/// is set to true iff this insert grew the map's count.
unsafe fn insert(
    node: *mut MapNode,
    level: u32,
    hash: u64,
    key: *mut Value,
    val: *mut Value,
    is_new_key: &mut bool,
) -> *mut MapNode {
    let idx = level_index(hash, level);
    let new_node = if node.is_null() {
        MapNode::new()
    } else {
        MapNode::clone_node(node)
    };
    let existing = if node.is_null() {
        Child::Empty
    } else {
        (*node).children[idx]
    };

    match existing {
        Child::Empty => {
            crate::value::clorus_retain(key);
            crate::value::clorus_retain(val);
            (*new_node).children[idx] = Child::Entry(key, val);
            *is_new_key = true;
        }
        Child::Entry(k, v) => {
            if crate::value::clorus_equals(k, key) {
                // new_node's slot idx is being replaced outright (not shared
                // via the sibling-retain loop below), so it needs its own
                // retained reference to the new value -- but NOT a release
                // of the old (k, v): `node` (untouched) still owns that
                // reference via its own slot idx and will release it itself
                // whenever it is eventually freed.
                crate::value::clorus_retain(key);
                crate::value::clorus_retain(val);
                (*new_node).children[idx] = Child::Entry(key, val);
                *is_new_key = false;
            } else {
                // k, v are being duplicated from node's slot into a second,
                // independent branch reachable from new_node -- node still
                // owns its own reference, so this duplication needs its own
                // retain (build_branch_for_two already retains key/val).
                crate::value::clorus_retain(k);
                crate::value::clorus_retain(v);
                let existing_hash = crate::hash::clorus_hash(k);
                (*new_node).children[idx] =
                    build_branch_for_two(k, v, existing_hash, key, val, hash, level + 1);
                *is_new_key = true;
            }
        }
        Child::Node(child) => {
            let new_child = insert(child, level + 1, hash, key, val, is_new_key);
            (*new_node).children[idx] = Child::Node(new_child);
        }
        Child::Collision(bucket) => {
            let bucket_ref = &*bucket;
            let mut found = false;
            let mut new_entries = Vec::with_capacity(bucket_ref.entries.len() + 1);
            for &(k, v) in bucket_ref.entries.iter() {
                if crate::value::clorus_equals(k, key) {
                    // Same reasoning as the Entry case above: new_entries
                    // gets its own retained reference to the new value; the
                    // old (k, v) is not released here since `bucket` is
                    // untouched and still owns it.
                    crate::value::clorus_retain(key);
                    crate::value::clorus_retain(val);
                    new_entries.push((key, val));
                    found = true;
                } else {
                    crate::value::clorus_retain(k);
                    crate::value::clorus_retain(v);
                    new_entries.push((k, v));
                }
            }
            if !found {
                crate::value::clorus_retain(key);
                crate::value::clorus_retain(val);
                new_entries.push((key, val));
            }
            (*new_node).children[idx] = Child::Collision(CollisionNode::new(bucket_ref.hash, new_entries));
            *is_new_key = !found;
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

/// Persistent hash map. Every write returns a new map; existing maps are
/// never mutated. `assoc`/`get`/`count` are O(log32 n); `entries_iter`
/// walks the whole trie.
#[repr(C)]
pub struct ClorusHashMap {
    refcount: AtomicU64,
    count: u64,
    root: *mut MapNode,
}

impl ClorusHashMap {
    pub fn empty() -> *mut Self {
        Box::into_raw(Box::new(ClorusHashMap {
            refcount: AtomicU64::new(1),
            count: 0,
            root: null_mut(),
        }))
    }

    /// Return a new map with `key` associated to `val`. Does not modify
    /// `map` -- any other reference to it keeps seeing the old value.
    pub unsafe fn assoc(map: *const Self, key: *mut Value, val: *mut Value) -> *mut Self {
        let map_ref = &*map;
        let hash = crate::hash::clorus_hash(key);
        let mut is_new_key = false;
        let new_root = insert(map_ref.root, 0, hash, key, val, &mut is_new_key);
        let new_count = if is_new_key {
            map_ref.count + 1
        } else {
            map_ref.count
        };
        Box::into_raw(Box::new(ClorusHashMap {
            refcount: AtomicU64::new(1),
            count: new_count,
            root: new_root,
        }))
    }

    /// Get a value by key, returns nil if not found. Retains the returned
    /// value before returning it; the caller releases it when done.
    pub fn get(&self, key: *mut Value) -> *mut Value {
        match unsafe { lookup(self.root, 0, crate::hash::clorus_hash(key), key) } {
            Some(v) => {
                unsafe {
                    crate::value::clorus_retain(v);
                }
                v
            }
            None => Value::nil(),
        }
    }

    /// Get a value by key without retaining (internal use).
    pub fn get_entry(&self, key: *mut Value) -> Option<*mut Value> {
        unsafe { lookup(self.root, 0, crate::hash::clorus_hash(key), key) }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    /// All (key, value) pairs in the map, in unspecified order. Pointers
    /// are borrowed (not retained) -- same contract as before.
    pub fn entries_iter(&self) -> Vec<(*mut Value, *mut Value)> {
        let mut out = Vec::with_capacity(self.count as usize);
        unsafe {
            collect_node(self.root, &mut out);
        }
        out
    }
}

/// Release a hash map: decrement its refcount and, once it reaches zero,
/// release the trie and free the map itself.
pub unsafe fn release_map(map: *mut ClorusHashMap) {
    if map.is_null() {
        return;
    }
    if (*map).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        release_node((*map).root);
        drop(Box::from_raw(map));
    }
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
    val: *mut Value,
) -> *mut Value {
    unsafe {
        let target_map = if map_val.is_null() {
            clorus_map_empty()
        } else if (*map_val).header().tag() != crate::value::ValueTag::HashMap {
            clorus_map_empty()
        } else if (*map_val).as_ptr().is_null() {
            clorus_map_empty()
        } else {
            crate::value::clorus_retain(map_val);
            map_val
        };
        let map_ptr = (*target_map).as_ptr() as *mut ClorusHashMap;
        if map_ptr.is_null() {
            return target_map;
        }
        if key.is_null() {
            return target_map;
        }
        let value_to_store = if val.is_null() { Value::nil() } else { val };

        let new_map = ClorusHashMap::assoc(map_ptr, key, value_to_store);
        crate::value::clorus_release(target_map);
        Value::from_ptr(crate::value::ValueTag::HashMap, new_map as *mut u8)
    }
}

#[no_mangle]
pub extern "C" fn clorus_map_get(map_val: *mut Value, key: *mut Value) -> *mut Value {
    if map_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*map_val).header().tag() != crate::value::ValueTag::HashMap {
            return Value::nil();
        }
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        if map_ptr.is_null() {
            return Value::nil();
        }
        (*map_ptr).get(key)
    }
}

#[no_mangle]
pub extern "C" fn clorus_map_count(map_val: *mut Value) -> u64 {
    if map_val.is_null() {
        return 0;
    }

    unsafe {
        if (*map_val).header().tag() != crate::value::ValueTag::HashMap {
            return 0;
        }
        let map_ptr = (*map_val).as_ptr() as *mut ClorusHashMap;
        if map_ptr.is_null() {
            return 0;
        }
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
        unsafe {
            crate::value::clorus_release(map_val);
        }
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
            crate::value::clorus_release(result);
            crate::value::clorus_release(map2);
            crate::value::clorus_release(map);
            crate::value::clorus_release(key);
            crate::value::clorus_release(val);
        }
    }

    #[test]
    fn test_assoc_does_not_mutate_original() {
        // The whole point of "persistent": assoc on map1 must not be
        // visible through map1 itself.
        let map1 = clorus_map_empty();
        let key = Value::double(1.0);
        let val_a = Value::string("a");
        let val_b = Value::string("b");

        let map1 = clorus_map_assoc(map1, key, val_a);
        let map2 = clorus_map_assoc(map1, key, val_b);

        unsafe {
            let from_map1 = clorus_map_get(map1, key);
            assert_eq!((*from_map1).as_string(), "a");

            let from_map2 = clorus_map_get(map2, key);
            assert_eq!((*from_map2).as_string(), "b");

            assert_eq!(clorus_map_count(map1), 1);
            assert_eq!(clorus_map_count(map2), 1);

            crate::value::clorus_release(from_map1);
            crate::value::clorus_release(from_map2);
            crate::value::clorus_release(map1);
            crate::value::clorus_release(map2);
            crate::value::clorus_release(key);
            crate::value::clorus_release(val_a);
            crate::value::clorus_release(val_b);
        }
    }

    #[test]
    fn test_assoc_many_keys_and_lookup() {
        let mut map = clorus_map_empty();
        let mut keys = Vec::new();
        let mut vals = Vec::new();

        for i in 0..200i64 {
            let k = Value::long(i);
            let v = Value::long(i * 10);
            let new_map = clorus_map_assoc(map, k, v);
            unsafe {
                crate::value::clorus_release(map);
            }
            map = new_map;
            keys.push(k);
            vals.push(v);
        }

        unsafe {
            assert_eq!(clorus_map_count(map), 200);
            for i in 0..200i64 {
                let result = clorus_map_get(map, keys[i as usize]);
                assert_eq!((*result).as_long(), i * 10);
                crate::value::clorus_release(result);
            }

            crate::value::clorus_release(map);
            for k in keys {
                crate::value::clorus_release(k);
            }
            for v in vals {
                crate::value::clorus_release(v);
            }
        }
    }

    #[test]
    fn test_update_existing_key_keeps_count() {
        let map = clorus_map_empty();
        let key = Value::string("k");
        let val1 = Value::long(1);
        let val2 = Value::long(2);

        let map1 = clorus_map_assoc(map, key, val1);
        let map2 = clorus_map_assoc(map1, key, val2);

        unsafe {
            assert_eq!(clorus_map_count(map2), 1);
            let result = clorus_map_get(map2, key);
            assert_eq!((*result).as_long(), 2);
            crate::value::clorus_release(result);

            crate::value::clorus_release(map);
            crate::value::clorus_release(map1);
            crate::value::clorus_release(map2);
            crate::value::clorus_release(key);
            crate::value::clorus_release(val1);
            crate::value::clorus_release(val2);
        }
    }
}
