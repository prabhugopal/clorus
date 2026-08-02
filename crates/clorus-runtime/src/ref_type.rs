/// Transactional references (refs) for coordinated state management
///
/// Refs enable Software Transactional Memory (STM) - coordinated, synchronous
/// updates to multiple values within atomic transactions.
///
/// Unlike atoms (single-value updates), refs allow multiple values to be
/// updated atomically within a dosync transaction block.
///
/// Example:
/// ```clojure
/// (def account-a (ref 1000))
/// (def account-b (ref 500))
///
/// (dosync
///   (alter account-a - 100)
///   (alter account-b + 100))
/// ```

use crate::value::{Value, ValueTag};
use std::sync::{Arc, Mutex};

/// A transactional reference
///
/// Refs use MVCC (Multi-Version Concurrency Control) for optimistic concurrency:
/// - Each ref has a version counter
/// - Reads create snapshots
/// - Writes create new versions
/// - Commits validate that read versions haven't changed
pub struct ClorusRef {
    /// Current value with version tracking
    value: Arc<Mutex<RefValue>>,
}

/// Internal ref value with versioning
pub struct RefValue {
    /// Current value pointer
    pub value: *mut Value,
    /// Version number (incremented on each write)
    pub version: u64,
}

impl ClorusRef {
    /// Create a new ref with initial value
    pub fn new(initial: *mut Value) -> Self {
        unsafe {
            // Retain the initial value
            (*initial).header().retain();
        }

        ClorusRef {
            value: Arc::new(Mutex::new(RefValue {
                value: initial,
                version: 0,
            })),
        }
    }

    /// Dereference the ref - read current value
    ///
    /// This is a non-transactional read. For transactional reads,
    /// use within a dosync block.
    pub fn deref(&self) -> *mut Value {
        let guard = self.value.lock().unwrap();
        let val = guard.value;

        unsafe {
            // Retain the value before returning
            (*val).header().retain();
        }

        val
    }

    /// Get current version (for transaction validation)
    pub fn version(&self) -> u64 {
        let guard = self.value.lock().unwrap();
        guard.version
    }

    /// Set value and increment version (called during transaction commit)
    ///
    /// Returns the old value for cleanup
    pub fn set_versioned(&self, new_value: *mut Value) -> *mut Value {
        let mut guard = self.value.lock().unwrap();

        unsafe {
            // Retain new value
            (*new_value).header().retain();
        }

        let old_value = guard.value;
        guard.value = new_value;
        guard.version += 1;

        old_value
    }

    /// Get ref ID for transaction tracking
    pub fn id(&self) -> usize {
        Arc::as_ptr(&self.value) as usize
    }
}

fn current_value_without_tracking(ref_ptr: *mut ClorusRef) -> *mut Value {
    unsafe {
        let guard = (*ref_ptr).value.lock().unwrap();
        let value = guard.value;
        crate::value::clorus_retain(value);
        value
    }
}

unsafe fn apply_commute_function(
    base_value: *mut Value,
    func_val: *mut Value,
    args_vec: *mut Value,
) -> *mut Value {
    let extra_count = crate::vector::clorus_vector_count(args_vec) as usize;
    let mut args: Vec<*mut Value> = Vec::with_capacity(extra_count + 1);
    args.push(base_value);
    for i in 0..extra_count {
        let arg = crate::vector::clorus_vector_nth(args_vec, i as u64);
        args.push(arg);
    }
    crate::function::clorus_function_call(func_val, args.as_ptr(), args.len() as i32)
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new ref with initial value
///
/// (ref initial-value) => Ref
#[no_mangle]
pub extern "C" fn clorus_ref(initial: *mut Value) -> *mut Value {
    if initial.is_null() {
        return Value::nil();
    }

    let ref_obj = Box::new(ClorusRef::new(initial));
    Value::from_ptr(ValueTag::Ref, Box::into_raw(ref_obj) as *mut u8)
}

/// Dereference a ref - read current value
///
/// @ref or (deref ref) => value
#[no_mangle]
pub extern "C" fn clorus_ref_deref(ref_val: *mut Value) -> *mut Value {
    if ref_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*ref_val).header().tag() != ValueTag::Ref {
            return Value::nil();
        }

        let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
        let ref_id = (*ref_ptr).id();

        if let Some(staged_value) = crate::transaction::tx_get_write(ref_id) {
            crate::value::clorus_retain(staged_value);
            return staged_value;
        }

        if let Some(commute_value) = crate::transaction::tx_get_commute_value(ref_id) {
            crate::value::clorus_retain(commute_value);
            return commute_value;
        }

        let version = (*ref_ptr).version();
        let value = (*ref_ptr).deref();
        crate::transaction::tx_record_read(ref_id, version);
        value
    }
}

/// Set ref value directly within a transaction
///
/// (ref-set ref value) => value
/// Must be called within a dosync block
#[no_mangle]
pub extern "C" fn clorus_ref_set(
    ref_val: *mut Value,
    new_value: *mut Value,
) -> *mut Value {
    if ref_val.is_null() || new_value.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*ref_val).header().tag() != ValueTag::Ref {
            // Not a ref - return nil
            // TODO: Better error handling
            return Value::nil();
        }

        // Check if we're in a transaction
        if !crate::transaction::clorus_tx_active() {
            // Error: ref-set must be in dosync
            // For now, return nil
            // TODO: Proper error handling
            return Value::nil();
        }

        let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
        let ref_id = (*ref_ptr).id();

        crate::transaction::tx_stage_write(ref_id, new_value);

        crate::value::clorus_retain(new_value);
        new_value
    }
}

/// Alter ref by applying a function within a transaction
///
/// (alter ref func & args) => new-value
/// Must be called within a dosync block
///
/// Note: func and args must be pre-applied by the caller.
/// This function receives the result of applying func to the current value.
#[no_mangle]
pub extern "C" fn clorus_alter(
    ref_val: *mut Value,
    new_value: *mut Value,
) -> *mut Value {
    // For now, alter works the same as ref-set
    // The function application happens in the codegen layer
    clorus_ref_set(ref_val, new_value)
}

/// Commute ref by applying a commutative function within a transaction.
///
/// (commute ref func & args) => new-value
/// Must be called within a dosync block.
///
/// The function and extra arguments are recorded in the transaction and replayed
/// at commit time against the latest committed value for the ref.
#[no_mangle]
pub extern "C" fn clorus_commute(
    ref_val: *mut Value,
    func_val: *mut Value,
    args_vec: *mut Value,
) -> *mut Value {
    if ref_val.is_null() || func_val.is_null() || args_vec.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*ref_val).header().tag() != ValueTag::Ref {
            return Value::nil();
        }

        if !crate::transaction::clorus_tx_active() {
            return Value::nil();
        }

        let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
        let ref_id = (*ref_ptr).id();

        let (base_value, release_base) = if let Some(staged_value) = crate::transaction::tx_get_write(ref_id) {
            (staged_value, false)
        } else if let Some(commute_value) = crate::transaction::tx_get_commute_value(ref_id) {
            (commute_value, false)
        } else {
            (current_value_without_tracking(ref_ptr), true)
        };

        let new_value = apply_commute_function(base_value, func_val, args_vec);

        if release_base {
            crate::value::clorus_release(base_value);
        }

        if crate::transaction::tx_has_write(ref_id) {
            crate::transaction::tx_stage_write(ref_id, new_value);
        } else {
            crate::transaction::tx_stage_commute(ref_id, func_val, args_vec, new_value);
        }

        crate::value::clorus_retain(new_value);
        new_value
    }
}

/// Ensure ref is protected in transaction
///
/// (ensure ref) => ref-value
/// Must be called within a dosync block
///
/// Ensures that a ref is part of the transaction's read set, even if
/// it's not being modified. This prevents write skew anomalies.
#[no_mangle]
pub extern "C" fn clorus_ensure(ref_val: *mut Value) -> *mut Value {
    if ref_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*ref_val).header().tag() != ValueTag::Ref {
            return Value::nil();
        }

        // Check if we're in a transaction
        if !crate::transaction::clorus_tx_active() {
            // ensure must be in dosync
            return Value::nil();
        }

        let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
        let ref_id = (*ref_ptr).id();

        // If not already in write set, record a read
        if !crate::transaction::tx_has_write(ref_id) {
            let version = (*ref_ptr).version();
            crate::transaction::tx_record_read(ref_id, version);
        }

        // Return the current value
        clorus_ref_deref(ref_val)
    }
}

// ============================================================================
// Cleanup
// ============================================================================

impl Drop for ClorusRef {
    fn drop(&mut self) {
        unsafe {
            let guard = self.value.lock().unwrap();
            let val = guard.value;

            if !val.is_null() {
                // Release the stored value
                crate::value::clorus_release(val);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_create_and_deref() {
        unsafe {
            let val = Value::double(42.0);
            let ref_val = clorus_ref(val);

            assert!(!ref_val.is_null());
            assert_eq!((*ref_val).header().tag(), ValueTag::Ref);

            let deref_val = clorus_ref_deref(ref_val);
            assert_eq!((*deref_val).as_double(), 42.0);

            crate::value::clorus_release(deref_val);
            crate::value::clorus_release(ref_val);
        }
    }

    #[test]
    fn test_ref_version() {
        unsafe {
            let val = Value::double(10.0);
            let ref_val = clorus_ref(val);
            let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;

            assert_eq!((*ref_ptr).version(), 0);

            // Update value (simulating transaction commit)
            let new_val = Value::double(20.0);
            let old_val = (*ref_ptr).set_versioned(new_val);

            assert_eq!((*ref_ptr).version(), 1);

            crate::value::clorus_release(old_val);
            crate::value::clorus_release(ref_val);
        }
    }

    #[test]
    fn test_ref_deref_non_ref() {
        unsafe {
            let num = Value::double(99.0);
            let result = clorus_ref_deref(num);

            assert_eq!((*result).header().tag(), ValueTag::Nil);

            crate::value::clorus_release(result);
            crate::value::clorus_release(num);
        }
    }
}
