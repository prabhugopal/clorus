/// CSP-style Channels for Clorus
///
/// Channels provide message passing between threads/go-blocks.
/// Inspired by Go channels and Clojure's core.async.
///
/// Example:
/// ```clojure
/// (def ch (chan 10))
/// (>!! ch 42)      ; Put (blocks if full)
/// (println (<!! ch))  ; Take (blocks if empty)
/// (close! ch)
/// ```

use crate::value::{Value, ValueTag};
use crate::parking::ParkedTask;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar, OnceLock};
use std::time::Duration;

/// Channel ID type
pub type ChannelId = u64;

/// Mutable channel state guarded by one lock so a close, a buffered value, and
/// the rendezvous waiter count are observed atomically by putters and takers.
/// Keeping those facts under separate locks makes it possible to lose the
/// handoff that an unbuffered channel depends on.
struct ChannelState {
    buffer: VecDeque<*mut Value>,
    closed: bool,
    /// Number of blocking takes waiting for a value on a rendezvous channel.
    waiting_takers: usize,
    parked_takers: VecDeque<ParkedTask>,
}

/// A CSP-style channel
pub struct ClorusChannel {
    /// Buffer, close state, and rendezvous waiter count.
    state: Arc<Mutex<ChannelState>>,

    /// Maximum capacity (None = unbounded)
    capacity: Option<usize>,

    /// Condition variable for "not full" (signals putters)
    not_full: Arc<Condvar>,

    /// Condition variable for "not empty" (signals takers)
    not_empty: Arc<Condvar>,

    /// Channel ID
    id: ChannelId,
}

use std::sync::atomic::{AtomicU64, Ordering};
static CHANNEL_COUNTER: AtomicU64 = AtomicU64::new(0);

fn alts_wait_state() -> &'static (Mutex<u64>, Condvar) {
    static STATE: OnceLock<(Mutex<u64>, Condvar)> = OnceLock::new();
    STATE.get_or_init(|| (Mutex::new(0), Condvar::new()))
}

fn notify_alts_waiters() {
    let (lock, condvar) = alts_wait_state();
    let mut generation = lock.lock().unwrap();
    *generation = generation.wrapping_add(1);
    condvar.notify_all();
}

fn current_alts_generation() -> u64 {
    let (lock, _) = alts_wait_state();
    *lock.lock().unwrap()
}

fn wait_for_alts_activity(last_seen: u64) -> u64 {
    let (lock, condvar) = alts_wait_state();
    let mut generation = lock.lock().unwrap();
    while *generation == last_seen {
        generation = condvar.wait(generation).unwrap();
    }
    *generation
}

impl ClorusChannel {
    /// Create a new channel with optional capacity
    ///
    /// - capacity = None: unbounded channel
    /// - capacity = Some(0): rendezvous channel (synchronous)
    /// - capacity = Some(n): buffered channel
    pub fn new(capacity: Option<usize>) -> Self {
        let id = CHANNEL_COUNTER.fetch_add(1, Ordering::SeqCst);

        ClorusChannel {
            state: Arc::new(Mutex::new(ChannelState {
                buffer: VecDeque::new(),
                closed: false,
                waiting_takers: 0,
                parked_takers: VecDeque::new(),
            })),
            capacity,
            not_full: Arc::new(Condvar::new()),
            not_empty: Arc::new(Condvar::new()),
            id,
        }
    }

    /// Put a value into the channel (blocking)
    ///
    /// Returns true if successful, false if channel closed
    pub fn put(&self, value: *mut Value) -> bool {
        let mut state = self.state.lock().unwrap();

        while !self.can_accept_value(&state) {
            if state.closed {
                return false;
            }
            state = self.not_full.wait(state).unwrap();
        }

        if state.closed {
            return false;
        }

        if let Some(task) = state.parked_takers.pop_front() {
            // `put` borrows its argument, while a parked continuation owns
            // the resumed value. Create that owned handoff before unlocking.
            unsafe { (*value).header().retain(); }
            drop(state);
            unsafe { task.resume(value); }
            notify_alts_waiters();
            return true;
        }

        // Retain the value for storage in channel
        unsafe {
            (*value).header().retain();
        }

        // Put value in buffer
        state.buffer.push_back(value);

        // Notify waiting takers
        self.not_empty.notify_one();
        notify_alts_waiters();

        true
    }

    /// Put a value with timeout (milliseconds)
    ///
    /// Returns true if successful, false if timeout or closed
    pub fn put_timeout(&self, value: *mut Value, timeout_ms: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        let timeout = Duration::from_millis(timeout_ms);

        let start = std::time::Instant::now();
        while !self.can_accept_value(&state) {
            if state.closed {
                return false;
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return false; // Timeout
            }

            let remaining = timeout - elapsed;
            let result = self.not_full.wait_timeout(state, remaining).unwrap();
            state = result.0;

            if result.1.timed_out() {
                return false;
            }
        }

        if state.closed {
            return false;
        }

        // Retain the value
        unsafe {
            (*value).header().retain();
        }

        state.buffer.push_back(value);
        self.not_empty.notify_one();
        notify_alts_waiters();

        true
    }

    /// Take a value from the channel (blocking)
    ///
    /// Returns the value, or nil if channel closed and empty
    pub fn take(&self) -> *mut Value {
        let mut state = self.state.lock().unwrap();
        let mut registered_rendezvous_take = false;

        loop {
            if let Some(value) = state.buffer.pop_front() {
                if registered_rendezvous_take {
                    state.waiting_takers -= 1;
                }
                return self.transfer_taken_value(value);
            }

            if state.closed {
                if registered_rendezvous_take {
                    state.waiting_takers -= 1;
                }
                return Value::nil();
            }

            if self.capacity == Some(0) && !registered_rendezvous_take {
                registered_rendezvous_take = true;
                state.waiting_takers += 1;
                self.not_full.notify_one();
            }

            state = self.not_empty.wait(state).unwrap();
        }
    }

    /// Take a value with timeout (milliseconds)
    ///
    /// Returns the value, or nil if timeout/closed
    pub fn take_timeout(&self, timeout_ms: u64) -> *mut Value {
        let mut state = self.state.lock().unwrap();
        let timeout = Duration::from_millis(timeout_ms);
        let mut registered_rendezvous_take = false;
        let start = std::time::Instant::now();
        loop {
            if let Some(value) = state.buffer.pop_front() {
                if registered_rendezvous_take {
                    state.waiting_takers -= 1;
                }
                return self.transfer_taken_value(value);
            }

            if state.closed {
                if registered_rendezvous_take {
                    state.waiting_takers -= 1;
                }
                return Value::nil();
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                if registered_rendezvous_take {
                    state.waiting_takers -= 1;
                }
                return Value::nil(); // Timeout
            }

            if self.capacity == Some(0) && !registered_rendezvous_take {
                registered_rendezvous_take = true;
                state.waiting_takers += 1;
                self.not_full.notify_one();
            }

            let remaining = timeout - elapsed;
            let result = self.not_empty.wait_timeout(state, remaining).unwrap();
            state = result.0;

            if result.1.timed_out() {
                if registered_rendezvous_take {
                    state.waiting_takers -= 1;
                }
                return Value::nil();
            }
        }
    }

    /// Close the channel
    ///
    /// No more puts allowed, but remaining values can be taken
    pub fn close(&self) {
        let parked_takers = {
            let mut state = self.state.lock().unwrap();
            state.closed = true;
            std::mem::take(&mut state.parked_takers)
        };

        // Wake all waiting threads
        self.not_full.notify_all();
        self.not_empty.notify_all();
        notify_alts_waiters();

        for task in parked_takers {
            unsafe { task.resume(Value::nil()); }
        }
    }

    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        self.state.lock().unwrap().closed
    }

    /// Whether a put can complete now. An unbuffered channel accepts exactly
    /// one value only after a blocking taker has registered its handoff.
    fn can_accept_value(&self, state: &ChannelState) -> bool {
        if !state.parked_takers.is_empty() {
            return true;
        }
        match self.capacity {
            Some(0) => state.buffer.is_empty() && state.waiting_takers > 0,
            Some(capacity) => state.buffer.len() < capacity,
            None => true,
        }
    }

    fn transfer_taken_value(&self, value: *mut Value) -> *mut Value {
        unsafe {
            crate::value::clorus_retain(value);
            crate::value::clorus_release(value);
        }
        self.not_full.notify_one();
        notify_alts_waiters();
        value
    }

    /// Get channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Try to take a value without blocking
    ///
    /// Returns Some(value) if available, None if empty
    pub fn try_take(&self) -> Option<*mut Value> {
        let mut state = self.state.lock().unwrap();

        if let Some(value) = state.buffer.pop_front() {
            Some(self.transfer_taken_value(value))
        } else if state.closed {
            // Closed and empty
            Some(Value::nil())
        } else {
            None
        }
    }

    /// Register a continuation for a non-blocking take. The task is either
    /// resumed with an owned value now or retained by the channel until put or
    /// close can resume it. No worker thread waits in either case.
    pub fn park_take(&self, task: ParkedTask) {
        let mut task = Some(task);
        let value = {
            let mut state = self.state.lock().unwrap();
            if let Some(value) = state.buffer.pop_front() {
                self.not_full.notify_one();
                Some(value)
            } else if state.closed {
                Some(Value::nil())
            } else {
                state.parked_takers.push_back(task.take().unwrap());
                None
            }
        };
        if let (Some(task), Some(value)) = (task, value) {
            unsafe { task.resume(value); }
            notify_alts_waiters();
        }
    }

}

impl Drop for ClorusChannel {
    fn drop(&mut self) {
        // Release all values in buffer
        let state = self.state.lock().unwrap();
        for value in state.buffer.iter() {
            unsafe {
                crate::value::clorus_release(*value);
            }
        }
    }

}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new channel
///
/// (chan) => unbounded channel
/// (chan n) => buffered channel with capacity n
#[no_mangle]
pub extern "C" fn clorus_chan(capacity: i64) -> *mut Value {
    let cap = if capacity < 0 {
        None // Unbounded
    } else if capacity == 0 {
        Some(0) // Rendezvous (synchronous)
    } else {
        Some(capacity as usize)
    };

    let chan = Box::new(ClorusChannel::new(cap));
    Value::from_ptr(ValueTag::Channel, Box::into_raw(chan) as *mut u8)
}

/// Put a value into a channel (blocking)
///
/// (>!! chan value) => true or false
#[no_mangle]
pub extern "C" fn clorus_chan_put(chan_val: *mut Value, value: *mut Value) -> *mut Value {
    if chan_val.is_null() || value.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        if (*chan_val).header().tag() != ValueTag::Channel {
            return Value::boolean(false);
        }

        let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
        let result = (*chan_ptr).put(value);

        Value::boolean(result)
    }
}

/// Take a value from a channel (blocking)
///
/// (<!! chan) => value or nil
#[no_mangle]
pub extern "C" fn clorus_chan_take(chan_val: *mut Value) -> *mut Value {
    if chan_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*chan_val).header().tag() != ValueTag::Channel {
            return Value::nil();
        }

        let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
        (*chan_ptr).take()
    }
}

/// Close a channel
///
/// (close! chan) => nil
#[no_mangle]
pub extern "C" fn clorus_chan_close(chan_val: *mut Value) -> *mut Value {
    if chan_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*chan_val).header().tag() == ValueTag::Channel {
            let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
            (*chan_ptr).close();
        }

        Value::nil()
    }
}

/// Select from multiple channels (alts!!)
///
/// Takes a vector of channels and returns [value channel] from first available
/// (alts!! [ch1 ch2 ch3]) => [value source-channel]
#[no_mangle]
pub extern "C" fn clorus_alts(channels_vec: *mut Value) -> *mut Value {
    if channels_vec.is_null() {
        return Value::nil();
    }

    unsafe {
        // Verify it's a vector
        if (*channels_vec).header().tag() != ValueTag::Vector {
            return Value::nil();
        }

        // Get channel count
        let count = crate::vector::clorus_vector_count(channels_vec);

        if count == 0 {
            return Value::nil();
        }

        // Wait until one channel produces a value or all channels close.
        let mut generation = current_alts_generation();
        loop {
            let mut all_closed = true;

            for i in 0..count {
                let chan_val = crate::vector::clorus_vector_nth(channels_vec, i);

                if chan_val.is_null() || (*chan_val).header().tag() != ValueTag::Channel {
                    continue;
                }

                let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;

                if let Some(value) = (*chan_ptr).try_take() {
                    let result = crate::vector::clorus_vector_empty();
                    let result = crate::vector::clorus_vector_conj(result, value);
                    let result = crate::vector::clorus_vector_conj(result, chan_val);
                    return result;
                }

                if !(*chan_ptr).is_closed() {
                    all_closed = false;
                }
            }

            if all_closed {
                return Value::nil();
            }

            generation = wait_for_alts_activity(generation);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::thread;

    static PARKED_VALUE: AtomicI64 = AtomicI64::new(-1);

    unsafe extern "C" fn record_parked_value(
        _captures: *mut Value,
        value: *mut Value,
        _result_channel: *mut Value,
    ) -> *mut Value {
        PARKED_VALUE.store((*value).as_long(), Ordering::SeqCst);
        Value::nil()
    }

    #[test]
    fn test_channel_create() {
        unsafe {
            let chan = clorus_chan(10);
            assert!(!chan.is_null());
            assert_eq!((*chan).header().tag(), ValueTag::Channel);
            crate::value::clorus_release(chan);
        }
    }

    #[test]
    fn test_channel_put_take() {
        unsafe {
            let chan = clorus_chan(10);

            // Put a value
            let val = Value::double(42.0);
            let put_result = clorus_chan_put(chan, val);
            assert_eq!((*put_result).as_bool(), true);

            // Take the value
            let taken = clorus_chan_take(chan);
            assert_eq!((*taken).as_double(), 42.0);

            crate::value::clorus_release(put_result);
            crate::value::clorus_release(taken);
            crate::value::clorus_release(chan);
        }
    }

    #[test]
    fn test_channel_close() {
        unsafe {
            let chan = clorus_chan(10);

            // Close channel
            clorus_chan_close(chan);

            // Put should fail
            let val = Value::double(42.0);
            let put_result = clorus_chan_put(chan, val);
            assert_eq!((*put_result).as_bool(), false);

            // Take should return nil
            let taken = clorus_chan_take(chan);
            assert_eq!((*taken).header().tag(), ValueTag::Nil);

            crate::value::clorus_release(put_result);
            crate::value::clorus_release(taken);
            crate::value::clorus_release(chan);
        }
    }

    #[test]
    fn test_channel_concurrent() {
        unsafe {
            let chan = clorus_chan(5);

            // Convert to usize for thread safety (raw pointers don't implement Send)
            let chan_addr = chan as usize;

            // Spawn producer
            let producer = thread::spawn(move || {
                let chan_clone = chan_addr as *mut Value;
                for i in 0..10 {
                    let val = Value::double(i as f64);
                    clorus_chan_put(chan_clone, val);
                }
            });

            // Consumer
            let mut sum = 0.0;
            for _ in 0..10 {
                let val = clorus_chan_take(chan);
                sum += (*val).as_double();
                crate::value::clorus_release(val);
            }

            producer.join().unwrap();

            assert_eq!(sum, 45.0); // 0+1+2+...+9 = 45

            crate::value::clorus_release(chan);
        }
    }

    #[test]
    fn test_rendezvous_channel_handoffs_only_to_waiting_taker() {
        unsafe {
            let chan = clorus_chan(0);
            crate::value::clorus_retain(chan);
            let chan_addr = chan as usize;
            let (started_tx, started_rx) = mpsc::channel();
            let (finished_tx, finished_rx) = mpsc::channel();

            let producer = thread::spawn(move || {
                let chan = chan_addr as *mut Value;
                started_tx.send(()).unwrap();
                let value = Value::long(42);
                let result = clorus_chan_put(chan, value);
                unsafe {
                    assert!((*result).as_bool());
                    crate::value::clorus_release(result);
                    crate::value::clorus_release(value);
                    crate::value::clorus_release(chan);
                }
                finished_tx.send(()).unwrap();
            });

            started_rx.recv().unwrap();
            assert!(finished_rx
                .recv_timeout(Duration::from_millis(25))
                .is_err());

            let value = clorus_chan_take(chan);
            assert_eq!((*value).as_long(), 42);
            crate::value::clorus_release(value);

            finished_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            producer.join().unwrap();
            crate::value::clorus_release(chan);
        }
    }

    #[test]
    fn test_parked_take_resumes_on_put_without_waiting_thread() {
        unsafe {
            PARKED_VALUE.store(-1, Ordering::SeqCst);
            let channel = ClorusChannel::new(Some(1));
            let task = ParkedTask::new(record_parked_value, std::ptr::null_mut(), std::ptr::null_mut());
            channel.park_take(task);
            assert_eq!(PARKED_VALUE.load(Ordering::SeqCst), -1);

            let value = Value::long(73);
            assert!(channel.put(value));
            assert_eq!(PARKED_VALUE.load(Ordering::SeqCst), 73);
            crate::value::clorus_release(value);
        }
    }
}
