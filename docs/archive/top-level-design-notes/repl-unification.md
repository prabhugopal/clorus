# REPL Unification Plan

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/PARITY_EXECUTION_PLAN.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


Goal: keep REPL and compiled mode aligned by using a single runtime model and a consistent IR pipeline.

## Current Strategy (Phase 1)
Default REPL behavior:
- Load `clorus-core` dynamic library
- Register stdlib symbols only (no JIT stdlib compilation)

This ensures REPL uses the same core implementations as compiled mode, avoiding JIT stdlib issues.

### Environment Flags
- `CLORUS_REPL_NO_CORE_LIB=1`  
  Skip loading `clorus-core` dylib.
- `CLORUS_REPL_LOAD_STDLIB=1`  
  Legacy fallback: JIT-compile stdlib forms (not default).

## Next Steps (Phase 2+)
1. Single IR pipeline for REPL + AOT.
2. Eliminate REPL-only IR emission (no extra `(use ...)` codegen).
3. Remove JIT stdlib mode once stable.

## Rationale
The unified path reduces drift between REPL and compiled behavior and simplifies debugging.

## How To Test
1. Build and install:
```
./scripts/build/build.sh
./scripts/install/install.sh
```

2. Run REPL in a project (core dylib default):
```
clorus repl
```

3. Optional legacy path:
```
CLORUS_REPL_LOAD_STDLIB=1 clorus repl
```
