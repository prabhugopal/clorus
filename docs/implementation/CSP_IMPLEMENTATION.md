# Clorus CSP Concurrency - COMPLETE ✅

> **Caveat added 2026-09-23:** "complete" here means the features below work correctly (verified directly), not that they match `core.async` semantics. `go` blocks run on a fixed-size worker pool (sized to CPU count) rather than as lightweight suspended state machines, so a blocking channel op inside `go` occupies a pool thread for its duration — more concurrently-blocked `go` blocks than CPU cores can exhaust the pool. `alts!!` wakes on a single process-wide condvar shared by every channel, not per-channel notification. Separately, STM (`dosync`/`alter`/`commute`, not covered by this doc) is not yet trustworthy under contention — see `docs/PRODUCTION_PLAN.md` P0-2.

## Summary

**All phases complete and tested!**

✅ **Phase 1:** Channels (CSP-style message passing)
✅ **Phase 2:** Go blocks with closures (lightweight concurrency)
✅ **Phase 3:** Select/Alts (channel multiplexing)
✅ **Phase 4:** Return channels (promise-like results)

---

## Phase 1: Channels ✅
**Status:** Complete and tested

**Features:**
- Buffered and unbounded channels
- Blocking operations: `>!!` (put), `<!!` (take)
- Channel closing: `close!`
- Thread-safe message passing

**Tests:** All passing

---

## Phase 2: Go Blocks with Closures ✅
**Status:** Complete and tested

**Features:**
- Lightweight concurrent execution on thread pool
- Automatic closure capture for outer scope variables
- Multiple variables captured simultaneously
- Free variable detection via AST traversal

**Implementation:**
- `find_free_variables()` - detects variables to capture
- Captures packed into vector at call site
- Unpacked in go block function prologue

**Tests:** All passing
- Single variable capture (channels)
- Multiple variable capture
- Concurrent go blocks

---

## Phase 3: Select/Alts ✅
**Status:** Complete and tested

**Features:**
- `alts!!` - wait on multiple channels simultaneously
- Returns `[value channel]` from first available
- Handles closed channels correctly
- Polling-based implementation with short sleeps

**Use Cases:**
- Multi-producer patterns
- First-response scenarios
- Event multiplexing
- Timeout patterns (with go blocks)

**Tests:** All passing
- Single alts call
- Multiple alts calls
- Priority/ordering verification
- Comprehensive producer-consumer test (1400 = 100 + 400 + 900)

---

## Example: Full CSP Pattern

```clojure
;; Multiple producers with select
(def results1 (chan 10))
(def results2 (chan 10))
(def results3 (chan 10))

;; Concurrent producers
(go (>!! results1 (* 10 10)))  ;; 100
(go (>!! results2 (* 20 20)))  ;; 400
(go (>!! results3 (* 30 30)))  ;; 900

;; Consumer with select
(def r1 (alts!! [results1 results2 results3]))
(def r2 (alts!! [results1 results2 results3]))
(def r3 (alts!! [results1 results2 results3]))

;; Process results
(+ (+ (nth r1 0) (nth r2 0)) (nth r3 0))  ;; => 1400
```

---

## Phase 4: Return Channels ✅
**Status:** Complete and tested

**Features:**
- Go blocks automatically create and return a result channel
- Result is put into the channel when computation completes
- Enables promise-like async/await patterns
- Works seamlessly with alts!! for result collection

**Implementation:**
- Create buffered channel (capacity 1) in `clorus_go`
- Put computation result into channel when done
- Return channel immediately to caller

**Tests:** All passing
- Single go block with result retrieval (30 = 10 + 20)
- Multiple go blocks collecting results (1400 = 100 + 400 + 900)
- Comprehensive test with closures + return channels + alts!! (420 = 70 + 140 + 210)

---

## Complete Example: All Features Together

```clojure
;; Comprehensive CSP: Closures + Return Channels + Alts
(def multiplier 7)

;; Launch go blocks that capture 'multiplier' and return results
(def task1 (go (* multiplier 10)))   ;; Returns channel with 70
(def task2 (go (* multiplier 20)))   ;; Returns channel with 140
(def task3 (go (* multiplier 30)))   ;; Returns channel with 210

;; Use alts!! to collect results as they become available
(def result1 (alts!! [task1 task2 task3]))
(def result2 (alts!! [task1 task2 task3]))
(def result3 (alts!! [task1 task2 task3]))

;; Sum all values: 70 + 140 + 210 = 420
(+ (+ (nth result1 0) (nth result2 0)) (nth result3 0))
```

---

## Remaining Work

### Phase 4: Return Channels (Optional Enhancement)
**Status:** ~~Not started~~ **COMPLETE! ✅**

**~~Goal:~~ ACHIEVED:** ~~Make go blocks return a channel containing the result~~

**Before:**
```clojure
(go (+ 1 2))  ;; => nil (executes async, no result)
```

**Now:**
```clojure
(def result-ch (go (+ 1 2)))
(<!! result-ch)  ;; => 3
```

**Benefits:**
- ✅ More ergonomic async/await patterns
- ✅ Promise-like behavior
- ✅ Easier result collection from concurrent operations
- ✅ Works perfectly with alts!! for multiplexing

---

## Files Modified

### Runtime
- `crates/clorus-runtime/src/channel.rs` - Added `try_take()` and `clorus_alts()`
- `crates/clorus-runtime/src/go_block.rs` - Updated for captures parameter and return channels

### Codegen
- `crates/clorus-codegen/src/codegen.rs`
  - Added FFI declarations for alts
  - Added "alts!!" to core_functions
  - Implemented builtin dispatch for alts!!
  - Added free variable detection

### CLI/REPL
- `crates/clorus-cli/src/main.rs` - Symbol linking for alts
- `crates/clorus-repl/src/main.rs` - Symbol linking for alts

---

## Performance Notes

- Alts uses polling (100μs sleep) - room for optimization
- Thread pool shared between agents and go blocks
- Channel operations use condition variables for blocking
- Return channels use buffered channels (capacity 1) for efficiency

---

## Future Optimizations

1. **Optimize alts** - Use condition variables instead of polling for better performance
2. **Add timeout support** - `(alts!! [ch] :timeout 1000)` for timeout patterns
3. **Benchmark** - Measure throughput and latency under load
4. **Select priorities** - Allow weighted channel selection
5. **Non-blocking operations** - Add `>!` and `<!` for non-blocking put/take
