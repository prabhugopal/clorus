# Channels (CSP) Implementation Plan

**Feature:** CSP-style Channels for Communication Between Threads
**Priority:** 🔥 High - Essential concurrency primitive
**Effort:** 1-2 weeks
**Status:** Planning

---

## Overview

Implement Go/core.async-style channels for **Communicating Sequential Processes (CSP)** - a powerful concurrency model where threads communicate by passing messages through channels rather than sharing memory.

**Current State:**
- ✅ Atoms - Shared mutable state
- ✅ Refs - Coordinated shared state (STM)
- 🚧 Agents - Async state updates
- ❌ Channels - CSP communication

**Goal State:**
- ✅ `chan` - Create buffered/unbuffered channel
- ✅ `>!!` / `put!` - Blocking put
- ✅ `<!!` / `take!` - Blocking take
- ✅ `close!` - Close channel
- ✅ `alts!!` - Select from multiple channels
- ✅ `go` blocks - Lightweight threading (core.async style)

---

## Key Concepts

### What is CSP?

**Communicating Sequential Processes** is a concurrency model where:
- **Channels** are typed queues for passing values
- **Processes** (goroutines/go blocks) communicate via channels
- **Blocking** operations synchronize execution
- **Select** waits on multiple channels

### Why Channels?

**Traditional concurrency:**
```clojure
;; Shared mutable state (error-prone)
(def counter (atom 0))
(future (dotimes [_ 1000] (swap! counter inc)))
(future (dotimes [_ 1000] (swap! counter inc)))
;; Race conditions possible
```

**Channel-based concurrency:**
```clojure
;; Communication via channels (safer)
(def ch (chan))
(go (>! ch 1))
(go (println (<! ch)))  ; => 1
;; Clear data flow, no shared state
```

### Channels vs Other Concurrency

| Feature | Atoms/Refs | Agents | Channels |
|---------|------------|--------|----------|
| Communication | Shared state | Message passing | Message passing |
| Blocking | No | No | Yes (can block) |
| Coordination | Locks/STM | Queues | Rendezvous |
| Use case | State | Background work | Pipelines/coordination |

---

## Architecture

### Channel Structure

```rust
pub struct Channel<T> {
    /// Internal buffer (bounded or unbounded)
    buffer: Arc<Mutex<VecDeque<T>>>,

    /// Capacity (None = unbounded)
    capacity: Option<usize>,

    /// Closed flag
    closed: Arc<RwLock<bool>>,

    /// Waiting senders (for unbuffered/full channels)
    waiting_senders: Arc<Mutex<Vec<Sender<()>>>>,

    /// Waiting receivers (for empty channels)
    waiting_receivers: Arc<Mutex<Vec<Sender<T>>>>,
}

pub type ClorusChannel = Channel<*mut Value>;
```

### Channel Types

1. **Unbuffered Channel** - Rendezvous point (put blocks until take)
```clojure
(def ch (chan))  ; Size 0, blocks immediately
```

2. **Buffered Channel** - Fixed-size queue
```clojure
(def ch (chan 10))  ; Holds 10 items before blocking
```

3. **Dropping Buffer** - Drops oldest when full
```clojure
(def ch (chan (dropping-buffer 10)))
```

4. **Sliding Buffer** - Drops newest when full
```clojure
(def ch (chan (sliding-buffer 10)))
```

---

## API Design

### 1. Creating Channels

#### `chan` - Create channel
```clojure
(chan)           ; Unbuffered (size 0)
(chan 10)        ; Buffered (size 10)
(chan (dropping-buffer 10))
(chan (sliding-buffer 10))

;; Examples
(def messages (chan))
(def events (chan 100))
```

**Signature:** `(chan & [size-or-buffer]) -> Channel`
**FFI:** `clorus_chan(size: i64) -> *mut Value`

---

### 2. Putting Values

#### `>!!` / `put!` - Blocking put
```clojure
(>!! channel value)   ; Blocks until space available
(put! channel value)  ; Same as >!!

;; Example
(def ch (chan 2))
(>!! ch 1)  ; Returns immediately (buffer has space)
(>!! ch 2)  ; Returns immediately (buffer has space)
(>!! ch 3)  ; BLOCKS until someone takes from channel
```

**Signature:** `(>!! chan value) -> true-or-false`
**FFI:** `clorus_chan_put(chan: *mut Value, value: *mut Value) -> bool`

**Behavior:**
- Blocks current thread if buffer full
- Returns `true` if successful
- Returns `false` if channel closed
- Unblocks when space available or channel closed

#### `>!` - Non-blocking put (inside go block)
```clojure
(go
  (>! ch value))  ; Parks goroutine, doesn't block OS thread
```

---

### 3. Taking Values

#### `<!!` / `take!` - Blocking take
```clojure
(<!! channel)   ; Blocks until value available
(take! channel) ; Same as <!!

;; Example
(def ch (chan))
(go (>!! ch 42))
(<!! ch)  ; => 42
(<!! ch)  ; BLOCKS until value available
```

**Signature:** `(<!! chan) -> value-or-nil`
**FFI:** `clorus_chan_take(chan: *mut Value) -> *mut Value`

**Behavior:**
- Blocks current thread if buffer empty
- Returns value when available
- Returns `nil` if channel closed and empty
- Unblocks when value available or channel closed

#### `<!` - Non-blocking take (inside go block)
```clojure
(go
  (let [val (<! ch)]
    (println val)))
```

---

### 4. Closing Channels

#### `close!` - Close channel
```clojure
(close! channel)

;; Example
(def ch (chan))
(close! ch)
(>!! ch 1)  ; => false (closed)
(<!! ch)    ; => nil (closed and empty)
```

**Signature:** `(close! chan) -> nil`
**FFI:** `clorus_chan_close(chan: *mut Value) -> *mut Value`

**Behavior:**
- Closes channel (no more puts allowed)
- Pending takes receive remaining values, then `nil`
- Idempotent (safe to call multiple times)

---

### 5. Channel Operations

#### `alts!!` - Select from multiple channels
```clojure
(alts!! channels & options)

;; Example - Wait on multiple channels
(def ch1 (chan))
(def ch2 (chan))

(alts!! [ch1 ch2])  ; Blocks until any channel has value
;; => [value channel] or [nil channel] if closed

;; With timeout
(alts!! [ch1 ch2] :timeout 1000)
;; => [value channel] or [:timeout nil]
```

**Signature:** `(alts!! chans & opts) -> [value channel]`

**Behavior:**
- Waits on multiple channels simultaneously
- Returns first available value + channel
- Timeout support
- Priority option

#### `alt!` - Select (inside go block)
```clojure
(go
  (alt!
    ch1 ([v] (println "ch1:" v))
    ch2 ([v] (println "ch2:" v))
    :timeout 1000 (println "timeout!")))
```

---

### 6. Go Blocks (Lightweight Concurrency)

#### `go` - Launch go block
```clojure
(go & body)

;; Example - Lightweight concurrent process
(go
  (let [val (<! ch)]
    (println "Received:" val)
    (>! result-ch (process val))))
```

**Signature:** `(go & body) -> channel-with-result`

**Behavior:**
- Creates lightweight "goroutine"
- Runs on thread pool (not OS thread per go block)
- Uses parking (not blocking) for channel operations
- Returns channel that receives return value

**How it works:**
- Transforms code into state machine
- Parks at `<!` / `>!` (doesn't block thread)
- Resumes when channel ready
- Many go blocks per OS thread

---

## Example Use Cases

### Pipeline Pattern
```clojure
(defn pipeline [in process out]
  (go
    (loop []
      (when-let [val (<! in)]
        (>! out (process val))
        (recur)))
    (close! out)))

(def numbers (chan 10))
(def squares (chan 10))

(pipeline numbers #(* % %) squares)

(go (dotimes [n 10] (>! numbers n)))
(<!! squares)  ; => 0
(<!! squares)  ; => 1
(<!! squares)  ; => 4
```

### Fan-Out (Multiple Workers)
```clojure
(defn worker [id jobs results]
  (go
    (loop []
      (when-let [job (<! jobs)]
        (println "Worker" id "processing" job)
        (>! results (process job))
        (recur)))))

(def jobs (chan 100))
(def results (chan 100))

;; Start 5 workers
(dotimes [i 5]
  (worker i jobs results))

;; Send work
(go (dotimes [n 20] (>! jobs n)))
```

### Fan-In (Merge Channels)
```clojure
(defn merge-channels [& chans]
  (let [out (chan)]
    (doseq [ch chans]
      (go
        (loop []
          (when-let [val (<! ch)]
            (>! out val)
            (recur)))))
    out))

(def ch1 (chan))
(def ch2 (chan))
(def merged (merge-channels ch1 ch2))

(go (>! ch1 1))
(go (>! ch2 2))
(<!! merged)  ; => 1 or 2 (nondeterministic)
```

### Timeout Pattern
```clojure
(defn fetch-with-timeout [url timeout-ms]
  (let [result (chan)
        timeout (timeout timeout-ms)]
    (go (>! result (fetch url)))
    (let [[val ch] (alts!! [result timeout])]
      (if (= ch timeout)
        :timeout
        val))))
```

---

## Implementation Plan

### Phase 1: Basic Channel Structure (Day 1-2)

**File:** `/crates/clorus-runtime/src/channel.rs`

```rust
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock, Condvar};

pub struct ClorusChannel {
    buffer: Arc<Mutex<VecDeque<*mut Value>>>,
    capacity: Option<usize>,
    closed: Arc<RwLock<bool>>,
    not_full: Arc<Condvar>,
    not_empty: Arc<Condvar>,
}

impl ClorusChannel {
    pub fn new(capacity: Option<usize>) -> Self {
        ClorusChannel {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            capacity,
            closed: Arc::new(RwLock::new(false)),
            not_full: Arc::new(Condvar::new()),
            not_empty: Arc::new(Condvar::new()),
        }
    }

    pub fn put(&self, value: *mut Value) -> bool {
        let mut buffer = self.buffer.lock().unwrap();

        // Check if closed
        if *self.closed.read().unwrap() {
            return false;
        }

        // Wait for space if buffer full
        while self.is_full(&buffer) && !*self.closed.read().unwrap() {
            buffer = self.not_full.wait(buffer).unwrap();
        }

        if *self.closed.read().unwrap() {
            return false;
        }

        // Put value
        unsafe { (*value).header().retain(); }
        buffer.push_back(value);

        // Notify waiting takers
        self.not_empty.notify_one();

        true
    }

    pub fn take(&self) -> Option<*mut Value> {
        let mut buffer = self.buffer.lock().unwrap();

        // Wait for value
        while buffer.is_empty() && !*self.closed.read().unwrap() {
            buffer = self.not_empty.wait(buffer).unwrap();
        }

        let value = buffer.pop_front();

        // Notify waiting putters
        self.not_full.notify_one();

        value
    }

    pub fn close(&self) {
        *self.closed.write().unwrap() = true;
        self.not_full.notify_all();
        self.not_empty.notify_all();
    }

    fn is_full(&self, buffer: &VecDeque<*mut Value>) -> bool {
        if let Some(cap) = self.capacity {
            buffer.len() >= cap
        } else {
            false  // Unbounded
        }
    }
}
```

#### FFI Functions
```rust
#[no_mangle]
pub extern "C" fn clorus_chan(capacity: i64) -> *mut Value {
    let cap = if capacity <= 0 {
        None
    } else {
        Some(capacity as usize)
    };

    let chan = Box::new(ClorusChannel::new(cap));
    Value::from_ptr(ValueTag::Channel, Box::into_raw(chan) as *mut u8)
}

#[no_mangle]
pub extern "C" fn clorus_chan_put(
    chan_val: *mut Value,
    value: *mut Value,
) -> bool {
    unsafe {
        if (*chan_val).header().tag() != ValueTag::Channel {
            return false;
        }

        let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
        (*chan_ptr).put(value)
    }
}

#[no_mangle]
pub extern "C" fn clorus_chan_take(chan_val: *mut Value) -> *mut Value {
    unsafe {
        if (*chan_val).header().tag() != ValueTag::Channel {
            return Value::nil();
        }

        let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
        (*chan_ptr).take().unwrap_or(Value::nil())
    }
}

#[no_mangle]
pub extern "C" fn clorus_chan_close(chan_val: *mut Value) -> *mut Value {
    unsafe {
        if (*chan_val).header().tag() == ValueTag::Channel {
            let chan_ptr = (*chan_val).as_ptr() as *mut ClorusChannel;
            (*chan_ptr).close();
        }
        Value::nil()
    }
}
```

---

### Phase 2: Go Blocks (Day 3-5)

**Challenge:** Go blocks require code transformation (CPS/state machine)

**Simplified Approach (MVP):**
Use OS threads for now, optimize later with green threads

```rust
#[no_mangle]
pub extern "C" fn clorus_go(
    func: *mut Value,
    args: *mut Value,
) -> *mut Value {
    // For MVP: spawn OS thread
    // Future: Use green threads/goroutines

    let result_chan = clorus_chan(1);  // Result channel

    thread::spawn(move || {
        // Call function
        // Put result into channel
    });

    result_chan
}
```

---

### Phase 3: Select/Alts (Day 5-6)

```rust
pub fn alts(channels: Vec<*mut ClorusChannel>) -> (*mut Value, usize) {
    // Wait on multiple channels
    // Return first value + channel index

    // Implementation using select! macro or custom logic
}
```

---

### Phase 4: Codegen Integration (Day 6-7)

Similar to agents - add FFI declarations and builtin dispatch

---

## Success Criteria

### Functional Requirements ✅
- [ ] `chan` creates buffered/unbuffered channels
- [ ] `>!!` / `put!` blocking put
- [ ] `<!!` / `take!` blocking take
- [ ] `close!` closes channel
- [ ] `alts!!` selects from multiple channels
- [ ] `go` creates lightweight processes
- [ ] Proper blocking/unblocking semantics

---

## Timeline

### Week 1-2: Full Channels Implementation

**Day 1-2:** Basic channels (put/take/close)
**Day 3-5:** Go blocks (OS threads initially)
**Day 5-6:** Select/alts implementation
**Day 6-7:** Codegen integration
**Day 8-10:** Testing and optimization
**Day 11-14:** Green threads (optional, future)

---

## Conclusion

Channels provide powerful CSP-style concurrency:
- ✅ **Atoms/Refs** - Shared state
- 🚧 **Agents** - Message passing (async)
- 🚧 **Channels** - Message passing (sync/async with go blocks)

**Timeline:** 1-2 weeks for full implementation
**Complexity:** High (go blocks, select, green threads)
**Impact:** Very High (essential for concurrent systems)

---

**Status:** 📋 Plan Complete - Ready to Implement After Agents
**Next Step:** Implement Agents first, then Channels

---

*Last Updated: January 27, 2026*
*Contributors: Prabhu Gopal + Claude Code*
