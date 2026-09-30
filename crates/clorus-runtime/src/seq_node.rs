//! General sequence steps used to join a realized head to a possibly lazy tail.
//!
//! `PersistentList` intentionally stores a list-node tail for compact finite
//! structural sharing. A `SeqNode` is the separate public-sequence boundary
//! for a head whose tail may be a `LazySeq`, another `SeqNode`, or a list.

use crate::collections::clorus_seq;
use crate::value::{clorus_is_exception, clorus_release, clorus_retain, Value, ValueTag};

/// One immutable sequence step. Both fields are owned references.
pub(crate) struct SeqNode {
    head: *mut Value,
    tail: *mut Value,
}

impl SeqNode {
    unsafe fn new(head: *mut Value, tail: *mut Value) -> Self {
        if !head.is_null() {
            clorus_retain(head);
        }
        if !tail.is_null() {
            clorus_retain(tail);
        }
        Self { head, tail }
    }

    pub(crate) unsafe fn head(&self) -> *mut Value {
        if !self.head.is_null() {
            clorus_retain(self.head);
        }
        self.head
    }

    pub(crate) unsafe fn tail(&self) -> *mut Value {
        if !self.tail.is_null() {
            clorus_retain(self.tail);
        }
        self.tail
    }
}

impl Drop for SeqNode {
    fn drop(&mut self) {
        unsafe {
            if !self.head.is_null() {
                clorus_release(self.head);
            }
            if !self.tail.is_null() {
                clorus_release(self.tail);
            }
        }
    }
}

/// Build one sequence step without forcing the tail.
///
/// The tail is normalized through `seq` only when it is an eager collection.
/// A `LazySeq` and another `SeqNode` are preserved unchanged, which is the
/// essential property needed for a genuinely incremental producer.
#[no_mangle]
pub extern "C" fn clorus_seq_cons(head: *mut Value, tail: *mut Value) -> *mut Value {
    unsafe {
        let canonical_tail = if tail.is_null() || (*tail).tag() == ValueTag::Nil {
            Value::nil()
        } else {
            match (*tail).tag() {
                ValueTag::List | ValueTag::SeqNode | ValueTag::LazySeq => {
                    clorus_retain(tail);
                    tail
                }
                ValueTag::Vector | ValueTag::HashMap | ValueTag::HashSet => clorus_seq(tail),
                _ => {
                    let message = Value::string("sequence tail must be a sequence or nil");
                    let exception = Value::exception(message);
                    clorus_release(message);
                    return exception;
                }
            }
        };

        if clorus_is_exception(canonical_tail) {
            return canonical_tail;
        }

        let node = SeqNode::new(head, canonical_tail);
        if !canonical_tail.is_null() {
            clorus_release(canonical_tail);
        }
        Value::from_seq_node(Box::into_raw(Box::new(node)))
    }
}

pub(crate) unsafe fn seq_node_head(value: *mut Value) -> *mut Value {
    let node = (*value).as_seq_node();
    (*node).head()
}

pub(crate) unsafe fn seq_node_tail(value: *mut Value) -> *mut Value {
    let node = (*value).as_seq_node();
    (*node).tail()
}

#[cfg(test)]
mod tests {
    use super::{clorus_seq_cons, seq_node_head, seq_node_tail};
    use crate::lazy_seq::clorus_lazy_seq_new;
    use crate::value::{clorus_release, Value, ValueTag};

    #[test]
    fn sequence_step_owns_head_and_lazy_tail() {
        unsafe {
            let head = Value::long(7);
            let lazy_tail = clorus_lazy_seq_new(std::ptr::null_mut());
            let step = clorus_seq_cons(head, lazy_tail);
            assert_eq!((*step).tag(), ValueTag::SeqNode);

            // The step's retained references must remain valid after callers
            // release their original head and tail references.
            clorus_release(head);
            clorus_release(lazy_tail);

            let observed_head = seq_node_head(step);
            assert_eq!((*observed_head).as_long(), 7);
            clorus_release(observed_head);

            let observed_tail = seq_node_tail(step);
            assert_eq!((*observed_tail).tag(), ValueTag::LazySeq);
            clorus_release(observed_tail);
            clorus_release(step);
        }
    }
}
