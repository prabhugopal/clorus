# Language (Single Source of Truth)

This is the canonical language status/spec document for Clorus.

## Scope

- Core syntax and semantics
- Clojure parity status (non-JVM)
- Supported and unsupported language features
- REPL language behavior expectations

## Canonical References

- Parity baseline: `docs/LANGUAGE_PARITY.md`
- Parity checklist: `docs/PARITY_CHECKLIST.md`
- Language spec reference: `docs/reference/LANGUAGE_SPEC.md`

## Current Policy

- Treat this file as the authoritative language entrypoint.
- Feature changes must update this file and linked canonical trackers.
- Session notes and completed milestone docs are historical context, not source of truth.

## Status Snapshot

- Primary target: high Clojure compatibility without JVM interop.
- Runtime/compiler parity work is tracked via `docs/PARITY_CHECKLIST.md`.

## Open Work (Top Level)

- Remaining long-tail parity edge cases
- Ongoing REPL semantics validation for parity-sensitive flows
- Keep language behavior tests passing on both `jit` and `legacy`
