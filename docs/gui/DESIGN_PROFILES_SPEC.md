# Design Profiles Spec

Status: active
Owner: Coral UI

This defines the design-system contract for Coral UI profiles aligned to:
- Apple HIG (`:apple-hig`)
- Material 3 (`:material3`)
- Microsoft Fluent (`:fluent`)

## Goals

- Keep one behavior model across components.
- Vary visual language via profile tokens and profile rules.
- Prevent drift with explicit pass/fail conformance checks.

## 1) Token Contract

Every profile must define these keys.

### Core surfaces and text
- `:surface`
- `:surface-2`
- `:surface-3`
- `:text-primary`
- `:text-muted`

### Accent and semantic colors
- `:primary`
- `:primary-strong`
- `:accent`
- `:success`

### Borders and depth
- `:border`
- `:border-soft`
- `:shadow`

### Control-specific tokens
- `:control-track`
- `:control-track-disabled`
- `:control-active`
- `:control-active-disabled`
- `:control-knob`
- `:control-knob-border`
- `:control-knob-border-disabled`
- `:checkbox-border`
- `:checkbox-bg`
- `:checkbox-check`
- `:toggle-off`
- `:toggle-on`

### Scale tokens
- `:radius-sm`
- `:radius-md`
- `:radius-lg`
- `:space-1`
- `:space-2`
- `:space-3`

### Default style selectors
- `:input-style`
- `:button-style`
- `:card-style`

## 2) Profile Rules

These are mandatory profile-level rules (not optional visual tweaks).

### Apple HIG
- Rounded controls, soft depth.
- Clear focus ring; low visual noise.
- Compact spacing; neutral surfaces.
- Prefer `:soft` control styles.

### Material 3
- Larger corner radii, tonal surfaces.
- Strong primary/accent role in focused/active states.
- Clear state layers for hover/focus/pressed.
- Prefer `:default` button style and tonal elevation.

### Fluent
- Crisper corners, moderate depth.
- High legibility, clear borders.
- Spacious but efficient layout rhythm.
- Emphasize contrast in interactive state transitions.

## 3) Component Spec Matrix

Each component must define profile-specific behavior for:
- Size defaults (height, padding)
- Radius source (`:radius-*`)
- Shadow/elevation behavior
- Focus style
- Hover/pressed/disabled colors
- Iconography conventions

Current coverage targets:
- `button`
- `textfield`
- `textarea`
- `dropdown`
- `card`
- `slider`
- `checkbox`
- `toggle`

## 4) Layering and Overlay Rules

Global rule:
- Popup surfaces (dropdown menus, future menus/tooltips/dialog overlays) must render in a top overlay pass.

Do not rely on per-demo draw order as a long-term solution.

Required behavior:
- Overlay draw on top of non-overlay components.
- Overlay hit-testing has priority while open.
- Outside click closes overlay unless consumed by overlay children.

## 5) Interaction and Accessibility Rules

Minimum requirements:
- Visible focus state on keyboard navigation.
- Deterministic keyboard behavior for controls.
- Minimum hit target size for pointer interactions.
- Contrast targets documented and testable.

## 6) Conformance Checklist

Per profile (`:apple-hig`, `:material3`, `:fluent`) and per component:
- [ ] Token contract complete.
- [ ] Visual state parity (default/hover/focus/pressed/disabled).
- [ ] Keyboard interaction parity.
- [ ] Overlay behavior parity (if applicable).
- [ ] Gallery demo validated.
- [ ] Regression snapshot/log attached.

## 7) Validation Workflow

1. Apply profile in gallery Theme demo.
2. Validate all core widgets from `docs/gui/WIDGET_PARITY_MATRIX.md` Tier 0.
3. Run build + gallery smoke.
4. Record pass/fail deltas in `docs/gui/EXECUTION_CHECKLIST.md`.

## 8) Versioning

- Any token key removal is a breaking change.
- Any profile rule change requires checklist rerun for all profiles.
- New widgets must include profile mappings before marked complete.
