/// Go blocks for concurrent execution
///
/// Go blocks execute code asynchronously on the thread pool.
/// They provide lightweight concurrency for channel operations.
///
/// Example:
/// ```clojure
/// (def ch (chan 10))
/// (go (>!! ch 42))
/// (<!! ch)  ; => 42
/// ```

use crate::thread_pool::ThreadPool;
use crate::value::Value;
use std::sync::OnceLock;
use std::thread;

/// Global thread pool for go blocks (shared with agents)
static GO_POOL: OnceLock<ThreadPool> = OnceLock::new();

/// Get or create the go block thread pool
fn get_go_pool() -> &'static ThreadPool {
    GO_POOL.get_or_init(|| {
        let size = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        ThreadPool::new(size, "clorus-go")
    })
}

/// Execute a function in a go block with captured variables
///
/// The function is executed asynchronously with access to captured variables.
///
/// # Arguments
/// * `func_ptr` - Function to execute (signature: fn(*mut Value) -> *mut Value)
/// * `captures` - Vector of captured variable values
///
/// # Returns
/// Channel that will contain the result when execution completes
#[no_mangle]
pub extern "C" fn clorus_go(func_ptr: *mut Value, captures: *mut Value) -> *mut Value {
    if func_ptr.is_null() {
        return Value::nil();
    }

    // Create result channel (buffered with capacity 1)
    let result_chan = crate::channel::clorus_chan(1);

    // Get the thread pool
    let pool = get_go_pool();

    // Convert pointers to usize for thread safety
    let func_addr = func_ptr as usize;
    let captures_addr = if captures.is_null() { 0 } else { captures as usize };
    let result_chan_addr = result_chan as usize;

    // Retain captures and result channel
    if !captures.is_null() {
        unsafe {
            (*captures).header().retain();
        }
    }
    unsafe {
        (*result_chan).header().retain();
    }

    // Execute in pool
    pool.execute(move || {
        unsafe {
            // Convert back to pointers
            let func_ptr = func_addr as *mut Value;
            let captures_ptr = if captures_addr == 0 {
                std::ptr::null_mut()
            } else {
                captures_addr as *mut Value
            };
            let result_chan_ptr = result_chan_addr as *mut Value;

            // Transmute to function that takes captures and returns *mut Value
            type GoFunc = unsafe extern "C" fn(*mut Value) -> *mut Value;
            let func: GoFunc = std::mem::transmute(func_ptr);

            // Execute the function with captures
            let result = func(captures_ptr);

            // Put result into channel
            crate::channel::clorus_chan_put(result_chan_ptr, result);

            // Release captures and channel
            if !captures_ptr.is_null() {
                crate::value::clorus_release(captures_ptr);
            }
            crate::value::clorus_release(result_chan_ptr);
        }
    });

    // Return channel immediately (async execution continues)
    result_chan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_execution() {
        // Create a test function that returns a value
        extern "C" fn test_func(_captures: *mut Value) -> *mut Value {
            Value::number(42.0)
        }

        // Execute in go block
        let result_chan = clorus_go(test_func as *mut Value, std::ptr::null_mut());

        // Should return a channel immediately
        unsafe {
            assert_eq!((*result_chan).header().tag(), crate::value::ValueTag::Channel);
        }

        // Take from channel to get result
        let result = crate::channel::clorus_chan_take(result_chan);

        unsafe {
            let num = crate::value::clorus_value_as_number(result);
            assert_eq!(num, 42.0);
        }

        // Clean up
        unsafe {
            crate::value::clorus_release(result);
            crate::value::clorus_release(result_chan);
        }
    }

    #[test]
    fn test_go_multiple() {
        // Execute multiple go blocks
        extern "C" fn test_func(_captures: *mut Value) -> *mut Value {
            Value::number(1.0)
        }

        let mut channels = Vec::new();
        for _ in 0..10 {
            let result_chan = clorus_go(test_func as *mut Value, std::ptr::null_mut());
            channels.push(result_chan);
        }

        // Collect all results
        for chan in channels {
            let result = crate::channel::clorus_chan_take(chan);
            unsafe {
                crate::value::clorus_release(result);
                crate::value::clorus_release(chan);
            }
        }
    }
}
