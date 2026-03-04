# Clorus Standard Library - Implementation Roadmap

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/ROADMAP_TO_100_PARITY.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Based on Clojure's proven design**
**Date:** 2025-01-30

---

## Core Principle

> **Like Clojure:** `clorus.core` is auto-loaded and provides 95% of what users need daily.
> **Other namespaces:** Available via `(require)` for specialized needs.

---

## Phase 1: clorus.core (PRIORITY: IMMEDIATE)

**Status:** Foundation for everything else
**Timeline:** 2-3 days
**Goal:** Stable, auto-loaded `clorus.core` namespace

### What Goes in clorus.core

Based on Clojure's clojure.core, include:

#### Essential (Must Have - Week 1)
```clojure
;; Arithmetic
+, -, *, /, mod, rem, quot
inc, dec, zero?, pos?, neg?, even?, odd?
min, max, abs

;; Collections
list, vector, hash-map, hash-set
first, rest, last, nth, count, empty?
conj, cons, assoc, dissoc, get
vec, seq

;; Higher-order
map, filter, reduce, remove
take, drop, take-while, drop-while
partition, partition-by

;; Logic
nil?, some?, true?, false?, boolean
not, and, or

;; I/O (from runtime)
println, print, pr, prn
slurp, spit, read-line

;; Type predicates
number?, string?, keyword?, symbol?
vector?, list?, map?, set?, fn?
atom?, ref?, agent?
```

#### Important (Week 2)
```clojure
;; Function utilities
partial, comp, identity, constantly
complement, juxt, apply, fn?

;; Sequences
range, repeat, repeatedly
concat, interleave, interpose
flatten, distinct, dedupe

;; Control flow macros
for, doseq, dotimes, while
loop, recur, when, if-let, when-let
```

### Implementation Steps

**Day 1: Bootstrap**
1. Fix parse error in current stdlib/core.clr
2. Create minimal working clorus.core with essential 20 functions
3. Test auto-loading in compiler

**Day 2: Core Functions**
1. Add all arithmetic functions (with tests)
2. Add all collection functions (with tests)
3. Add all predicates (with tests)

**Day 3: Macros & HOFs**
1. Fix `for` and `doseq` macros
2. Add function utilities
3. Comprehensive integration tests

---

## Phase 2: Specialized Namespaces (NEXT 2 WEEKS)

### clorus.string (Priority: HIGH)

**Why:** String manipulation is essential for any real program
**Timeline:** 2 days

```clojure
(ns clorus.string)

;; From Clojure
split, join, replace, replace-first
upper-case, lower-case, capitalize
trim, trim-left, trim-right
starts-with?, ends-with?, includes?

;; Clorus-specific
format         ; String formatting
```

### clorus.set (Priority: MEDIUM)

**Why:** Set operations are common
**Timeline:** 1 day

```clojure
(ns clorus.set)

union, intersection, difference
subset?, superset?
select, project, rename
```

### clorus.io (Priority: HIGH)

**Why:** File I/O is essential
**Timeline:** 2 days

```clojure
(ns clorus.io
  "Like clojure.java.io - file operations")

;; File operations
file, delete-file, make-parents
copy, reader, writer

;; Path operations
file-exists?, directory?, file?
absolute-path, relative-path
```

### clorus.repl (Priority: MEDIUM)

**Why:** Developer experience
**Timeline:** 1 day

```clojure
(ns clorus.repl)

doc          ; Show documentation
source       ; Show source code
dir          ; List namespace contents
find-doc     ; Search docs
apropos      ; Find by name
pst          ; Print stack trace
```

### clorus.pprint (Priority: LOW)

**Why:** Debugging & readability
**Timeline:** 2 days

```clojure
(ns clorus.pprint)

pprint       ; Pretty-print data structures
print-table  ; Print as table
cl-format    ; Common Lisp style formatting
```

---

## Phase 3: Advanced Namespaces (FUTURE)

### clorus.walk (Priority: MEDIUM)
```clojure
walk, prewalk, postwalk
prewalk-replace, postwalk-replace
```

### clorus.xml (Priority: LOW)
```clojure
parse, emit
```

### clorus.zip (Priority: LOW)
```clojure
zipper, up, down, left, right
node, replace, edit
```

---

## Current Status & Immediate Next Steps

### ✅ What Works Now
- Basic compiler infrastructure
- LLVM codegen
- Runtime library (atoms, refs, agents, STM)
- First-class functions & closures
- Transducers
- `inc`, `dec` from minimal stdlib

### ❌ What's Broken
- Full stdlib/core.clr parse error: `#Some(' ')`
- Macros (`for`, `doseq`) not working
- No namespace system yet

### 🎯 Immediate Action Plan (Next 24 Hours)

**Step 1: Fix Parse Error** (2 hours)
```bash
# Debug the parse error
1. Test stdlib/core.clr line by line
2. Find the problematic macro syntax
3. Fix or remove broken macros temporarily
4. Get a working baseline
```

**Step 2: Create Clean clorus.core** (4 hours)
```bash
# Start fresh with professional code
1. Create stdlib/core-clean.clr
2. Add essential 50 functions (documented, tested)
3. Skip macros for now (add later)
4. Test with real programs
```

**Step 3: Auto-Load in Compiler** (2 hours)
```bash
# Make it automatic
1. Update compiler to load stdlib/core-clean.clr
2. Show "✓ clorus.core loaded (50 forms)"
3. Test in REPL
4. Verify no performance regression
```

**Step 4: Documentation** (2 hours)
```bash
# Professional docs
1. Document every function in clorus.core
2. Add examples to each
3. Create STDLIB_REFERENCE.md
4. Update README with stdlib info
```

---

## Testing Strategy

### Unit Tests (Per Namespace)

```clojure
;; tests/stdlib/core-test.clr
(ns clorus.core-test
  (:require [clorus.test :refer [deftest is testing]]))

(deftest arithmetic
  (testing "inc"
    (is (= 6 (inc 5)))
    (is (= 0 (inc -1))))

  (testing "dec"
    (is (= 4 (dec 5)))
    (is (= -1 (dec 0)))))

(deftest collections
  (testing "map"
    (is (= [2 3 4] (map inc [1 2 3])))
    (is (= [] (map inc []))))

  (testing "filter"
    (is (= [2 4] (filter even? [1 2 3 4])))
    (is (= [] (filter even? [1 3 5])))))

;; ... comprehensive tests
```

### Integration Tests

```clojure
;; tests/integration/stdlib-integration-test.clr

;; Test that commonly used combinations work
(deftest real-world-usage
  (testing "data pipeline"
    (is (= 30
           (->> [1 2 3 4 5]
                (map inc)
                (filter even?)
                (reduce +)))))

  (testing "file processing"
    (spit "test.txt" "hello\nworld")
    (is (= "hello\nworld"
           (slurp "test.txt")))))
```

### Performance Benchmarks

```clojure
;; benchmarks/core-bench.clr
(defn bench-map []
  (let [data (range 10000)]
    (time (map inc data))))

;; Target: < 10ms for 10k elements
```

---

## Documentation Standards

### Every Function Must Have:

```clojure
(defn map
  "Transform each element of a collection using a function.

  Applies f to each element of coll, returning a new collection of results.
  Lazy when possible. Returns transducer when no collection provided.

  Examples:
    (map inc [1 2 3])          ; => [2 3 4]
    (map + [1 2] [10 20])      ; => [11 22]
    (map str [:a :b])          ; => [\"a\" \"b\"]

  See also: filter, reduce, for

  Added in: v0.3.0"
  [f coll]
  ,,,)
```

**Required sections:**
1. **Summary** - One line, what it does
2. **Details** - How it works, edge cases
3. **Examples** - 2-3 real examples
4. **See also** - Related functions
5. **Since** - Version added

---

## Release Checklist

### Before v1.0:
- [ ] clorus.core with 100+ functions
- [ ] clorus.string complete
- [ ] clorus.set complete
- [ ] clorus.io complete
- [ ] clorus.repl complete
- [ ] 95%+ test coverage
- [ ] All functions documented
- [ ] Performance benchmarks pass
- [ ] No breaking changes in 6 months
- [ ] Community feedback incorporated

---

## Questions to Answer

### Q: Should clorus.core be compatible with clojure.core?
**A:** YES. Goal is maximum compatibility.
- Same function names
- Same behavior (where possible)
- Same API surface
- Makes porting code easy

### Q: What about Java interop (clojure.java.*)?
**A:** Create clorus.rust.* instead
- clorus.rust.io (file I/O)
- clorus.rust.fs (filesystem)
- clorus.rust.http (HTTP)
- Follows same pattern as Clojure

### Q: Performance vs. compatibility trade-offs?
**A:** Compatibility first, then optimize
- Correct behavior > fast behavior
- But don't be obviously slow
- Profile and optimize hot paths
- Provide unchecked-* variants for performance

---

**Next Update:** After Phase 1 complete
**Owner:** Clorus Core Team
