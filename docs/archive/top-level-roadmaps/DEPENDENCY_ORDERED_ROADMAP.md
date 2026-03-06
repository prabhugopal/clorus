# Dependency-Ordered Roadmap to 100% Parity

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/ROADMAP_TO_100_PARITY.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Philosophy:** Build each layer ONCE on solid foundation. Never go backwards.

**Last Updated:** January 26, 2026

---

## Dependency Analysis

The key insight: **Some features are FOUNDATIONS that everything else builds on.**

### Critical Dependencies

```
Layer 0 (NOTHING DEPENDS ON THESE):
  → Anonymous functions (fn)
  → I/O (println)
  → String concatenation (str)

Layer 1 (DEPENDS ON Layer 0):
  → Collection access (get, nth, first, rest, count)
  → Higher-order functions (map, filter, reduce)

Layer 2 (DEPENDS ON Layer 1):
  → Collection modification (conj, assoc, dissoc)
  → Loop/recur
  → Destructuring

Layer 3 (DEPENDS ON Layer 2):
  → Sets
  → Control flow (cond, when, case)
  → String operations (split, join, etc.)

Layer 4+ (DEPENDS ON Layer 3):
  → Macros (defmacro)
  → Concurrency (atom, ref, agent)
  → Polymorphism (protocols, multimethods)
```

**The problem with the previous roadmap:** It tried to do collections without `fn`, which makes HOFs impossible!

---

## Layer 0: Absolute Foundation (Week 1-3)

**These are the BEDROCK. Everything builds on these.**

### L0.1: Anonymous Functions (Week 1)

**WHY FIRST:** You can't do `map`, `filter`, or anything functional without lambdas!

```clojure
; This is impossible without fn:
(map (fn [x] (* x 2)) [1 2 3])

; Currently in Clorus: ERROR!
```

**Implementation:**
- Parser: Recognize `(fn [params] body)`
- AST: Add `Fn { params, body }` variant
- Codegen: LLVM lambda generation
- Closure support: Capture environment

**Files to create/modify:**
1. `crates/clorus-syntax/src/ast.rs`:
   ```rust
   Fn {
       params: Vec<String>,
       body: Box<Expr>,
   }
   ```

2. `crates/clorus-syntax/src/parser.rs`:
   ```rust
   fn parse_fn(&mut self) -> Result<Expr, String> {
       // (fn [x y] (+ x y))
   }
   ```

3. `crates/clorus-codegen/src/lambda.rs` (NEW):
   ```rust
   pub fn compile_lambda(&mut self, params: &[String], body: &Expr)
       -> Result<PointerValue<'ctx>, String>
   ```

**Test criteria:**
```clojure
; All these must work:
(fn [x] x)                     ; Identity
(fn [x y] (+ x y))             ; Multiple params
((fn [x] (* x 2)) 5)           ; Immediate call => 10

; Closure:
(let [y 10]
  ((fn [x] (+ x y)) 5))        ; => 15
```

**Deliverable:** Can create and call anonymous functions

---

### L0.2: Basic I/O (Week 2)

**WHY SECOND:** Can't debug without printing!

```clojure
; Currently impossible:
(println "x =" x)              ; ERROR: println not defined
```

**Implementation:**
- Rust FFI to `println!` macro
- Variadic argument support
- String conversion for all types

**Files to create:**
1. `crates/clorus-core/src/io.rs` (Rust side):
   ```rust
   #[no_mangle]
   pub extern "C" fn clorus_println(args: *mut *mut Value, len: usize) {
       // Convert each Value* to string, print with newline
   }

   #[no_mangle]
   pub extern "C" fn clorus_print(args: *mut *mut Value, len: usize) {
       // Same but no newline
   }
   ```

2. `crates/clorus-codegen/src/codegen.rs`:
   ```rust
   // In compile_expr, handle:
   "println" => self.compile_println(args),
   "print" => self.compile_print(args),
   ```

**Test criteria:**
```clojure
(println "Hello")              ; Prints: Hello\n
(println "x =" 42)             ; Prints: x = 42\n
(print "No newline")           ; Prints: No newline
```

**Deliverable:** Can print for debugging

---

### L0.3: String Concatenation (Week 3)

**WHY THIRD:** Every other feature needs string building!

```clojure
; Currently impossible:
(str "Hello " "World")         ; ERROR: str not defined
(str "Count: " 42)             ; ERROR
```

**Implementation:**
- Variadic `str` function
- Convert all types to strings
- Efficient string building (StringBuilder)

**Files to create:**
1. `crates/clorus-core/src/string.rs`:
   ```rust
   #[no_mangle]
   pub extern "C" fn clorus_str(args: *mut *mut Value, len: usize) -> *mut Value {
       let mut result = String::new();
       for i in 0..len {
           let val = unsafe { *args.add(i) };
           result.push_str(&value_to_string(val));
       }
       Value::string(&result)
   }
   ```

**Test criteria:**
```clojure
(str "Hello")                  ; => "Hello"
(str "Hello " "World")         ; => "Hello World"
(str "x=" 42)                  ; => "x=42"
(str)                          ; => ""
```

**Deliverable:** Can build strings from any values

---

## Layer 1: Collections Usable (Week 4-6)

**NOW we can build on the foundation!**

### L1.1: Collection Access (Week 4)

**Dependencies:** ✅ `fn` (for error handlers)

```clojure
(get {:a 1 :b 2} :a)           ; => 1
(nth [10 20 30] 1)             ; => 20
(first [1 2 3])                ; => 1
(rest [1 2 3])                 ; => (2 3)
(last [1 2 3])                 ; => 3
(count [1 2 3])                ; => 3
```

**Implementation:**
1. `crates/clorus-runtime/src/collections.rs`:
   ```rust
   #[no_mangle]
   pub extern "C" fn clorus_get(coll: *mut Value, key: *mut Value) -> *mut Value

   #[no_mangle]
   pub extern "C" fn clorus_nth(vec: *mut Value, n: i64) -> *mut Value

   #[no_mangle]
   pub extern "C" fn clorus_first(coll: *mut Value) -> *mut Value

   #[no_mangle]
   pub extern "C" fn clorus_rest(coll: *mut Value) -> *mut Value

   #[no_mangle]
   pub extern "C" fn clorus_count(coll: *mut Value) -> i64
   ```

**Test criteria:**
```clojure
; Maps
(get {:a 1} :a)                ; => 1
(get {:a 1} :b)                ; => nil
(get {:a 1} :b :default)       ; => :default

; Vectors
(nth [10 20 30] 0)             ; => 10
(nth [10 20 30] 5)             ; => nil (or error?)
(first [1 2 3])                ; => 1
(first [])                     ; => nil
(rest [1 2 3])                 ; => (2 3)
(rest [1])                     ; => ()
(count [1 2 3])                ; => 3
(count {})                     ; => 0
```

---

### L1.2: Higher-Order Functions (Week 5-6)

**Dependencies:** ✅ `fn`, ✅ collection access

**NOW these are possible!**

```clojure
(map (fn [x] (* x 2)) [1 2 3])           ; => (2 4 6)
(filter (fn [x] (> x 2)) [1 2 3 4])      ; => (3 4)
(reduce (fn [acc x] (+ acc x)) 0 [1 2 3]) ; => 6
```

**Implementation:**
1. `crates/clorus-core/src/hof.clrs` (PURE CLORUS!):
   ```clojure
   (defn map [f coll]
     (if (empty? coll)
       ()
       (cons (f (first coll))
             (map f (rest coll)))))

   (defn filter [pred coll]
     (if (empty? coll)
       ()
       (let [x (first coll)
             xs (rest coll)]
         (if (pred x)
           (cons x (filter pred xs))
           (filter pred xs)))))

   (defn reduce [f init coll]
     (if (empty? coll)
       init
       (reduce f (f init (first coll)) (rest coll))))
   ```

**NOTE:** These are RECURSIVE! Need tail-call optimization soon.

**Test criteria:**
```clojure
(map (fn [x] (+ x 1)) [1 2 3])           ; => (2 3 4)
(filter (fn [x] (= (% x 2) 0)) [1 2 3 4]) ; => (2 4)
(reduce (fn [a x] (+ a x)) 0 [1 2 3 4])  ; => 10

; Edge cases
(map (fn [x] x) [])                      ; => ()
(filter (fn [x] true) [])                ; => ()
(reduce (fn [a x] (+ a x)) 100 [])       ; => 100
```

---

## Layer 2: Language Completeness (Week 7-12)

### L2.1: Collection Modification (Week 7-8)

**Dependencies:** ✅ collection access, ✅ HOFs

```clojure
(conj [1 2] 3)                 ; => [1 2 3]
(assoc {:a 1} :b 2)            ; => {:a 1 :b 2}
(dissoc {:a 1 :b 2} :b)        ; => {:a 1}
(update {:a 1} :a (fn [x] (+ x 1))) ; => {:a 2}
```

**Implementation:**
- Persistent data structures (copy-on-write)
- Structural sharing for efficiency
- Reference counting integration

**Files:**
1. `crates/clorus-runtime/src/persistent.rs`:
   ```rust
   #[no_mangle]
   pub extern "C" fn clorus_conj(coll: *mut Value, item: *mut Value) -> *mut Value

   #[no_mangle]
   pub extern "C" fn clorus_assoc(map: *mut Value, key: *mut Value, val: *mut Value) -> *mut Value
   ```

---

### L2.2: Loop/Recur (Week 9-10)

**Dependencies:** ✅ `fn`, ✅ collection ops

**WHY NOW:** Replace recursion with efficient loops

```clojure
; Before (recursive, stack overflow risk):
(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

; After (tail-call optimized):
(defn factorial [n]
  (loop [i n acc 1]
    (if (< i 2)
      acc
      (recur (- i 1) (* acc i)))))
```

**Implementation:**
- Transform `loop` into LLVM loop construct
- Verify `recur` only in tail position
- Phi nodes for loop variables

---

### L2.3: Destructuring (Week 10-11)

**Dependencies:** ✅ collection access

```clojure
; Vector destructuring
(let [[a b c] [1 2 3]]
  (+ a b c))                   ; => 6

; Map destructuring
(let [{:keys [name age]} {:name "Alice" :age 30}]
  (str name " is " age))       ; => "Alice is 30"
```

---

### L2.4: Control Flow (Week 11-12)

**Dependencies:** ✅ `if` (already have)

```clojure
(when (> x 5)
  (println "Greater than 5")
  x)

(cond
  (< x 0) "negative"
  (= x 0) "zero"
  (> x 0) "positive")

(case x
  1 "one"
  2 "two"
  "other")
```

---

## Layer 3: Production Features (Week 13-20)

### L3.1: Sets (Week 13-14)

**Dependencies:** ✅ collection ops

```clojure
#{1 2 3}
(conj #{1 2} 3)
(disj #{1 2 3} 2)
```

---

### L3.2: String Operations (Week 14-16)

**Dependencies:** ✅ `str`

```clojure
(upper-case "hello")
(split "a,b,c" #",")
(join "," ["a" "b"])
```

---

### L3.3: Reader Macros (Week 16-17)

**Dependencies:** ✅ `fn`

```clojure
#(* % 2)                       ; => (fn [x] (* x 2))
```

---

### L3.4: Predicates (Week 17-18)

**Dependencies:** ✅ collection access

```clojure
(nil? x)
(empty? coll)
(even? n)
(number? x)
```

---

### L3.5: More Collection Functions (Week 18-20)

**Dependencies:** ✅ All Layer 2

```clojure
(take 5 [1 2 3 4 5 6 7])
(drop 3 [1 2 3 4 5])
(partition 2 [1 2 3 4])
(group-by (fn [x] (% x 2)) [1 2 3 4])
```

---

## Layer 4: Advanced Language (Week 21-32)

### L4.1: defmacro (Week 21-25)

**Dependencies:** ✅ Syntax quoting, ✅ All core features

```clojure
(defmacro unless [condition then else]
  `(if (not ~condition) ~then ~else))
```

---

### L4.2: Concurrency - Atoms (Week 26-28)

**Dependencies:** ✅ `fn` (for swap!)

```clojure
(def counter (atom 0))
(swap! counter inc)
```

---

### L4.3: Protocols (Week 29-32)

**Dependencies:** ✅ Records, ✅ `defn`

```clojure
(defprotocol IDrawable
  (draw [this]))
```

---

## Layer 5: Ecosystem (Week 33+)

### L5.1: Lazy Sequences
### L5.2: Transducers
### L5.3: Spec
### L5.4: Async/Await

---

## Critical Path: What to Build RIGHT NOW

**Week 1-3: Foundation**
1. ✅ Anonymous functions (`fn`)
2. ✅ I/O (`println`, `print`)
3. ✅ String concat (`str`)

**Week 4-6: Collections Usable**
4. ✅ Collection access (`get`, `nth`, `first`, `rest`, `count`)
5. ✅ HOFs (`map`, `filter`, `reduce`)

**Week 7-12: Complete Language**
6. ✅ Collection modification (`conj`, `assoc`, etc.)
7. ✅ Loop/recur
8. ✅ Destructuring
9. ✅ Control flow

**After this (Week 12):** You have a COMPLETE, USABLE language!

---

## Success Metrics

**After Week 3:** Can write simple functions with lambdas
**After Week 6:** Can process collections functionally
**After Week 12:** Can write real applications
**After Week 20:** Production-ready standard library
**After Week 32:** Full Clojure parity (core language)

---

## The Smart Start: Week 1 Task List

### Day 1-2: Anonymous Function Parser

**Task:** Parse `(fn [x] body)`

**File:** `crates/clorus-syntax/src/parser.rs`

```rust
// Add after defn parsing:
if symbol == "fn" {
    return self.parse_fn();
}

fn parse_fn(&mut self) -> Result<Expr, String> {
    // (fn [params] body)
    self.expect(Token::LBracket)?;
    let params = self.parse_param_list()?;
    self.expect(Token::RBracket)?;
    let body = Box::new(self.parse_expr()?);
    self.expect(Token::RParen)?;

    Ok(Expr::Fn { params, body })
}
```

### Day 3-4: Lambda Codegen

**Task:** Generate LLVM IR for lambdas

**File:** `crates/clorus-codegen/src/lambda.rs` (new)

```rust
impl<'ctx> CodeGen<'ctx> {
    pub fn compile_lambda(&mut self, params: &[String], body: &Expr)
        -> Result<PointerValue<'ctx>, String>
    {
        // 1. Create function type
        // 2. Generate function
        // 3. Compile body
        // 4. Return function pointer
    }
}
```

### Day 5: Testing

**Test:** All lambda cases work

```clojure
(fn [x] x)                     ; Identity
((fn [x] (* x 2)) 5)           ; => 10
(let [f (fn [x y] (+ x y))]
  (f 10 20))                   ; => 30
```

**Days 6-7: Buffer/weekend**

---

## Next Action

**RIGHT NOW, let's start with Day 1-2: Anonymous Function Parser**

I can help you:
1. Update the AST with `Fn` variant
2. Update the parser to recognize `fn` forms
3. Write tests

**Ready to start? Let's implement `fn` together!**

Which file should we start with?
- `crates/clorus-syntax/src/ast.rs` (add Fn variant)
- `crates/clorus-syntax/src/parser.rs` (parse fn)
- Tests first (TDD style)
