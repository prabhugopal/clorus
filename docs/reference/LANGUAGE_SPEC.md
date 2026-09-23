# Clorus Language Specification

**Version:** 0.5.1
**Last Updated:** January 29, 2026
**Status:** Alpha. This doc's own status/parity claims are directional, not exact — see `docs/generated/PARITY_STATUS.md` and `docs/PRODUCTION_PLAN.md` for current, verified status.
**Language reference** for Clorus features (see status note above for how "done" a feature actually is)

---

## Quick Reference

**What is Clorus?**
- Clojure-inspired systems programming language
- LLVM-compiled to native code
- Rust FFI with real (non-zero) marshaling overhead for the type shapes it auto-bridges today
- REPL-driven development
- No garbage collector (reference counting)

**Current State:** see `docs/generated/PARITY_STATUS.md` for a verified breakdown (the ~80-85% figure here is an old estimate)
**Ready for:** experimentation and language/tooling development; not yet recommended for production use — see `docs/PRODUCTION_PLAN.md`

---

## Table of Contents

1. [Data Types](#data-types)
2. [Collections](#collections)
3. [Functions](#functions)
4. [Control Flow](#control-flow)
5. [State Management](#state-management)
6. [Polymorphism](#polymorphism)
7. [Macros](#macros)
8. [Namespaces](#namespaces)
9. [Sequences](#sequences)
10. [Type System](#type-system)
11. [FFI](#ffi)
12. [REPL](#repl)
13. [Not Implemented](#not-implemented)

---

## Data Types

### Scalar Types ✅ Complete
- **Numbers**: integers are a real `i64` type (`ValueTag::Long`), not f64-cast; `f64` used for decimals
  ```clojure
  42      ; => 42
  3.14    ; => 3.14
  ```
- **Strings**: UTF-8, immutable
  ```clojure
  "hello"
  "multi\nline"
  ```
- **Booleans**: `true`, `false`
- **Nil**: `nil` (null/none)
- **Keywords**: Interned identifiers
  ```clojure
  :name
  :age
  ```
- **Symbols**: For identifiers (mostly internal)

**Missing:**
- ❌ Characters - individual chars (`\a` currently hangs the compiler instead of erroring — see `docs/PRODUCTION_PLAN.md` P0-4)
- ❌ Rationals - fractions
- ❌ BigInt/BigDecimal - arbitrary precision

---

## Collections

### Core Collections ✅ Complete

#### Vectors - Indexed, ordered
```clojure
[1 2 3]
(nth [10 20 30] 1)  ; => 20
(conj [1 2] 3)      ; => [1 2 3]
```
- Implementation: 32-way branching tree
- Performance: O(log32 n) ≈ O(1)
- Structural sharing: Yes

#### Lists - Sequential, linked
```clojure
'(1 2 3)
(list 1 2 3)
(first '(a b c))    ; => a
(rest '(a b c))     ; => (b c)
```
- Implementation: Singly-linked list
- Performance: O(1) cons/first/rest, O(n) nth
- Structural sharing: Yes

#### Maps - Key-value pairs
```clojure
{:name "Alice" :age 30}
(get {:x 1} :x)     ; => 1
(assoc {:a 1} :b 2) ; => {:a 1 :b 2}
```
- Implementation: HashMap (HAMT planned Phase C)
- Performance: O(1) average
- Structural sharing: Partial (full in Phase C)

#### Sets - Unique values
```clojure
#{1 2 3}
(contains? #{:a :b} :a)  ; => true
(disj #{1 2 3} 2)        ; => #{1 3}
```
- Implementation: HashSet
- Performance: O(1) membership
- Structural sharing: Partial

**Detailed docs:** `docs/STDLIB_PARITY_MATRIX.md`

---

## Functions

### Function Definition ✅ Complete

#### Named Functions
```clojure
(defn add [x y]
  (+ x y))

(add 1 2)  ; => 3
```

#### Anonymous Functions
```clojure
(fn [x] (* x 2))
#(* % 2)            ; Shorthand
```

#### Multi-arity Functions
```clojure
(defn greet
  ([] "Hello!")
  ([name] (str "Hello " name))
  ([name age] (str name " is " age)))
```

#### Variadic Functions
```clojure
(defn sum [& nums]
  (reduce + 0 nums))

(sum 1 2 3 4)  ; => 10
```

#### Closures
```clojure
(defn make-adder [n]
  (fn [x] (+ x n)))

(def add5 (make-adder 5))
(add5 10)  ; => 15
```

**Detailed docs:** `docs/generated/PARITY_STATUS.md`

---

## Control Flow

### Basic Forms ✅ Complete
```clojure
(if test then else)
(when test & body)
(when-not test & body)
(if-let [binding test] then else)
(when-let [binding test] & body)
```

### Multi-way Conditionals ✅ Complete
```clojure
(cond
  (< x 0) "negative"
  (> x 0) "positive"
  :else   "zero")

(case x
  1 "one"
  2 "two"
  "other")
```

### Logical Operators ✅ Complete
```clojure
(and test1 test2 ...)  ; Short-circuit
(or test1 test2 ...)   ; Short-circuit
(not x)
```

### Threading Macros ✅ Complete
```clojure
(-> x f g h)        ; Thread-first
(->> x f g h)       ; Thread-last
(some-> x f g h)    ; Nil-safe thread-first
(some->> x f g h)   ; Nil-safe thread-last
(doto x (f a) (g b)); Mutation chaining
```

### Loops ✅ Complete
```clojure
;; Tail-call optimized
(loop [i 0 acc 0]
  (if (< i 10)
    (recur (inc i) (+ acc i))
    acc))

;; While loop
(while test & body)

;; Dotimes
(dotimes [i n] & body)
```

**Detailed docs:** `docs/generated/PARITY_STATUS.md`

---

## State Management

### Atoms ✅ Complete
Mutable references with atomic updates:
```clojure
(def counter (atom 0))
(swap! counter inc)
@counter  ; => 1
(reset! counter 0)
```

### Refs (STM) ✅ Complete
Software Transactional Memory:
```clojure
(def account (ref 100))
(dosync
  (alter account + 50))
@account  ; => 150
```

### Agents ✅ Complete
Asynchronous state updates:
```clojure
(def logger (agent []))
(send logger conj "event")
(await logger)
@logger  ; => ["event"]
```

### Channels (CSP) ✅ Complete
Go-style channels:
```clojure
(def ch (chan 10))
(>!! ch "value")
(println (<!!, ch))  ; => "value"
(close! ch)
```

**Detailed docs:** `docs/implementation/LOOP_AND_ATOMS_COMPLETE.md`, `docs/generated/PARITY_STATUS.md`

---

## Polymorphism

### Records ✅ Complete
Named data structures:
```clojure
(defrecord Point [x y])
(def p (->Point 10 20))
(get p :x)  ; => 10
```

### Protocols ✅ Complete
Type-based dispatch, called directly (automatic dispatch, verified working 2026-09-23):
```clojure
(defprotocol Drawable
  (draw [this])
  (area [this]))

(extend-type Circle
  Drawable
  (draw [this] ...)
  (area [this] ...))

(draw c)  ; direct call, dispatches on c's type automatically
```

### Multimethods ✅ Complete
Custom dispatch, called directly:
```clojure
(defmulti calculate-price
  (fn [item] (get item :type)))

(defmethod calculate-price :book [item]
  (get item :price))

(calculate-price item)  ; direct call, dispatches on the multi-fn's dispatch value
```

Hierarchy support (`derive`/`isa?`) and `:default` dispatch also exist — see `tests/language/test-multimethod-hierarchy.clr` and `test-multimethod-dispatch.clr`.

**Detailed docs:** `docs/generated/PARITY_STATUS.md`

---

## Macros

### Macro System ✅ 90% Complete

#### User-defined Macros
```clojure
(defmacro unless [test then else]
  `(if ~test ~else ~then))

(unless false "yes" "no")  ; => "yes"
```

#### Quote Forms
```clojure
'(1 2 3)              ; Quote
`(a ~b c)             ; Syntax-quote
~x                    ; Unquote
~@xs                  ; Unquote-splicing
```

#### Hygienic Macros
```clojure
(gensym)              ; Generate unique symbol
(gensym "temp")       ; => temp_1234
```

**Missing:**
- ❌ `macroexpand` - Debug expansion
- ❌ `macroexpand-1` - Single-step expansion

**Detailed docs:** `docs/generated/PARITY_STATUS.md`

---

## Namespaces

### Namespace System ✅ 90% Complete

```clojure
(ns my.app
  (:require [other.lib :as lib]
            [utils :refer [helper]]))

(lib/function x)
(helper x)
```

**Features:**
- ✅ `ns` - Namespace declaration
- ✅ `:require` - Load namespaces
- ✅ `:as` - Alias namespaces
- ✅ `:refer` - Import specific symbols
- ✅ `:refer :all` - Import all
- ✅ File structure validation

**Missing:**
- ❌ `:import` - Rust type imports (partial)
- ❌ Dynamic namespace manipulation

**Detailed docs:** `docs/generated/PARITY_STATUS.md`

---

## Sequences

### Lazy Sequences ✅ 90% Complete
Full library in `stdlib/lazy.clr`:

#### Construction
```clojure
(lazy-range)                ; [0 1 2 ...]
(lazy-range 10 20)          ; [10 11 ... 19]
(lazy-repeat 42)            ; [42 42 42 ...]
(lazy-iterate inc 0)        ; [0 1 2 3 ...]
```

#### Transformations
```clojure
(lazy-map f coll)
(lazy-filter pred coll)
(lazy-take n coll)
(lazy-drop n coll)
(lazy-take-while pred coll)
```

#### Realization
```clojure
(realize lazy-seq)          ; Full realization
(realize-n 10 lazy-seq)     ; Safe for infinite
(doall coll)                ; Force all side effects
```

### Eager Sequences ✅ 100% Complete
Stdlib implementations:

```clojure
(map inc [1 2 3])           ; => [2 3 4]
(filter even? [1 2 3 4])    ; => [2 4]
(reduce + 0 [1 2 3])        ; => 6
(remove odd? [1 2 3 4])     ; => [2 4]
(keep identity [1 nil 3])   ; => [1 3]
(mapcat #(list % %) [1 2])  ; => (1 1 2 2)
```

**Detailed docs:** `docs/LAZY_SEQUENCES_GUIDE.md`

---

## Type System

### Type Predicates ✅ 100% Complete

All predicates implemented:
```clojure
(vector? [1 2])      ; => true
(list? '(1 2))       ; => true
(map? {:a 1})        ; => true
(set? #{1 2})        ; => true
(string? "hi")       ; => true
(number? 42)         ; => true
(keyword? :a)        ; => true
(symbol? 'x)         ; => true
(nil? nil)           ; => true
(boolean? true)      ; => true
(seq? [1 2])         ; => true (list or vector)
(coll? {:a 1})       ; => true (any collection)
(atom? (atom 0))     ; => true
(ref? (ref 0))       ; => true
(agent? (agent 0))   ; => true
(channel? (chan 1))  ; => true
```

**Missing:**
- ❌ `fn?` - Functions not runtime values yet

### Destructuring ✅ 75% Complete

#### Vector Destructuring
```clojure
(let [[a b c] [1 2 3]]
  (+ a b c))  ; => 6

(let [[a & rest] [1 2 3 4]]
  rest)  ; => [2 3 4]

(let [[a _ c] [1 2 3]]
  (+ a c))  ; => 4

;; Nested
(let [[x [y z]] [1 [2 3]]]
  (+ x y z))  ; => 6
```

#### Map Destructuring
```clojure
(let [{:keys [name age]} {:name "Alice" :age 30}]
  (str name " is " age))
```

**Missing:**
- ❌ `:as` aliasing
- ❌ `:or` defaults
- ❌ `:strs` / `:syms`

---

## FFI

### Rust FFI ✅ 110% Complete
Better than Clojure!

```clojure
;; In Clorus.toml
[dependencies.rust]
fs = { path = "crates/rust-fs" }

;; In code
(use rust.fs)
(fs/exists? "test.txt")
(fs/read "data.txt")
```

**Features:**
- ✅ Direct Rust library integration
- ✅ Automatic FFI generation (for the type shapes `docs/RUST_INTEROP_STATUS.md` lists as supported)
- ✅ Type marshalling (has real overhead, e.g. `String` round-trips via `CString` — not zero-cost)
- ✅ Memory safety (refcounting)

**Detailed docs:** `docs/RUST_INTEROP_STATUS.md`

---

## REPL

### REPL Features ✅ 95% Complete

```bash
clorus repl
```

**Features:**
- ✅ Tab autocomplete
- ✅ Command history (↑↓)
- ✅ Multiline editing
- ✅ Namespace awareness
- ✅ Value display (all types)
- ✅ Error reporting
- ✅ Project context
- ✅ Commands (`:help`, `:quit`, `:examples`)

**Missing:**
- ❌ GUI from REPL (macOS main thread limitation)
- ❌ `doc` function
- ❌ `source` function

**Detailed docs:** `docs/features/repl/REPL_FEATURES.md`

---

## Not Implemented

### Data Types
- ❌ Integers (i64)
- ❌ Characters
- ❌ Rationals
- ❌ BigInt/BigDecimal

### Advanced Features
- ❌ Transducers
- ❌ `deftype`
- ❌ Metadata (`^`)
- ❌ Dynamic vars
- ❌ `eval`
- ❌ Regular expressions
- ❌ core.async (have basic channels)
- ❌ clojure.spec

### Stdlib Gaps
Many can be implemented in pure Clorus:
- ❌ Full math library
- ❌ Date/time
- ❌ JSON/EDN parsing
- ❌ HTTP client
- ❌ Testing framework

---

## Documentation Index

### Implementation Complete Docs
- `COLLECTION_ACCESS_COMPLETE.md` - Collection operations
- `LAZY_SEQUENCES_COMPLETE.md` - Lazy sequence library
- `THREADING_MACROS_COMPLETE.md` - Threading macros
- `LOOP_AND_ATOMS_COMPLETE.md` - Loop/recur and atoms
- `REFS_STM_COMPLETE.md` - STM and refs
- `AGENTS_COMPLETE.md` - Agent system
- `CHANNELS_PHASE1_COMPLETE.md` - CSP channels
- `PHASE_C_COMPLETE.md` - Polymorphism system
- `STRING_OPERATIONS_COMPLETE.md` - String operations
- `NAMESPACE_VALIDATION_COMPLETE.md` - Namespace system

### Feature Tracking
- `LANGUAGE_PARITY.md` - Detailed feature comparison
- `FEATURE_MATRIX.md` - Feature matrix
- `ROADMAP_TO_100_PARITY.md` - Path to 100% parity

### Session Notes
- `sessions/SESSION_COLLECTIONS_LAZY_SEQUENCES.md` - Jan 29, 2026
- `sessions/SESSION_COMPLETE_2026_01_25.md` - Polymorphism
- All other `sessions/` docs

### Guides
- `guides/AUTOCOMPLETE_TEST_GUIDE.md` - REPL autocomplete

---

## Quick Start

### Installation
```bash
git clone https://github.com/prabhugopal/clorus
cd clorus
cargo build --release
```

### Hello World
```clojure
;; Create src/main.clrs
(ns my.app)

(defn -main []
  (println "Hello, Clorus!"))

;; Run
clorus run
```

### REPL
```bash
clorus repl
```

---

## Current Status Summary

✅ **Production-Ready For:**
- Data processing pipelines
- CLI tools and automation
- File manipulation
- Concurrent programs (atoms/refs/agents)
- Rust FFI wrappers
- Numeric computation

⚠️ **Not Quite Ready For:**
- Large-scale production systems
- Web services (no HTTP library)
- GUI applications (macOS REPL limitation)
- Ecosystem-dependent projects

**Estimated Timeline:**
- **Now**: Medium projects, serious experimentation
- **1-2 weeks**: Add missing stdlib functions
- **1-2 months**: Transducers, advanced features
- **6-12 months**: Ecosystem maturity

---

## Contributing

See implementation docs in `docs/` for adding new features.

Most stdlib functions can be implemented in pure Clorus!

---

**Last Updated:** January 29, 2026
**Contributors:** Prabhu Gopal, Claude Code
**License:** See LICENSE file
