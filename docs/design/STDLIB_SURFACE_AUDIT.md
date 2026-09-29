# Core Stdlib Surface Audit

This document prevents a runtime capability from being mistaken for a complete
language API. A Clorus core function is complete only when its public name,
semantics, loading behavior, and execution engines are all covered.

## Definition of done

Every public `clorus.core` function must have all of the following:

1. A documented public owner: source stdlib, runtime primitive, or a deliberate
   compiler form.
2. A source-load position that comes after every public source dependency.
3. JIT and AOT coverage in `tests/run_all_tests.sh`.
4. A REPL smoke check when the function is part of startup-loaded core.
5. A Clojure parity case for pure, portable behavior, or a file-based contract
   test when namespaces, exceptions, or process state are involved.

Do not add compiler fast paths merely because a public name is missing. The
runtime remains a small kernel; source stdlib owns policy and composition.

## Current ownership

| Area | Canonical owner | Notes |
|---|---|---|
| Value creation, persistent collections, arithmetic, basic predicates | Runtime + codegen declarations | These are the small native kernel. |
| Core collection policy and higher-order operations | `stdlib/clorus/core.clr` | Includes `set`, collection transforms, ordering, map utilities, and predicates. |
| Transducer completion utilities | `stdlib/clorus/transducers.clr` | Only `cat`, `completing`, `transduce`, and `into`; transforms remain in core. |
| Set algebra | `stdlib/clorus/set.clr` | Required explicitly as `clorus.set`. |
| String helpers | `stdlib/clorus/string.clr` | Required explicitly as `clorus.string`. |
| Lazy sequence experiment | `stdlib/clorus/lazy.clr` | Not yet the integrated `clojure.core` lazy-seq model. |

The `set` conversion illustrates the distinction: hash-set literals and
runtime set operations already existed, but `(set coll)` was missing until it
was added as a source-level core function with JIT, AOT, REPL, and parity
coverage.

## Source load order

Startup order is:

1. Native runtime declarations
2. `clorus.core`
3. `clorus.transducers`
4. User/project forms

Within a source file, function bodies are compiled in source order. A source
definition must therefore appear after public helpers it invokes. For example,
`set` is placed after `conj`, because its portable implementation is
`(reduce conj #{} coll)`.

## Audit backlog

### P0: contracts and discoverability

- Keep `tests/parity/run_core_parity.py` running Clojure, JIT, and AOT for
  portable expressions.
- Add a core API inventory test that fails if a documented startup-loaded core
  function is absent from the REPL.
- Add explicit contract cases for error shape and arity, not only happy paths.

### P1: high-value portable core gaps

Audit these as public APIs before adding them; implementation follows only when
the behavior and owner are agreed:

- `seq` / `vec` conversion semantics
- `some`, `find`, `keep-indexed`, and `map-indexed`
- Complete multi-arity collection contracts (`concat`, `interleave`,
  `interpose`, `dedupe`, and related sequence functions)
- `empty` / `not-empty` behavior for every supported collection kind

These are intentionally not claimed as complete merely because a similarly
named runtime helper exists.

### P2: semantic decisions, not piecemeal APIs

- Specify the single official lazy-sequence model before making core transforms
  lazy by default.
- Specify character and ratio value representations before adding their reader
  literals.
- Finish `core.async` only after the shared execution plan and ownership model
  are stable.

## Adding a core API

1. Write the Clojure-compatible contract and classify it as pure or
   process/module-dependent.
2. Select the smallest owner: source stdlib first; native kernel only when the
   operation needs representation-level support.
3. Place source definitions after their dependencies.
4. Add JIT, AOT, and REPL coverage; add Clojure differential coverage when
   portable.
5. Update this audit and the generated parity inventory if the status changes.
