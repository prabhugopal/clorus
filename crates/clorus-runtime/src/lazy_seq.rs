//! Internal ownership and realization state for native lazy sequences.
//!
//! This is intentionally not public language surface yet.  The `LazySeq`
//! `ValueTag` wrapper is added only after this cell's lifecycle is tested.

use std::sync::{Condvar, Mutex};
use std::thread::{self, ThreadId};

use crate::collections::clorus_seq;
use crate::function::clorus_function_call;
use crate::value::{clorus_is_exception, clorus_release, clorus_retain, Value, ValueTag};

#[derive(Debug)]
enum State {
    Unforced,
    Forcing(ThreadId),
    Realized(*mut Value),
}

/// A memoized zero-arity Clorus thunk.
///
/// The thunk and cached result are owned references. `force` returns its own
/// reference, so callers may release it independently of this cell.
pub(crate) struct LazySeqCell {
    thunk: Mutex<*mut Value>,
    state: Mutex<State>,
    realized: Condvar,
}

impl LazySeqCell {
    pub(crate) unsafe fn new(thunk: *mut Value) -> Self {
        if !thunk.is_null() {
            clorus_retain(thunk);
        }
        Self {
            thunk: Mutex::new(thunk),
            state: Mutex::new(State::Unforced),
            realized: Condvar::new(),
        }
    }

    /// Force once, memoizing a successful result.
    ///
    /// A second thread waits for the thread currently evaluating the thunk and
    /// then observes its cached result (or retries after an exception). A thunk
    /// which recursively forces its own cell is rejected rather than deadlocking.
    /// The public wrapper turns that rejection into a Clorus exception.
    pub(crate) unsafe fn force(&self) -> Result<*mut Value, ()> {
        let current_thread = thread::current().id();
        let mut state = self.state.lock().expect("lazy sequence state poisoned");
        loop {
            match *state {
                State::Unforced => {
                    *state = State::Forcing(current_thread);
                    break;
                }
                State::Realized(value) => {
                    if !value.is_null() {
                        clorus_retain(value);
                    }
                    return Ok(value);
                }
                State::Forcing(owner) if owner == current_thread => return Err(()),
                State::Forcing(_) => {
                    state = self.realized.wait(state).expect("lazy sequence state poisoned");
                }
            }
        }
        drop(state);

        let thunk = *self.thunk.lock().expect("lazy sequence thunk poisoned");
        let result = clorus_function_call(thunk, std::ptr::null(), 0);

        let mut state = self.state.lock().expect("lazy sequence state poisoned");
        if clorus_is_exception(result) {
            *state = State::Unforced;
            self.realized.notify_all();
            return Ok(result);
        }

        // A lazy-seq thunk has the same contract as `seq`: it returns nil or
        // something sequenceable. Normalize finite collections here, once,
        // so `(seq lazy-value)` always yields the canonical list sequence.
        let sequenceable = result.is_null() || matches!(
            (*result).tag(),
            ValueTag::Nil | ValueTag::List | ValueTag::Vector | ValueTag::HashMap | ValueTag::HashSet | ValueTag::LazySeq
        );
        if !sequenceable {
            let message = Value::string("lazy-seq body must return a sequence or nil");
            let exception = Value::exception(message);
            clorus_release(message);
            clorus_release(result);
            *state = State::Unforced;
            self.realized.notify_all();
            return Ok(exception);
        }
        let sequence = clorus_seq(result);
        if !result.is_null() {
            clorus_release(result);
        }
        let result = sequence;

        if !result.is_null() {
            clorus_retain(result);
        }
        *state = State::Realized(result);
        self.realized.notify_all();

        let mut thunk_slot = self.thunk.lock().expect("lazy sequence thunk poisoned");
        if !(*thunk_slot).is_null() {
            clorus_release(*thunk_slot);
            *thunk_slot = std::ptr::null_mut();
        }
        Ok(result)
    }
}

impl Drop for LazySeqCell {
    fn drop(&mut self) {
        unsafe {
            let thunk = *self.thunk.get_mut().expect("lazy sequence thunk poisoned");
            if !thunk.is_null() {
                clorus_release(thunk);
            }
            if let State::Realized(value) = *self.state.get_mut().expect("lazy sequence state poisoned") {
                if !value.is_null() {
                    clorus_release(value);
                }
            }
        }
    }
}

/// Construct a native lazy-sequence value from a zero-arity thunk. This is an
/// internal runtime primitive; the language-level `lazy-seq` form will own its
/// syntax and arity checks.
#[no_mangle]
pub extern "C" fn clorus_lazy_seq_new(thunk: *mut Value) -> *mut Value {
    unsafe { Value::from_lazy_seq(Box::into_raw(Box::new(LazySeqCell::new(thunk)))) }
}

/// Force one lazy-sequence cell. Recursive realization is an ordinary Clorus
/// exception, never a blocking wait or a poisoned cache entry.
#[no_mangle]
pub extern "C" fn clorus_lazy_seq_force(value: *mut Value) -> *mut Value {
    unsafe {
        if value.is_null() || (*value).tag() != crate::value::ValueTag::LazySeq {
            return Value::nil();
        }
        match (*value).as_lazy_seq().as_ref().expect("lazy sequence cell missing").force() {
            Ok(result) => result,
            Err(()) => {
                let message = Value::string("recursive lazy sequence realization");
                let exception = Value::exception(message);
                clorus_release(message);
                exception
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn clorus_is_lazy_seq_i32(value: *mut Value) -> i32 {
    unsafe {
        (!value.is_null() && (*value).tag() == crate::value::ValueTag::LazySeq) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::{clorus_is_lazy_seq_i32, clorus_lazy_seq_force, clorus_lazy_seq_new, LazySeqCell};
    use crate::collections::{clorus_first, clorus_rest, clorus_seq};
    use crate::value::{clorus_is_seq, clorus_release};

    #[test]
    fn nil_result_is_cached_and_released_with_the_cell() {
        unsafe {
            // A null thunk follows the runtime's normal defensive function
            // call behavior and returns nil. This exercises both cache paths
            // without needing LLVM-generated test code.
            let cell = LazySeqCell::new(std::ptr::null_mut());
            let first = cell.force().expect("first force should not recurse");
            let second = cell.force().expect("cached force should not recurse");
            clorus_release(first);
            clorus_release(second);
            drop(cell);
        }
    }

    #[test]
    fn native_wrapper_forces_and_releases_cached_result() {
        unsafe {
            let lazy = clorus_lazy_seq_new(std::ptr::null_mut());
            assert_eq!(clorus_is_lazy_seq_i32(lazy), 1);
            assert!(clorus_is_seq(lazy));
            let first = clorus_lazy_seq_force(lazy);
            let second = clorus_lazy_seq_force(lazy);
            clorus_release(first);
            clorus_release(second);
            clorus_release(lazy);
        }
    }

    #[test]
    fn core_sequence_boundary_accepts_an_empty_native_lazy_sequence() {
        unsafe {
            let lazy = clorus_lazy_seq_new(std::ptr::null_mut());
            let seq = clorus_seq(lazy);
            let first = clorus_first(lazy);
            let rest = clorus_rest(lazy);
            clorus_release(seq);
            clorus_release(first);
            clorus_release(rest);
            clorus_release(lazy);
        }
    }
}
