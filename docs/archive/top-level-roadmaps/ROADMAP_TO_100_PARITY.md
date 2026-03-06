# Roadmap to 100% Clojure Parity

**Current Status:** ~20% feature complete
**Target:** 100% Clojure language parity
**Estimated Timeline:** 18-24 months
**Last Updated:** January 26, 2026

---

## Overview

This roadmap prioritizes features by impact and dependency order. Each phase builds on previous phases and delivers immediate value.

**Guiding Principles:**
1. **Value first** - Implement what makes the language usable
2. **Foundation first** - Core features before advanced ones
3. **Testing always** - Comprehensive tests for each feature
4. **Documentation always** - Doc strings and guides

---

## Phase 1: Critical Standard Library (4-6 weeks)

**Goal:** Make collections actually usable

**Status:** 🔴 Not Started
**Priority:** 🔥 CRITICAL
**Effort:** Medium

### 1.1: Collection Access (Week 1-2)

**Functions to implement:**

```clojure
; Core access
(get {:a 1 :b 2} :a)           ; => 1
(nth [10 20 30] 1)             ; => 20
(first [1 2 3])                ; => 1
(rest [1 2 3])                 ; => (2 3)
(last [1 2 3])                 ; => 3
(count [1 2 3])                ; => 3

; Predicates
(empty? [])                    ; => true
(nil? x)                       ; => true/false
(contains? {:a 1} :a)          ; => true
```

**Implementation:**
- `crates/clorus-runtime/src/collections.rs` - Runtime helpers
- `crates/clorus-codegen/src/stdlib/collections.rs` - Codegen integration
- `crates/clorus-core/src/collections.clrs` - Clorus implementations

**Tests:**
- Test all edge cases (empty collections, nil values, etc.)
- Performance benchmarks

---

### 1.2: Collection Modification (Week 2-3)

**Functions to implement:**

```clojure
; Add elements
(conj [1 2] 3)                 ; => [1 2 3]
(cons 0 [1 2 3])               ; => (0 1 2 3)

; Modify maps
(assoc {:a 1} :b 2)            ; => {:a 1 :b 2}
(dissoc {:a 1 :b 2} :b)        ; => {:a 1}
(update {:a 1} :a inc)         ; => {:a 2}

; Combine
(merge {:a 1} {:b 2})          ; => {:a 1 :b 2}
(concat [1 2] [3 4])           ; => (1 2 3 4)
```

**Implementation:**
- Persistent data structures (structural sharing)
- Copy-on-write for vectors/maps
- Reference counting integration

---

### 1.3: Higher-Order Functions (Week 3-4)

**Functions to implement:**

```clojure
; Core HOFs
(map inc [1 2 3])              ; => (2 3 4)
(filter even? [1 2 3 4])       ; => (2 4)
(reduce + [1 2 3 4])           ; => 10

; Additional
(remove odd? [1 2 3 4])        ; => (2 4)
(keep identity [1 nil 2 nil])  ; => (1 2)
(mapcat vec [[1 2] [3 4]])     ; => (1 2 3 4)
(partition 2 [1 2 3 4])        ; => ((1 2) (3 4))
```

**Implementation:**
- Anonymous function support required first!
- Lazy sequence foundation
- Memory-efficient iteration

---

### 1.4: I/O & Printing (Week 4-5)

**Functions to implement:**

```clojure
; Printing
(println "Hello" "World")      ; Prints with newline
(print "Count:" 42)            ; Prints without newline
(pr {:a 1})                    ; Print readably
(prn {:a 1})                   ; Print readably with newline

; String building
(str "Hello " "World")         ; => "Hello World"
(str 42)                       ; => "42"
(format "x=%d" 10)             ; => "x=10"

; Reading
(read-line)                    ; Read from stdin
(read-string "{:a 1}")         ; Parse string to data
```

**Implementation:**
- FFI to Rust's println! macro
- String concatenation with StringBuilder
- Format strings via Rust's format! macro

---

### 1.5: Predicates & Utilities (Week 5-6)

**Functions to implement:**

```clojure
; Type checks
(number? 42)                   ; => true
(string? "hi")                 ; => true
(vector? [1 2])                ; => true
(map? {:a 1})                  ; => true
(keyword? :name)               ; => true
(fn? println)                  ; => true

; Numeric predicates
(zero? 0)                      ; => true
(pos? 5)                       ; => true
(neg? -5)                      ; => true
(even? 4)                      ; => true
(odd? 3)                       ; => true

; Comparisons
(= 1 1)                        ; => true
(not= 1 2)                     ; => true
(identical? x y)               ; Same object
```

**Implementation:**
- Runtime type checking via ValueTag
- Numeric comparisons
- Reference equality checks

---

### Milestone 1 Deliverable

**By end of Phase 1, you can:**

```clojure
; Real-world example
(defn process-users [users]
  (let [adults (filter (fn [u] (> (:age u) 18)) users)
        names (map (fn [u] (:name u)) adults)]
    (println "Adult users:")
    (reduce (fn [acc name]
              (do (println "  -" name)
                  (inc acc)))
            0
            names)))

(def users [{:name "Alice" :age 25}
            {:name "Bob" :age 17}
            {:name "Carol" :age 30}])

(process-users users)
; Output:
; Adult users:
;   - Alice
;   - Carol
; => 2
```

---

## Phase 2: Core Language Features (6-8 weeks)

**Goal:** Complete core language constructs

**Status:** 🔴 Not Started
**Priority:** 🔥 HIGH
**Effort:** High

### 2.1: Anonymous Functions (Week 1-2)

**Syntax to implement:**

```clojure
; Long form
(fn [x] (* x 2))
(fn [x y] (+ x y))

; With name (for recursion)
(fn factorial [n]
  (if (< n 2) 1 (* n (factorial (- n 1)))))

; Short form (reader macro)
#(* % 2)
#(+ %1 %2)
```

**Implementation:**
- Update parser for `fn` special form
- Lambda capture in codegen
- LLVM function generation for lambdas
- Reader macro `#()` support

**Files to modify:**
- `crates/clorus-syntax/src/parser.rs`
- `crates/clorus-syntax/src/ast.rs` - Add `Fn` variant
- `crates/clorus-codegen/src/codegen.rs` - Lambda codegen

---

### 2.2: Loop/Recur (Week 2-3)

**Syntax to implement:**

```clojure
; Basic loop
(loop [i 0 acc 1]
  (if (< i 10)
    (recur (+ i 1) (* acc 2))
    acc))

; Function recursion
(defn factorial [n]
  (if (< n 2)
    1
    (recur (* n (factorial (- n 1))))))
```

**Implementation:**
- TCO (Tail Call Optimization)
- LLVM phi nodes for loop variables
- Loop detection in codegen
- Recur validation (only in tail position)

**Challenge:** LLVM doesn't have native TCO - need to transform to loops

---

### 2.3: Destructuring (Week 3-5)

**Syntax to implement:**

```clojure
; Vector destructuring
(let [[a b c] [1 2 3]]
  (+ a b c))  ; => 6

(let [[first & rest] [1 2 3 4]]
  rest)  ; => (2 3 4)

; Map destructuring
(let [{:keys [name age]} {:name "Alice" :age 30}]
  (str name " is " age))  ; => "Alice is 30"

(let [{n :name a :age} {:name "Bob" :age 25}]
  (str n " is " a))  ; => "Bob is 25"

; Nested
(let [{:keys [user]} {:user {:name "Alice" :age 30}}
      {name :name} user]
  name)  ; => "Alice"

; In function params
(defn greet [{:keys [name age]}]
  (str "Hello " name ", age " age))
```

**Implementation:**
- Parser extension for destructuring patterns
- AST representation of destructuring
- Codegen to lower destructuring to lets
- Support in let, defn, fn, loop

**Files:**
- `crates/clorus-syntax/src/parser.rs` - Parse destructuring
- `crates/clorus-syntax/src/ast.rs` - Add DestructurePattern
- `crates/clorus-codegen/src/destructure.rs` - New module

---

### 2.4: Control Flow (Week 5-6)

**Forms to implement:**

```clojure
; do - sequence expressions
(do
  (println "Step 1")
  (println "Step 2")
  42)  ; Returns last value

; when - if without else
(when (> x 5)
  (println "Greater than 5")
  x)

; cond - multiple conditions
(cond
  (< x 0) "negative"
  (= x 0) "zero"
  (> x 0) "positive"
  :else "unknown")

; case - constant matching
(case x
  1 "one"
  2 "two"
  3 "three"
  "other")
```

**Implementation:**
- Parser for each form
- AST variants
- Codegen with LLVM basic blocks
- Optimization for case (jump table)

---

### 2.5: Sets (Week 6-7)

**Syntax to implement:**

```clojure
; Set literals
#{1 2 3}

; Set operations
(conj #{1 2} 3)                ; => #{1 2 3}
(disj #{1 2 3} 2)              ; => #{1 3}
(contains? #{1 2 3} 2)         ; => true

; Set theory
(union #{1 2} #{2 3})          ; => #{1 2 3}
(intersection #{1 2} #{2 3})   ; => #{2}
(difference #{1 2 3} #{2})     ; => #{1 3}
```

**Implementation:**
- Runtime set data structure (HashSet)
- Parser for `#{}` literals
- Set operations via Rust FFI
- Reference counting for sets

**Files:**
- `crates/clorus-runtime/src/set.rs` - New module
- `crates/clorus-syntax/src/lexer.rs` - Recognize `#{`
- `crates/clorus-syntax/src/parser.rs` - Parse sets

---

### 2.6: String Operations (Week 7-8)

**Functions to implement:**

```clojure
; Core
(str "Hello " "World")         ; => "Hello World"
(subs "Hello" 1 4)             ; => "ell"
(count "Hello")                ; => 5

; Manipulation
(upper-case "hello")           ; => "HELLO"
(lower-case "HELLO")           ; => "hello"
(trim "  hi  ")                ; => "hi"
(replace "hello" "l" "L")      ; => "heLLo"

; Splitting/joining
(split "a,b,c" #",")           ; => ["a" "b" "c"]
(join "," ["a" "b" "c"])       ; => "a,b,c"

; Predicates
(starts-with? "hello" "he")    ; => true
(ends-with? "hello" "lo")      ; => true
(includes? "hello" "ll")       ; => true
```

**Implementation:**
- Rust String methods via FFI
- Regex support (via regex crate)
- Efficient string building

---

### Milestone 2 Deliverable

**By end of Phase 2, you can:**

```clojure
; Realistic application
(defn process-csv [filename]
  (let [content (slurp filename)
        lines (split content #"\n")
        rows (map #(split % #",") lines)
        data (map (fn [[name age]]
                    {:name name :age (parse-int age)})
                  rows)]
    (filter (fn [{:keys [age]}]
              (> age 18))
            data)))

(defn main []
  (let [adults (process-csv "users.csv")]
    (println "Found" (count adults) "adults:")
    (doseq [user adults]
      (println "  -" (:name user)))))
```

---

## Phase 3: User-Defined Macros (4-6 weeks)

**Goal:** Full macro system

**Status:** 🔴 Not Started
**Priority:** 🔥 HIGH
**Effort:** High

### 3.1: Syntax Quoting (Week 1-2)

**Syntax to implement:**

```clojure
; Quote
'(1 2 3)                       ; => (1 2 3)
(quote (+ 1 2))                ; => (+ 1 2)

; Syntax quote
`(1 2 3)                       ; => (1 2 3)
`(+ 1 ~x)                      ; Unquote x

; Unquote
(let [x 42]
  `(+ 1 ~x))                   ; => (+ 1 42)

; Unquote-splicing
(let [xs [2 3 4]]
  `(1 ~@xs 5))                 ; => (1 2 3 4 5)
```

**Implementation:**
- Reader-level syntax quoting
- Unquote evaluation during macro expansion
- Gensym for symbol generation
- Namespace qualification in syntax quote

**Challenge:** This is complex - needs reader modification

---

### 3.2: defmacro (Week 2-4)

**Syntax to implement:**

```clojure
; Simple macro
(defmacro unless [condition then else]
  `(if (not ~condition) ~then ~else))

(unless false "yes" "no")      ; => "yes"

; Variable arguments
(defmacro and* [& args]
  (if (empty? args)
    true
    `(if ~(first args)
       (and* ~@(rest args))
       false)))

; With destructuring
(defmacro with-open [[binding resource] & body]
  `(let [~binding ~resource]
     (try
       ~@body
       (finally (.close ~binding)))))
```

**Implementation:**
- Macro expansion phase before codegen
- Macro environment (separate from runtime)
- Recursive macro expansion
- Macro hygiene (gensym)

**Files:**
- `crates/clorus-syntax/src/macro_expander.rs` - Macro expansion engine
- `crates/clorus-syntax/src/ast.rs` - Add Defmacro
- `crates/clorus/src/macro_env.rs` - Macro environment

---

### 3.3: Reader Macros (Week 4-5)

**Syntax to implement:**

```clojure
; Function literal
#(* % 2)                       ; Same as (fn [x] (* x 2))
#(+ %1 %2)                     ; Same as (fn [x y] (+ x y))

; Regex
#"[a-z]+"                      ; Regex pattern

; Set
#{1 2 3}                       ; Set literal (already done)

; Var quote
#'my-var                       ; Get var object

; Discard
#_(+ 1 2)                      ; Ignore this form
```

**Implementation:**
- Reader dispatch table
- Transform reader macros to AST
- Integration with parser

---

### 3.4: Macro Utilities (Week 5-6)

**Functions to implement:**

```clojure
; Expansion
(macroexpand '(unless false 1 2))
; => (if (not false) 1 2)

(macroexpand-1 '(-> x f g))
; => (-> (f x) g)  ; One level only

; Debugging
(pprint (macroexpand '(-> x f g)))
; Pretty-print expanded form

; Generation
(gensym "x")                   ; => x__1234
```

---

### Milestone 3 Deliverable

**By end of Phase 3, you can:**

```clojure
; User-defined macros
(defmacro defroute [method path & body]
  `(register-handler
     ~method
     ~path
     (fn [req#]
       ~@body)))

(defroute :get "/api/users"
  (get-all-users))

(defmacro time [expr]
  `(let [start# (System/currentTimeMillis)]
     (let [result# ~expr]
       (println "Elapsed:" (- (System/currentTimeMillis) start#) "ms")
       result#)))

(time (expensive-computation))
```

---

## Phase 4: Concurrency (4-6 weeks)

**Goal:** Safe concurrent programming

**Status:** 🔴 Not Started
**Priority:** 🔥 MEDIUM-HIGH
**Effort:** High

### 4.1: Atoms (Week 1-2)

**API to implement:**

```clojure
; Create
(def counter (atom 0))
(def state (atom {:count 0 :name "App"}))

; Deref
@counter                       ; => 0
(deref counter)                ; => 0

; Modify
(swap! counter inc)            ; => 1
(swap! counter + 10)           ; => 11
(reset! counter 0)             ; => 0

; Compare-and-swap
(compare-and-set! counter 0 100)  ; => true

; Watches
(add-watch counter :logger
  (fn [key atom old new]
    (println "Changed from" old "to" new)))

(remove-watch counter :logger)

; Validators
(set-validator! counter pos?)
(swap! counter - 20)           ; Error: validation failed
```

**Implementation:**
- Rust `Arc<Mutex<Value>>` wrapper
- FFI functions for atom operations
- Lock-free updates where possible (CAS)
- Validation and watch infrastructure

**Files:**
- `crates/clorus-runtime/src/atom.rs` - New module
- `crates/clorus-core/src/atom.clrs` - Clorus wrappers

---

### 4.2: Refs & STM (Week 2-4)

**API to implement:**

```clojure
; Create refs
(def balance1 (ref 100))
(def balance2 (ref 200))

; Transactions
(dosync
  (alter balance1 - 50)
  (alter balance2 + 50))

; Read in transaction
(dosync
  (let [b1 @balance1
        b2 @balance2]
    (+ b1 b2)))

; Commute (order-independent)
(dosync
  (commute counter inc))

; Ensure (touch)
(dosync
  (ensure balance1)
  (alter balance2 + 100))
```

**Implementation:**
- Software Transactional Memory
- MVCC (Multi-Version Concurrency Control)
- Transaction log and commit protocol
- Retry logic on conflicts

**Challenge:** This is HARD - STM is complex

**Alternative:** Skip STM, just use atoms + locks

---

### 4.3: Agents (Week 4-5)

**API to implement:**

```clojure
; Create agent
(def logger (agent []))

; Send (async)
(send logger conj "Log entry 1")
(send logger conj "Log entry 2")

; Send-off (for blocking operations)
(send-off logger
  (fn [logs]
    (spit "log.txt" (str/join "\n" logs))
    logs))

; Wait
(await logger)
(await-for 1000 logger)

; Error handling
(set-error-handler! logger
  (fn [agent exception]
    (println "Error:" exception)))

(agent-error logger)
(restart-agent logger new-state)
```

**Implementation:**
- Thread pool for agent actions
- Queue per agent
- Error handling infrastructure
- FFI to Tokio runtime

---

### 4.4: Promises & Futures (Week 5-6)

**API to implement:**

```clojure
; Promise
(def p (promise))
(deliver p 42)
@p                             ; => 42

; Future
(def f (future
         (Thread/sleep 1000)
         (+ 1 2)))
@f                             ; => 3 (blocks until ready)

; Realized?
(realized? f)                  ; => true/false

; Deref with timeout
(deref f 500 :timeout)         ; Wait max 500ms
```

**Implementation:**
- Promise type with single-assignment
- Future backed by thread pool
- Blocking deref with timeout
- Integration with Tokio

---

### Milestone 4 Deliverable

**By end of Phase 4, you can:**

```clojure
; Concurrent web server
(def request-count (atom 0))
(def active-sessions (atom #{}))

(defn handle-request [req]
  (swap! request-count inc)
  (swap! active-sessions conj (:session req))

  (let [result (future (process-request req))]
    (send logger log-request req @result)
    @result))

; Concurrent data processing
(defn process-files [files]
  (let [results (map #(future (process-file %)) files)]
    (doall (map deref results))))
```

---

## Phase 5: Polymorphism (6-8 weeks)

**Goal:** Protocols and multimethods

**Status:** 🔴 Not Started
**Priority:** 🔥 MEDIUM
**Effort:** Very High

### 5.1: Protocols (Week 1-4)

**Syntax to implement:**

```clojure
; Define protocol
(defprotocol IDrawable
  "Things that can be drawn"
  (draw [this] "Draw the object")
  (bounds [this] "Get bounding box"))

; Implement for type
(defrecord Circle [x y radius]
  IDrawable
  (draw [this]
    (println "Circle at" x y))
  (bounds [this]
    {:x (- x radius) :y (- y radius)
     :width (* 2 radius) :height (* 2 radius)}))

(defrecord Rectangle [x y width height]
  IDrawable
  (draw [this]
    (println "Rectangle at" x y))
  (bounds [this]
    {:x x :y y :width width :height height}))

; Use
(def shapes [(Circle. 10 20 5)
             (Rectangle. 0 0 100 200)])

(doseq [shape shapes]
  (draw shape))

; Extend existing types
(extend-protocol IDrawable
  String
  (draw [s] (println s))
  (bounds [s] {:width (count s) :height 1}))
```

**Implementation:**
- Protocol registry (like Rust trait objects)
- Virtual dispatch table per protocol
- Type checking at runtime
- Method lookup optimization

**Files:**
- `crates/clorus-runtime/src/protocol.rs` - Protocol system
- `crates/clorus-syntax/src/ast.rs` - Protocol AST nodes
- `crates/clorus-codegen/src/protocol.rs` - Codegen

**Challenge:** This is very complex - requires runtime type system

---

### 5.2: Records (Week 4-6)

**Syntax to implement:**

```clojure
; Define record
(defrecord Person [name age email])

; Create instances
(def p (Person. "Alice" 30 "alice@example.com"))
(def p2 (->Person "Bob" 25 "bob@example.com"))
(def p3 (map->Person {:name "Carol" :age 35 :email "carol@example.com"}))

; Access fields
(:name p)                      ; => "Alice"
(get p :age)                   ; => 30
(:occupation p :unknown)       ; => :unknown (default)

; Modify (returns new instance)
(assoc p :age 31)              ; => Person with age=31
(dissoc p :email)              ; => Person without :email
(update p :age inc)            ; => Person with age=31

; As map
(keys p)                       ; => (:name :age :email)
(vals p)                       ; => ("Alice" 30 "alice@example.com")
(merge p {:age 32})            ; => Person with age=32
```

**Implementation:**
- Compile-time struct generation
- Fast field access (no hash lookup)
- Implements map interface
- Interop with Rust structs

---

### 5.3: Multimethods (Week 6-8)

**Syntax to implement:**

```clojure
; Define multimethod
(defmulti encounter
  "Handle encounters between game entities"
  (fn [entity1 entity2]
    [(:type entity1) (:type entity2)]))

; Define methods
(defmethod encounter [:player :monster]
  [player monster]
  (println "Player attacks monster!"))

(defmethod encounter [:monster :player]
  [monster player]
  (println "Monster attacks player!"))

(defmethod encounter [:player :treasure]
  [player treasure]
  (println "Player collects treasure!"))

(defmethod encounter :default
  [e1 e2]
  (println "Nothing happens"))

; Use
(encounter {:type :player} {:type :monster})
; => Player attacks monster!

; Hierarchies
(derive ::dog ::animal)
(derive ::cat ::animal)

(defmulti speak :type)
(defmethod speak ::animal [x]
  "Generic animal sound")
(defmethod speak ::dog [x]
  "Woof!")
```

**Implementation:**
- Dispatch function evaluation
- Method table per multimethod
- Hierarchy support
- Method precedence resolution

**Challenge:** Dynamic dispatch is complex

---

### Milestone 5 Deliverable

**By end of Phase 5, you can:**

```clojure
; Polymorphic application
(defprotocol IHandler
  (handle [this request]))

(defrecord GetHandler [db]
  IHandler
  (handle [this req]
    (db/query db (:query req))))

(defrecord PostHandler [db validator]
  IHandler
  (handle [this req]
    (when (validator (:body req))
      (db/insert db (:body req)))))

(defmulti route :method)

(defmethod route :get [req]
  (handle (GetHandler. db) req))

(defmethod route :post [req]
  (handle (PostHandler. db validate-user) req))
```

---

## Phase 6: Advanced Features (8-12 weeks)

**Goal:** Production-ready language

**Status:** 🔴 Not Started
**Priority:** 🟡 MEDIUM
**Effort:** Very High

### 6.1: Lazy Sequences (Week 1-3)

```clojure
; Infinite sequences
(def naturals (iterate inc 0))
(take 10 naturals)             ; => (0 1 2 3 4 5 6 7 8 9)

(def fibs
  (lazy-seq
    (cons 0 (cons 1 (map + fibs (rest fibs))))))

(take 10 fibs)                 ; => (0 1 1 2 3 5 8 13 21 34)

; Lazy operations
(take 5 (filter even? naturals))
; => (0 2 4 6 8)

; Chunked sequences (optimization)
(chunked-seq? (range 1000000))
```

**Implementation:**
- Lazy cons cells
- Memoization of realized values
- Chunking for performance

---

### 6.2: Transducers (Week 3-5)

```clojure
; Composable transformations
(def xf
  (comp
    (filter even?)
    (map inc)
    (take 5)))

(transduce xf conj [] (range 100))
; => [1 3 5 7 9]

; Reusable across contexts
(into [] xf (range 100))
(sequence xf (range 100))
(reduce (xf +) 0 (range 100))
```

---

### 6.3: Error Handling (Week 5-6)

```clojure
; Try/catch
(try
  (/ 1 0)
  (catch ArithmeticException e
    (println "Math error:" e))
  (catch Exception e
    (println "Error:" e))
  (finally
    (println "Cleanup")))

; Throwing
(throw (Exception. "Something went wrong"))

; Custom exceptions
(defn validate [x]
  (when (nil? x)
    (throw (IllegalArgumentException. "Cannot be nil"))))
```

---

### 6.4: Namespaced Keywords (Week 6-7)

```clojure
; Fully qualified keywords
:user/name
:db/id
::local-keyword               ; Expands to :current.ns/local-keyword

; In maps
{:user/name "Alice"
 :user/age 30
 :db/id 12345}

; Auto-qualification in syntax quote
`{:name "Alice"}              ; => {:current.ns/name "Alice"}
```

---

### 6.5: Metadata (Week 7-8)

```clojure
; Attach metadata
(def x ^{:doc "A special var"} 42)
(meta #'x)                     ; => {:doc "A special var"}

; Type hints
(defn add ^long [^long a ^long b]
  (+ a b))

; Dynamic vars
(def ^:dynamic *config* {})

(binding [*config* {:debug true}]
  (println *config*))
```

---

### 6.6: Spec (Week 9-12)

```clojure
; Define specs
(require '[clojure.spec.alpha :as s])

(s/def ::name string?)
(s/def ::age (s/and int? #(>= % 0)))
(s/def ::email (s/and string? #(re-matches #".+@.+" %)))

(s/def ::person
  (s/keys :req [::name ::age]
          :opt [::email]))

; Validation
(s/valid? ::person {:name "Alice" :age 30})  ; => true
(s/explain ::person {:name "Alice"})          ; Missing :age

; Function specs
(s/fdef user-name
  :args (s/cat :user ::person)
  :ret string?)

; Generative testing
(s/exercise ::person 5)
; Generates 5 random valid persons
```

---

### Milestone 6 Deliverable

**Production-ready Clorus with:**
- Full lazy sequence support
- Transducers for efficient transformations
- Proper error handling
- Spec for validation and testing
- All metadata support

---

## Phase 7: Ecosystem & Polish (Ongoing)

**Goal:** Production ecosystem

### 7.1: Standard Library Expansion

- Math functions (sin, cos, sqrt, pow, etc.)
- Regex support
- Date/time
- JSON/EDN parsing
- HTTP client
- File system utilities
- Process spawning

### 7.2: Async/Await Integration

```clojure
(defn async fetch-users []
  (let [response (await (http/get "https://api.example.com/users"))
        json (await (response/json))]
    json))

(tokio/run
  (let [users (await (fetch-users))]
    (println "Fetched" (count users) "users")))
```

### 7.3: Tooling

- LSP (Language Server Protocol)
- Formatter (like cljfmt)
- Linter (like clj-kondo)
- REPL enhancements
- Debugger integration
- Build tool improvements

### 7.4: Documentation

- Complete API reference
- Language guide
- Cookbook
- Migration guide (Clojure → Clorus)
- Performance guide
- Contribution guide

---

## Summary Timeline

| Phase | Duration | Cumulative | Features |
|-------|----------|------------|----------|
| **Phase 1** | 4-6 weeks | 1.5 months | Standard library basics |
| **Phase 2** | 6-8 weeks | 4 months | Core language features |
| **Phase 3** | 4-6 weeks | 6 months | User macros |
| **Phase 4** | 4-6 weeks | 7.5 months | Concurrency |
| **Phase 5** | 6-8 weeks | 10 months | Polymorphism |
| **Phase 6** | 8-12 weeks | 13 months | Advanced features |
| **Phase 7** | Ongoing | 18-24 months | Ecosystem |
| **Total** | **18-24 months** | - | **100% parity** |

---

## Success Criteria

**After each phase:**
- [ ] All features documented
- [ ] Comprehensive test coverage (>80%)
- [ ] Performance benchmarks
- [ ] Example applications
- [ ] Migration guide updates

**Final 100% parity checklist:**
- [ ] All Clojure data types
- [ ] All core forms
- [ ] Complete standard library (~600 functions)
- [ ] User-defined macros
- [ ] Full concurrency support
- [ ] Protocols and multimethods
- [ ] Lazy sequences
- [ ] Transducers
- [ ] Spec integration
- [ ] Production ecosystem

---

## Next Steps (Start Now!)

### Week 1-2: Collection Access

**Immediate tasks:**

1. **Create standard library foundation:**
   ```bash
   mkdir -p crates/clorus-stdlib/src
   cargo new --lib crates/clorus-stdlib
   ```

2. **Implement first 5 functions:**
   - `get` - Map/vector access
   - `nth` - Vector access by index
   - `first` - First element
   - `rest` - All but first
   - `count` - Collection size

3. **Write tests:**
   - Edge cases (empty, nil, out of bounds)
   - Performance tests
   - Integration tests

4. **Update REPL:**
   - Make functions available
   - Add autocomplete
   - Add documentation

**Want to start? I can help you implement the first function right now!**

Which would you like to begin with?
- `(get {:a 1} :a)` - Map access
- `(count [1 2 3])` - Collection size
- `(first [1 2 3])` - First element
- Or another function?
