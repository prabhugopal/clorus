# Stdlib Layout

Canonical stdlib source files live under `stdlib/clorus/`.

- `clorus/core.clr`
- `clorus/lazy.clr`
- `clorus/set.clr`
- `clorus/string.clr`
- `clorus/transducers.clr`
- `clorus/walk.clr`

Namespace-to-file mapping for `clorus.*` modules is currently:

- `clorus.core` -> `stdlib/clorus/core.clr`
- `clorus.set` -> `stdlib/clorus/set.clr`
- `clorus.string` -> `stdlib/clorus/string.clr` (partial: only a handful of functions have moved out of `clorus.core`'s bare globals so far)
- `clorus.lazy` -> `stdlib/clorus/lazy.clr`
- `clorus.transducers` -> `stdlib/clorus/transducers.clr`
- `clorus.walk` -> `stdlib/clorus/walk.clr`

Notes:

- `clorus.core` is the canonical home for public collection transforms and
  reduced-value helpers. In particular, `map`, `filter`, `take`, `drop`,
  `remove`, `mapcat`, `partition-by`, `take-while`, and `drop-while` are
  defined there exactly once.
- `clorus.transducers` supplements core with `cat`, `completing`, `transduce`,
  and `into`; it must not redefine public core APIs. This keeps semantics
  independent of stdlib load order.
- REPL, `run`, and `build` load `core.clr` before `transducers.clr`. Source
  stdlib definitions must therefore only depend on definitions that occur
  earlier in that order; use the normal public API rather than a compiler-only
  shortcut.
- The public-surface inventory and the process for adding a core function are
  maintained in [`docs/design/STDLIB_SURFACE_AUDIT.md`](../docs/design/STDLIB_SURFACE_AUDIT.md).
