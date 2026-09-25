# Agents Implementation Plan

**Feature:** Asynchronous Agents for Independent State Updates
**Priority:** 🔥 High - Completes state management trio
**Effort:** 1 week
**Status:** Planning

---

## Overview

Implement Clojure-style agents for **asynchronous, independent state updates**. Unlike refs (coordinated) and atoms (synchronous), agents process updates asynchronously in background threads.

**Current State:**
- ✅ Atoms - Synchronous, uncoordinated updates
- ✅ Refs - Synchronous, coordinated updates (STM)
- ❌ Agents - Asynchronous, independent updates

**Goal State:**
- ✅ `agent` - Create agent with initial value
- ✅ `send` - Queue action for async execution (thread pool)
- ✅ `send-off` - Queue action for potentially blocking operations (expandable pool)
- ✅ `await` - Block until all actions complete
- ✅ `@agent` / `deref` - Read current value (immediate, no blocking)
- ✅ Error handling with `agent-error`, `restart-agent`

---

## Key Concepts

### What are Agents?

**Agents** provide asynchronous, independent state management:
- **Asynchronous** - Actions queued and executed in background threads
- **Independent** - No coordination with other agents
- **Serialized** - Actions for same agent execute sequentially
- **Non-blocking** - `send` returns immediately

### Atoms vs Refs vs Agents

| Feature | Atoms | Refs | Agents |
|---------|-------|------|--------|
| Coordination | ❌ No | ✅ Yes | ❌ No |
| Synchronous | ✅ Yes | ✅ Yes | ❌ No |
| Blocking | ❌ No | ✅ Yes (tx) | ❌ No |
| Use case | Fast sync updates | Coordinated changes | Async background work |

**Example Use Cases:**
```clojure
;; Agent for logging (async, non-blocking)
(def logger (agent []))
(send logger conj "Event 1")  ; Returns immediately
(send logger conj "Event 2")

;; Agent for background computation
(def stats (agent {:count 0 :sum 0}))
(send stats (fn [s] (assoc s :count (inc (:count s)))))

;; Agent for I/O operations
(def file-writer (agent nil))
(send-off file-writer write-to-file data)  ; Potentially blocking
```

---

## Architecture

### Agent Structure

```rust
pub struct ClorusAgent {
    /// Current value (lock-free reads)
    value: Arc<RwLock<*mut Value>>,

    /// Action queue (MPSC channel)
    action_tx: Arc<Mutex<mpsc::Sender<Action>>>,

    /// Error state
    error: Arc<RwLock<Option<AgentError>>>,

    /// Validator function (optional)
    validator: Option<fn(*mut Value) -> bool>,

    /// Agent ID for tracking
    id: AgentId,
}

struct Action {
    /// Function to apply
    func: *mut Value,
    /// Arguments to function
    args: Vec<*mut Value>,
}

struct AgentError {
    /// Error value
    error: *mut Value,
    /// Action that caused error
    action: Action,
}
```

### Thread Pool Architecture

```rust
/// Global thread pool for agents
lazy_static! {
    static ref SEND_POOL: ThreadPool = ThreadPool::new(
        num_cpus::get(),  // Fixed size
        "clorus-agent-send"
    );

    static ref SEND_OFF_POOL: ThreadPool = ThreadPool::new_expandable(
        "clorus-agent-send-off"
    );
}

struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;
```

### Action Processing

```
send() → Queue action → Worker thread picks up → Apply function → Update value
  ↓                                                   ↓
Returns immediately                            Catch errors, store in agent
```

---

## API Design

### 1. Creating Agents

#### `agent` - Create agent with initial value
```clojure
(agent initial-value)
(agent initial-value :validator validate-fn)
(agent initial-value :error-mode :continue)  ; or :fail

;; Examples
(def counter (agent 0))
(def log (agent []))
(def config (agent {} :validator map?))
```

**Signature:** `(agent value & {:keys [validator error-mode]}) -> Agent`
**FFI:** `clorus_agent(value: *mut Value) -> *mut Value`

#### `@` / `deref` - Read current value (non-blocking)
```clojure
@counter  ; => Current value (immediate read)
```

**Note:** Deref is non-blocking - returns current value immediately, even if actions are queued.

---

### 2. Sending Actions

#### `send` - Queue action for fixed thread pool
```clojure
(send agent function & args)

;; Examples
(send counter inc)
(send log conj "message")
(send config assoc :key "value")
```

**Signature:** `(send agent func & args) -> agent`
**FFI:** `clorus_send(agent: *mut Value, func: *mut Value, args: *mut Value) -> *mut Value`

**Behavior:**
- Returns immediately (non-blocking)
- Queues action for background execution
- Uses fixed-size thread pool
- Actions execute serially for each agent
- Good for CPU-bound, fast operations

**Example:**
```clojure
(def counter (agent 0))
(send counter inc)  ; Returns immediately
(send counter inc)  ; Queued after first inc
@counter  ; => Could be 0, 1, or 2 (depends on timing)
```

#### `send-off` - Queue action for expandable thread pool
```clojure
(send-off agent function & args)

;; Example - I/O operations
(send-off file-writer write-data data)
(send-off http-client fetch-url url)
```

**Signature:** `(send-off agent func & args) -> agent`
**FFI:** `clorus_send_off(agent: *mut Value, func: *mut Value, args: *mut Value) -> *mut Value`

**Behavior:**
- Returns immediately
- Uses expandable thread pool
- Good for blocking I/O operations
- Threads created/destroyed as needed

**When to use send-off:**
- File I/O
- Network requests
- Database queries
- Any potentially blocking operation

---

### 3. Waiting for Completion

#### `await` - Block until all actions complete
```clojure
(await & agents)

;; Example
(def a (agent 0))
(def b (agent 0))
(send a inc)
(send b inc)
(await a b)  ; Blocks until both finish
@a  ; => 1 (guaranteed)
@b  ; => 1 (guaranteed)
```

**Signature:** `(await agent1 agent2 ...) -> nil`
**FFI:** `clorus_await(agents: *mut Value) -> *mut Value`

**Behavior:**
- Blocks current thread
- Waits for all queued actions to complete
- Timeout variant: `(await-for timeout-ms & agents)`

#### `await-for` - Wait with timeout
```clojure
(await-for timeout-ms & agents)

;; Example
(if (await-for 1000 agent)  ; Wait max 1 second
  (println "Done!")
  (println "Timeout!"))
```

---

### 4. Error Handling

#### `agent-error` - Get error from agent
```clojure
(agent-error agent)

;; Example
(def a (agent 0))
(send a (fn [x] (/ x 0)))  ; Causes error
(await a)
(agent-error a)  ; => Error value
```

**Signature:** `(agent-error agent) -> error-or-nil`

#### `restart-agent` - Restart agent with new value
```clojure
(restart-agent agent new-value & options)

;; Example
(restart-agent a 0 :clear-actions true)
```

**Signature:** `(restart-agent agent value & opts) -> value`

#### Error Modes
- `:continue` - Continue processing actions after error
- `:fail` - Stop processing, all future sends fail until restart

---

## Implementation Plan

### Phase 1: Basic Agent Structure (Day 1-2)

**File:** `/crates/clorus-runtime/src/agent.rs`

#### Step 1.1: Agent Structure
```rust
use crate::value::{Value, ValueTag};
use std::sync::{Arc, RwLock, Mutex, mpsc};
use std::thread;

pub type AgentId = u64;

pub struct ClorusAgent {
    /// Current value
    value: Arc<RwLock<*mut Value>>,

    /// Action sender
    action_tx: Arc<Mutex<mpsc::Sender<Action>>>,

    /// Error state
    error: Arc<RwLock<Option<String>>>,

    /// Agent ID
    id: AgentId,
}

struct Action {
    func: *mut Value,
    args: Vec<*mut Value>,
}

impl ClorusAgent {
    pub fn new(initial: *mut Value) -> Self {
        unsafe { (*initial).header().retain(); }

        let (tx, rx) = mpsc::channel();
        let value = Arc::new(RwLock::new(initial));
        let error = Arc::new(RwLock::new(None));

        let value_clone = value.clone();
        let error_clone = error.clone();

        // Spawn worker thread
        thread::spawn(move || {
            process_actions(rx, value_clone, error_clone);
        });

        static AGENT_COUNTER: AtomicU64 = AtomicU64::new(0);

        ClorusAgent {
            value,
            action_tx: Arc::new(Mutex::new(tx)),
            error,
            id: AGENT_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    pub fn deref(&self) -> *mut Value {
        let guard = self.value.read().unwrap();
        let val = *guard;
        unsafe { (*val).header().retain(); }
        val
    }

    pub fn send_action(&self, func: *mut Value, args: Vec<*mut Value>) {
        let tx = self.action_tx.lock().unwrap();
        tx.send(Action { func, args }).ok();
    }
}

fn process_actions(
    rx: mpsc::Receiver<Action>,
    value: Arc<RwLock<*mut Value>>,
    error: Arc<RwLock<Option<String>>>,
) {
    while let Ok(action) = rx.recv() {
        // Apply function to current value
        let current = {
            let guard = value.read().unwrap();
            *guard
        };

        // TODO: Call function with current + args
        // For now, just update with function result

        // Update value
        // let mut guard = value.write().unwrap();
        // *guard = new_value;
    }
}
```

#### Step 1.2: Add ValueTag::Agent
```rust
// In value.rs
pub enum ValueTag {
    // ... existing
    Agent = 12,  // New!
}
```

#### Step 1.3: Basic FFI Functions
```rust
#[no_mangle]
pub extern "C" fn clorus_agent(initial: *mut Value) -> *mut Value {
    let agent = Box::new(ClorusAgent::new(initial));
    Value::from_ptr(ValueTag::Agent, Box::into_raw(agent) as *mut u8)
}

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
```

---

### Phase 2: Thread Pool (Day 2-3)

**File:** `/crates/clorus-runtime/src/thread_pool.rs`

```rust
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize, name: &str) -> ThreadPool {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(
                id,
                Arc::clone(&receiver),
                name.to_string()
            ));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>, name: String) -> Worker {
        let thread = thread::Builder::new()
            .name(format!("{}-{}", name, id))
            .spawn(move || loop {
                let job = receiver.lock().unwrap().recv();

                match job {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            })
            .unwrap();

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
```

---

### Phase 3: Send Operations (Day 3-4)

**File:** `/crates/clorus-runtime/src/agent.rs` (continued)

```rust
// Global thread pools
lazy_static! {
    static ref SEND_POOL: ThreadPool = ThreadPool::new(
        num_cpus::get(),
        "agent-send"
    );
}

#[no_mangle]
pub extern "C" fn clorus_send(
    agent_val: *mut Value,
    func: *mut Value,
    args: *mut Value,
) -> *mut Value {
    unsafe {
        if (*agent_val).header().tag() != ValueTag::Agent {
            return agent_val;  // Return agent for chaining
        }

        let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;
        let agent = &*agent_ptr;

        // Extract args from vector
        let mut arg_vec = Vec::new();
        if (*args).header().tag() == ValueTag::Vector {
            let vec_ptr = (*args).as_ptr() as *mut PersistentVector;
            let count = (*vec_ptr).count();
            for i in 0..count {
                let arg = PersistentVector::nth(vec_ptr, i);
                (*arg).header().retain();
                arg_vec.push(arg);
            }
        }

        // Retain function for async execution
        (*func).header().retain();

        // Queue action
        agent.send_action(func, arg_vec);

        // Return agent for chaining
        (*agent_val).header().retain();
        agent_val
    }
}
```

---

### Phase 4: Await Implementation (Day 4-5)

```rust
impl ClorusAgent {
    pub fn await_completion(&self, timeout_ms: Option<u64>) -> bool {
        // Send sentinel message and wait for response
        let (tx, rx) = mpsc::channel();

        let sentinel = Action {
            func: std::ptr::null_mut(),
            args: vec![],
        };

        self.action_tx.lock().unwrap().send(sentinel).ok();

        if let Some(timeout) = timeout_ms {
            rx.recv_timeout(Duration::from_millis(timeout)).is_ok()
        } else {
            rx.recv().is_ok()
        }
    }
}

#[no_mangle]
pub extern "C" fn clorus_await(agents: *mut Value) -> *mut Value {
    unsafe {
        if (*agents).header().tag() == ValueTag::Vector {
            let vec_ptr = (*agents).as_ptr() as *mut PersistentVector;
            let count = (*vec_ptr).count();

            for i in 0..count {
                let agent_val = PersistentVector::nth(vec_ptr, i);
                if (*agent_val).header().tag() == ValueTag::Agent {
                    let agent_ptr = (*agent_val).as_ptr() as *mut ClorusAgent;
                    (*agent_ptr).await_completion(None);
                }
            }
        }

        Value::nil()
    }
}
```

---

### Phase 5: Codegen Integration (Day 5-6)

**File:** `/crates/clorus-codegen/src/codegen.rs`

#### FFI Declarations
```rust
// Agent functions
let agent_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
self.module.add_function("clorus_agent", agent_type, None);

let agent_deref_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
self.module.add_function("clorus_agent_deref", agent_deref_type, None);

let send_type = i8_ptr_type.fn_type(
    &[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()],
    false
);
self.module.add_function("clorus_send", send_type, None);

let await_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
self.module.add_function("clorus_await", await_type, None);
```

#### Builtin Dispatch
```rust
"agent" => {
    if args.len() != 1 {
        return Err("agent requires 1 argument".to_string());
    }

    let initial = self.compile_expr(&args[0])?;
    let agent_fn = self.module.get_function("clorus_agent")?;

    let result = self.builder.build_call(
        agent_fn,
        &[initial.into()],
        "agent_call"
    ).unwrap();

    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
}

"send" => {
    if args.len() < 2 {
        return Err("send requires at least 2 args".to_string());
    }

    let agent_val = self.compile_expr(&args[0])?;
    let func_name = match &args[1] {
        Expr::Symbol(name) => name.clone(),
        _ => return Err("send requires function".to_string()),
    };

    let function = self.functions.get(&func_name)?.clone();
    let func_ptr = function.as_global_value().as_pointer_value();

    // Pack remaining args
    // ... (similar to swap!)

    let send_fn = self.module.get_function("clorus_send")?;
    let result = self.builder.build_call(
        send_fn,
        &[agent_val.into(), func_ptr.into(), args_val.into()],
        "send_call"
    ).unwrap();

    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
}
```

---

### Phase 6: Testing (Day 6-7)

#### Unit Tests (Rust)
```rust
#[test]
fn test_agent_create_and_deref() {
    unsafe {
        let val = Value::number(42.0);
        let agent = clorus_agent(val);
        let deref_val = clorus_agent_deref(agent);
        assert_eq!((*deref_val).as_number(), 42.0);
    }
}

#[test]
fn test_agent_send() {
    // Create agent, send actions, verify async execution
}
```

#### Integration Tests (Clorus)
```clojure
;; Test 1: Basic agent
(def counter (agent 0))
@counter  ; => 0

;; Test 2: Send and await
(send counter inc)
(await counter)
@counter  ; => 1

;; Test 3: Multiple sends
(send counter inc)
(send counter inc)
(send counter inc)
(await counter)
@counter  ; => 4

;; Test 4: Async logging
(def log (agent []))
(send log conj "Event 1")
(send log conj "Event 2")
(await log)
@log  ; => ["Event 1" "Event 2"]
```

---

## Success Criteria

### Functional Requirements ✅
- [ ] `agent` creates an asynchronous agent
- [ ] `@` / `deref` reads current value (non-blocking)
- [ ] `send` queues action for async execution
- [ ] `send-off` uses expandable thread pool
- [ ] `await` blocks until all actions complete
- [ ] Actions execute serially per agent
- [ ] Thread pools manage execution
- [ ] Error handling with `agent-error`

### Performance Requirements ✅
- [ ] `send` returns immediately (non-blocking)
- [ ] `deref` is fast (lock-free read)
- [ ] Thread pool reuses threads efficiently
- [ ] Actions execute in order per agent

---

## Dependencies

### Required ✅
- Value system (already have)
- Reference counting (already have)
- Thread pool (need to implement)
- Function calling (already have)

### Optional
- num_cpus crate (for CPU count)
- crossbeam (for better channels)

---

## Timeline

### Week 1: Full Agents Implementation

**Day 1-2:** Basic agent structure
- Create ClorusAgent type
- Add ValueTag::Agent
- Worker thread per agent
- Basic send/deref

**Day 3:** Thread pool
- Fixed-size pool for `send`
- Expandable pool for `send-off`
- Job queue and workers

**Day 4:** Send operations
- Implement `send` FFI
- Function application in worker
- Error capture

**Day 5:** Await and sync
- Implement `await` blocking
- Timeout support
- Agent completion tracking

**Day 6:** Codegen integration
- FFI declarations
- Builtin dispatch
- Function pointer handling

**Day 7:** Testing & docs
- Unit tests
- Integration tests
- Documentation
- Examples

---

## Comparison with Clojure

| Feature | Clojure | Clorus (Planned) | Status |
|---------|---------|------------------|--------|
| agent | ✅ | 🚧 | Planned |
| send | ✅ | 🚧 | Planned |
| send-off | ✅ | 🚧 | Planned |
| await | ✅ | 🚧 | Planned |
| await-for | ✅ | 🚧 | Planned |
| agent-error | ✅ | 🚧 | Planned |
| restart-agent | ✅ | ❌ | Future |
| set-error-mode! | ✅ | ❌ | Future |
| Validators | ✅ | ❌ | Future |

**MVP Coverage:** ~70% of Clojure agent features

---

## Conclusion

Agents will complete Clorus's state management story:
- ✅ **Atoms** - Synchronous, uncoordinated
- ✅ **Refs** - Synchronous, coordinated (STM)
- 🚧 **Agents** - Asynchronous, independent (THIS!)

**Timeline:** 1 week for full implementation
**Complexity:** Medium (simpler than STM, need thread pool)
**Impact:** High (enables async background processing)

---

**Status:** 📋 Plan Complete - Ready to Implement
**Next Step:** Create `/crates/clorus-runtime/src/agent.rs` and begin Phase 1

---

*Last Updated: January 27, 2026*
*Contributors: Prabhu Gopal + Claude Code*
