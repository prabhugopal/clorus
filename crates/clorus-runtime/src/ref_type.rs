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
        if (*ref_val).header().tag() == ValueTag::Ref {
            let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;

            // Check if we're in a transaction and have a staged write
            let ref_id = (*ref_ptr).id();
            if let Some(staged_value) = crate::transaction::tx_get_write(ref_id) {
                // Return staged write (retain before returning)
                (*staged_value).header().retain();
                return staged_value;
            }

            // No staged write - do normal deref and record read
            let version = (*ref_ptr).version();
            let value = (*ref_ptr).deref();

            // Record read in transaction (if active)
            crate::transaction::tx_record_read(ref_id, version);

            value
        } else {
            // Not a ref - return nil
            Value::nil()
        }
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

        // Stage the write in the transaction
        crate::transaction::tx_stage_write(ref_id, new_value);

        // Return the new value (retain before returning)
        (*new_value).header().retain();
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

/// Commute ref by applying a commutative function within a transaction
///
/// (commute ref func & args) => new-value
/// Must be called within a dosync block
///
/// Like alter, but marks the operation as commutative, allowing it to be
/// applied at commit time for better concurrency.
///
/// Note: For now, commute is implemented the same as alter.
/// True commutative semantics (deferred application) will be added later.
#[no_mangle]
pub extern "C" fn clorus_commute(
    ref_val: *mut Value,
    new_value: *mut Value,
) -> *mut Value {
    // For now, commute works the same as alter
    // TODO: Implement true commutative semantics (apply at commit time)
    clorus_ref_set(ref_val, new_value)
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
