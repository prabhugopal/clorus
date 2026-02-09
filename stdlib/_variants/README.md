# Stdlib Variants (Archived)

This directory contains archived variants of the standard library that were used during development for specific purposes. These are kept for historical reference but are not actively used.

## Files

- **core-minimal.clr** - Minimal stdlib for basic testing (7 lines)
  - Contains only: identity, inc, dec
  - Used for quick sanity checks during development

- **core-working.clr** - Stdlib without destructuring in macros (29 lines)
  - Created when destructuring wasn't fully implemented
  - Contains: doseq-simple, for-simple (without destructuring support)
  - Superseded by main stdlib/core.clr

- **core-full.clr** - Full stdlib snapshot (419 lines)
  - Historical snapshot of stdlib at a point in time
  - May contain experimental features
  - Superseded by main stdlib/core.clr

## Current Active Stdlib

The canonical standard library is in the parent directory:
- **../core.clr** - Main standard library (currently active)
- **../set.clr** - Set operations
- **../lazy.clr** - Lazy sequences

Use these instead of the archived variants.
