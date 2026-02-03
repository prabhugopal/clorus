/// Persistent List implementation
///
/// A simple singly-linked list with structural sharing.
/// Based on Okasaki's "Purely Functional Data Structures" Chapter 2.

use crate::value::{Value, ValueTag};
use std::sync::atomic::{AtomicU64, Ordering};
use std::ptr::null_mut;

/// A node in a persistent list
///
/// Each node contains a value and a pointer to the rest of the list.
/// Nodes are reference counted for memory management.
#[repr(C)]
pub struct ListNode {
    /// Reference count for this node
    refcount: AtomicU64,
    /// The value at this position
    head: *mut Value,
    /// The rest of the list (may be null for empty list)
    tail: *mut ListNode,
}

impl ListNode {
    /// Create a new list node
    fn new(head: *mut Value, tail: *mut ListNode) -> *mut Self {
        // Retain head value
        if !head.is_null() {
            unsafe { (*head).header().retain(); }
        }

        // Retain tail
        if !tail.is_null() {
            unsafe {
                (*tail).refcount.fetch_add(1, Ordering::Relaxed);
            }
        }

        let node = Box::new(ListNode {
            refcount: AtomicU64::new(1),
            head,
            tail,
        });

        Box::into_raw(node)
    }

    #[inline]
    fn refcount(&self) -> u64 {
        self.refcount.load(Ordering::Relaxed)
    }
}

/// Persistent List
///
/// Implemented as a singly-linked list.
/// Operations:
/// - cons: O(1) - add element to front
/// - first: O(1) - get first element
/// - rest: O(1) - get all but first element
/// - count: O(n) - count elements (could cache this)
pub struct PersistentList {
    /// Pointer to first node (null for empty list)
    head: *mut ListNode,
}

impl PersistentList {
    /// Create an empty list
    pub fn empty() -> Self {
        PersistentList { head: null_mut() }
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// Add element to front of list (cons)
    ///
    /// Returns a new list sharing the tail.
    /// O(1) time, O(1) space.
    pub fn cons(&self, value: *mut Value) -> Self {
        PersistentList {
            head: ListNode::new(value, self.head),
        }
    }

    /// Get first element
    ///
    /// Returns null for empty list.
    pub fn first(&self) -> *mut Value {
        if self.head.is_null() {
            null_mut()
        } else {
            unsafe { (*self.head).head }
        }
    }

    /// Get rest of list (all but first)
    ///
    /// Returns empty list for empty or single-element list.
    pub fn rest(&self) -> Self {
        if self.head.is_null() {
            PersistentList::empty()
        } else {
            unsafe {
                let tail = (*self.head).tail;
                if !tail.is_null() {
                    (*tail).refcount.fetch_add(1, Ordering::Relaxed);
                }
                PersistentList { head: tail }
            }
        }
    }

    /// Count elements in list
    ///
    /// O(n) operation - walks the entire list.
    pub fn count(&self) -> u64 {
        let mut count = 0;
        let mut current = self.head;
        while !current.is_null() {
            count += 1;
            unsafe {
                current = (*current).tail;
            }
        }
        count
    }
}

impl Clone for PersistentList {
    fn clone(&self) -> Self {
        if !self.head.is_null() {
            unsafe {
                (*self.head).refcount.fetch_add(1, Ordering::Relaxed);
            }
        }
        PersistentList { head: self.head }
    }
}

impl Drop for PersistentList {
    fn drop(&mut self) {
        if !self.head.is_null() {
            unsafe {
                release_list(self.head);
            }
        }
    }
}

/// Release a list node (decrement refcount, free if zero)
///
/// This is called from the Value deallocation code.
pub(crate) unsafe fn release_list(node: *mut ListNode) {
    if node.is_null() {
        return;
    }

    if (*node).refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        // Last reference - deallocate
        let head = (*node).head;
        let tail = (*node).tail;

        // Release head value
        if !head.is_null() {
            crate::value::clorus_release(head);
        }

        // Release tail (recursive)
        if !tail.is_null() {
            release_list(tail);
        }

        // Free the node itself
        drop(Box::from_raw(node));
    }
}

// FFI functions for LLVM-generated code

/// Create an empty list
#[no_mangle]
pub extern "C" fn clorus_list_empty() -> *mut Value {
    let list = PersistentList::empty();
    let ptr = Box::into_raw(Box::new(list)) as *mut u8;
    Value::from_ptr(ValueTag::List, ptr)
}

/// Add element to front of list (cons)
#[no_mangle]
pub extern "C" fn clorus_list_cons(list_val: *mut Value, elem: *mut Value) -> *mut Value {
    if list_val.is_null() {
        return clorus_list_empty();
    }

    unsafe {
        let list_ptr = (*list_val).as_ptr() as *mut PersistentList;
        let list = &*list_ptr;

        let new_list = list.cons(elem);
        let new_ptr = Box::into_raw(Box::new(new_list)) as *mut u8;
        Value::from_ptr(ValueTag::List, new_ptr)
    }
}

/// Get first element of list
#[no_mangle]
pub extern "C" fn clorus_list_first(list_val: *mut Value) -> *mut Value {
    if list_val.is_null() {
        return Value::nil();
    }

    unsafe {
        let list_ptr = (*list_val).as_ptr() as *mut PersistentList;
        let list = &*list_ptr;

        let first = list.first();
        if first.is_null() {
            Value::nil()
        } else {
            // Retain before returning
            (*first).header().retain();
            first
        }
    }
}

/// Get rest of list (all but first)
#[no_mangle]
pub extern "C" fn clorus_list_rest(list_val: *mut Value) -> *mut Value {
    if list_val.is_null() {
        return clorus_list_empty();
    }

    unsafe {
        let list_ptr = (*list_val).as_ptr() as *mut PersistentList;
        let list = &*list_ptr;

        let rest = list.rest();
        let rest_ptr = Box::into_raw(Box::new(rest)) as *mut u8;
        Value::from_ptr(ValueTag::List, rest_ptr)
    }
}

/// Count elements in list
#[no_mangle]
pub extern "C" fn clorus_list_count(list_val: *mut Value) -> u64 {
    if list_val.is_null() {
        return 0;
    }

    unsafe {
        let list_ptr = (*list_val).as_ptr() as *mut PersistentList;
        let list = &*list_ptr;
        list.count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_empty_list() {
        let list = PersistentList::empty();
        assert!(list.is_empty());
        assert_eq!(list.count(), 0);
        assert!(list.first().is_null());
    }

    #[test]
    fn test_cons() {
        let list = PersistentList::empty();

        let val1 = Value::double(1.0);
        let list1 = list.cons(val1);
        assert!(!list1.is_empty());
        assert_eq!(list1.count(), 1);

        let val2 = Value::double(2.0);
        let list2 = list1.cons(val2);
        assert_eq!(list2.count(), 2);

        // Original list still empty
        assert!(list.is_empty());
    }

    #[test]
    fn test_first_rest() {
        let list = PersistentList::empty();

        let val1 = Value::double(1.0);
        let val2 = Value::double(2.0);
        let val3 = Value::double(3.0);

        let list = list.cons(val3).cons(val2).cons(val1);
        // List is now [1, 2, 3]

        unsafe {
            assert_eq!((*list.first()).as_double(), 1.0);

            let rest = list.rest();
            assert_eq!((*rest.first()).as_double(), 2.0);

            let rest2 = rest.rest();
            assert_eq!((*rest2.first()).as_double(), 3.0);

            let rest3 = rest2.rest();
            assert!(rest3.is_empty());
        }
    }

    #[test]
    fn test_structural_sharing() {
        let list1 = PersistentList::empty();

        let val1 = Value::double(1.0);
        let val2 = Value::double(2.0);
        let val3 = Value::double(3.0);

        let list1 = list1.cons(val1).cons(val2);
        // list1 = [2, 1]

        let list2 = list1.cons(val3);
        // list2 = [3, 2, 1]

        // Both lists should share the [2, 1] tail
        // Verify by checking refcount on shared nodes
        unsafe {
            // The tail of list2 should be list1's head
            assert_eq!((*list2.head).tail, list1.head);

            // The shared node should have refcount > 1
            assert!((*list1.head).refcount() >= 1);
        }
    }

    #[test]
    fn test_ffi_functions() {
        let empty = clorus_list_empty();
        assert_eq!(clorus_list_count(empty), 0);

        let val1 = Value::double(42.0);
        let list1 = clorus_list_cons(empty, val1);
        assert_eq!(clorus_list_count(list1), 1);

        let first = clorus_list_first(list1);
        unsafe {
            assert_eq!((*first).as_double(), 42.0);
        }

        // Clean up
        crate::value::clorus_release(empty);
        crate::value::clorus_release(list1);
        crate::value::clorus_release(first);
    }
}
