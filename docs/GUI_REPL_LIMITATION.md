# GUI + REPL Known Limitation (macOS)

## The Problem

On macOS, GUI windows created with eframe/egui **cannot be reliably closed and reopened** in the same REPL session.

### What Happens

```clojure
gui.demo.coreλ> (-main)
; GUI opens ✓
; You interact with it ✓
; You click "Quit" ✓
; REPL shows result: 5 ✓
; BUT: Window stays open (zombie) ❌
; AND: REPL becomes unresponsive ❌
```

### Why This Happens

**macOS NSApplication Limitation:**
- egui/eframe uses `winit` which initializes `NSApplication` on first window
- macOS `NSApplication` is designed to run for the lifetime of the process
- You **cannot** tear down and reinitialize `NSApplication` in the same process
- This is a fundamental macOS platform limitation, not a bug in our code

**Technical Details:**
```rust
// First call - works fine
eframe::run_native(...)  // Initializes NSApplication, runs event loop, returns

// Second call - BROKEN on macOS
eframe::run_native(...)  // NSApplication already initialized, undefined behavior
```

## The Solution: Use `clorus run`

GUI works perfectly when run as a standalone executable:

```bash
$ cd gui-demo
$ clorus run
# GUI opens, you interact, close window, program exits cleanly ✓
```

### Why This Works

In `clorus run` mode:
- Process starts
- NSApplication initializes
- GUI runs
- Window closes
- **Process exits** (NSApplication destroyed)
- Clean state every time ✓

## REPL Workaround

### Option 1: One GUI per REPL Session (Recommended)

```clojure
$ repl
gui.demo.coreλ> (-main)
; Open GUI once, use it, close it
; Restart REPL for another GUI session
```

**Recommendation:** After closing GUI, type `:quit` and restart REPL.

### Option 2: Don't Use GUI in REPL

Keep GUI functions for `clorus run` only. In REPL, test logic without GUI:

```clojure
gui.demo.coreλ> (get-version)
"egui 0.30"

gui.demo.coreλ> demo-message
"Welcome to Clorus!..."

; Test functions without opening GUI
```

## Comparison to HumbleUI

**Why HumbleUI doesn't have this problem:**

| Aspect | HumbleUI | Clorus + egui |
|--------|----------|---------------|
| **GUI Framework** | JWM (Java Window Manager) | eframe/winit (Rust) |
| **Language** | Java/Clojure | Rust |
| **Platform Layer** | Java AWT/Swing | Native (NSApplication) |
| **Lifecycle** | JVM manages NSApplication | Direct NSApplication control |
| **Multiple Windows** | ✅ JVM handles it | ❌ Can't reinitialize |

Java/JVM has **decades** of solving this exact problem. The JVM abstracts away NSApplication management and allows multiple window lifecycles.

## Future Solutions

### Option A: Process-Based Windows

Spawn each GUI window in a separate process:

```rust
pub fn show_gui_async(message: String) -> WindowHandle {
    // Spawn child process for GUI
    std::process::Command::new("egui-launcher")
        .arg(message)
        .spawn()
}
```

**Pros:** Clean separation, reliable lifecycle
**Cons:** More complex, IPC needed for communication

### Option B: Different GUI Framework

Switch to a GUI library with better REPL support:
- **iced**: Async-first, better lifecycle management
- **dioxus**: React-like, designed for hot-reload
- **tauri**: Web-based, separate process model

### Option C: JVM Integration

Use JWM (like HumbleUI) via JNI:
- Requires JVM dependency
- Adds complexity
- But solves the problem completely

## Current Status

✅ **GUI works perfectly in `clorus run`**
⚠️ **GUI has limitations in REPL on macOS**
📝 **Documented limitation with workarounds**

## Recommendation

**For Examples/Demos:**
- Use `clorus run` to show GUI functionality
- Document that GUI is single-use in REPL

**For Production Apps:**
- GUI apps should use `clorus run` (not REPL)
- Or implement process-based window management

**For REPL Development:**
- Test logic without GUI
- Open GUI once when needed
- Restart REPL for fresh GUI session

---

**Platform-Specific:**
- **macOS**: Limitation exists (NSApplication)
- **Linux/Windows**: Same limitation (for consistency)
  - Could potentially be fixed on these platforms
  - But keeping behavior consistent across platforms

## Example Usage

### ✅ CORRECT: clorus run

```bash
$ cd gui-demo
$ clorus run
# GUI opens, works perfectly
```

### ⚠️ LIMITED: REPL

```bash
$ cd gui-demo
$ repl
gui.demo.coreλ> (-main)
# GUI opens, works ONCE
# After closing, restart REPL

gui.demo.coreλ> :quit
$ repl
# Fresh session, can use GUI again
```

### ❌ AVOID: Multiple GUI calls in REPL

```clojure
gui.demo.coreλ> (-main)
; Works
gui.demo.coreλ> (-main)
; BROKEN - zombie window, frozen REPL
```

---

**Bottom Line:** This is a known platform limitation. Use `clorus run` for GUI applications. REPL is for testing logic, not for repeated GUI interactions.
