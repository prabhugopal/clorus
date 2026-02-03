# Clorus Language Feature Matrix

**Based on:** Jank Programming Language Feature Checklist
**Reference:** https://jank-lang.org/progress/
**Last Updated:** January 26, 2026

---

## How to Read This Document

Each feature has 4 implementation phases:
- **Lex:** Lexical analysis (tokenization)
- **Parse:** Parsing to AST
- **Analyze:** Semantic analysis / type checking
- **Eval:** Code generation / evaluation

**Status Indicators:**
- ✅ **Done** - Fully implemented
- 🟨 **Partial** - Partially implemented
- ❌ **Missing** - Not implemented
- 🔵 **In Progress** - Currently being worked on
- ⚪ **N/A** - Not applicable for Clorus

---

## Core Data Types

### Primitive Types

| Feature | Lex | Parse | Analyze | Eval | Notes |
|---------|-----|-------|---------|------|-------|
| **nil** | ✅ | ✅ | ✅ | ✅ | Complete |
| **integers** | 🟨 | 🟨 | 🟨 | 🟨 | Only via f64, no i64/i32 |
| **reals** | ✅ | ✅ | ✅ | ✅ | f64 support complete |
| **bools** | ✅ | ✅ | ✅ | ✅ | `true` / `false` |
| **chars** | ❌ | ❌ | ❌ | ❌ | **Missing: `\a` `\newline`** |
| **strings** | ✅ | ✅ | ✅ | ✅ | String literals work |

**Priority:** 🔥 Add chars (low priority, rarely used)

---

### Keywords

| Feature | Lex | Parse | Analyze | Eval | Notes |
|---------|-----|-------|---------|------|-------|
| **keywords/unqualified** | ✅ | ✅ | ✅ | ✅ | `:name` - with interning |
| **keywords/qualified** | ❌ | ❌ | ❌ | ❌ | **Missing: `:user/name`** |
| **keywords/auto-resolved-unqualified** | ❌ | ❌ | ❌ | ❌ | **Missing: `::name`** |
| **keywords/auto-resolved-qualified** | ❌ | ❌ | ❌ | ❌ | **Missing: `::user/name`** |

**Priority:** 🔥 High - Qualified keywords needed for namespacing

**Example of missing:**
```clojure
; Currently works:
:name                          ; ✅ Unqualified

; Currently missing:
:user/name                     ; ❌ Qualified
::name                         ; ❌ Auto-resolved (expands to :current.ns/name)
::alias/name                   ; ❌ Auto-resolved qualified
```

---

### Collections

| Feature | Lex | Parse | Analyze | Eval | Notes |
|---------|-----|-------|---------|------|-------|
| **maps** | ✅ | ✅ | ✅ | ✅ | `{:a 1 :b 2}` literals work |
| **vectors** | ✅ | ✅ | ✅ | ✅ | `[1 2 3]` literals work |
| **sets** | ❌ | ❌ | ❌ | ❌ | **Missing: `#{1 2 3}`** |
| **lists** | ✅ | ✅ | ✅ | ✅ | `'(1 2 3)` work |

**Priority:** 🔥 High - Sets needed for set operations

---

### Other Types

| Feature | Lex | Parse | Analyze | Eval | Notes |
|---------|-----|-------|---------|------|-------|
| **symbols** | ✅ | ✅ | ✅ | ✅ | Variable names |
| **ratios** | ❌ | ❌ | ❌ | ❌ | **Missing: `22/7`** |

**Priority:** 🟡 Low - Ratios rarely used

---

## Special Forms

### Core Special Forms

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **def** | ✅ | ✅ | ✅ | ✅ | Complete | - |
| **if** | ✅ | ✅ | ✅ | ✅ | Complete | - |
| **do** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🔥 High |
| **let*** | ✅ | ✅ | ✅ | ✅ | Done as `let` | - |
| **quote** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🔥 High |
| **var** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Medium |

**do - Missing Example:**
```clojure
; Currently impossible:
(do
  (println "Step 1")
  (println "Step 2")
  42)  ; Returns last value
```

**quote - Missing Example:**
```clojure
; Currently impossible:
'(1 2 3)                       ; Returns list as data
(quote (+ 1 2))                ; Returns (+ 1 2) unevaluated
```

---

### Function Special Forms

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **fn*/base** | ❌ | ❌ | ❌ | ❌ | **CRITICAL** | 🔥🔥🔥 |
| **fn*/arities** | ❌ | ❌ | ❌ | ❌ | **CRITICAL** | 🔥🔥 |
| **fn*/variadic** | ❌ | ❌ | ❌ | ❌ | **CRITICAL** | 🔥🔥 |
| **fn*/recur** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🔥 High |

**This is the FOUNDATION - nothing works without fn!**

**fn*/base - Basic lambda:**
```clojure
(fn [x] (* x 2))               ; ❌ Not implemented!
((fn [x y] (+ x y)) 10 20)     ; ❌ Can't do this!
```

**fn*/arities - Multiple arities:**
```clojure
(fn
  ([x] (inc x))
  ([x y] (+ x y))
  ([x y z] (+ x y z)))         ; ❌ Not implemented!
```

**fn*/variadic - Variable arguments:**
```clojure
(fn [x & rest]
  (apply + x rest))            ; ❌ Not implemented!
```

---

### Loop/Recur

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **loop*** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🔥 High |
| **loop*/recur** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🔥 High |

**Example:**
```clojure
; Currently impossible:
(loop [i 0 acc 1]
  (if (< i 10)
    (recur (+ i 1) (* acc 2))
    acc))  ; ❌ Not implemented!
```

---

### Error Handling

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **throw** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Medium |
| **try** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Medium |

**Example:**
```clojure
; Currently impossible:
(try
  (/ 1 0)
  (catch Exception e
    (println "Error:" e))
  (finally
    (println "Cleanup")))  ; ❌ Not implemented!
```

---

### Advanced Special Forms

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **monitor-enter** | ⚪ | ⚪ | ⚪ | ⚪ | N/A (Rust threads) | - |
| **monitor-exit** | ⚪ | ⚪ | ⚪ | ⚪ | N/A (Rust threads) | - |
| **set!** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Low |
| **case*** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Medium |
| **letfn*** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Low |

---

## Language Features

### Bindings & Calls

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **bindings/thread-local** | ✅ | ✅ | ✅ | ✅ | Done via let | - |
| **bindings/conveyance** | ✅ | ✅ | ✅ | ✅ | Done via closures | - |
| **calls** | ✅ | ✅ | ✅ | ✅ | Function calls work | - |
| **destructuring** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🔥 High |

**Destructuring - Missing Example:**
```clojure
; Currently impossible:
(let [[a b c] [1 2 3]]
  (+ a b c))  ; ❌ Vector destructuring

(let [{:keys [name age]} {:name "Alice" :age 30}]
  (str name " " age))  ; ❌ Map destructuring
```

---

### Macros

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **macros** | 🟨 | 🟨 | 🟨 | 🟨 | Only `->` and `->>` | 🔥🔥 |
| **macros/&env param** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Low |
| **syntax-quoting** | ❌ | ❌ | ⚪ | ⚪ | **MISSING** | 🔥🔥 |
| **syntax-quoting/unquote** | ❌ | ❌ | ⚪ | ⚪ | **MISSING** | 🔥🔥 |

**What's missing:**
```clojure
; We have:
(-> 10 (+ 5) (* 2))            ; ✅ Thread-first
(->> 2 (+ 10) (* 5))           ; ✅ Thread-last

; We DON'T have:
(defmacro unless [cond then else]
  `(if (not ~cond) ~then ~else))  ; ❌ Can't define macros!

(unless false "yes" "no")      ; ❌ Not possible!
```

---

### Meta & Hints

| Feature | Lex | Parse | Analyze | Eval | Status | Priority |
|---------|-----|-------|---------|------|--------|----------|
| **meta hints** | ❌ | ❌ | ❌ | ❌ | **MISSING** | 🟡 Low |

**Example:**
```clojure
; Currently impossible:
(defn ^:private my-fn [x] x)   ; ❌ Metadata
(def ^long x 42)               ; ❌ Type hints
```

---

## Reader Macros

| Feature | Lex | Parse | Status | Priority | Example |
|---------|-----|-------|--------|----------|---------|
| **comment** | ✅ | ✅ | Done | - | `; comment` |
| **set** | ❌ | ❌ | **MISSING** | 🔥 High | `#{1 2 3}` |
| **shorthand fns** | ❌ | ❌ | **MISSING** | 🔥🔥 | `#(* % 2)` |
| **regex** | ❌ | ❌ | **MISSING** | 🟡 Medium | `#"[a-z]+"` |
| **deref** | ❌ | ❌ | **MISSING** | 🔥 High | `@atom` |
| **quote** | ❌ | ❌ | **MISSING** | 🔥 High | `'(1 2 3)` |
| **var quoting** | ❌ | ❌ | **MISSING** | 🟡 Low | `#'my-var` |
| **conditional** | ❌ | ❌ | **MISSING** | 🟡 Low | `#?(:clj ...)` |
| **tagged-literal** | ❌ | ❌ | **MISSING** | 🟡 Low | `#inst "..."` |

**Priority Examples:**

**🔥🔥 CRITICAL - Shorthand fns:**
```clojure
#(* % 2)                       ; ❌ Must implement!
; Same as: (fn [x] (* x 2))
```

**🔥 HIGH - Deref:**
```clojure
(def counter (atom 0))
@counter                       ; ❌ Must implement!
; Same as: (deref counter)
```

**🔥 HIGH - Quote:**
```clojure
'(1 2 3)                       ; ❌ Must implement!
; Same as: (quote (1 2 3))
```

---

## Summary Statistics

### Overall Completion

| Category | Total Features | Complete | Partial | Missing | % Complete |
|----------|----------------|----------|---------|---------|------------|
| **Data Types** | 12 | 7 | 1 | 4 | 58% |
| **Special Forms** | 17 | 3 | 0 | 14 | 18% |
| **Language Features** | 7 | 3 | 1 | 3 | 43% |
| **Reader Macros** | 9 | 1 | 0 | 8 | 11% |
| **TOTAL** | 45 | 14 | 2 | 29 | **31%** |

---

## Critical Path Analysis

### Layer 0: ABSOLUTE FOUNDATION (Must Do First!)

| Feature | Status | Blocks | Priority |
|---------|--------|--------|----------|
| **fn*/base** | ❌ | Everything! | 🔥🔥🔥 |
| **reader/shorthand fns** | ❌ | HOFs | 🔥🔥🔥 |
| **do** | ❌ | Multi-expr | 🔥🔥 |
| **quote** | ❌ | Macros | 🔥🔥 |

**Why fn is critical:**
- Can't do `map`, `filter`, `reduce` without lambdas
- Can't pass functions as values
- Can't write higher-order functions
- 90% of functional programming depends on this!

---

### Layer 1: Collections Functional

| Feature | Status | Depends On | Priority |
|---------|--------|------------|----------|
| **sets** | ❌ | - | 🔥 |
| **destructuring** | ❌ | fn | 🔥 |
| **loop*/recur** | ❌ | fn | 🔥 |

---

### Layer 2: Macros

| Feature | Status | Depends On | Priority |
|---------|--------|------------|----------|
| **syntax-quoting** | ❌ | quote | 🔥🔥 |
| **defmacro** | ❌ | syntax-quote | 🔥🔥 |
| **unquote** | ❌ | syntax-quote | 🔥🔥 |

---

### Layer 3: Advanced

| Feature | Status | Depends On | Priority |
|---------|--------|------------|----------|
| **try/catch** | ❌ | - | 🟡 |
| **case*** | ❌ | - | 🟡 |
| **qualified keywords** | ❌ | - | 🔥 |
| **deref reader** | ❌ | atom | 🔥 |

---

## Implementation Priority Queue

### Immediate (Week 1-4)

1. ✅ **fn*/base** - Anonymous functions
2. ✅ **reader/shorthand fns** - `#()`
3. ✅ **do** - Multiple expressions
4. ✅ **quote** - Quoted forms

**After this:** Can write functional code!

### Short-term (Week 5-8)

5. ✅ **sets** - `#{1 2 3}`
6. ✅ **loop*/recur** - Tail recursion
7. ✅ **destructuring** - Vector & map
8. ✅ **keywords/qualified** - `:ns/name`

**After this:** Have complete language!

### Medium-term (Week 9-16)

9. ✅ **syntax-quoting** - `` ` `` and `~`
10. ✅ **defmacro** - User macros
11. ✅ **fn*/arities** - Multiple arities
12. ✅ **fn*/variadic** - `&` rest args

**After this:** Full macro system!

### Long-term (Week 17+)

13. ✅ **try/catch/finally** - Error handling
14. ✅ **case*** - Pattern matching
15. ✅ **letfn*** - Mutual recursion
16. ✅ **meta hints** - Type hints

---

## Feature Dependencies Graph

```
fn*/base (FOUNDATION)
  ├─→ reader/shorthand fns (#())
  ├─→ HOFs (map, filter, reduce)
  ├─→ fn*/arities
  ├─→ fn*/variadic
  └─→ fn*/recur
      └─→ loop*/recur

quote
  └─→ syntax-quoting
      └─→ unquote
          └─→ defmacro

do
  └─→ multiple expressions anywhere

sets
  └─→ set operations (union, intersection, etc.)

destructuring
  ├─→ (depends on fn)
  └─→ makes everything easier

qualified keywords
  └─→ proper namespacing
```

---

## Next 4 Weeks Plan

### Week 1: fn*/base

**Goal:** Implement anonymous functions

**Tasks:**
- [ ] Update AST with `Fn` variant
- [ ] Parser for `(fn [params] body)`
- [ ] Codegen for lambdas (LLVM)
- [ ] Closure capture
- [ ] Tests

**Deliverable:**
```clojure
(fn [x] (* x 2))               ; Works!
((fn [x y] (+ x y)) 10 20)     ; => 30
```

---

### Week 2: reader/shorthand fns + do

**Goal:** Shorthand syntax + multiple expressions

**Tasks:**
- [ ] Reader macro `#()` → `(fn [%] ...)`
- [ ] Parse `do` form
- [ ] Codegen for `do`
- [ ] Tests

**Deliverable:**
```clojure
#(* % 2)                       ; Works!
(do
  (println "Step 1")
  (println "Step 2")
  42)                          ; => 42
```

---

### Week 3: quote + sets

**Goal:** Quoted forms and set literals

**Tasks:**
- [ ] Parser for `quote` / `'`
- [ ] AST for quoted forms
- [ ] Lexer for `#{`
- [ ] Parser for sets
- [ ] Runtime set type
- [ ] Tests

**Deliverable:**
```clojure
'(1 2 3)                       ; Works!
#{1 2 3}                       ; Works!
```

---

### Week 4: Integration + Testing

**Goal:** Make everything work together

**Tasks:**
- [ ] Comprehensive tests
- [ ] REPL integration
- [ ] Documentation
- [ ] Examples

**Deliverable:** Can write real functional code!

```clojure
(def nums [1 2 3 4 5])

; NOW THIS WORKS:
(map #(* % 2) nums)            ; => (2 4 6 8 10)
(filter #(> % 2) nums)         ; => (3 4 5)
(reduce + 0 nums)              ; => 15
```

---

## Tracking Progress

### Create GitHub Issues

For each feature, create an issue:

```markdown
Title: [Layer 0] Implement fn*/base (anonymous functions)

Description:
Implement basic anonymous function support.

Acceptance Criteria:
- [ ] Lex: Recognize `fn` keyword
- [ ] Parse: Parse `(fn [params] body)`
- [ ] Analyze: Build AST with Fn variant
- [ ] Eval: Generate LLVM IR for lambdas
- [ ] Test: All test cases pass

References:
- Jank: https://jank-lang.org/progress/
- Clojure spec: https://clojure.org/reference/special_forms#fn

Priority: 🔥🔥🔥 CRITICAL
```

### Progress Dashboard

Create `docs/FEATURE_PROGRESS.md`:

```markdown
# Feature Implementation Progress

Last updated: 2026-01-26

## Layer 0: Foundation (0/4)
- [ ] fn*/base
- [ ] reader/shorthand fns
- [ ] do
- [ ] quote

## Layer 1: Collections (0/4)
- [ ] sets
- [ ] loop*/recur
- [ ] destructuring
- [ ] keywords/qualified
```

---

## References

- **Jank Progress:** https://jank-lang.org/progress/
- **Clojure Reference:** https://clojure.org/reference
- **Clojure Special Forms:** https://clojure.org/reference/special_forms
- **ClojureScript Reader:** https://github.com/clojure/clojurescript/blob/master/src/main/clojure/cljs/reader.clj

---

## Questions to Consider

1. **Integer types:** Do we want i64/i32 or just f64?
   - Clojure has full integer support
   - Clorus currently only has f64
   - Decision: Add i64 for interop with Rust

2. **Ratios:** Do we need `22/7`?
   - Rarely used in practice
   - Decision: Low priority, maybe never

3. **Characters:** Do we need `\a` `\newline`?
   - Needed for full Clojure compat
   - Decision: Low priority, add eventually

4. **Monitor primitives:** Java-specific?
   - Clorus uses Rust concurrency
   - Decision: Skip, use Rust primitives

---

## Success Criteria

**By Week 4:**
- [ ] Can write lambda functions
- [ ] Can use `map`, `filter`, `reduce`
- [ ] Can write functional pipelines
- [ ] Can use sets
- [ ] Can quote data

**By Week 12:**
- [ ] Have all critical features (Layer 0-2)
- [ ] Can define macros
- [ ] Can write complex applications
- [ ] Have comprehensive standard library

**By Week 24:**
- [ ] 80%+ Clojure feature parity
- [ ] Production-ready
- [ ] Comprehensive documentation

---

**Next Action:** Start implementing `fn*/base` (Week 1)!
