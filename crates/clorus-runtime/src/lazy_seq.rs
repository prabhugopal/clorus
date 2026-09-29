//! Internal ownership and realization state for native lazy sequences.
//!
//! This is intentionally not public language surface yet.  The `LazySeq`
//! `ValueTag` wrapper is added only after this cell's lifecycle is tested.

use std::sync::Mutex;

use crate::function::clorus_function_call;
use crate::value::{clorus_is_exception, clorus_release, clorus_retain, Value};

#[derive(Debug)]
enum State {
    Unforced,
    Forcing,
    Realized(*mut Value),
}

/// A memoized zero-arity Clorus thunk.
///
/// The thunk and cached result are owned references. `force` returns its own
/// reference, so callers may release it independently of this cell.
pub(crate) struct LazySeqCell {
    thunk: Mutex<*mut Value>,
    state: Mutex<State>,
}

impl LazySeqCell {
    pub(crate) unsafe fn new(thunk: *mut Value) -> Self {
        if !thunk.is_null() {
            clorus_retain(thunk);
        }
        Self {
            thunk: Mutex::new(thunk),
            state: Mutex::new(State::Unforced),
        }
    }

    /// Force once. Recursive forcing is represented as a language exception
    /// by the eventual public wrapper; this internal layer reports it without
    /// blocking so the wrapper can choose that representation.
    pub(crate) unsafe fn force(&self) -> Result<*mut Value, ()> {
        {
            let state = self.state.lock().expect("lazy sequence state poisoned");
            match *state {
                State::Realized(value) => {
                    if !value.is_null() {
                        clorus_retain(value);
                    }
                    return Ok(value);
                }
                State::Forcing => return Err(()),
                State::Unforced => {}
            }
        }

        {
            let mut state = self.state.lock().expect("lazy sequence state poisoned");
            match *state {
                State::Unforced => *state = State::Forcing,
                State::Realized(value) => {
                    if !value.is_null() {
                        clorus_retain(value);
                    }
                    return Ok(value);
                }
                State::Forcing => return Err(()),
            }
        }

        let thunk = *self.thunk.lock().expect("lazy sequence thunk poisoned");
        let result = clorus_function_call(thunk, std::ptr::null(), 0);

        let mut state = self.state.lock().expect("lazy sequence state poisoned");
        if clorus_is_exception(result) {
            *state = State::Unforced;
            return Ok(result);
        }

        if !result.is_null() {
            clorus_retain(result);
        }
        *state = State::Realized(result);

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

#[cfg(test)]
mod tests {
    use super::LazySeqCell;
    use crate::value::clorus_release;

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
}
