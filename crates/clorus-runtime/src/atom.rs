/// Atom implementation for Clorus
/// Provides thread-safe mutable references with atomic updates

use crate::value::{Value, ValueTag};
use std::sync::atomic::{AtomicPtr, Ordering};

/// Atom - A mutable reference that can be updated atomically
/// Similar to Clojure's atom, provides thread-safe mutation
pub struct ClorusAtom {
    /// Current value (atomic pointer for lock-free updates)
    current: AtomicPtr<Value>,
}

impl ClorusAtom {
    /// Create a new atom with the given initial value
    pub unsafe fn new(initial: *mut Value) -> *mut Self {
        // Retain the initial value
        if !initial.is_null() {
            crate::value::clorus_retain(initial);
        }

        Box::into_raw(Box::new(ClorusAtom {
            current: AtomicPtr::new(initial),
        }))
    }

    /// Read the current value (deref)
    pub unsafe fn deref(&self) -> *mut Value {
        let val = self.current.load(Ordering::Acquire);
        // Retain before returning to ensure it stays alive
        if !val.is_null() {
            crate::value::clorus_retain(val);
        }
        val
    }

    /// Set the atom to a new value (reset!)
    pub unsafe fn reset(&self, new_val: *mut Value) -> *mut Value {
        // Retain the new value
        if !new_val.is_null() {
            crate::value::clorus_retain(new_val);
        }

        // Swap in the new value
        let old_val = self.current.swap(new_val, Ordering::AcqRel);

        // Release the old value
        if !old_val.is_null() {
            crate::value::clorus_release(old_val);
        }

        // Return the new value (already retained)
        new_val
    }

    /// Update the atom by applying a function (swap!)
    pub unsafe fn swap(&self, func_ptr: *mut u8, arg: *mut Value) -> *mut Value {
        type SwapFn = extern "C" fn(*mut Value, *mut Value) -> *mut Value;
        let func: SwapFn = std::mem::transmute(func_ptr);

        loop {
            let current_val = self.current.load(Ordering::Acquire);
            let new_val = func(current_val, arg);

            if self.compare_and_set(current_val, new_val) {
                return new_val;
            }
        }
    }

    /// Compare and set (CAS operation)
    pub unsafe fn compare_and_set(&self, old: *mut Value, new: *mut Value) -> bool {
        if !new.is_null() {
            crate::value::clorus_retain(new);
        }

        let result = self.current.compare_exchange(
            old,
            new,
            Ordering::AcqRel,
            Ordering::Acquire
        );

        match result {
            Ok(actual_old) => {
                // Success - release the old value
                if !actual_old.is_null() {
                    crate::value::clorus_release(actual_old);
                }
                true
            }
            Err(_) => {
                // Failed - release the new value we didn't use
                if !new.is_null() {
                    crate::value::clorus_release(new);
                }
                false
            }
        }
    }
}

/// Release an atom and its contents
pub unsafe fn release_atom(atom: *mut ClorusAtom) {
    if atom.is_null() {
        return;
    }

    // Release the current value
    let current = (*atom).current.load(Ordering::Acquire);
    if !current.is_null() {
        crate::value::clorus_release(current);
    }

    // Drop the atom itself
    drop(Box::from_raw(atom));
}

// ========================================
// FFI exports for LLVM
// ========================================

/// Create a new atom
#[no_mangle]
pub extern "C" fn clorus_atom(initial: *mut Value) -> *mut Value {
    unsafe {
        let atom_ptr = ClorusAtom::new(initial);
        Value::from_ptr(ValueTag::Atom, atom_ptr as *mut u8)
    }
}

/// Dereference an atom - read its current value
#[no_mangle]
pub extern "C" fn clorus_deref(atom_val: *mut Value) -> *mut Value {
    if atom_val.is_null() {
        return Value::nil();
    }

    unsafe {
        let tag = (*atom_val).header().tag();

        // Handle atoms
        if tag == ValueTag::Atom {
            let atom_ptr = (*atom_val).as_ptr() as *mut ClorusAtom;
            return (*atom_ptr).deref();
        }

        // Handle refs
        if tag == ValueTag::Ref {
            return crate::ref_type::clorus_ref_deref(atom_val);
        }

        // Handle agents
        if tag == ValueTag::Agent {
            return crate::agent::clorus_agent_deref(atom_val);
        }

        // Not a deref-able type
        Value::nil()
    }
}

/// Reset an atom to a new value
#[no_mangle]
pub extern "C" fn clorus_reset(atom_val: *mut Value, new_val: *mut Value) -> *mut Value {
    if atom_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*atom_val).header().tag() != ValueTag::Atom {
            return Value::nil();
        }

        let atom_ptr = (*atom_val).as_ptr() as *mut ClorusAtom;
        (*atom_ptr).reset(new_val)
    }
}

/// Swap an atom - update by applying a function
/// For now, simplified version without retry loop
#[no_mangle]
pub extern "C" fn clorus_swap(
    atom_val: *mut Value,
    func_ptr: *mut u8,
    arg: *mut Value
) -> *mut Value {
    if atom_val.is_null() || func_ptr.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*atom_val).header().tag() != ValueTag::Atom {
            return Value::nil();
        }

        let atom_ptr = (*atom_val).as_ptr() as *mut ClorusAtom;
        (*atom_ptr).swap(func_ptr, arg)
    }
}

/// Compare and set
#[no_mangle]
pub extern "C" fn clorus_compare_and_set(
    atom_val: *mut Value,
    old_val: *mut Value,
    new_val: *mut Value
) -> *mut Value {
    if atom_val.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        if (*atom_val).header().tag() != ValueTag::Atom {
            return Value::boolean(false);
        }

        let atom_ptr = (*atom_val).as_ptr() as *mut ClorusAtom;
        let success = (*atom_ptr).compare_and_set(old_val, new_val);
        Value::boolean(success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_create_and_deref() {
        let val = Value::double(42.0);
        let atom = clorus_atom(val);

        unsafe {
            assert_eq!((*atom).header().tag(), ValueTag::Atom);

            let derefed = clorus_deref(atom);
            assert_eq!((*derefed).as_double(), 42.0);

            crate::value::clorus_release(atom);
            crate::value::clorus_release(val);
            crate::value::clorus_release(derefed);
        }
    }

    #[test]
    fn test_atom_reset() {
        let val1 = Value::double(10.0);
        let val2 = Value::double(20.0);
        let atom = clorus_atom(val1);

        let result = clorus_reset(atom, val2);
        unsafe {
            assert_eq!((*result).as_double(), 20.0);

            let derefed = clorus_deref(atom);
            assert_eq!((*derefed).as_double(), 20.0);

            crate::value::clorus_release(atom);
            crate::value::clorus_release(val1);
            crate::value::clorus_release(val2);
            crate::value::clorus_release(result);
            crate::value::clorus_release(derefed);
        }
    }
}
