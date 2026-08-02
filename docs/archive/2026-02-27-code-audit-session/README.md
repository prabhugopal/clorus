# 2026-02-27 code audit session

Six files produced by a single AI-driven code audit on 2026-02-27, previously
sitting at the repo root, never referenced again after that session:

- `CLORUS_CODE_ANALYSIS.md` / `CLORUS_CODE_ANALYSIS_CODE_ONLY.md` — two takes
  on the same source-level audit.
- `CODE_AUDIT_DETAILS.md` / `CODE_AUDIT_INDEX.md` — detail doc + index for the
  same audit.
- `PATCH_PLAN_TOP3_FIXES.md` / `PLAN_TOP3_FIXES.md` — two takes on a plan to
  fix the audit's top 3 findings (macro-expansion scope, map/set correctness,
  `swap!` CAS retry).

Superseded by `docs/site/progress.html` (2026-07-30), which re-verified the
codebase against source directly rather than trusting these. Kept here for
history, not as a status source.

`REFACTOR_PLAN_MAINTAINABILITY.md`, from the same session, stayed at the repo
root until its codegen-split plan was actually executed — see
`docs/site/progress.html` roadmap Phase 0/1 and git history for when it moved
here.
