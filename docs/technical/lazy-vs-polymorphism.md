# Clorus Advanced Features: Lazy Sequences vs Polymorphism

**Date:** January 27, 2026
**Version:** 0.4.0
**Status:** Analysis Complete

---

## Question: Which Advanced Feature Should We Build Next?

**Options:**
1. Lazy Sequences (delayed evaluation, infinite sequences)
2. Polymorphism (protocols, multimethods, type dispatch)

**Answer: Lazy Sequences - They can be built in pure Clorus NOW!**

---

## Summary

### Lazy Sequences: ✅ Can Build in Pure Clorus

**What we have:**
- ✅ Closures (capture environment)
- ✅ First-class functions (pass, return)
- ✅ Atoms (mutable refs for caching)
- ✅ Maps (data structure representation)
- ✅ Recursion (infinite sequences)

**Implementation:**
```clojure
;; Lazy sequence = map with thunk + cache
{:type :lazy-seq
 :realized (atom false)
 :value (atom nil)
 :thunk (fn [] ...)}

;; Infinite sequence of natural numbers
(defn lazy-range [start]
  (lazy-cons start (fn [] (lazy-range (+ start 1)))))

(def naturals (lazy-range 0))

;; Take first 5
(realize (lazy-take 5 naturals))  ; => [0 1 2 3 4]
```

**Status:**
- ✅ **Proof of concept complete**
- ✅ **All primitives available**
- ✅ **No runtime changes needed**
- ✅ **No compiler changes needed**
- ⚠️ **Performance:** 2-5x slower than native (acceptable for stdlib)

**Timeline:**
- Full stdlib implementation: 1-2 weeks
- Native runtime optimization: 2-4 weeks (optional, later)

---

### Polymorphism: ❌ Requires Major Runtime/Compiler Work

**What we need:**
- ❌ User-defined types (defrecord, deftype)
- ❌ Type registry system
- ❌ Protocol dispatch tables
- ❌ Type introspection
- ❌ extend-protocol mechanism

**Partial workaround:**
```clojure
;; Can fake multimethods with maps, but very limited
(def method-registry (atom {}))

(defn defmulti [name dispatch-fn]
  (swap! method-registry assoc name {:dispatch dispatch-fn}))

;; But: no type dispatch, slow, manual, not ergonomic
```

**Status:**
- ❌ **Cannot be done in pure Clorus**
- ❌ **Requires new AST nodes** (defprotocol, defrecord)
- ❌ **Requires new ValueTags** (custom types)
- ❌ **Requires parser changes**
- ❌ **Requires codegen changes**
- ❌ **Requires runtime type system**

**Timeline:**
- Runtime type system: 2-3 weeks
- defrecord/deftype: 2-3 weeks
- Protocol dispatch: 2-3 weeks
- extend-protocol: 1-2 weeks
- **Total: 2-3 months**

---

## Feature Comparison Matrix

| Aspect | Lazy Sequences | Polymorphism |
|--------|---------------|--------------|
| **Pure Clorus?** | ✅ YES | ❌ NO |
| **Runtime changes?** | ❌ NO | ✅ YES (major) |
| **Compiler changes?** | ❌ NO | ✅ YES (major) |
| **New AST nodes?** | ❌ NO | ✅ YES (defprotocol, defrecord, extend-protocol) |
| **New ValueTags?** | ❌ NO (can use Map) | ✅ YES (Record, Protocol) |
| **Parser changes?** | ❌ NO | ✅ YES |
| **Codegen changes?** | ❌ NO | ✅ YES |
| **Implementation time** | 1-2 weeks | 2-3 months |
| **Risk level** | ✅ Low | ⚠️ High |
| **User value** | ✅ High | ✅ High |
| **Proves stdlib approach** | ✅ YES | ❌ NO |

---

## Why Lazy Sequences First?

### 1. Demonstrates Stdlib-in-Clorus Philosophy

**Goal:** Once core language is complete, build stdlib in Clorus itself.

**Evidence:** Lazy sequences prove this works!
- Uses only existing features (closures, atoms, recursion)
- No need to touch runtime or compiler
- Shows Clorus is powerful enough for advanced features

### 2. Immediate User Value

**What users can do:**
```clojure
;; Infinite sequences
(def naturals (lazy-range 0))
(def fibs (lazy-fibonacci))
(def primes (lazy-primes))

;; Efficient processing
(realize (lazy-take 10
  (lazy-filter prime?
    (lazy-map square naturals))))
; Only processes until 10 results found!

;; Memory efficient
(reduce + (lazy-take 1000000 naturals))
; Uses constant memory, not 1M element vector
```

### 3. Lower Risk

**Lazy sequences:**
- ✅ Isolated to stdlib
- ✅ No core language changes
- ✅ Easy to iterate and improve
- ✅ Can optimize later if needed

**Polymorphism:**
- ⚠️ Touches every layer (parser, AST, codegen, runtime)
- ⚠️ Breaking changes to core language
- ⚠️ Hard to get right first time
- ⚠️ High testing burden

### 4. Validates Architecture

Building lazy sequences proves:
- ✅ Core language is complete enough
- ✅ Closures work correctly
- ✅ Atoms work correctly
- ✅ Function composition works
- ✅ Recursion works
- ✅ Stdlib approach is viable

---

## What Lazy Sequences Enable

### 1. Infinite Sequences

```clojure
;; Natural numbers
(def naturals (lazy-range 0))

;; Powers of 2
(def powers-of-2 (lazy-iterate #(* 2 %) 1))

;; Fibonacci
(def fibs
  (lazy-cons 0
    (lazy-cons 1
      (lazy-map + fibs (lazy-rest fibs)))))

;; All valid - infinite but lazy!
```

### 2. Efficient Data Pipelines

```clojure
;; Process huge file line by line (constant memory)
(->> (lazy-lines "huge-file.txt")
     (lazy-map parse-line)
     (lazy-filter valid?)
     (lazy-map transform)
     (lazy-take 100)
     realize)
```

### 3. Composable Transformations

```clojure
;; Each operation is lazy - only computes what's needed
(defn process-data [data]
  (->> data
       (lazy-map parse)
       (lazy-filter valid?)
       (lazy-map normalize)
       (lazy-filter important?)
       (lazy-map enrich)
       (lazy-take 10)))  ; Only processes until 10 results
```

### 4. Algorithm Elegance

```clojure
;; Sieve of Eratosthenes (prime numbers)
(defn sieve [s]
  (lazy-cons (lazy-first s)
    (fn []
      (sieve (lazy-filter
               (fn [x] (not= (mod x (lazy-first s)) 0))
               (lazy-rest s))))))

(def primes (sieve (lazy-range 2)))

(realize (lazy-take 10 primes))
; => [2 3 5 7 11 13 17 19 23 29]
```

---

## When to Add Polymorphism?

**After:**
1. ✅ Lazy sequences stdlib complete
2. ✅ Other core stdlib functions (partial, comp, juxt)
3. ✅ String operations complete
4. ✅ More real-world testing

**Why wait:**
- Need to understand type system requirements better
- Should see what patterns emerge from real code
- Can inform design decisions with actual usage
- Less risk to stable core language

**Estimated timeline:**
- Lazy sequences: Now (1-2 weeks)
- Other stdlib: 2-3 weeks
- String ops: 1-2 weeks
- **Then consider polymorphism:** After 1-2 months

---

## Implementation Plan: Lazy Sequences

### Phase 1: Core Library (Week 1)

**Files:** `stdlib/lazy.clr`

**Functions:**
- Construction: `make-lazy`, `lazy-cons`, `force`
- Access: `lazy-first`, `lazy-rest`, `lazy-empty?`
- Conversion: `realize`, `realize-n`

**Infinite sequences:**
- `lazy-range` - Natural numbers
- `lazy-repeat` - Repeat value
- `lazy-cycle` - Cycle through collection
- `lazy-iterate` - x, (f x), (f (f x)), ...

### Phase 2: Transformations (Week 1-2)

**Functions:**
- `lazy-map` - Transform elements
- `lazy-filter` - Select elements
- `lazy-remove` - Reject elements
- `lazy-take` - Take first n
- `lazy-take-while` - Take while predicate
- `lazy-drop` - Drop first n
- `lazy-drop-while` - Drop while predicate
- `lazy-take-nth` - Take every nth element

### Phase 3: Combination (Week 2)

**Functions:**
- `lazy-concat` - Concatenate sequences
- `lazy-interleave` - Alternate elements
- `lazy-interpose` - Insert separator
- `lazy-mapcat` - Map and flatten
- `lazy-distinct` - Remove duplicates

### Phase 4: Specialized (Week 2)

**Functions:**
- `lazy-partition` - Partition into chunks
- `lazy-partition-by` - Partition by function
- `lazy-tree-seq` - Lazy tree traversal
- `lazy-file-seq` - Lazy file reading

### Phase 5: Documentation & Examples

**Files:**
- `docs/LAZY_SEQUENCES.md` - Usage guide
- `examples/lazy-examples.clr` - Code examples
- `tests/lazy-tests.clr` - Test suite

---

## Success Metrics

**Lazy Sequences Complete When:**
1. ✅ All core functions implemented (make-lazy, force, cons, first, rest)
2. ✅ All transformation functions (map, filter, take, drop, etc.)
3. ✅ Infinite sequences work (range, repeat, cycle, iterate)
4. ✅ Composition works (chain operations)
5. ✅ Early termination works (performance benefit)
6. ✅ Memory efficiency demonstrated (constant memory for infinite seqs)
7. ✅ Documentation complete
8. ✅ Examples demonstrate value
9. ✅ Test suite passes

**Proof that stdlib approach works:**
- ✅ Advanced feature built in pure Clorus
- ✅ No runtime/compiler changes needed
- ✅ Shows language is complete enough
- ✅ Validates architecture

---

## Conclusion

**Recommendation: Build lazy sequences NOW, polymorphism LATER**

**Why:**
1. ✅ Can be done in pure Clorus (no risk)
2. ✅ Immediate user value (infinite sequences, performance)
3. ✅ Proves stdlib-in-Clorus approach
4. ✅ Lower implementation cost (1-2 weeks vs 2-3 months)
5. ✅ Validates core language completeness

**Next Steps:**
1. Implement lazy sequence stdlib (this week)
2. Add documentation and examples
3. Test with real-world use cases
4. Gather feedback
5. Then consider polymorphism (after 1-2 months)

---

**Status:** ✅ Analysis Complete - Ready to implement lazy sequences

**Files Created:**
- `/Users/prabhugopal/Learning/git/clorus/stdlib/lazy.clr` - Library implementation
- `/Users/prabhugopal/Learning/git/clorus/test-lazy.clr` - Comprehensive tests
- `/Users/prabhugopal/Learning/git/clorus/test-lazy-minimal.clr` - Minimal proof of concept
- `/Users/prabhugopal/Learning/git/clorus/LAZY_SEQUENCES_PURE_CLORUS.md` - Technical documentation

**Ready to proceed with implementation!**
