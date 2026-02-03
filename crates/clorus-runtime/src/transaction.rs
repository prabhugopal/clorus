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
    /// Commutative updates that can be applied at commit time
    commutes: HashMap<RefId, Vec<CommuteFn>>,

    /// Transaction start time (for debugging/metrics)
    start_time: Instant,

    /// Nesting depth (for nested dosync support)
    depth: u32,
}

/// Commutative function to be applied at commit time
struct CommuteFn {
    /// Function to apply
    func: *mut Value,
    /// Arguments to pass to function
    args: Vec<*mut Value>,
}

impl Transaction {
    /// Create a new transaction
    fn new() -> Self {
        Transaction {
            id: TX_COUNTER.fetch_add(1, Ordering::SeqCst),
            reads: HashMap::new(),
            writes: HashMap::new(),
            commutes: HashMap::new(),
            start_time: Instant::now(),
            depth: 1,
        }
    }

    /// Record a read from a ref
    pub fn record_read(&mut self, ref_id: RefId, version: u64) {
        // Only record if not already in write set
        // (writes subsume reads)
        if !self.writes.contains_key(&ref_id) {
            self.reads.insert(ref_id, version);
        }
    }

    /// Stage a write to a ref
    pub fn stage_write(&mut self, ref_id: RefId, value: *mut Value) {
        // Retain the value
        unsafe {
            (*value).header().retain();
        }

        // Remove from read set if present (write subsumes read)
        self.reads.remove(&ref_id);

        // Release old write if replacing
        if let Some(old_val) = self.writes.insert(ref_id, value) {
            unsafe {
                crate::value::clorus_release(old_val);
            }
        }
    }

    /// Get staged write for a ref (if any)
    pub fn get_write(&self, ref_id: RefId) -> Option<*mut Value> {
        self.writes.get(&ref_id).copied()
    }

    /// Check if we have a staged write for this ref
    pub fn has_write(&self, ref_id: RefId) -> bool {
        self.writes.contains_key(&ref_id)
    }

    /// Get the read set for validation
    pub fn read_set(&self) -> &HashMap<RefId, u64> {
        &self.reads
    }

    /// Get the write set for commit
    pub fn write_set(&self) -> &HashMap<RefId, *mut Value> {
        &self.writes
    }

    /// Clean up transaction resources
    fn cleanup(&mut self) {
        // Release all staged writes
        for (_ref_id, value) in self.writes.drain() {
            unsafe {
                crate::value::clorus_release(value);
            }
        }

        // Release commute function arguments
        for (_ref_id, commutes) in self.commutes.drain() {
            for commute in commutes {
                unsafe {
                    crate::value::clorus_release(commute.func);
                    for arg in commute.args {
                        crate::value::clorus_release(arg);
                    }
                }
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

/// Get staged write for a ref (returns None if not in transaction or no write)
pub fn tx_get_write(ref_id: RefId) -> Option<*mut Value> {
    TX_CONTEXT.with(|ctx| {
        ctx.borrow()
            .as_ref()
            .and_then(|tx| tx.get_write(ref_id))
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

    // Collect all refs involved (reads + writes)
    let mut ref_ids: Vec<RefId> = tx
        .reads
        .keys()
        .chain(tx.writes.keys())
        .copied()
        .collect();

    // Sort to prevent deadlock
    ref_ids.sort();
    ref_ids.dedup();

    // For now, use a simple validation approach
    // TODO: Proper locking and validation in Phase 3

    // Validate all reads - check versions haven't changed
    for (ref_id, expected_version) in tx.read_set() {
        // RefId is the address of the Arc's inner Mutex<RefValue>
        let mutex_ptr = *ref_id as *const Mutex<RefValue>;
        unsafe {
            let guard = (*mutex_ptr).lock().unwrap();
            let current_version = guard.version;
            if current_version != *expected_version {
                // Conflict detected - abort
                return false;
            }
        }
    }

    // Apply all writes
    for (ref_id, new_value) in tx.write_set() {
        // RefId is the address of the Arc's inner Mutex<RefValue>
        let mutex_ptr = *ref_id as *const Mutex<RefValue>;
        unsafe {
            let mut guard = (*mutex_ptr).lock().unwrap();

            // Retain new value
            (&**new_value).header().retain();

            // Replace value and increment version
            let old_value = guard.value;
            guard.value = *new_value;
            guard.version += 1;

            // Release old value
            crate::value::clorus_release(old_value);
        }
    }

    // Success!
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
