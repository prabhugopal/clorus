/// Asynchronous Agents for Clorus
///
/// Agents provide asynchronous, independent state management where actions
/// are queued and executed sequentially in background threads.
///
/// Unlike atoms (synchronous) and refs (coordinated), agents are:
/// - **Asynchronous** - send() returns immediately
/// - **Independent** - No coordination with other agents
/// - **Serialized** - Actions for same agent execute in order
/// - **Non-blocking** - Deref reads current value without waiting
///
/// Example:
/// ```clojure
/// (def logger (agent []))
/// (send logger conj "Event 1")  ; Returns immediately
/// (send logger conj "Event 2")
/// (await logger)  ; Wait for all actions
/// @logger  ; => ["Event 1" "Event 2"]
/// ```

use crate::value::{Value, ValueTag};
use crate::thread_pool::ThreadPool;
use std::sync::{Arc, RwLock, Mutex, mpsc, Condvar, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

/// Agent ID type
pub type AgentId = u64;

/// Global agent counter
static AGENT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Global thread pool for agent execution
/// Fixed size based on CPU count (matches Clojure's send-pool)
static AGENT_POOL: OnceLock<ThreadPool> = OnceLock::new();

fn get_agent_pool() -> &'static ThreadPool {
    AGENT_POOL.get_or_init(|| {
        let size = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        ThreadPool::new(size, "clorus-agent")
    })
}


/// Wrapper for *mut Value to implement Send
/// Safety: We ensure proper synchronization with locks
struct ValuePtr(*mut Value);
unsafe impl Send for ValuePtr {}
unsafe impl Sync for ValuePtr {}

impl ValuePtr {
    fn new(ptr: *mut Value) -> Self {
        ValuePtr(ptr)
    }

    fn get(&self) -> *mut Value {
        self.0
    }
}

impl Clone for ValuePtr {
    fn clone(&self) -> Self {
        unsafe {
            if !self.0.is_null() {
                (*self.0).header().retain();
            }
        }
        ValuePtr(self.0)
    }
}

impl Drop for ValuePtr {
    fn drop(&mut self) {
        // Don't release here - we manage refcounts explicitly
        // This is because ValuePtr is just a wrapper
        // Release is called explicitly where needed
    }
}

/// An asynchronous agent
///
/// Agents process actions sequentially in a background thread.
/// Multiple agents can run concurrently, but each agent's actions
/// are serialized.
pub struct ClorusAgent {
    /// Current value (lock-free reads via RwLock)
    value: Arc<RwLock<ValuePtr>>,

    /// Action sender (for queueing new actions)
    action_tx: Arc<Mutex<mpsc::Sender<Action>>>,

    /// Error state (None if no error)
    error: Arc<RwLock<Option<AgentError>>>,

    /// Agent ID
    id: AgentId,

    /// Count of pending actions (for await)
    pending_count: Arc<AtomicU64>,

    /// Condition variable for await notification
    completion_notify: Arc<(Mutex<()>, Condvar)>,
}

/// An action to be executed on an agent
struct Action {
    /// Function to apply
    func: ValuePtr,

    /// Arguments to pass to function
    args: Vec<ValuePtr>,
}

/// Agent error information
struct AgentError {
    /// The error value
    error: ValuePtr,

    /// The action that caused the error
    action_name: String,
}

impl ClorusAgent {
    /// Create a new agent with initial value
    pub fn new(initial: *mut Value) -> Self {
        unsafe {
            // Retain initial value
            (*initial).header().retain();
        }

        // Create channel for actions
        let (tx, rx) = mpsc::channel();

        let value = Arc::new(RwLock::new(ValuePtr::new(initial)));
        let error = Arc::new(RwLock::new(None));
        let pending_count = Arc::new(AtomicU64::new(0));
        let completion_notify = Arc::new((Mutex::new(()), Condvar::new()));

        // Clone for worker thread
        let value_clone = value.clone();
        let error_clone = error.clone();
        let pending_clone = pending_count.clone();
        let notify_clone = completion_notify.clone();

        let id = AGENT_COUNTER.fetch_add(1, Ordering::SeqCst);

        // Submit worker to thread pool instead of spawning dedicated thread
        let pool = get_agent_pool();
        pool.execute(move || {
            Self::process_actions(rx, value_clone, error_clone, pending_clone, notify_clone);
        });

        ClorusAgent {
            value,
            action_tx: Arc::new(Mutex::new(tx)),
            error,
            id,
            pending_count,
            completion_notify,
        }
    }

    /// Get agent ID
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Dereference agent - read current value (non-blocking)
    pub fn deref(&self) -> *mut Value {
        let guard = self.value.read().unwrap();
        let val = guard.get();

        unsafe {
            // Retain before returning
            (*val).header().retain();
        }

        val
    }

    /// Queue an action for execution
    ///
    /// Returns true if queued successfully, false if agent has error or closed
    pub fn send_action(&self, func: *mut Value, args: Vec<*mut Value>) -> bool {
        // Check if agent has error
        if self.error.read().unwrap().is_some() {
            return false;
        }

        // Increment pending count
        self.pending_count.fetch_add(1, Ordering::SeqCst);

        // Queue action
        let tx = self.action_tx.lock().unwrap();

        unsafe {
            // Retain args for async execution
            // NOTE: func is a function pointer, not a Value*, so we don't retain it
            for arg in &args {
                (*(*arg)).header().retain();
            }
        }

        let wrapped_args: Vec<ValuePtr> = args.into_iter().map(ValuePtr::new).collect();

        let result = tx.send(Action {
            func: ValuePtr::new(func),
            args: wrapped_args,
        }).is_ok();

        // If send failed, decrement counter
        if !result {
            self.pending_count.fetch_sub(1, Ordering::SeqCst);
        }

        result
    }

    /// Get current error (if any)
    pub fn get_error(&self) -> Option<*mut Value> {
        let error_guard = self.error.read().unwrap();
        error_guard.as_ref().map(|e| {
            let err_ptr = e.error.get();
            unsafe {
                (*err_ptr).header().retain();
            }
            err_ptr
        })
    }

    /// Clear error and restart agent with new value
    pub fn restart(&self, new_value: *mut Value) {
        unsafe {
            (*new_value).header().retain();
        }

        // Clear error
        *self.error.write().unwrap() = None;

        // Update value
        let mut value_guard = self.value.write().unwrap();
        let old_value = value_guard.get();
        *value_guard = ValuePtr::new(new_value);

        unsafe {
            crate::value::clorus_release(old_value);
        }
    }

    /// Wait for all queued actions to complete
    ///
    /// Returns true if completed, false if timed out
    pub fn await_completion(&self, timeout_ms: Option<u64>) -> bool {
        let (lock, cvar) = &*self.completion_notify.as_ref();

        // Fast path: no pending actions
        if self.pending_count.load(Ordering::SeqCst) == 0 {
            return true;
        }

        if let Some(timeout) = timeout_ms {
            // Wait with timeout
            let guard = lock.lock().unwrap();
            let result = cvar.wait_timeout_while(
                guard,
                Duration::from_millis(timeout),
                |_| self.pending_count.load(Ordering::SeqCst) > 0
            ).unwrap();

            !result.1.timed_out()
        } else {
            // Wait indefinitely
            let guard = lock.lock().unwrap();
            let _guard = cvar.wait_while(
                guard,
                |_| self.pending_count.load(Ordering::SeqCst) > 0
            ).unwrap();

            true
        }
    }

    /// Call a function with current value and arguments
    /// Returns the new value or None if call failed
    unsafe fn call_agent_function(
        func_ptr: *mut Value,
        current: *mut Value,
        args: &[ValuePtr],
    ) -> Option<*mut Value> {
        // Agent functions follow the calling convention:
        // fn(current_value: *mut Value, arg1: *mut Value, ...) -> *mut Value

        // For MVP, we support functions with 0-2 additional arguments
        // (plus the current value as first arg)

        match args.len() {
            0 => {
                // Function takes only current value: fn(current) -> new_value
                type AgentFn1 = extern "C" fn(*mut Value) -> *mut Value;
                let func: AgentFn1 = std::mem::transmute(func_ptr);
                Some(func(current))
            }
            1 => {
                // Function takes current + 1 arg: fn(current, arg) -> new_value
                type AgentFn2 = extern "C" fn(*mut Value, *mut Value) -> *mut Value;
                let func: AgentFn2 = std::mem::transmute(func_ptr);
                Some(func(current, args[0].get()))
            }
            2 => {
                // Function takes current + 2 args
                type AgentFn3 = extern "C" fn(*mut Value, *mut Value, *mut Value) -> *mut Value;
                let func: AgentFn3 = std::mem::transmute(func_ptr);
                Some(func(current, args[0].get(), args[1].get()))
            }
            _ => {
                // For more args, we'd need variadic support or a different approach
                // For now, return None (error)
                None
            }
        }
    }

    /// Worker thread that processes actions sequentially
    fn process_actions(
        rx: mpsc::Receiver<Action>,
        value: Arc<RwLock<ValuePtr>>,
        error: Arc<RwLock<Option<AgentError>>>,
        pending_count: Arc<AtomicU64>,
        completion_notify: Arc<(Mutex<()>, Condvar)>,
    ) {
        while let Ok(action) = rx.recv() {
            // Check if there's an error
            if error.read().unwrap().is_some() {
                // Skip actions if in error state
                // NOTE: func is a function pointer, not a Value*, so we don't release it
                unsafe {
                    for arg in action.args {
                        crate::value::clorus_release(arg.get());
                    }
                }

                // Decrement counter and notify
                pending_count.fetch_sub(1, Ordering::SeqCst);
                let (_lock, cvar) = &*completion_notify;
                cvar.notify_all();

                continue;
            }

            // Get current value
            let current = {
                let guard = value.read().unwrap();
                guard.get()
            };

            unsafe {
                (*current).header().retain();
            }

            // Apply function to current value
            let new_value = unsafe {
                match Self::call_agent_function(action.func.get(), current, &action.args) {
                    Some(val) => val,
                    None => {
                        // Function call failed - store error and keep current value
                        *error.write().unwrap() = Some(AgentError {
                            error: ValuePtr::new(crate::value::Value::string("Function call failed: too many arguments")),
                            action_name: "send".to_string(),
                        });
                        current
                    }
                }
            };

            // Update value
            {
                let mut guard = value.write().unwrap();
                let old_value = guard.get();
                *guard = ValuePtr::new(new_value);

                unsafe {
                    (*new_value).header().retain();
                    crate::value::clorus_release(old_value);
                }
            }

            // Release args and current
            // NOTE: func is a function pointer, not a Value*, so we don't release it
            unsafe {
                for arg in action.args {
                    crate::value::clorus_release(arg.get());
                }
                crate::value::clorus_release(current);
            }

            // Decrement counter and notify waiters
            pending_count.fetch_sub(1, Ordering::SeqCst);
            let (_lock, cvar) = &*completion_notify;
            cvar.notify_all();
        }
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new agent with initial value
///
/// (agent initial-value) => Agent
#[no_mangle]
pub extern "C" fn clorus_agent(initial: *mut Value) -> *mut Value {
    if initial.is_null() {
        return Value::nil();
    }

    let agent = Box::new(ClorusAgent::new(initial));
    Value::from_ptr(ValueTag::Agent, Box::into_raw(agent) as *mut u8)
}

/// Dereference an agent - read current value (non-blocking)
///
/// @agent or (deref agent) => current-value
#[no_mangle]
pub extern "C" fn clorus_agent_deref(agent_val: *mut Value) -> *mut Value {
    if agent_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*agent_val).header().tag() == ValueTag::Agent {
            let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;
            (*agent_ptr).deref()
        } else {
            Value::nil()
        }
    }
}

/// Send an action to an agent for async execution
///
/// (send agent func & args) => agent
#[no_mangle]
pub extern "C" fn clorus_send(
    agent_val: *mut Value,
    func: *mut Value,
    args: *mut Value,
) -> *mut Value {
    if agent_val.is_null() || func.is_null() {
        return agent_val;
    }

    unsafe {
        if (*agent_val).header().tag() != ValueTag::Agent {
            return agent_val;
        }

        let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;

        // Extract args from vector
        let mut arg_vec = Vec::new();
        if !args.is_null() && (*args).header().tag() == ValueTag::Vector {
            let vec_ptr = (*args).as_ptr() as *mut crate::vector::PersistentVector;
            let count = (*vec_ptr).count();
            for i in 0..count {
                let arg = crate::vector::PersistentVector::nth(vec_ptr, i);
                arg_vec.push(arg);
            }
        }

        // Queue action
        (*agent_ptr).send_action(func, arg_vec);

        // Return agent for chaining
        (*agent_val).header().retain();
        agent_val
    }
}

/// Get agent error (if any)
///
/// (agent-error agent) => error-or-nil
#[no_mangle]
pub extern "C" fn clorus_agent_error(agent_val: *mut Value) -> *mut Value {
    if agent_val.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*agent_val).header().tag() == ValueTag::Agent {
            let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;
            (*agent_ptr).get_error().unwrap_or(Value::nil())
        } else {
            Value::nil()
        }
    }
}

/// Wait for agent actions to complete
///
/// (await agent) or (await agent1 agent2 ...) => nil
#[no_mangle]
pub extern "C" fn clorus_await(agents: *mut Value) -> *mut Value {
    if agents.is_null() {
        return Value::nil();
    }

    unsafe {
        // Handle single agent
        if (*agents).header().tag() == ValueTag::Agent {
            let agent_ptr = (*agents).as_ptr() as *mut ClorusAgent;
            (*agent_ptr).await_completion(None);
            return Value::nil();
        }

        // Handle vector of agents
        if (*agents).header().tag() == ValueTag::Vector {
            let vec_ptr = (*agents).as_ptr() as *mut crate::vector::PersistentVector;
            let count = (*vec_ptr).count();

            for i in 0..count {
                let agent_val = crate::vector::PersistentVector::nth(vec_ptr, i);
                if (*agent_val).header().tag() == ValueTag::Agent {
                    let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;
                    (*agent_ptr).await_completion(None);
                }
                crate::value::clorus_release(agent_val);
            }
        }

        Value::nil()
    }
}

/// Wait for agent actions with timeout (milliseconds)
///
/// (await-for agent timeout-ms) => true-or-false
#[no_mangle]
pub extern "C" fn clorus_await_for(agent_val: *mut Value, timeout_ms: i64) -> *mut Value {
    if agent_val.is_null() || timeout_ms < 0 {
        return Value::boolean(false);
    }

    unsafe {
        if (*agent_val).header().tag() == ValueTag::Agent {
            let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;
            let completed = (*agent_ptr).await_completion(Some(timeout_ms as u64));
            Value::boolean(completed)
        } else {
            Value::boolean(false)
        }
    }
}

// ============================================================================
// Cleanup
// ============================================================================

impl Drop for ClorusAgent {
    fn drop(&mut self) {
        unsafe {
            let guard = self.value.read().unwrap();
            let val = guard.get();

            if !val.is_null() {
                crate::value::clorus_release(val);
            }
        }

        // Worker thread will exit when channel closes
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test function that increments a number
    extern "C" fn test_inc(val: *mut Value) -> *mut Value {
        unsafe {
            if (*val).header().tag() == ValueTag::Number {
                let n = (*val).as_number();
                Value::number(n + 1.0)
            } else {
                val
            }
        }
    }

    #[test]
    fn test_agent_create_and_deref() {
        unsafe {
            let val = Value::number(42.0);
            let agent = clorus_agent(val);

            assert!(!agent.is_null());
            assert_eq!((*agent).header().tag(), ValueTag::Agent);

            let deref_val = clorus_agent_deref(agent);
            assert_eq!((*deref_val).as_number(), 42.0);

            crate::value::clorus_release(deref_val);
            crate::value::clorus_release(agent);
        }
    }

    #[test]
    fn test_agent_send() {
        unsafe {
            let val = Value::number(10.0);
            let agent = clorus_agent(val);

            // Use real function pointer
            let func = test_inc as *mut Value;
            let args = crate::vector::clorus_vector_empty();

            let result = clorus_send(agent, func, args);
            assert!(!result.is_null());

            // Give worker thread time to process
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Check that value was incremented
            let deref_val = clorus_agent_deref(agent);
            assert_eq!((*deref_val).as_number(), 11.0);

            crate::value::clorus_release(deref_val);
            crate::value::clorus_release(result);
            crate::value::clorus_release(agent);
        }
    }

    #[test]
    fn test_agent_error_none() {
        unsafe {
            let val = Value::number(5.0);
            let agent = clorus_agent(val);

            let error = clorus_agent_error(agent);
            assert_eq!((*error).header().tag(), ValueTag::Nil);

            crate::value::clorus_release(error);
            crate::value::clorus_release(agent);
        }
    }

    #[test]
    fn test_agent_await() {
        unsafe {
            let val = Value::number(1.0);
            let agent = clorus_agent(val);

            // Send multiple actions
            let func = test_inc as *mut Value;
            let args = crate::vector::clorus_vector_empty();

            clorus_send(agent, func, args);
            clorus_send(agent, func, crate::vector::clorus_vector_empty());
            clorus_send(agent, func, crate::vector::clorus_vector_empty());

            // Wait for all actions to complete
            clorus_await(agent);

            // Check that value was incremented 3 times: 1 + 1 + 1 + 1 = 4
            let deref_val = clorus_agent_deref(agent);
            assert_eq!((*deref_val).as_number(), 4.0);

            crate::value::clorus_release(deref_val);
            crate::value::clorus_release(agent);
        }
    }

    #[test]
    fn test_agent_await_for_timeout() {
        unsafe {
            let val = Value::number(10.0);
            let agent = clorus_agent(val);

            // Send action
            let func = test_inc as *mut Value;
            let args = crate::vector::clorus_vector_empty();
            clorus_send(agent, func, args);

            // Wait with generous timeout - should succeed
            let result = clorus_await_for(agent, 1000);
            assert_eq!((*result).as_bool(), true);

            crate::value::clorus_release(result);
            crate::value::clorus_release(agent);
        }
    }
}
