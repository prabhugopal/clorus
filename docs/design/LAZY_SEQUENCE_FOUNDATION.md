# Native Lazy Sequence Foundation

## Decision

Clorus will have one sequence model. A lazy sequence is a native runtime value,
not a map convention implemented by `clorus.lazy`.

`stdlib/clorus/lazy.clr` remains an experiment and must not be auto-loaded or
used by `clorus.core`. Its `lazy-*` names are deliberately separate from the
public core API until this design is complete.

## Runtime representation

Add `ValueTag::LazySeq` whose payload is a heap-owned cell containing:

```text
state: unforced | forcing(thread-id) | realized
thunk: Function value                 ; retained until realization
realized: nil | canonical List | SeqNode ; retained after realization
```

The thunk is a zero-arity Clorus function. On forcing it returns either `nil`
or a Clorus sequenceable value. Vectors, maps, and sets are normalized through
`seq` once and cached as the canonical list representation. Other values are a
language exception. A native `SeqNode { head, tail: Value }` represents the
other sequence shape: its tail is `nil`, a finite `List`, another `SeqNode`, or
a `LazySeq`. It owns both references without forcing the tail. This preserves
the compact persistent-list implementation while allowing an incrementally
produced sequence to be genuinely unbounded.

Forcing is synchronized. A different thread waits for the current realization
and observes its cached result; a recursive attempt by the owning thread is a
language exception, not a deadlock or undefined behavior. Successful forcing
is memoized; the thunk is released exactly once after the realized value has
been stored. Exception results are not cached as successful realization, so a
later traversal may retry.

## Public protocol boundary

`lazy-seq` is a compiler-recognized core form. It compiles its body into a
capturing zero-arity closure; it is not a source-level function, because a
function would evaluate its arguments eagerly. `seq` is the forcing boundary.
Its result is either `nil` or a non-empty canonical sequence value. Core
traversal is then expressed through these operations:

| Operation | Lazy behavior |
|---|---|
| `seq` | Forces at most one cell; returns `nil`, a list, or a sequence step. |
| `first` | Calls `seq`; returns the head or `nil`. |
| `rest` | Calls `seq`; returns the tail or empty list. |
| `next` | Calls `rest`, then `seq`. |
| `seq?` | True for realized list cells and native lazy sequence values. |
| `count`, `nth`, `last` | Force the cell, then operate on its cached canonical sequence. `count` uses a value-returning runtime boundary so force errors propagate rather than becoming zero. |

Display traversal is bounded to 64 elements and prints `...` when a remaining
tail might continue; it never forces an additional tail merely to decide that.
Equality and hashing do not use that display bound: they remain exact,
structural, and stack-safe across vectors, lists, `SeqNode`, and `LazySeq`.
Consequently, comparing or hashing distinct unbounded sequences has Clojure's
natural non-termination behavior, while comparing the same value uses identity
as a fast path.

`empty?` calls `seq` for sequence values, realizing at most the first cell. It
continues to use `count` for finite counted collections.

## Core migration order

1. Done: add native `LazySeq`, ownership, printer, forcing API, `seq?`, and
   direct runtime unit tests.
2. Done: route `seq`, `first`, and `rest` through the native cell in both JIT
   and AOT, including a finite public regression.
3. Done: add the public `lazy-seq` form and verify lexical capture,
   memoization, normalization, error handling, and JIT/AOT behavior. Route
   `count`, `nth`, and `last` through its forcing boundary as well.
4. Done: add `SeqNode`, an owned head plus a non-forced sequence tail; route
   `seq`, `first`, `rest`, `nth`, `last`, and public `count` through it.
   Zero-arity `range` is the first unbounded producer.
5. Done: add a 64-element bounded display representation and exact structural
   equality/hash for finite general sequences. General sequence traversal is
   iterative, so semantic non-termination never becomes a Rust stack overflow.
6. Done: add Clojure-compatible unbounded arities for `repeat`, `repeatedly`,
   `iterate`, and `cycle`, retaining Clorus's existing finite arities for
   source compatibility. Empty `cycle` terminates as an empty sequence.
7. Done: convert two-argument `map` and `filter` to incremental native lazy
   transforms; their transducer arities are unchanged.
8. Done: convert `take`, `drop`, and `concat` to incremental native lazy transforms;
   their transducer arities are unchanged.
9. Done: convert `distinct`, `dedupe`, `interleave`, `interpose`, `partition`, `partition-all`, `partition-by`,
   `take-while`, `drop-while`, `keep`, `keep-indexed`, `map-indexed`, `mapcat`, and `butlast`
   to incremental state-machine
   transforms. Each
   conversion requires
   finite JIT/AOT/Clojure parity plus an infinite-prefix test.
10. Add a reducing-function protocol before exposing transducer arities for
   lazy transforms such as `dedupe`.

No eager API becomes lazy without the matching contract and parity cases.

## Non-goals for the first milestone

- Character values and string sequencing
- User-configurable dynamic print bounds (`*print-length*`)
- Parallel realization, futures, promises, or IPC
- Reusing the atom-backed map representation from `clorus.lazy`

## Acceptance tests for the completed foundation

Both JIT and AOT must demonstrate:

```clojure
(let [calls (atom 0)
      xs (lazy-seq (swap! calls inc) (list 1))]
  [(first xs) (first xs) @calls])
;; => [1 1 1]
```

The test suite also proves vector normalization, `empty?`/`take`/`drop`
traversal, invalid thunk-result handling, zero-arity `range` prefix
consumption, JIT/AOT equivalence, and releasing unforced/realized cells or a
sequence-step lazy tail without a use-after-free. Recursive forcing needs a
language-level cycle construction test when self-referential lazy bindings are
available.
