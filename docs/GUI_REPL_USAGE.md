# GUI in REPL: Usage Guide

**Date:** January 28, 2025

## Summary

This document explains how to use GUI windows in the Clorus REPL, and documents the macOS platform limitation with GUI event loops.

## The Problem: macOS Requires GUI on Main Thread

On macOS, AppKit (the GUI framework) **requires** that all GUI event loops run on the main thread. This is enforced by the operating system and cannot be bypassed:

```rust
// ❌ This fails on macOS:
std::thread::spawn(|| {
    eframe::run_simple_native(...);  // Panic: EventLoop must be on main thread!
});
```

Since the REPL runs on the main thread, and GUI also needs the main thread, they cannot run simultaneously.

## How HumbleUI Solves This

HumbleUI uses JWM (Java Window Manager), which has special handling for this:

```clojure
(defmacro start-app! [& body]
  `(util/thread           ; Spawns future/thread
     (app/start           ; Java-based window manager
       (fn [] ~@body))))  ; Handles platform-specific threading
```

The `App/start` call (from `io.github.humbleui.jwm.App`) internally manages the threading correctly for each platform. On macOS, it properly coordinates between Java's thread model and AppKit's requirements.

## Our Solution: Manual GUI Launch

Since we use egui/eframe (Rust-based, not Java), we don't have HumbleUI's infrastructure. Our approach:

### 1. Don't Auto-Execute GUI During Load

**Before (❌ Blocked REPL):**
```clojure
(defn -main []
  (gui/show-gui "Hello!"))

(-main)  ; <-- Called during project load, blocks REPL
```

**After (✅ REPL Loads):**
```clojure
(defn -main []
  (gui/show-gui "Hello!"))

;; NOTE: For REPL usage, manually call (-main) when ready
;; For compiled executables (clorus run), uncomment:
;; (-main)
```

### 2. Call GUI Functions Manually in REPL

```clojure
; REPL session:
gui.demo.coreλ> (-main)
; GUI window opens and blocks
; User interacts with GUI
; User closes window
; REPL becomes responsive again
42  ; <- Return value (e.g., click count)

gui.demo.coreλ> (+ 1 2)
3

gui.demo.coreλ> (-main)
; Can open GUI again when needed
```

## Usage Patterns

### Pattern 1: Interactive GUI (Best for REPL)

```clojure
(ns my.app
  (:rust [egui-hello :as gui]))

(defn show-window []
  (gui/show-gui "Hello from REPL!"))

; In REPL:
; my.appλ> (show-window)
; ; Window opens, interact, close
; ; Returns to REPL
```

### Pattern 2: Main Entry Point (Best for Executables)

```clojure
(ns my.app
  (:rust [egui-hello :as gui]))

(defn -main [args]
  (gui/show-gui "Hello from Clorus!"))

; For REPL: Call manually when ready
; my.appλ> (-main [])

; For executable: Uncomment top-level call
; (-main [])
```

### Pattern 3: JSON-Configured GUI

```clojure
(ns my.app
  (:rust [egui-hello :as gui]))

(def window-config "{
  \"title\": \"My App\",
  \"width\": 600.0,
  \"height\": 400.0,
  \"message\": \"Welcome!\",
  \"buttons\": [\"Start\", \"Stop\", \"Quit\"]
}")

(defn launch-gui []
  (gui/show-gui-json window-config))

; In REPL:
; my.appλ> (launch-gui)
; ; Complex GUI opens
```

## Platform Comparison

| Platform | Thread Model | Clorus Status |
|----------|--------------|---------------|
| **macOS** | GUI must be on main thread | ⚠️ Blocking behavior required |
| **Linux (X11)** | GUI can run on any thread | ⚠️ Blocking (could be non-blocking) |
| **Windows** | GUI can run on any thread | ⚠️ Blocking (could be non-blocking) |

**Current Implementation:** All platforms use blocking behavior for consistency.

## Compiled Executables vs REPL

### Compiled Executable (clorus run)

```clojure
; src/main.clrs
(ns my.app
  (:rust [egui-hello :as gui]))

(defn -main [args]
  (gui/show-gui "My App"))

(-main [])  ; <-- Auto-execute for executable
```

```bash
$ clorus run
# GUI opens immediately
# Blocks until window closes
# Program exits
```

### REPL Usage

```clojure
; src/main.clrs
(ns my.app
  (:rust [egui-hello :as gui]))

(defn -main [args]
  (gui/show-gui "My App"))

;; (-main [])  <-- Commented out for REPL
```

```clojure
$ repl
my.appλ> (+ 1 2)
3
my.appλ> (-main [])
; GUI opens, user interacts
; Window closes
0.0
my.appλ> (+ 3 4)
7
```

## Future Improvements

### Option 1: Conditional Auto-Execute

```clojure
;; Only auto-execute if NOT in REPL
(when-not (System/getProperty "clorus.repl")
  (-main []))
```

**Requires:** REPL to set an environment variable/property.

### Option 2: Async GUI API

Create a proper async wrapper around eframe:

```clojure
(gui/create-window config)  ; Returns window-id, non-blocking
(gui/window-open? window-id)  ; Check if still open
(gui/close-window window-id)  ; Close programmatically
```

**Requires:** Restructuring REPL to run on background thread (complex).

### Option 3: Use Different GUI Library

Switch to a library with better threading support:
- **iced** (Rust) - Better async support
- **tauri** (Rust) - Web-based, better for async
- **JWM** (Java) - Like HumbleUI, but requires JVM integration

## Best Practices

### ✅ DO:
- Define GUI launch functions but don't call them at top level
- Call GUI functions manually in REPL when ready
- Use meaningful function names (`show-dashboard`, `launch-editor`)
- Document that GUI functions block
- Return meaningful values (click counts, results)

### ❌ DON'T:
- Call GUI functions at top level in files loaded by REPL
- Assume GUI is non-blocking
- Try to interact with REPL while GUI is open (won't work)
- Spawn threads manually (won't work on macOS)

## Example Projects

### gui-demo (Simple Example)

```bash
$ cd ~/Learning/clorus/gui-demo
$ repl
gui.demo.coreλ> (-main)
# Opens simple demo window
```

### gui-test (JSON Configuration)

```bash
$ cd ~/Learning/clorus/gui-test
$ repl
examples.gui-componentsλ> (egui-hello/show-gui-json my-window)
# Opens configured GUI
```

## Comparison to Clojure/HumbleUI

| Aspect | HumbleUI | Clorus |
|--------|----------|---------|
| **GUI Library** | JWM (Java) | eframe (Rust) |
| **Threading** | App/start handles it | Must block on main thread |
| **REPL Usage** | GUI & REPL concurrent | GUI blocks REPL |
| **Auto-Execute** | Yes, doesn't block | No, would block |
| **Window Management** | Full API (close, focus, etc.) | Basic (block until close) |

## Technical Details

### Why Can't We Thread Like HumbleUI?

HumbleUI's approach:
```clojure
(util/thread          ; Spawns Clojure future
  (app/start          ; Starts Java AWT event loop
    (fn []
      (window ...))))  ; Creates window
```

What `App/start` does internally (pseudocode):
```java
public static void start(Runnable callback) {
  if (Platform.isMacOS()) {
    // Special handling for macOS
    runOnMainThread(() -> {
      initializeAppKit();
      callback.run();
      startEventLoop();  // Blocks main thread
    });
  } else {
    // Windows/Linux can use any thread
    callback.run();
    startEventLoop();
  }
}
```

Our situation with eframe:
```rust
pub fn show_gui(message: String) -> f64 {
    // eframe doesn't have platform-specific thread handling
    eframe::run_simple_native(title, options, |ctx, frame| {
        // GUI code
    });  // BLOCKS until window closes (macOS requirement)
}
```

The key difference: Java's AWT/Swing and JWM have decades of platform abstraction built in. They handle the threading complexity internally. Rust's egui/eframe is simpler but leaves threading to the developer.

## Recommendation

**For now:** Use the manual launch pattern documented here. It's simple, works reliably, and users understand the behavior.

**For future:** Consider switching to a GUI library with better async support (iced, tauri) or implementing a custom threading wrapper if GUI+REPL concurrency becomes critical.

## Status

✅ **Working:** GUI functions work correctly, don't block REPL loading
✅ **Documented:** Clear usage patterns and limitations
⚠️ **Limitation:** Cannot use REPL while GUI is open (inherent to platform)
🔮 **Future:** Could improve with better threading infrastructure

---

**Related Documentation:**
- `docs/GUI_COMPONENTS.md` - GUI component examples
- `docs/RUST_FFI_GUIDE.md` - Rust FFI integration
- `docs/sessions/SESSION_REPL_STRING_FIX.md` - REPL value display

**Related Issues:**
- macOS AppKit main thread requirement
- eframe blocking API
- REPL project auto-loading behavior
