//! Ownership-safe representation of a parked `go` continuation.
//!
//! The compiler will emit continuations with this ABI.  A parked task owns its
//! captures and result channel until a channel resumes it; the resumed value is
//! owned for the duration of the continuation call.

use crate::value::{clorus_release, Value};

pub type Continuation = unsafe extern "C" fn(*mut Value, *mut Value, *mut Value) -> *mut Value;

pub struct ParkedTask {
    continuation: Continuation,
    captures: *mut Value,
    result_channel: *mut Value,
}

unsafe impl Send for ParkedTask {}

impl ParkedTask {
    /// Retains the values that must outlive the suspended compiled frame.
    pub unsafe fn new(
        continuation: Continuation,
        captures: *mut Value,
        result_channel: *mut Value,
    ) -> Self {
        if !captures.is_null() {
            (*captures).header().retain();
        }
        if !result_channel.is_null() {
            (*result_channel).header().retain();
        }
        Self { continuation, captures, result_channel }
    }

    /// Resume exactly once. The caller transfers ownership of `value`; it is
    /// released after the compiled continuation has consumed it.
    pub unsafe fn resume(self, value: *mut Value) {
        (self.continuation)(self.captures, value, self.result_channel);
        if !value.is_null() {
            clorus_release(value);
        }
    }
}

impl Drop for ParkedTask {
    fn drop(&mut self) {
        unsafe {
            if !self.captures.is_null() {
                clorus_release(self.captures);
            }
            if !self.result_channel.is_null() {
                clorus_release(self.result_channel);
            }
        }
    }
}
