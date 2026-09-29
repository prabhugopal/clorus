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
realized: nil | canonical List        ; retained after realization
```

The thunk is a zero-arity Clorus function. On forcing it returns either `nil`
or a Clorus sequenceable value. Vectors, maps, and sets are normalized through
`seq` once and cached as the canonical list representation. Other values are a
language exception. A sequence tail may itself be another `LazySeq` or a
`List`.

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
| `seq` | Forces at most one cell; returns `nil` or a sequence cell. |
| `first` | Calls `seq`; returns the head or `nil`. |
| `rest` | Calls `seq`; returns the tail or empty list. |
| `next` | Calls `rest`, then `seq`. |
| `seq?` | True for realized list cells and native lazy sequence values. |
| `count`, `nth`, `last` | Not lazy-aware in the initial foundation; do not use them to consume a lazy value yet. |

`empty?` calls `seq` for sequence values, realizing at most the first cell. It
continues to use `count` for finite counted collections.

## Core migration order

1. Done: add native `LazySeq`, ownership, printer, forcing API, `seq?`, and
   direct runtime unit tests.
2. Done: route `seq`, `first`, and `rest` through the native cell in both JIT
   and AOT, including a finite public regression.
3. Done: add the public `lazy-seq` form and verify lexical capture,
   memoization, normalization, error handling, and JIT/AOT behavior.
4. Next: make `count`, `nth`, and `last` lazy-aware with an explicit bounded
   consumption policy, then convert `range`, `repeat`, `iterate`, `concat`, `map`, `filter`, `take`,
   `drop`, `interleave`, and `dedupe` one family at a time. Each conversion
   requires finite JIT/AOT/Clojure parity plus an infinite-prefix test.
5. Add a reducing-function protocol before exposing transducer arities for
   lazy transforms such as `dedupe`.

No eager API becomes lazy without the matching contract and parity cases.

## Non-goals for the first milestone

- Character values and string sequencing
- Lazy `count`, `nth`, or `last`
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
traversal, invalid thunk-result handling, JIT/AOT equivalence, and releasing
unforced or realized cells without a use-after-free. Recursive forcing needs
a language-level cycle construction test when self-referential lazy bindings
are available.
