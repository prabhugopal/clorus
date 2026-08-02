# Profile Conformance Status

Scope: `button`, `textfield`, `textarea`, `dropdown`, `card` against design profiles:
- `:apple-hig`
- `:material3`
- `:fluent`

Date: 2026-03-12

## Pass Summary

- `button`: PASS (tokenized radius/shadow/color + style selector)
- `textfield`: PARTIAL (tokenized radius/shadow/border/text; no stable style-variant branch yet)
- `textarea`: PARTIAL (tokenized radius/shadow/border/text; no stable style-variant branch yet)
- `dropdown`: PARTIAL (tokenized radius/shadow/border + chevron icons; overlay-order still demo-managed)
- `card`: PASS (style variants + tokenized radius/shadow/colors)

## What Was Hardened In This Slice

- Removed hardcoded geometry/color values where safe and switched to theme tokens:
  - `textfield`: radius + shadow token usage
  - `textarea`: radius + shadow token usage
  - `dropdown`: radius + shadow token usage, selected-row radius token
  - `card`: shadow token usage
- Kept implementations on compiler-stable code paths (no known module crash introduced).

## Remaining Gaps

- Global overlay pass in core rendering (dropdown currently depends on render order in demos).
- Full profile-specific interaction rules (focus ring thickness, hover/press state layering) not yet normalized for all controls.
- `textfield`/`textarea`/`dropdown` style branches need incremental reintroduction with compiler-safe patterns.

## Validation Performed

- `coral-ui` build: PASS
- `coral-core` workspace pack: PASS
- `coral-examples/gallery` build against repacked clips: PASS

## Next Target

1. Implement core overlay pipeline for popups.
2. Reintroduce stable style variants for `textfield`/`textarea`/`dropdown`.
3. Add profile conformance gates to `docs/gui/EXECUTION_CHECKLIST.md`.
