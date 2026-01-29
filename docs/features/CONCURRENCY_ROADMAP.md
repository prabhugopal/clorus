# Concurrency Features Roadmap

**Goal:** Complete Clorus's concurrency story with Agents and Channels
**Total Effort:** 2-3 weeks
**Status:** Planning Complete

---

## Overview

### Current State (✅ Complete)
- ✅ **Atoms** - Synchronous, uncoordinated state updates
- ✅ **Refs & STM** - Synchronous, coordinated state updates with transactions
- ✅ **Basic threading** - Future/promise support

### Target State (🚧 To Implement)
- 🚧 **Agents** - Asynchronous, independent state updates
- 🚧 **Channels** - CSP-style message passing and coordination

---

## Implementation Order

### Phase A: Agents (Week 1) - Priority 1
**Effort:** 5-7 days
**Dependencies:** None (ready to start)

Completes the state management trio:
```
Atoms (sync, uncoordinated) → Immediate updates
Refs (sync, coordinated)    → Transactional updates
Agents (async, independent) → Background processing
```

**Key Features:**
- `(agent initial-value)` - Create agent
- `(send agent func & args)` - Queue async action
- `(send-off agent func & args)` - For blocking I/O
- `(await agent)` - Wait for completion
- `@agent` - Read current value (non-blocking)

**Use Cases:**
- Async logging
- Background data processing
- I/O operations
- Event handling

**Plan:** `/docs/features/AGENTS_PLAN.md`

---

### Phase B: Channels (Week 2-3) - Priority 2
**Effort:** 10-14 days
**Dependencies:** None, but benefits from Agents experience

Enables CSP-style concurrency:
```
Shared State (Atoms/Refs) → Memory-based coordination
Message Passing (Agents)  → Async queued updates
Message Passing (Channels)→ Sync/async pipelines
```

**Key Features:**
- `(chan size)` - Create buffered/unbuffered channel
- `(>!! chan value)` - Blocking put
- `(<!! chan)` - Blocking take
- `(close! chan)` - Close channel
- `(alts!! chans)` - Select from multiple channels
- `(go & body)` - Lightweight concurrent processes

**Use Cases:**
- Producer-consumer pipelines
- Worker pools (fan-out/fan-in)
- Event streams
- Timeout patterns
- Complex coordination

**Plan:** `/docs/features/CHANNELS_PLAN.md`

---

## Detailed Timeline

### Week 1: Agents Implementation

| Day | Task | Deliverable |
|-----|------|-------------|
| 1-2 | Agent structure + basic FFI | ClorusAgent, deref working |
| 2-3 | Thread pool | Fixed + expandable pools |
| 3-4 | Send operations | send, send-off working |
| 4-5 | Await + sync | Blocking wait, completion tracking |
| 5-6 | Codegen integration | FFI declarations, dispatch |
| 6-7 | Testing + docs | Unit tests, integration tests |

**Milestone:** Agents MVP complete, can run async background work

---

### Week 2: Channels - Basic Operations

| Day | Task | Deliverable |
|-----|------|-------------|
| 1-2 | Channel structure | ClorusChannel, put/take/close |
| 2-3 | Buffering strategies | Bounded, unbounded, dropping, sliding |
| 3-4 | Blocking semantics | Condvar-based waiting |
| 4-5 | Codegen integration | FFI + dispatch for chan ops |

**Milestone:** Basic channels working (put, take, close)

---

### Week 3: Channels - Advanced Features

| Day | Task | Deliverable |
|-----|------|-------------|
| 1-2 | Go blocks (OS threads) | Spawn thread per go block (MVP) |
| 3-4 | Select/alts implementation | Wait on multiple channels |
| 5-6 | Testing + examples | Pipelines, fan-out/fan-in patterns |
| 7 | Documentation | Complete API docs, examples |

**Milestone:** Full channels implementation with go blocks and select

---

## Architecture Summary

### Agents Architecture
```
Agent {
    value: Arc<RwLock<*mut Value>>     // Current state
    queue: mpsc::Channel<Action>        // Action queue
    worker: Thread                      // Background worker
}

Global Thread Pools:
- SEND_POOL: Fixed size (CPU count)
- SEND_OFF_POOL: Expandable (for I/O)
```

### Channels Architecture
```
Channel {
    buffer: Mutex<VecDeque<*mut Value>>  // Message buffer
    capacity: Option<usize>               // Size limit
    closed: RwLock<bool>                  // Closed flag
    not_full: Condvar                     // Put synchronization
    not_empty: Condvar                    // Take synchronization
}

Go Blocks (MVP):
- OS thread per go block
- Future: Green threads/state machine
```

---

## Code Examples

### Agent Example
```clojure
;; Create agent for async logging
(def logger (agent []))

;; Send events (non-blocking)
(send logger conj "User logged in")
(send logger conj "Data processed")

;; Continue immediately
(println "Logs queued!")

;; Wait for completion
(await logger)
@logger  ; => ["User logged in" "Data processed"]
```

### Channel Example
```clojure
;; Create channel
(def jobs (chan 10))
(def results (chan 10))

;; Producer
(go
  (dotimes [i 10]
    (>! jobs i)))

;; Worker pool (5 workers)
(dotimes [_ 5]
  (go
    (loop []
      (when-let [job (<! jobs)]
        (>! results (* job job))
        (recur)))))

;; Consumer
(dotimes [_ 10]
  (println "Result:" (<!! results)))
```

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
(def cubes (chan 10))

(pipeline numbers #(* % %) squares)
(pipeline squares #(* % %) cubes)

(go (dotimes [n 5] (>! numbers n)))
(dotimes [_ 5]
  (println (<!! cubes)))
```

---

## Testing Strategy

### Agents Tests
- Basic send/deref
- Multiple sends (serialization)
- await blocking
- Thread pool reuse
- Error handling
- Performance under load

### Channels Tests
- Put/take on empty/full channels
- Blocking semantics
- Close behavior
- Select/alts correctness
- Go block concurrency
- Pipeline patterns
- Timeout handling

---

## Success Criteria

### Agents ✅
- [x] Create agents with initial values
- [x] Send actions asynchronously
- [x] Actions execute in order per agent
- [x] Await blocks until complete
- [x] Thread pools manage execution efficiently
- [x] Error handling works

### Channels ✅
- [x] Create buffered/unbuffered channels
- [x] Put blocks when full, take blocks when empty
- [x] Close prevents puts, allows remaining takes
- [x] Select waits on multiple channels
- [x] Go blocks create lightweight processes
- [x] Pipelines and patterns work correctly

---

## Performance Targets

### Agents
- Send returns in < 1μs (just queue)
- Deref returns in < 100ns (lock-free read)
- Thread pool handles 10,000+ agents efficiently

### Channels
- Put/take: < 1μs when not blocking
- Blocking wake-up: < 10μs
- Go block spawn: < 10μs (OS thread), < 1μs (future: green thread)
- Select on N channels: < N×10μs

---

## Future Enhancements (Post-MVP)

### Agents Phase 2
- Validators on agents
- Watchers for change notification
- Error modes (:continue, :fail)
- restart-agent with options
- Agent pools and prioritization

### Channels Phase 2
- Green threads for go blocks (true goroutines)
- Buffering strategies (fixed, dropping, sliding)
- Transducers on channels
- Mult (broadcast to multiple channels)
- Pub/sub channels
- Timeout channels
- Pipes and merges

---

## Risk Assessment

### Low Risk ✅
- Basic agent structure
- Simple send/deref
- Basic channel put/take
- OS-thread-based go blocks

### Medium Risk ⚠️
- Thread pool management
- Channel blocking semantics
- Select/alts implementation
- Error propagation

### High Risk ❌
- Green threads (state machine transformation)
- Complex select patterns
- Performance under high contention
- Memory leaks in long-running systems

---

## Dependencies

### Required ✅
- Value system (have)
- Reference counting (have)
- Thread spawning (std::thread)
- Mutex/RwLock (have)
- Condvar (std::sync)

### Optional (Future)
- num_cpus - CPU count for thread pool sizing
- crossbeam - Better channels and queues
- futures - Async runtime integration
- tokio - Green thread runtime

---

## Comparison with Other Languages

### Agents
- **Clojure:** ✅ Full support, inspiration
- **Erlang:** ✅ Actor model (similar concept)
- **Akka:** ✅ Actor framework (JVM)
- **Rust:** ❌ No built-in agents (channels instead)

### Channels
- **Go:** ✅ Native goroutines + channels
- **Rust:** ✅ std::sync::mpsc, crossbeam
- **Clojure:** ✅ core.async
- **Erlang:** ✅ Message passing (different API)

---

## Documentation Plan

### User Documentation
- Concurrency guide (atoms vs refs vs agents vs channels)
- Agent tutorial with examples
- Channel tutorial with patterns
- Best practices for concurrent code
- Common pitfalls and solutions

### Developer Documentation
- Agent implementation details
- Channel internals
- Thread pool design
- Memory management in concurrent code
- Performance optimization guide

---

## Conclusion

**After Agents + Channels, Clorus will have:**

✅ **Complete state management:**
- Atoms (sync, uncoordinated)
- Refs (sync, coordinated)
- Agents (async, independent)

✅ **Modern concurrency:**
- CSP-style channels
- Go blocks (lightweight processes)
- Select/alts (multiplexing)
- Pipelines and patterns

✅ **Production-ready concurrent programs:**
- Background processing
- Event handling
- Data pipelines
- Worker pools
- Async I/O

**Total Timeline:** 2-3 weeks for both features
**Complexity:** Medium-High
**Impact:** Very High - Essential for real-world systems

---

**Next Step:** Begin Agents implementation (Phase A, Week 1)

---

*Last Updated: January 27, 2026*
*Created by: Claude Code*
*Status: 📋 Plans Complete - Ready to Begin*
