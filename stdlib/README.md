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
- A `transducers.clr.disabled` file previously sat here, claiming to be a fuller transducer implementation than `clorus/transducers.clr` (`remove`/`drop`/`take-while`/`cat`/`mapcat`/`dedupe`/`partition-by` etc.) blocked only by a multi-arity-closure-returning-a-closure codegen bug. That bug is fixed (verified directly: multi-arity closures dispatch correctly by arg count now), but re-testing the file against current code showed it's independently broken in ways unrelated to that bug -- most of its transducers (`map`/`filter`/`remove`/`drop`/`cat`/`mapcat`/`dedupe`/`partition-by`) returned empty output, and `take-while` produced a doubly-wrapped `reduced` value instead of a real result. It was removed rather than kept as a stale "just re-enable this" pointer. The extra functions it attempted remain a real, open stdlib gap -- `clorus/transducers.clr`'s existing `map`/`filter`/`take` pattern (both a 1-arity transducer form and a 2-arity eager-collection form) is a working template to extend, not something to resurrect from the old file.
