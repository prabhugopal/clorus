# Docs Consolidation Plan

Last updated: 2026-03-03

## Why
- `docs/` currently has high duplication and drift.
- Current inventory: `164` markdown files, `54` top-level files.
- Multiple files describe the same feature with conflicting status.

## Goal
Create a small, reliable canonical documentation set and archive historical/duplicate docs without losing information.

## Canonical Set (source of truth)
- `docs/reference/LANGUAGE_SPEC.md` — language behavior.
- `docs/PARITY_CHECKLIST.md` — paired vs partial vs missing.
- `docs/PARITY_EXECUTION_PLAN.md` — current implementation plan.
- `docs/ROADMAP_TO_100_PARITY.md` — strategic roadmap.
- `docs/DOCUMENTATION_INDEX.md` — entrypoint and links.
- `tests/FEATURE_COVERAGE.md` — test inventory only (non-canonical for feature status).

## Consolidation Rules
- One topic → one canonical file.
- Historical notes move to `docs/archive/` or `docs/sessions/`.
- If status conflicts:
  - `PARITY_CHECKLIST.md` wins for feature status.
  - test runner output wins for pass/fail claims.
- Keep old docs but add a top banner:
  - `Status: Archived`
  - `Canonical replacement: <path>`

## Execution Phases
1. Inventory + tagging
   - Tag each non-canonical doc as `canonical`, `historical`, or `duplicate`.
2. Merge by topic
   - Collections, macros, exceptions, polymorphism, REPL/CLI, FFI.
3. Archive and cross-link
   - Move duplicates to `docs/archive/`.
   - Add replacement pointers.
4. CI guardrail
   - Add a simple docs check that rejects new top-level docs unless linked from `DOCUMENTATION_INDEX.md`.

## Immediate Next Steps
- [ ] Add `Status/Canonical replacement` banner to high-drift docs.
- [ ] Consolidate macro + exception status docs into parity docs.
- [ ] Consolidate REPL runbook into one guide and archive old variants.
- [ ] Recompute and refresh `tests/FEATURE_COVERAGE.md` summary section.
