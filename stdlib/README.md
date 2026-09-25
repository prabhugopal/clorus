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

- REPL and CLI startup load stdlib from the namespaced paths above.
- A `transducers.clr.disabled` file previously sat here, claiming to be a fuller transducer implementation than `clorus/transducers.clr` (`remove`/`drop`/`take-while`/`cat`/`mapcat`/`dedupe`/`partition-by` etc.) blocked only by a multi-arity-closure-returning-a-closure codegen bug. That bug is fixed (verified directly: multi-arity closures dispatch correctly by arg count now), but re-testing the file against current code showed it's independently broken in ways unrelated to that bug -- most of its transducers (`map`/`filter`/`remove`/`drop`/`cat`/`mapcat`/`dedupe`/`partition-by`) returned empty output, and `take-while` produced a doubly-wrapped `reduced` value instead of a real result. It was removed rather than kept as a stale "just re-enable this" pointer.
- That gap is now closed: `clorus/transducers.clr` has real, from-scratch implementations of `remove`/`drop`/`take-while`/`drop-while`/`cat`/`mapcat`/`partition-by`, each following the same dual-arity pattern as `map`/`filter`/`take` (a 1-arity transducer-producing form plus a 2-arity eager-collection form). `dedupe` is deliberately *not* among them: it's already a fixed-arity-1 runtime builtin (`clorus_dedupe` in `crates/clorus-codegen/src/codegen/calls.rs`) that takes precedence over any same-named stdlib `defn`, so it stays eager-only -- a transducer-producing `(dedupe)` form would need that builtin's call dispatch taught multi-arity first, which is out of scope here. See `tests/stdlib/test-transducers-completion-gap.clr` for coverage of both the eager and composed-transducer forms of each.
