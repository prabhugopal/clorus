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
    ///
    /// This is *only* about commit-time validation (has this ref changed
    /// since I first touched it?), which is orthogonal to what value
    /// `clorus_ref_deref` returns for computation within the transaction
    /// (that's handled separately via `tx_get_write`/`tx_get_commute_value`,
    /// checked before this is ever reached). Once a ref has been read, its
    /// observed version must stay recorded and validated at commit time even
    /// if the transaction later also writes to it (e.g. `alter`, which reads
    /// then stages a write) -- see `stage_write`.
    pub fn record_read(&mut self, ref_id: RefId, version: u64) {
        if !self.commutes.contains_key(&ref_id) {
            self.reads.entry(ref_id).or_insert(version);
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

        // A regular write supersedes any deferred commute state, but it must
        // NOT clear a prior read record: `alter` reads the ref's current
        // value (recording its version for validation) and then stages a
        // write computed from that value. If staging the write wiped the
        // read record, commit-time validation would have nothing left to
        // check for this ref -- silently disabling conflict detection for
        // the single most common STM pattern (read-then-write). The read
        // version and the staged write are validated/applied independently
        // at commit time; see commit_transaction.
        self.clear_commutes_for_ref(ref_id);

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
/// 2. Lock ALL of them up front and hold every lock for the entire critical
///    section below -- this is required for atomicity across multiple refs.
///    (Earlier versions of this function locked/validated/wrote one ref at a
///    time across three separate loops, releasing each lock before moving to
///    the next; that let another thread's transaction interleave between a
///    ref being validated and a different ref in the same transaction being
///    written, breaking the atomicity dosync is supposed to guarantee.)
/// 3. Validate read set (check versions) using the held locks
/// 4. Apply write set and commutes using the held locks
/// 5. Release all locks together when this function returns
fn commit_transaction(tx: &mut Transaction) -> bool {
    use crate::ref_type::RefValue;
    use std::sync::{Mutex, MutexGuard};

    // Collect all refs involved (reads + writes + commutes), sorted so every
    // transaction acquires locks in the same global order regardless of which
    // refs it touches -- this is what makes locking all of them upfront
    // deadlock-safe against other concurrent transactions.
    let mut ref_ids: Vec<RefId> = tx
        .reads
        .keys()
        .chain(tx.writes.keys())
        .chain(tx.commutes.keys())
        .copied()
        .collect();

    ref_ids.sort();
    ref_ids.dedup();

    let mut guards: HashMap<RefId, MutexGuard<'_, RefValue>> = HashMap::with_capacity(ref_ids.len());
    for ref_id in &ref_ids {
        let mutex_ptr = *ref_id as *const Mutex<RefValue>;
        let guard = unsafe { (*mutex_ptr).lock().unwrap() };
        guards.insert(*ref_id, guard);
    }

    // Validate all reads - check versions haven't changed since they were read.
    for (ref_id, expected_version) in tx.read_set() {
        let guard = guards.get(ref_id).expect("read ref missing its held lock");
        if guard.version != *expected_version {
            return false; // All guards drop here, releasing every lock together.
        }
    }

    // Apply regular writes.
    for (ref_id, new_value) in tx.write_set() {
        let guard = guards.get_mut(ref_id).expect("write ref missing its held lock");
        unsafe {
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

        let guard = guards.get_mut(ref_id).expect("commute ref missing its held lock");
        let original_value = guard.value;
        let mut current_value = original_value;
        let mut current_owned = false;

        for commute in commutes {
            unsafe {
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
        }

        if current_owned {
            guard.value = current_value;
            guard.version += 1;
            unsafe {
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

    /// Concurrent stress test for cross-ref atomicity: the classic "bank
    /// transfer between two refs" invariant check. Many threads repeatedly
    /// move a random amount from ref A to ref B inside a single transaction
    /// (both refs read, both refs written, one commit). If commits across
    /// multiple refs aren't truly atomic -- e.g. if each ref is locked and
    /// released independently instead of all refs being locked for the whole
    /// validate+apply critical section -- two transactions can interleave
    /// such that value is created or destroyed, and A + B will drift from
    /// its starting total under real contention. This exercises the exact
    /// bug fixed in commit_transaction (previously-unused `ref_ids` locking
    /// scaffolding was computed but never actually used to hold all locks).
    #[test]
    fn test_tx_concurrent_transfer_preserves_total() {
        use crate::ref_type::{clorus_ref, clorus_ref_deref, clorus_ref_set};
        use crate::value::{clorus_release, Value};
        use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};
        use std::sync::Arc;
        use std::thread;

        const STARTING_BALANCE: i64 = 500;
        const THREADS: usize = 8;
        const TRANSFERS_PER_THREAD: usize = 300;

        unsafe {
            let ref_a = clorus_ref(Value::long(STARTING_BALANCE));
            let ref_b = clorus_ref(Value::long(STARTING_BALANCE));

            // Shared retry-count so a pathological livelock shows up as a
            // very large number rather than the test hanging forever.
            let total_retries = Arc::new(AtomicI64::new(0));

            let ref_a_addr = ref_a as usize;
            let ref_b_addr = ref_b as usize;

            let handles: Vec<_> = (0..THREADS)
                .map(|thread_idx| {
                    let total_retries = Arc::clone(&total_retries);
                    thread::spawn(move || {
                        let ref_a = ref_a_addr as *mut Value;
                        let ref_b = ref_b_addr as *mut Value;
                        let mut rng_state: u64 = 0x9E3779B97F4A7C15u64.wrapping_add(thread_idx as u64);

                        for _ in 0..TRANSFERS_PER_THREAD {
                            loop {
                                // xorshift, good enough for picking a small transfer amount
                                rng_state ^= rng_state << 13;
                                rng_state ^= rng_state >> 7;
                                rng_state ^= rng_state << 17;
                                let amount = 1 + (rng_state % 5) as i64;

                                assert!(clorus_tx_begin());

                                let a_val = clorus_ref_deref(ref_a);
                                let b_val = clorus_ref_deref(ref_b);
                                let a_current = (*a_val).as_long();
                                let b_current = (*b_val).as_long();
                                clorus_release(a_val);
                                clorus_release(b_val);

                                let new_a = Value::long(a_current - amount);
                                let new_b = Value::long(b_current + amount);
                                let ret_a = clorus_ref_set(ref_a, new_a);
                                let ret_b = clorus_ref_set(ref_b, new_b);
                                clorus_release(ret_a);
                                clorus_release(ret_b);
                                clorus_release(new_a);
                                clorus_release(new_b);

                                if clorus_tx_commit() {
                                    break;
                                }
                                clorus_tx_abort();
                                total_retries.fetch_add(1, AtomicOrdering::Relaxed);
                            }
                        }
                    })
                })
                .collect();

            for h in handles {
                h.join().unwrap();
            }

            let final_a_val = clorus_ref_deref(ref_a);
            let final_b_val = clorus_ref_deref(ref_b);
            let final_a = (*final_a_val).as_long();
            let final_b = (*final_b_val).as_long();
            clorus_release(final_a_val);
            clorus_release(final_b_val);

            assert_eq!(
                final_a + final_b,
                STARTING_BALANCE * 2,
                "total balance drifted under concurrent transactions: A={} B={} (retries observed: {})",
                final_a,
                final_b,
                total_retries.load(AtomicOrdering::Relaxed)
            );

            clorus_release(ref_a);
            clorus_release(ref_b);
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
