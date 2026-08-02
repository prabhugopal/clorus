# Stdlib Layout

Canonical stdlib source files live under `stdlib/clorus/`.

- `clorus/core.clr`
- `clorus/lazy.clr`
- `clorus/set.clr`
- `clorus/transducers.clr`

Namespace-to-file mapping for `clorus.*` modules is currently:

- `clorus.core` -> `stdlib/clorus/core.clr`
- `clorus.set` -> `stdlib/clorus/set.clr`
- `clorus.lazy` -> `stdlib/clorus/lazy.clr`
- `clorus.transducers` -> `stdlib/clorus/transducers.clr`

Notes:

- `transducers.clr.disabled` is a fuller transducer implementation than the active `clorus/transducers.clr` (`remove`/`drop`/`take-while`/`cat`/`mapcat`/`sequence`/`partition-by` etc.), disabled because multi-arity functions returning closures fail to compile when loaded as stdlib (works fine in user code) — see `docs/site/progress.html`. Fixing that codegen bug and re-enabling this file recovers those functions without new implementation work.
- REPL and CLI startup load stdlib from the namespaced paths above.
