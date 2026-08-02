# GUI Platform Roadmap (SwiftUI-Level Target)

This roadmap defines the minimum capabilities and acceptance tests to bring Clorus + Coral to production-quality desktop app development.

## Scope

## Related Specs

- `docs/gui/DESIGN_PROFILES_SPEC.md`
- `docs/gui/WIDGET_PARITY_MATRIX.md`
- `docs/gui/EXECUTION_CHECKLIST.md`


- Rendering engine: `coral/coral-core/coral-gfx`
- UI framework core: `coral/coral-core/coral-ui/src/core`
- Widgets: `coral/coral-core/coral-ui/src/components`
- App examples + validation: `coral/coral-examples/*`
- Clorus integration points: REPL/run/build workflows in this repo

## Capability Matrix

| Capability | Target | Current Status | Primary Owner Area |
|---|---|---|---|
| Declarative composition | Stable, nested composable UI trees | In progress | `coral-ui/core` |
| Diff/reconcile model | Deterministic updates and no stale nodes | In progress | `coral-ui/core` |
| State + binding model | Predictable app/local/component state | In progress | `coral-ui/core`, stdlib |
| Layout system | Stack/grid/flex-like primitives with constraints | In progress | `coral-ui/core/layout` |
| Text + input | Correct cursor, selection, IME, clipboard | In progress | `coral-ui/components` |
| Event + focus routing | Keyboard/mouse/focus traversal correctness | In progress | `coral-ui/core` |
| Animation + transitions | 60fps common transitions, no hitches | Planned | `coral-ui/core`, `coral-gfx` |
| Theming/styling | Global/local theme overrides, consistent tokens | In progress | `coral-ui/core`, components |
| Accessibility | Keyboard-first + semantic metadata | Planned | `coral-ui/core`, components |
| Multi-window lifecycle | Open/close/focus/restore without leaks | In progress | `coral-gfx`, app runtime |
| Tooling loop | Fast build/run/repl + useful diagnostics | In progress | Clorus CLI + Coral examples |

## Acceptance Test Plan

Each slice must include deterministic tests plus one executable example scenario.

### A) Rendering Correctness

- [ ] Draw ordering is stable across frames.
- [ ] Clipping/scissor behavior is correct for nested containers.
- [ ] Text baseline and bounds are consistent across font sizes.
- [ ] No crash or corruption on repeated create/destroy window loops.

Validation:
- Add/update scenario under `coral-examples` (render stress page).
- Add smoke runner that loops create/draw/destroy N times.

### B) Input + Focus Correctness

- [ ] Tab/shift-tab traverses focusable controls deterministically.
- [ ] Mouse capture + drag release are correct when leaving bounds.
- [ ] Text fields: insert/delete/select/copy/paste behavior is stable.
- [ ] Slider/scrollbar interactions never corrupt memory or freeze.

Validation:
- Add interactive example page with scripted event replay.
- Add regression case for the slider crash path.

### C) State + Reconcile Correctness

- [ ] State updates are visible exactly once per update cycle.
- [ ] Collection rendering preserves logical order.
- [ ] Node identity semantics are explicit and tested (keyed lists).
- [ ] No stale closures after repeated re-render.

Validation:
- Add list/table sample with insert/remove/reorder operations.
- Add deterministic snapshot checks for rendered tree metadata.

### D) Performance + Stability Budgets

- [ ] Frame budget: 16.6ms target in standard showcase screen.
- [ ] Input latency budget documented and measured.
- [ ] Memory usage does not monotonically grow during idle + interaction.
- [ ] Long session (30+ minutes) has no crash/heap corruption.

Validation:
- Add benchmark/stress script in Coral repo.
- Track baseline numbers in docs with date + hardware metadata.

## Implementation Order (Execution Slices)

1. **Stability-first hardening**
   - Fix any remaining heap corruption/double free classes.
   - Close slider/input crash regressions.
2. **Core loop correctness**
   - Reconcile determinism + event/focus routing invariants.
3. **Text/editing quality bar**
   - Textfield/textarea correctness under heavy interaction.
4. **Layout + widget completeness**
   - Fill critical widget behavior gaps and layout edge cases.
5. **Performance and profiling hooks**
   - Add repeatable perf/stress checks.
6. **Accessibility + polish**
   - Keyboard semantics, role metadata, narration hooks.

## Cross-Repo Mapping

- Clorus repo responsibilities:
  - Language/runtime/compiler stability for GUI-hosted programs
  - CLI workflows (`run`, `repl`, `build`) and packaging reliability
  - Test harnesses that can run Coral app smoke tests
- Coral repo responsibilities:
  - Rendering engine behavior, UI framework correctness
  - Widget implementations and style system
  - Desktop showcase/sample app scenarios

## Exit Criteria for “SwiftUI-level trajectory”

- No known memory corruption crashes in showcase workflows.
- All core desktop interactions pass scripted acceptance tests.
- Performance budgets are measured and met on target hardware.
- Docs and examples allow building non-trivial desktop apps without custom runtime patching.

## Tracking

Use this roadmap as the source for GUI platform progress. Keep updates incremental:

- Add date-stamped status lines per capability.
- Link each closed item to commit IDs and test files.
