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
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar};
use std::time::Duration;

/// Channel ID type
pub type ChannelId = u64;

/// A CSP-style channel
pub struct ClorusChannel {
    /// Buffer for pending values
    buffer: Arc<Mutex<VecDeque<*mut Value>>>,

    /// Maximum capacity (None = unbounded)
    capacity: Option<usize>,

    /// Closed flag
    closed: Arc<Mutex<bool>>,

    /// Condition variable for "not full" (signals putters)
    not_full: Arc<Condvar>,

    /// Condition variable for "not empty" (signals takers)
    not_empty: Arc<Condvar>,

    /// Channel ID
    id: ChannelId,
}

use std::sync::atomic::{AtomicU64, Ordering};
static CHANNEL_COUNTER: AtomicU64 = AtomicU64::new(0);

impl ClorusChannel {
    /// Create a new channel with optional capacity
    ///
    /// - capacity = None: unbounded channel
    /// - capacity = Some(0): rendezvous channel (synchronous)
    /// - capacity = Some(n): buffered channel
    pub fn new(capacity: Option<usize>) -> Self {
        let id = CHANNEL_COUNTER.fetch_add(1, Ordering::SeqCst);

        ClorusChannel {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            capacity,
            closed: Arc::new(Mutex::new(false)),
            not_full: Arc::new(Condvar::new()),
            not_empty: Arc::new(Condvar::new()),
            id,
        }
    }

    /// Put a value into the channel (blocking)
    ///
    /// Returns true if successful, false if channel closed
    pub fn put(&self, value: *mut Value) -> bool {
        let mut buffer = self.buffer.lock().unwrap();

        // Check if closed
        if *self.closed.lock().unwrap() {
            return false;
        }

        // Wait while buffer is full
        while self.is_full(&buffer) {
            // Check closed again before waiting
            if *self.closed.lock().unwrap() {
                return false;
            }

            buffer = self.not_full.wait(buffer).unwrap();

            // Check if closed while waiting
            if *self.closed.lock().unwrap() {
                return false;
            }
        }

        // Retain the value for storage in channel
        unsafe {
            (*value).header().retain();
        }

        // Put value in buffer
        buffer.push_back(value);

        // Notify waiting takers
        self.not_empty.notify_one();

        true
    }

    /// Put a value with timeout (milliseconds)
    ///
    /// Returns true if successful, false if timeout or closed
    pub fn put_timeout(&self, value: *mut Value, timeout_ms: u64) -> bool {
        let mut buffer = self.buffer.lock().unwrap();
        let timeout = Duration::from_millis(timeout_ms);

        // Check if closed
        if *self.closed.lock().unwrap() {
            return false;
        }

        // Wait while buffer is full (with timeout)
        let start = std::time::Instant::now();
        while self.is_full(&buffer) {
            if *self.closed.lock().unwrap() {
                return false;
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return false; // Timeout
            }

            let remaining = timeout - elapsed;
            let result = self.not_full.wait_timeout(buffer, remaining).unwrap();
            buffer = result.0;

            if result.1.timed_out() {
                return false;
            }

            if *self.closed.lock().unwrap() {
                return false;
            }
        }

        // Retain the value
        unsafe {
            (*value).header().retain();
        }

        buffer.push_back(value);
        self.not_empty.notify_one();

        true
    }

    /// Take a value from the channel (blocking)
    ///
    /// Returns the value, or nil if channel closed and empty
    pub fn take(&self) -> *mut Value {
        let mut buffer = self.buffer.lock().unwrap();

        // Wait while buffer is empty
        while buffer.is_empty() {
            let is_closed = *self.closed.lock().unwrap();

            if is_closed {
                // Channel closed and empty - return nil
                return Value::nil();
            }

            buffer = self.not_empty.wait(buffer).unwrap();
        }

        // Take value from buffer
        let value = buffer.pop_front().unwrap();

        // Notify waiting putters
        self.not_full.notify_one();

        value
    }

    /// Take a value with timeout (milliseconds)
    ///
    /// Returns the value, or nil if timeout/closed
    pub fn take_timeout(&self, timeout_ms: u64) -> *mut Value {
        let mut buffer = self.buffer.lock().unwrap();
        let timeout = Duration::from_millis(timeout_ms);

        let start = std::time::Instant::now();
        while buffer.is_empty() {
            let is_closed = *self.closed.lock().unwrap();

            if is_closed {
                return Value::nil();
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Value::nil(); // Timeout
            }

            let remaining = timeout - elapsed;
            let result = self.not_empty.wait_timeout(buffer, remaining).unwrap();
            buffer = result.0;

            if result.1.timed_out() {
                return Value::nil();
            }
        }

        let value = buffer.pop_front().unwrap();
        self.not_full.notify_one();

        value
    }

    /// Close the channel
    ///
    /// No more puts allowed, but remaining values can be taken
    pub fn close(&self) {
        *self.closed.lock().unwrap() = true;

        // Wake all waiting threads
        self.not_full.notify_all();
        self.not_empty.notify_all();
    }

    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        *self.closed.lock().unwrap()
    }

    /// Check if buffer is full
    fn is_full(&self, buffer: &VecDeque<*mut Value>) -> bool {
        if let Some(cap) = self.capacity {
            buffer.len() >= cap
        } else {
            false // Unbounded
        }
    }

    /// Get channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Try to take a value without blocking
    ///
    /// Returns Some(value) if available, None if empty
    pub fn try_take(&self) -> Option<*mut Value> {
        let mut buffer = self.buffer.lock().unwrap();

        if !buffer.is_empty() {
            let value = buffer.pop_front().unwrap();
            self.not_full.notify_one();
            Some(value)
        } else if self.is_closed() {
            // Closed and empty
            Some(Value::nil())
        } else {
            None
        }
    }
}

impl Drop for ClorusChannel {
    fn drop(&mut self) {
        // Release all values in buffer
        let buffer = self.buffer.lock().unwrap();
        for value in buffer.iter() {
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

        // Poll channels in a loop until one is ready
        loop {
            // Try each channel
            for i in 0..count {
                let chan_val = crate::vector::clorus_vector_nth(channels_vec, i);

                if chan_val.is_null() {
                    continue;
                }

                // Check if it's a channel
                if (*chan_val).header().tag() != ValueTag::Channel {
                    continue;
                }

                let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;

                // Try non-blocking take
                if let Some(value) = (*chan_ptr).try_take() {
                    // Create result vector [value channel]
                    let result = crate::vector::clorus_vector_empty();
                    let result = crate::vector::clorus_vector_conj(result, value);
                    let result = crate::vector::clorus_vector_conj(result, chan_val);
                    return result;
                }
            }

            // No channel ready, sleep briefly and retry
            std::thread::sleep(std::time::Duration::from_micros(100));

            // Check if all channels are closed
            let mut all_closed = true;
            for i in 0..count {
                let chan_val = crate::vector::clorus_vector_nth(channels_vec, i);
                if !chan_val.is_null() && (*chan_val).header().tag() == ValueTag::Channel {
                    let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
                    if !(*chan_ptr).is_closed() {
                        all_closed = false;
                        break;
                    }
                }
            }

            if all_closed {
                // All channels closed, return nil
                return Value::nil();
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
    use std::thread;

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
            let val = Value::number(42.0);
            let put_result = clorus_chan_put(chan, val);
            assert_eq!((*put_result).as_bool(), true);

            // Take the value
            let taken = clorus_chan_take(chan);
            assert_eq!((*taken).as_number(), 42.0);

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
            let val = Value::number(42.0);
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
                    let val = Value::number(i as f64);
                    clorus_chan_put(chan_clone, val);
                }
            });

            // Consumer
            let mut sum = 0.0;
            for _ in 0..10 {
                let val = clorus_chan_take(chan);
                sum += (*val).as_number();
                crate::value::clorus_release(val);
            }

            producer.join().unwrap();

            assert_eq!(sum, 45.0); // 0+1+2+...+9 = 45

            crate::value::clorus_release(chan);
        }
    }
}
