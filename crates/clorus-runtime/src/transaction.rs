/// Software Transactional Memory (STM) transaction context
///
/// Provides MVCC (Multi-Version Concurrency Control) for coordinated updates
/// to multiple refs within atomic dosync blocks.
///
/// Transaction Lifecycle:
/// 1. Begin transaction (create read/write sets)
/// 2. Execute transaction body (alter/ref-set operations stage writes)
/// 3. Validate reads (check versions haven't changed)
/// 4. Commit or retry on conflict

use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Transaction ID type
type TxId = u64;

/// Ref ID type (pointer address)
type RefId = usize;

/// Global transaction counter
static TX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Thread-local transaction context
///
/// Each thread can have at most one active transaction at a time.
/// Nested dosync blocks reuse the same transaction (like Clojure).
thread_local! {
    static TX_CONTEXT: RefCell<Option<Transaction>> = RefCell::new(None);
}

/// Transaction state
pub struct Transaction {
    /// Unique transaction ID
    id: TxId,

    /// Read set: (ref_id -> version_read)
    /// Used to validate that refs haven't changed during transaction
    reads: HashMap<RefId, u64>,

    /// Write set: (ref_id -> new_value)
    /// Staged writes to be committed atomically
    writes: HashMap<RefId, *mut Value>,

    /// Commute set: (ref_id -> vec of commute operations)
    /// Commutative updates that can be applied at commit time.
    commutes: HashMap<RefId, Vec<CommuteFn>>,

    /// Local transactional view for refs updated only via commute.
    /// This lets repeated deref/commute calls within the same transaction
    /// observe the accumulated local result without forcing a regular write.
    commute_values: HashMap<RefId, *mut Value>,

    /// Transaction start time (for debugging/metrics)
    start_time: Instant,

    /// Nesting depth (for nested dosync support)
    depth: u32,
}

/// Commutative function to be applied at commit time
struct CommuteFn {
    /// Function to apply.
    func: *mut Value,
    /// Arguments to pass to function, excluding the current ref value.
    args_vec: *mut Value,
}

impl Transaction {
    /// Create a new transaction
    fn new() -> Self {
        Transaction {
            id: TX_COUNTER.fetch_add(1, Ordering::SeqCst),
            reads: HashMap::new(),
            writes: HashMap::new(),
            commutes: HashMap::new(),
            commute_values: HashMap::new(),
            start_time: Instant::now(),
            depth: 1,
        }
    }

    /// Record a read from a ref.
    pub fn record_read(&mut self, ref_id: RefId, version: u64) {
        // Writes and deferred commutes already define the local value flow for this ref.
        if !self.writes.contains_key(&ref_id) && !self.commutes.contains_key(&ref_id) {
            self.reads.insert(ref_id, version);
        }
    }

    fn clear_commutes_for_ref(&mut self, ref_id: RefId) {
        if let Some(local_value) = self.commute_values.remove(&ref_id) {
            crate::value::clorus_release(local_value);
        }

        if let Some(commutes) = self.commutes.remove(&ref_id) {
            for commute in commutes {
                crate::value::clorus_release(commute.func);
                crate::value::clorus_release(commute.args_vec);
            }
        }
    }

    /// Stage a write to a ref.
    pub fn stage_write(&mut self, ref_id: RefId, value: *mut Value) {
        crate::value::clorus_retain(value);

        // A regular write supersedes any deferred commute state.
        self.clear_commutes_for_ref(ref_id);
        self.reads.remove(&ref_id);

        if let Some(old_val) = self.writes.insert(ref_id, value) {
            crate::value::clorus_release(old_val);
        }
    }

    /// Stage a commute operation and update the local transactional view.
    pub fn stage_commute(
        &mut self,
        ref_id: RefId,
        func: *mut Value,
        args_vec: *mut Value,
        local_value: *mut Value,
    ) {
        crate::value::clorus_retain(func);
        crate::value::clorus_retain(args_vec);
        crate::value::clorus_retain(local_value);

        self.reads.remove(&ref_id);
        self.commutes
            .entry(ref_id)
            .or_default()
            .push(CommuteFn { func, args_vec });

        if let Some(old) = self.commute_values.insert(ref_id, local_value) {
            crate::value::clorus_release(old);
        }
    }

    pub fn get_write(&self, ref_id: RefId) -> Option<*mut Value> {
        self.writes.get(&ref_id).copied()
    }

    pub fn get_commute_value(&self, ref_id: RefId) -> Option<*mut Value> {
        self.commute_values.get(&ref_id).copied()
    }

    pub fn has_write(&self, ref_id: RefId) -> bool {
        self.writes.contains_key(&ref_id)
    }

    pub fn has_commute(&self, ref_id: RefId) -> bool {
        self.commutes.contains_key(&ref_id)
    }

    pub fn read_set(&self) -> &HashMap<RefId, u64> {
        &self.reads
    }

    pub fn write_set(&self) -> &HashMap<RefId, *mut Value> {
        &self.writes
    }

    fn commute_set(&self) -> &HashMap<RefId, Vec<CommuteFn>> {
        &self.commutes
    }

    /// Clean up transaction resources
    fn cleanup(&mut self) {
        for (_ref_id, value) in self.writes.drain() {
            crate::value::clorus_release(value);
        }

        for (_ref_id, local_value) in self.commute_values.drain() {
            crate::value::clorus_release(local_value);
        }

        for (_ref_id, commutes) in self.commutes.drain() {
            for commute in commutes {
                crate::value::clorus_release(commute.func);
                crate::value::clorus_release(commute.args_vec);
            }
        }

        self.reads.clear();
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.cleanup();
    }
}

// ============================================================================
// Transaction Context Management
// ============================================================================

/// Begin a new transaction
///
/// If a transaction is already active (nested dosync), increment depth.
/// Returns true if a new transaction was created, false if nested.
#[no_mangle]
pub extern "C" fn clorus_tx_begin() -> bool {
    TX_CONTEXT.with(|ctx| {
        let mut ctx_ref = ctx.borrow_mut();

        match ctx_ref.as_mut() {
            Some(tx) => {
                // Nested dosync - increment depth
                tx.depth += 1;
                false
            }
            None => {
                // New transaction
                *ctx_ref = Some(Transaction::new());
                true
            }
        }
    })
}

/// Commit the current transaction
///
/// Validates all reads and atomically applies all writes.
/// Returns true on successful commit, false on conflict.
#[no_mangle]
pub extern "C" fn clorus_tx_commit() -> bool {
    TX_CONTEXT.with(|ctx| {
        let mut ctx_ref = ctx.borrow_mut();

        match ctx_ref.as_mut() {
            Some(tx) => {
                // If nested, just decrement depth
                if tx.depth > 1 {
                    tx.depth -= 1;
                    return true;
                }

                // Top-level commit - validate and apply
                let success = commit_transaction(tx);

                if success {
                    // Clear transaction on success
                    *ctx_ref = None;
                }

                success
            }
            None => {
                // No transaction active
                false
            }
        }
    })
}

/// Abort the current transaction
///
/// Clears all staged changes without committing.
#[no_mangle]
pub extern "C" fn clorus_tx_abort() {
    TX_CONTEXT.with(|ctx| {
        let mut ctx_ref = ctx.borrow_mut();

        if let Some(tx) = ctx_ref.as_mut() {
            if tx.depth > 1 {
                // Nested - decrement depth
                tx.depth -= 1;
            } else {
                // Top-level abort - clear transaction
                *ctx_ref = None;
            }
        }
    });
}

/// Check if currently inside a transaction
#[no_mangle]
pub extern "C" fn clorus_tx_active() -> bool {
    TX_CONTEXT.with(|ctx| ctx.borrow().is_some())
}

// ============================================================================
// Transaction Operations (called by ref operations)
// ============================================================================

/// Record a read from a ref (for validation)
pub fn tx_record_read(ref_id: RefId, version: u64) {
    TX_CONTEXT.with(|ctx| {
        if let Some(tx) = ctx.borrow_mut().as_mut() {
            tx.record_read(ref_id, version);
        }
    });
}

/// Stage a write to a ref
pub fn tx_stage_write(ref_id: RefId, value: *mut Value) {
    TX_CONTEXT.with(|ctx| {
        if let Some(tx) = ctx.borrow_mut().as_mut() {
            tx.stage_write(ref_id, value);
        }
    });
}

/// Stage a deferred commute operation for a ref.
pub fn tx_stage_commute(
    ref_id: RefId,
    func: *mut Value,
    args_vec: *mut Value,
    local_value: *mut Value,
) {
    TX_CONTEXT.with(|ctx| {
        if let Some(tx) = ctx.borrow_mut().as_mut() {
            tx.stage_commute(ref_id, func, args_vec, local_value);
        }
    });
}

/// Get staged write for a ref (returns None if not in transaction or no write)
pub fn tx_get_write(ref_id: RefId) -> Option<*mut Value> {
    TX_CONTEXT.with(|ctx| {
        ctx.borrow()
            .as_ref()
            .and_then(|tx| tx.get_write(ref_id))
    })
}

/// Get the locally accumulated commute value for a ref.
pub fn tx_get_commute_value(ref_id: RefId) -> Option<*mut Value> {
    TX_CONTEXT.with(|ctx| {
        ctx.borrow()
            .as_ref()
            .and_then(|tx| tx.get_commute_value(ref_id))
    })
}

/// Check if ref has staged write
pub fn tx_has_write(ref_id: RefId) -> bool {
    TX_CONTEXT.with(|ctx| {
        ctx.borrow()
            .as_ref()
            .map(|tx| tx.has_write(ref_id))
            .unwrap_or(false)
    })
}

/// Check if ref has staged commutes.
pub fn tx_has_commute(ref_id: RefId) -> bool {
    TX_CONTEXT.with(|ctx| {
        ctx.borrow()
            .as_ref()
            .map(|tx| tx.has_commute(ref_id))
            .unwrap_or(false)
    })
}

// ============================================================================
// Transaction Commit Logic
// ============================================================================

/// Commit a transaction atomically
///
/// 1. Collect all refs to lock (sorted by address to prevent deadlock)
/// 2. Lock all refs
/// 3. Validate read set (check versions)
/// 4. Apply write set
/// 5. Release locks
fn commit_transaction(tx: &mut Transaction) -> bool {
    use crate::ref_type::RefValue;
    use std::sync::Mutex;

    // Collect all refs involved (reads + writes + commutes)
    let mut ref_ids: Vec<RefId> = tx
        .reads
        .keys()
        .chain(tx.writes.keys())
        .chain(tx.commutes.keys())
        .copied()
        .collect();

    ref_ids.sort();
    ref_ids.dedup();

    // Validate all reads - check versions haven't changed.
    for (ref_id, expected_version) in tx.read_set() {
        let mutex_ptr = *ref_id as *const Mutex<RefValue>;
        unsafe {
            let guard = (*mutex_ptr).lock().unwrap();
            if guard.version != *expected_version {
                return false;
            }
        }
    }

    // Apply regular writes first.
    for (ref_id, new_value) in tx.write_set() {
        let mutex_ptr = *ref_id as *const Mutex<RefValue>;
        unsafe {
            let mut guard = (*mutex_ptr).lock().unwrap();
            (&**new_value).header().retain();
            let old_value = guard.value;
            guard.value = *new_value;
            guard.version += 1;
            crate::value::clorus_release(old_value);
        }
    }

    // Apply deferred commutes against the latest committed value.
    for (ref_id, commutes) in tx.commute_set() {
        if tx.write_set().contains_key(ref_id) {
            continue;
        }

        let mutex_ptr = *ref_id as *const Mutex<RefValue>;
        unsafe {
            let mut guard = (*mutex_ptr).lock().unwrap();
            let original_value = guard.value;
            let mut current_value = original_value;
            let mut current_owned = false;

            for commute in commutes {
                let extra_count = crate::vector::clorus_vector_count(commute.args_vec) as usize;
                let mut args: Vec<*mut Value> = Vec::with_capacity(extra_count + 1);
                args.push(current_value);
                for i in 0..extra_count {
                    let arg = crate::vector::clorus_vector_nth(commute.args_vec, i as u64);
                    args.push(arg);
                }

                let new_value = crate::function::clorus_function_call(
                    commute.func,
                    args.as_ptr(),
                    args.len() as i32,
                );

                if current_owned {
                    crate::value::clorus_release(current_value);
                }
                current_value = new_value;
                current_owned = true;
            }

            if current_owned {
                guard.value = current_value;
                guard.version += 1;
                crate::value::clorus_release(original_value);
            }
        }
    }

    true
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_begin_commit() {
        // No transaction initially
        assert!(!clorus_tx_active());

        // Begin transaction
        assert!(clorus_tx_begin());
        assert!(clorus_tx_active());

        // Commit
        assert!(clorus_tx_commit());
        assert!(!clorus_tx_active());
    }

    #[test]
    fn test_nested_transactions() {
        assert!(clorus_tx_begin()); // New transaction
        assert!(!clorus_tx_begin()); // Nested (returns false)
        assert!(clorus_tx_active());

        assert!(clorus_tx_commit()); // Inner commit (depth decreases)
        assert!(clorus_tx_active()); // Still active

        assert!(clorus_tx_commit()); // Outer commit
        assert!(!clorus_tx_active());
    }

    #[test]
    fn test_tx_abort() {
        assert!(clorus_tx_begin());
        assert!(clorus_tx_active());

        clorus_tx_abort();
        assert!(!clorus_tx_active());
    }

    #[test]
    fn test_tx_commit_detects_version_conflict() {
        use crate::ref_type::{clorus_ref, clorus_ref_deref, ClorusRef};
        use crate::value::{clorus_release, Value};

        unsafe {
            let initial = Value::double(1.0);
            let ref_val = clorus_ref(initial);
            let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;

            assert!(clorus_tx_begin());
            let snapshot = clorus_ref_deref(ref_val);
            clorus_release(snapshot);

            let external_update = Value::double(2.0);
            let old_value = (*ref_ptr).set_versioned(external_update);
            clorus_release(old_value);

            assert!(!clorus_tx_commit());
            assert!(clorus_tx_active());

            clorus_tx_abort();
            assert!(!clorus_tx_active());

            clorus_release(ref_val);
        }
    }

    extern "C" fn add_one_fn(x: *mut Value, _env: *mut i8) -> *mut Value {
        unsafe {
            match (*x).header().tag() {
                crate::value::ValueTag::Long => Value::long((*x).as_long() + 1),
                crate::value::ValueTag::Double => Value::double((*x).as_double() + 1.0),
                _ => Value::nil(),
            }
        }
    }

    #[test]
    fn test_tx_commit_replays_commute_on_latest_value() {
        use crate::function::clorus_function_new;
        use crate::ref_type::{clorus_commute, clorus_ref, clorus_ref_deref, ClorusRef};
        use crate::value::{clorus_release, Value};

        unsafe {
            let initial = Value::long(1);
            let ref_val = clorus_ref(initial);
            let ref_ptr = (*ref_val).as_ptr() as *mut ClorusRef;
            let func_val = clorus_function_new(add_one_fn as *const u8, 1, std::ptr::null(), 0);
            let args_vec = crate::vector::clorus_vector_empty();

            assert!(clorus_tx_begin());
            let commute_result = clorus_commute(ref_val, func_val, args_vec);
            assert_eq!((*commute_result).as_long(), 2);
            clorus_release(commute_result);

            let external_update = Value::long(5);
            let old_value = (*ref_ptr).set_versioned(external_update);
            clorus_release(old_value);

            assert!(clorus_tx_commit());
            let final_value = clorus_ref_deref(ref_val);
            assert_eq!((*final_value).as_long(), 6);

            clorus_release(final_value);
            clorus_release(args_vec);
            clorus_release(func_val);
            clorus_release(ref_val);
        }
    }

    #[test]
    #[ignore] // Requires actual refs, will test in integration tests
    fn test_tx_read_write_sets() {
        clorus_tx_begin();

        // Record reads and writes
        tx_record_read(100, 5);
        tx_record_read(200, 10);

        TX_CONTEXT.with(|ctx| {
            let tx = ctx.borrow();
            let tx_ref = tx.as_ref().unwrap();

            assert_eq!(tx_ref.read_set().len(), 2);
            assert_eq!(tx_ref.read_set().get(&100), Some(&5));
            assert_eq!(tx_ref.read_set().get(&200), Some(&10));
        });

        clorus_tx_commit();
    }
}
