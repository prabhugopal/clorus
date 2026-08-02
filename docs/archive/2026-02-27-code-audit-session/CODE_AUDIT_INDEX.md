# Code Audit Index

This index links the code-only audit artifacts added to the repo.

## Reports

- `CLORUS_CODE_ANALYSIS_CODE_ONLY.md`  
  High-level architecture map, feature gaps, stdlib coverage, and prioritized fixes.

- `CODE_AUDIT_DETAILS.md`  
  Deep, file-level audit with correctness risks, runtime/reader gaps, and refactor targets.

## Plans

- `PLAN_TOP3_FIXES.md`  
  Detailed, test-driven plans for:
  1. Macro expansion across forms  
  2. Map/Set correctness (structural hash + equality)  
  3. CAS retry loop for `swap!`
