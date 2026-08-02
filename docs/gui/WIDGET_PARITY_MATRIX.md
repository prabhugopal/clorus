# Widget Parity Matrix

Target: reach production-grade widget breadth and quality comparable to SwiftUI / egui / immediate-mode ecosystems where relevant.

## Priority tiers

- Tier 0: must-have for real desktop apps
- Tier 1: productivity and app-shell completeness
- Tier 2: advanced/data-heavy UX

## Tier 0 (Core)

| Widget | Status | Gaps | Acceptance |
|---|---|---|---|
| Button | Present | keyboard semantics/a11y pass | Enter/Space activate, focus visible |
| Label/Text | Present | rich text/span support | wrapping + alignment stable |
| TextField | Present | selection/IME edge cases | edit invariants + no crash |
| TextArea | Present | multiline selection polish | long-session stable |
| Checkbox | Present (new) | tri-state + full semantics | keyboard + toggling correctness |
| RadioGroup | Present (new) | arrow-key semantics + focus polish | arrow key + focus behavior |
| Slider | Present | drag edge-case stability | no corruption under aggressive drag |
| ProgressBar | Present | determinate/indeterminate polish | animation + value bounds |
| Scrollbar | Present | capture/release edge cases | stable under out-of-bounds release |
| ScrollView | Partial | nested scroll + momentum behavior | deterministic event routing |
| Image | Present | scaling/fit modes completeness | contain/cover/fill consistency |
| Container/Stack/Grid | Present | constraints + edge-case layout | resize regression suite |

## Tier 1 (App shell)

| Widget | Status | Gaps | Acceptance |
|---|---|---|---|
| List | Partial | virtualization + keyed identity | large list perf + stable order |
| Table/DataGrid | Planned | headers, sort, selection | 10k-row perf target |
| Tabs | Planned | keyboard semantics | tab switching + focus handoff |
| Menu/MenuBar | Planned | shortcuts + role metadata | full keyboard operation |
| ContextMenu | Planned | event lifecycle | right-click + keyboard open |
| Dialog/Sheet | Partial/Planned | modality + focus trap | escape/enter semantics |
| Tooltip | Planned | timing + positioning | no clipping regressions |
| Dropdown/Select | Partial/Planned | keyboard and filtering | deterministic open/close |
| Toggle/Switch | Present (new) | keyboard semantics + focus polish | keyboard + pointer parity |

## Tier 2 (Advanced)

| Widget | Status | Gaps | Acceptance |
|---|---|---|---|
| TreeView | Planned | expand/collapse + keyboard | large tree perf |
| Inspector/PropertyGrid | Planned | editor composition | nested model editing |
| ColorPicker | Planned | UX + color models | HSV/RGB/alpha parity |
| Date/Time Picker | Planned | locale/format handling | deterministic parsing |
| RichText editor | Planned | selection model complexity | stability over long sessions |
| Canvas/Custom draw host | Planned | input routing + transforms | predictable redraw |
| Plot/Chart primitives | Planned | perf + interactions | zoom/pan stability |

## Implementation sequencing

1. Finish Tier 0 quality/stability and acceptance tests.
2. Add Tier 1 widgets needed for app shell scenarios.
3. Add Tier 2 based on concrete product usage and perf budgets.

## Notes on parity framing

- SwiftUI parity means declarative ergonomics + integration quality.
- egui parity means broad practical widget coverage and fast iteration.
- Immediate-mode patterns can be supported where they improve tooling or debug UX, but core Coral should keep deterministic retained-mode semantics.
