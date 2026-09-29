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
state: unforced | forcing | realized
thunk: Function value                 ; retained until realization
realized: nil | List cell             ; retained after realization
```

The thunk is a zero-arity Clorus function. On forcing it returns either `nil`
or a one-element sequence cell `(head . tail)`. The tail is another sequence
value, normally another `LazySeq` or a `List`.

Forcing must be synchronized. A recursive attempt to force the same cell is a
language exception, not a deadlock or undefined behavior. Successful forcing
is memoized; the thunk is released exactly once after the realized value has
been stored. Exception results are not cached as successful realization.

## Public protocol boundary

Only `seq` may force an arbitrary lazy sequence. Its result is either `nil` or
a non-empty sequence value. Core traversal is then expressed through these
operations:

| Operation | Lazy behavior |
|---|---|
| `seq` | Forces at most one cell; returns `nil` or a sequence cell. |
| `first` | Calls `seq`; returns the head or `nil`. |
| `rest` | Calls `seq`; returns the tail or empty list. |
| `next` | Calls `rest`, then `seq`. |
| `seq?` | True for realized list cells and native lazy sequence values. |
| `count`, `nth`, `last` | Must not force unbounded input. Initial beta behavior is an explicit error for an unresolved lazy sequence. |

`empty?` remains valid for finite counted values only. It must not silently
force or claim to decide whether an arbitrary lazy sequence is empty.

## Core migration order

1. Add native `LazySeq`, ownership, printer, and forcing API with direct
   runtime unit tests.
2. Route `seq`, `first`, `rest`, `next`, and `seq?` through that API in both
   JIT and AOT. Add a finite hand-built lazy sequence regression.
3. Add public `lazy-seq` syntax/form backed by the native cell and verify that
   a side-effecting thunk runs once.
4. Convert `range`, `repeat`, `iterate`, `concat`, `map`, `filter`, `take`,
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

## Acceptance tests for milestone 1

Both JIT and AOT must demonstrate:

```clojure
(let [calls (atom 0)
      xs (lazy-seq (do (swap! calls inc) (cons 1 nil)))]
  [(first xs) (first xs) @calls])
;; => [1 1 1]

(into [] (take 5 (range)))
;; => [0 1 2 3 4]

(into [] (take 4 (map inc (range))))
;; => [1 2 3 4]
```

The test suite must also prove that a cyclic force fails as a normal language
exception and that releasing an unforced or realized lazy sequence has no leak
or use-after-free.
