# Clorus Lazy Sequences - User Guide

## Overview

The Clorus lazy sequence library provides powerful lazy evaluation capabilities, enabling you to work with infinite sequences, compose operations efficiently, and process data with minimal memory usage.

**Key Features:**
- 🚀 Infinite sequences (range, repeat, cycle, iterate)
- ⚡ On-demand computation (only evaluates what's needed)
- 💾 Memory efficient (constant space for infinite data)
- 🔗 Fully composable (chain operations seamlessly)
- 🎯 Pure Clorus implementation (no runtime changes needed)

**Version:** 1.0.0
**Date:** January 27, 2026
**Status:** Production Ready

---

## Quick Start

```clojure
;; Load the library
(use clorus.lazy)

;; Create an infinite sequence
(def naturals (lazy-range))

;; Take first 10 elements
(realize-n 10 naturals)
;; => [0 1 2 3 4 5 6 7 8 9]

;; Chain operations
(realize-n 5
  (lazy-map (fn [x] (* x x))
    (lazy-filter even? naturals)))
;; => [0 4 16 36 64]
```

---

## Core Concepts

### What is a Lazy Sequence?

A lazy sequence is a data structure that computes its elements on-demand rather than all at once. Each element is only calculated when you access it, and the result is cached for future use.

**Benefits:**
1. **Memory Efficient** - Don't need to store entire collections
2. **Infinite Sequences** - Can represent unbounded data
3. **Early Termination** - Stop computing once you have enough
4. **Composability** - Chain operations without intermediate collections

### Lazy vs Eager

```clojure
;; Eager evaluation (traditional)
(def eager-data (map square (range 0 1000000)))  ; Computes ALL 1M squares
(first eager-data)  ; Already computed

;; Lazy evaluation
(def lazy-data (lazy-map square (lazy-range)))  ; Infinite, nothing computed yet
(lazy-first lazy-data)  ; Computes only first square
```

---

## API Reference

### Construction

#### `make-lazy`
Create a lazy value from a thunk (zero-argument function).

```clojure
(def my-lazy (make-lazy (fn [] (+ 1 2))))
(force my-lazy)  ; => 3
```

#### `lazy-cons`
Cons an element onto a lazy tail.

```clojure
(def ones (lazy-cons 1 (fn [] ones)))  ; Infinite ones
(lazy-first ones)  ; => 1
```

#### `force`
Force evaluation of a lazy sequence.

```clojure
(def ls (make-lazy (fn [] 42)))
(force ls)  ; => 42 (computed and cached)
(force ls)  ; => 42 (uses cache)
```

### Accessors

#### `lazy-first`
Get the first element.

```clojure
(lazy-first (lazy-range))  ; => 0
```

#### `lazy-rest`
Get remaining elements (returns lazy sequence).

```clojure
(lazy-first (lazy-rest (lazy-range)))  ; => 1
```

#### `lazy-empty?`
Check if sequence is empty.

```clojure
(lazy-empty? nil)  ; => true
(lazy-empty? (lazy-range))  ; => false
```

#### `lazy-nth`
Get nth element (0-indexed).

```clojure
(lazy-nth (lazy-range) 5)  ; => 5
```

### Infinite Sequences

#### `lazy-range`
Generate sequences of numbers.

```clojure
(lazy-range)           ; 0, 1, 2, 3, ... (infinite)
(lazy-range 10)        ; 10, 11, 12, ... (infinite from 10)
(lazy-range 5 10)      ; 5, 6, 7, 8, 9 (finite)
```

#### `lazy-repeat`
Repeat a value infinitely.

```clojure
(realize-n 5 (lazy-repeat 42))  ; => [42 42 42 42 42]
```

#### `lazy-cycle`
Cycle through a collection infinitely.

```clojure
(realize-n 7 (lazy-cycle [1 2 3]))  ; => [1 2 3 1 2 3 1]
```

#### `lazy-iterate`
Generate infinite sequence by repeatedly applying function.

```clojure
(realize-n 5 (lazy-iterate (fn [x] (* x 2)) 1))  ; => [1 2 4 8 16]
```

#### `lazy-repeatedly`
Call function repeatedly to generate infinite sequence.

```clojure
(realize-n 3 (lazy-repeatedly (fn [] 99)))  ; => [99 99 99]
```

### Transformations

#### `lazy-map`
Apply function to each element.

```clojure
(realize-n 5 (lazy-map (fn [x] (* x 2)) (lazy-range)))  ; => [0 2 4 6 8]
```

#### `lazy-filter`
Keep only elements matching predicate.

```clojure
(realize-n 5 (lazy-filter even? (lazy-range)))  ; => [0 2 4 6 8]
```

#### `lazy-remove`
Remove elements matching predicate.

```clojure
(realize-n 5 (lazy-remove odd? (lazy-range 10)))  ; => [0 2 4 6 8]
```

#### `lazy-take`
Take first n elements.

```clojure
(realize (lazy-take 5 (lazy-range)))  ; => [0 1 2 3 4]
```

#### `lazy-take-while`
Take while predicate is true.

```clojure
(realize (lazy-take-while (fn [x] (< x 5)) (lazy-range)))  ; => [0 1 2 3 4]
```

#### `lazy-drop`
Drop first n elements.

```clojure
(realize-n 3 (lazy-drop 5 (lazy-range)))  ; => [5 6 7]
```

#### `lazy-drop-while`
Drop while predicate is true.

```clojure
(realize-n 3 (lazy-drop-while (fn [x] (< x 5)) (lazy-range)))  ; => [5 6 7]
```

#### `lazy-take-nth`
Take every nth element.

```clojure
(realize-n 5 (lazy-take-nth 2 (lazy-range)))  ; => [0 2 4 6 8]
```

### Combination

#### `lazy-concat`
Concatenate sequences.

```clojure
(realize (lazy-concat (lazy-range 0 3) (lazy-range 10 13)))  ; => [0 1 2 10 11 12]
```

#### `lazy-interleave`
Interleave elements from multiple sequences.

```clojure
(realize (lazy-interleave (lazy-range 0 3) (lazy-range 10 13)))  ; => [0 10 1 11 2 12]
```

#### `lazy-interpose`
Insert separator between elements.

```clojure
(realize (lazy-interpose :sep (lazy-range 0 3)))  ; => [0 :sep 1 :sep 2]
```

### Realization

#### `realize`
Fully realize a lazy sequence into a vector.

⚠️ **Warning:** Don't use on infinite sequences!

```clojure
(realize (lazy-range 5 10))  ; => [5 6 7 8 9]
```

#### `realize-n`
Realize first n elements (safe for infinite sequences).

```clojure
(realize-n 5 (lazy-range))  ; => [0 1 2 3 4]
```

#### `to-lazy`
Convert eager collection to lazy sequence.

```clojure
(def ls (to-lazy [1 2 3 4 5]))
(realize-n 3 ls)  ; => [1 2 3]
```

### Utilities

#### `lazy-count`
Count elements (⚠️ realizes entire sequence).

```clojure
(lazy-count (lazy-range 0 10))  ; => 10
```

#### `lazy-reduce`
Reduce with function and initial value (⚠️ realizes entire sequence).

```clojure
(lazy-reduce + 0 (lazy-range 1 6))  ; => 15
```

### Advanced

#### `lazy-distinct`
Remove duplicates (preserves order).

```clojure
(realize (lazy-distinct (to-lazy [1 2 2 3 3 3 4])))  ; => [1 2 3 4]
```

#### `lazy-partition`
Partition into chunks of size n.

```clojure
(realize (lazy-partition 3 (lazy-range 0 10)))  ; => [[0 1 2] [3 4 5] [6 7 8] [9]]
```

---

## Common Patterns

### Pattern 1: Process Large Data Efficiently

```clojure
;; Only processes until we have 10 valid results
(realize-n 10
  (lazy-filter valid?
    (lazy-map parse-line
      (lazy-lines "huge-file.txt"))))
```

### Pattern 2: Infinite Generators

```clojure
;; Fibonacci
(def fibs
  (lazy-cons 0
    (fn []
      (lazy-cons 1
        (fn []
          (lazy-map + fibs (lazy-rest fibs)))))))

(realize-n 10 fibs)  ; => [0 1 1 2 3 5 8 13 21 34]
```

### Pattern 3: Composition

```clojure
;; Chain multiple operations
(->> (lazy-range)
     (lazy-filter even?)
     (lazy-map (fn [x] (* x x)))
     (lazy-take 5)
     realize)
;; => [0 4 16 36 64]
```

### Pattern 4: Sliding Windows

```clojure
(defn sliding-window [n coll]
  (if (< (lazy-count (lazy-take n coll)) n)
    nil
    (lazy-cons (realize-n n coll)
      (fn [] (sliding-window n (lazy-rest coll))))))

(realize (sliding-window 3 (to-lazy [1 2 3 4 5])))
;; => [[1 2 3] [2 3 4] [3 4 5]]
```

### Pattern 5: Prime Numbers

```clojure
(defn sieve [s]
  (lazy-cons (lazy-first s)
    (fn []
      (sieve (lazy-filter
               (fn [x] (not= (mod x (lazy-first s)) 0))
               (lazy-rest s))))))

(def primes (sieve (lazy-range 2)))
(realize-n 10 primes)  ; => [2 3 5 7 11 13 17 19 23 29]
```

---

## Performance Characteristics

### Memory Usage

| Operation | Eager | Lazy |
|-----------|-------|------|
| Range 1M elements | ~8 MB | ~100 bytes |
| Map over 1M items | ~8 MB + ~8 MB | ~100 bytes |
| Filter 1M items | ~8 MB | ~100 bytes |

### Time Complexity

Most operations are O(1) to create (lazy), O(n) when realized:

| Operation | Creation | Realization |
|-----------|----------|-------------|
| lazy-range | O(1) | O(n) |
| lazy-map | O(1) | O(n) |
| lazy-filter | O(1) | O(n) worst case |
| lazy-take | O(1) | O(k) for k elements |
| realize-n | O(1) | O(k) for k elements |

### When to Use Lazy

✅ **Use lazy when:**
- Working with large or infinite data
- Not all elements may be needed
- Composing multiple transformations
- Memory is constrained

❌ **Avoid lazy when:**
- All elements will be used anyway
- Need random access frequently
- Working with small collections (<100 items)

---

## Common Pitfalls

### Pitfall 1: Realizing Infinite Sequences

```clojure
;; ❌ DON'T: This will never finish!
(realize (lazy-range))

;; ✅ DO: Use realize-n or lazy-take
(realize-n 10 (lazy-range))
(realize (lazy-take 10 (lazy-range)))
```

### Pitfall 2: Multiple Realizations

```clojure
;; ❌ Inefficient: Computes twice
(def ls (lazy-map expensive-fn (lazy-range)))
(lazy-first ls)
(lazy-first ls)  ; Computes again!

;; ✅ Better: Realize once if needed multiple times
(def realized (realize-n 10 ls))
(first realized)
(first realized)  ; Uses cached value
```

### Pitfall 3: Side Effects in Lazy Operations

```clojure
;; ❌ DON'T: Side effects may not happen as expected
(lazy-map (fn [x] (println x) x) (lazy-range))  ; Nothing printed!

;; ✅ DO: Force realization if side effects are needed
(realize-n 5 (lazy-map (fn [x] (println x) x) (lazy-range)))
```

---

## Examples

See `/examples/lazy-examples.clr` for comprehensive real-world examples including:
- Infinite mathematical sequences
- Prime number generation
- Data processing pipelines
- Tree traversal
- Game state simulation
- And more!

---

## Testing

Run the test suite:

```bash
./clorus tests/lazy-test.clr
```

All tests should pass, demonstrating:
- ✅ Core construction and forcing
- ✅ Infinite sequences
- ✅ Transformations
- ✅ Composition
- ✅ Edge cases
- ✅ Caching behavior

---

## Implementation

The entire lazy sequence library is implemented in **pure Clorus** without any runtime or compiler changes. It uses:

- **Closures** - To capture computation
- **Atoms** - For caching realized values
- **Maps** - For lazy sequence structure
- **Recursion** - For infinite sequences

This demonstrates the **stdlib-in-Clorus** philosophy: once core language features are complete, advanced features can be built in Clorus itself.

---

## Future Enhancements

### Phase 2: Runtime Optimization (Optional)
- Add native `ValueTag::LazySeq` type
- Automatic realization in collection operations
- Chunked sequences (realize 32 elements at a time)
- Better integration with eager collections

### Phase 3: Additional Operations
- `lazy-partition-by` - Partition by predicate
- `lazy-group-by` - Group elements by function
- `lazy-tree-seq` - Lazy tree traversal
- `lazy-file-seq` - Lazy file reading

---

## Contributing

The lazy sequence library is part of the Clorus standard library. Contributions welcome!

**Guidelines:**
- Keep functions pure (no side effects)
- Document with examples
- Add tests for new features
- Maintain lazy evaluation semantics

---

## License

Part of the Clorus programming language.

---

## See Also

- `/stdlib/lazy.clr` - Library implementation
- `/examples/lazy-examples.clr` - Practical examples
- `/tests/lazy-test.clr` - Test suite
- `/docs/LAZY_SEQUENCES_PURE_CLORUS.md` - Technical documentation

---

**Status:** ✅ Production Ready
**Version:** 1.0.0
**Last Updated:** January 27, 2026

---

*Implemented with ❤️ in pure Clorus*
