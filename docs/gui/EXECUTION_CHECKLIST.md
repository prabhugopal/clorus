# GUI Execution Checklist

This is the execution companion for `docs/gui/ROADMAP.md`.

## How to use

- Pick the current slice.
- Run all required commands.
- Mark gate pass/fail with date and commit.
- Do not advance if any must-pass gate fails.

### Command conventions

- `CLORUS_BIN=/Users/prabhugopal/Workspace/github/clorus/target/debug/clorus`
- Coral examples root: `/Users/prabhugopal/Workspace/github/coral/coral-examples`
- Coral core root: `/Users/prabhugopal/Workspace/github/coral/coral-core`

## Slice 1: Stability Hardening

Owner: Runtime + UI Core

Tasks:
- [ ] Close known crashers in slider/text interaction paths.
- [ ] Ensure repeated window lifecycle loops are stable.
- [ ] Confirm no heap corruption in `run --jit` and `repl --jit` GUI paths.

Must-pass gates:
- [ ] `clorus run --jit` on gallery-style app runs and exits cleanly.
- [ ] `clorus repl --jit` then `(-main)` for GUI sample runs without crash.
- [ ] Stress loop test (create/destroy window N times) passes.

Commands:
```bash
# 0) One-command smoke gate
cd /Users/prabhugopal/Workspace/github/coral
scripts/gui_slice1_smoke.sh

# 1) Build clips used by examples
cd /Users/prabhugopal/Workspace/github/coral/coral-core
$CLORUS_BIN pack --workspace --output dist

# 2) Run gallery app via JIT
cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN run --jit

# 3) REPL JIT path
cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN repl --jit
# then run:
# (-main)

# 4) Crash instrumentation (if needed)
cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
CLORUS_DEBUG_RELEASE=1 CLORUS_GUARD_RELEASE=1 $CLORUS_BIN run --jit |& tee /tmp/clorus_gui_slice1.log
rg -n "double free detected|Heap corruption|release on refcount=0" /tmp/clorus_gui_slice1.log
```

Evidence:
- [ ] commit:
- [ ] logs/test files:
- [ ] date:

## Slice 2: Reconcile + Event/Focus Correctness

Owner: UI Core

Tasks:
- [ ] Deterministic event ordering under rapid input.
- [ ] Focus traversal (`Tab`/`Shift-Tab`) correctness.
- [ ] No stale node/update artifacts after repeated rerender.

Must-pass gates:
- [ ] Focus traversal scripted scenario passes.
- [ ] Event replay scenario produces deterministic output.

Commands:
```bash
cd /Users/prabhugopal/Workspace/github/coral
scripts/gui_slice2_focus_event_check.sh

cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN run --jit
# manual scripted path:
# - keyboard tab/shift-tab across focusable controls
# - slider drag + mouse leave/re-enter
# - scrollbar drag + release outside bounds
```

Evidence:
- [ ] commit:
- [ ] logs/test files:
- [ ] date:

## Slice 3: Text/Input Quality

Owner: Widgets + Input

Tasks:
- [ ] Textfield editing invariants (insert/delete/select/paste).
- [ ] Textarea multiline behaviors.
- [ ] Cursor/selection correctness under keyboard + mouse.

Must-pass gates:
- [ ] Input replay suite passes.
- [ ] No crash under long editing session test.

Commands:
```bash
cd /Users/prabhugopal/Workspace/github/coral
scripts/gui_slice3_text_input_check.sh

cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN run --jit
# manual scripted path:
# - textfield: type, backspace, arrow-nav, selection, paste
# - textarea: multiline edits, selection across lines
# - 10+ minute continuous edit/interaction soak
```

Evidence:
- [ ] commit:
- [ ] logs/test files:
- [ ] date:

## Slice 4: Layout + Widget Completeness

Owner: UI Core + Components

Tasks:
- [ ] Stack/grid/layout edge cases covered.
- [ ] Core widget interaction parity (button, slider, scrollbar, text input).
- [ ] Collection/list rendering order and identity behavior validated.
- [ ] `docs/gui/WIDGET_PARITY_MATRIX.md` Tier 0 items are implemented or explicitly marked with gaps.

Must-pass gates:
- [ ] Layout regression suite passes.
- [ ] Widget smoke suite passes.

Commands:
```bash
cd /Users/prabhugopal/Workspace/github/coral
scripts/gui_slice4_layout_widget_check.sh

cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN run --jit
# validate:
# - stack/grid resizing behavior
# - list ordering stability
# - slider/scrollbar/button/text widgets
```

Evidence:
- [ ] commit:
- [ ] logs/test files:
- [ ] date:

## Slice 5: Performance + Profiling Hooks

Owner: GFX + UI Core

Tasks:
- [ ] Frame timing instrumentation in sample app.
- [ ] Input latency measurement path documented.
- [ ] Memory growth checks for long sessions.

Must-pass gates:
- [ ] 60fps target maintained on standard showcase path.
- [ ] No monotonic memory growth in 30-min run.

Commands:
```bash
cd /Users/prabhugopal/Workspace/github/coral
RUN_SECONDS=1800 scripts/gui_slice5_perf_soak_check.sh

cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN run --jit
# record frame timing and memory stats with local profiler/log hooks
# keep app running + interact for >= 30 minutes
```

Evidence:
- [ ] commit:
- [ ] logs/test files:
- [ ] date:

## Slice 6: Accessibility + Final Polish

Owner: UI Core + Components

Tasks:
- [ ] Keyboard-first navigation end-to-end.
- [ ] Basic semantic roles/labels coverage for widgets.
- [ ] Final docs + examples cleanup.

Must-pass gates:
- [ ] Accessibility checklist passes for core widgets.
- [ ] Final GUI docs reviewed and linked.

Commands:
```bash
cd /Users/prabhugopal/Workspace/github/coral
scripts/gui_slice6_accessibility_check.sh

cd /Users/prabhugopal/Workspace/github/coral/coral-examples/gallery
$CLORUS_BIN run --jit
# verify keyboard-only navigation for core widgets
# verify role/label metadata checks once hooks exist
```

Evidence:
- [ ] commit:
- [ ] logs/test files:
- [ ] date:

## Global Release Gate

- [ ] No known memory corruption crashes in core showcase flows.
- [ ] All slice gates marked pass.
- [ ] Docs updated (`ROADMAP`, this checklist, and references).
- [ ] Regression tests integrated in CI/test runner.

Verification commands:
```bash
cd /Users/prabhugopal/Workspace/github/clorus
CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh

cd /Users/prabhugopal/Workspace/github/coral/coral-core
/Users/prabhugopal/Workspace/github/clorus/target/debug/clorus pack --workspace --output dist
```
