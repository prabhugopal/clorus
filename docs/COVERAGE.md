# Coverage (Single Source of Truth)

This is the canonical coverage/testing status document for Clorus.

## Scope

- Test suite topology
- Engine coverage (`jit` and `legacy`)
- Regression policy and quality gates
- Coverage reporting references

## Canonical References

- Test runner: `tests/run_all_tests.sh`
- Coverage assessment: `docs/implementation/COVERAGE_ASSESSMENT.md`
- Feature matrix: `docs/implementation/FEATURE_MATRIX.md`
- Test docs: `tests/README.md`

## Current Policy

- Engine parity is required for language/runtime-sensitive behavior.
- New feature work should include tests in appropriate category plus cross-engine validation.
- Coverage status and known gaps are tracked from this entrypoint.

## Status Snapshot

- Comprehensive suite runs across language/stdlib/compiler/integration categories.
- Isolation and parallel controls are available in `tests/run_all_tests.sh`.

## Open Work (Top Level)

- Expand interop E2E coverage for newly-supported type signatures
- Continue parity-edge regression additions
- Keep docs and test inventory in sync
