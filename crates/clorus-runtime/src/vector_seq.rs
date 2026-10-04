//! A non-copying sequence cursor over an immutable vector.
//!
//! Repeated `(rest vector)` must not materialize a shorter vector at each
//! step: lazy transforms take one rest step per element, and copying makes
//! them quadratic. This cursor retains its backing vector and advances only
//! an offset, keeping each sequence step O(1).

use crate::value::{clorus_release, clorus_retain, Value};
use crate::vector::PersistentVector;

pub(crate) struct VectorSeq {
    vector: *mut Value,
    offset: u64,
}

impl VectorSeq {
    pub(crate) unsafe fn new(vector: *mut Value, offset: u64) -> Self {
        clorus_retain(vector);
        Self { vector, offset }
    }

    pub(crate) unsafe fn count(&self) -> u64 {
        let vector = (*self.vector).as_ptr() as *mut PersistentVector;
        (*vector).count().saturating_sub(self.offset)
    }

    pub(crate) unsafe fn nth(&self, index: u64) -> *mut Value {
        if index >= self.count() {
            Value::nil()
        } else {
            let vector = (*self.vector).as_ptr() as *mut PersistentVector;
            PersistentVector::nth(vector, self.offset + index)
        }
    }

    pub(crate) fn backing(&self) -> *mut Value {
        self.vector
    }

    pub(crate) fn next_offset(&self) -> u64 {
        self.offset.saturating_add(1)
    }
}

impl Drop for VectorSeq {
    fn drop(&mut self) {
        unsafe { clorus_release(self.vector) }
    }
}
