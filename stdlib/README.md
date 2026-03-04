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

- `_variants/` contains development snapshots and is not part of normal runtime loading.
- `transducers.clr.disabled` is retained as a historical artifact and is not loaded.
- REPL and CLI startup now load stdlib from these namespaced paths.
