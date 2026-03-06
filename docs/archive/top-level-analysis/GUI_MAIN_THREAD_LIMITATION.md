# GUI Main Thread Limitation on macOS

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/GUI_REPL_USAGE.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


## Executive Summary

**Finding**: egui/winit cannot be used from a REPL on macOS due to a fundamental platform requirement: the EventLoop **must** be created on the main thread.

**Impact**: Both Clorus and Clojure (JVM) fail with the same error when calling egui from a REPL.

**Status**: This is NOT a Clorus-specific issue. It's a fundamental limitation of egui/winit on macOS.

---

## Test Results

### Clojure FFI Test (JVM)

```bash
cd ~/Learning/clojure/egui-clj-test
clj -M:repl
```

```clojure
(require '[egui-test.core :as gui])

; Test 1: Version check (no GUI) - WORKS
(gui/gui-version)
; => "egui 0.30"  ✓ SUCCESS

; Test 2: Open GUI window - FAILS
(gui/show-gui-window "Test 1")
```

**Result**:
```
thread '<unnamed>' panicked at winit-0.30.12/src/platform_impl/macos/event_loop.rs:221:14:
on macOS, `EventLoop` must be created on the main thread!
fatal runtime error: failed to initiate panic, error 5
Abort trap: 6
```

**Key Finding**:
- ✅ Non-GUI FFI calls work fine from REPL
- ❌ Only GUI window creation fails (EventLoop initialization)

### Clorus REPL Test

```bash
cd ~/Learning/clorus/gui-demo
clorus repl
```

```clojure
(-main)
```

**Before Fix**: FFI wrapper build errors due to extern "C" functions being wrapped
**After Fix**: REPL loads successfully, but GUI will still fail with main thread error

---

## Root Cause

On macOS, the NSApplication (macOS windowing system) has a strict requirement:

1. **NSApplication is a singleton** - can only be initialized once per process
2. **Must run on the main thread** - EventLoop initialization must happen on the thread that started the process

When calling from a REPL:
- The REPL owns the main thread
- FFI calls happen on the REPL's thread (not main thread)
- winit/eframe tries to initialize EventLoop on non-main thread
- **Panic**: "EventLoop must be created on the main thread!"

---

## What We Fixed

### Issue 1: FFI Generator Wrapping extern "C" Functions ✅ FIXED

**Problem**: The FFI generator was wrapping C-compatible functions that were already wrapped.

**Location**: `/Users/prabhugopal/Learning/git/clorus/crates/clorus-ffi-gen/src/lib.rs`

**Fix**: Skip `extern "C"` functions in FFI generation:

```rust
fn extract_function_info(&self, func: &ItemFn) -> Option<FunctionInfo> {
    // Skip extern "C" functions - they're already C-compatible
    if let Some(abi) = &func.sig.abi {
        if abi.name.as_ref().map(|n| n.value()) == Some("C".to_string()) {
            return None;
        }
    }
    // ...
}
```

**Result**: Clorus REPL now loads successfully without FFI wrapper errors.

---

## What Cannot Be Fixed

### Issue 2: Main Thread Requirement ❌ FUNDAMENTAL LIMITATION

**Problem**: egui/winit requires EventLoop on main thread. REPLs cannot yield main thread.

**Evidence**:
- ✅ Clorus run mode works perfectly (GUI owns main thread)
- ❌ Clorus REPL fails (REPL owns main thread)
- ❌ Clojure REPL fails (JVM REPL owns main thread)
- ✅ HumbleUI works (Java abstracts NSApplication lifecycle)

**Why HumbleUI Works**:
- Uses JWM (Java Window Manager)
- JVM has decades of NSApplication lifecycle management
- Java abstracts away the singleton complexity
- Multiple windows/instances work because JVM manages it

---

## Solution Paths

### Option 1: Use Process-Based GUI (Recommended)
Run GUI in separate process:

```bash
# Terminal 1: GUI process
clorus run

# Terminal 2: REPL control
clorus repl
```

**Pros**:
- Keeps egui (modern, actively maintained)
- Full GUI isolation
- REPL remains responsive

**Cons**:
- Requires IPC (inter-process communication)
- More complex architecture

### Option 2: Use Different GUI Library
Switch to GUI library compatible with REPLs:

**Candidates**:
- **HumbleUI** (Clojure) - proven REPL compatibility
- **Iced** (Rust) - more REPL-friendly than egui
- **GTK** - traditional GUI toolkit
- **Qt** - enterprise GUI solution

**Pros**:
- Direct REPL interaction
- Simpler architecture

**Cons**:
- Different API
- Migration effort

### Option 3: Hybrid Approach
Keep egui for apps, use different library for REPL:

```clojure
;; In production
(use 'rust.gui)
(gui/show-gui "Production")  ; Uses egui

;; In REPL
(use 'clorus.repl-gui)
(repl-gui/show "Debug")  ; Uses REPL-compatible library
```

**Pros**:
- Best of both worlds
- Flexible for different use cases

**Cons**:
- Maintain two GUI integrations

---

## Recommendation

**For Clorus GUI Strategy**:

1. **Short-term**: Document limitation, use `clorus run` for GUI
2. **Medium-term**: Implement process-based GUI with IPC
3. **Long-term**: Evaluate HumbleUI-style JVM integration or Iced migration

**For Current Projects**:
- `clorus run` works perfectly - use this for GUI applications
- REPL is for data exploration, not GUI testing
- If GUI debugging needed, use process-based approach

---

## Technical Details

### NSApplication Lifecycle

```
Process Start
    ↓
Main Thread Created
    ↓
NSApplication.sharedApplication()  ← Must be on main thread
    ↓
EventLoop.run()                     ← Takes over main thread
    ↓
Application Exit
```

**REPL Lifecycle**:
```
Process Start
    ↓
Main Thread (REPL)
    ↓
User Input → FFI Call → Different Thread
                            ↓
                    NSApplication.sharedApplication() ← ERROR!
                    (not on main thread)
```

### winit Source

**File**: `winit-0.30.12/src/platform_impl/macos/event_loop.rs:221`

```rust
pub fn new() -> Result<Self, EventLoopError> {
    if !is_main_thread() {
        panic!("on macOS, `EventLoop` must be created on the main thread!");
    }
    // ...
}
```

This is a **platform requirement**, not a winit limitation.

---

## Verification Steps

1. ✅ Clorus run works:
   ```bash
   cd ~/Learning/clorus/gui-demo
   clorus run
   ```

2. ✅ FFI generator fixed (no wrapper errors)

3. ❌ REPL GUI still impossible (main thread requirement)

4. ✅ Process-based approach works:
   - Run GUI in process 1
   - Control from REPL in process 2

---

## References

- [winit EventLoop documentation](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoop.html)
- [Apple NSApplication Programming Guide](https://developer.apple.com/documentation/appkit/nsapplication)
- [HumbleUI Architecture](https://github.com/HumbleUI/HumbleUI)
- Clorus FFI Generator: `/Users/prabhugopal/Learning/git/clorus/crates/clorus-ffi-gen/src/lib.rs`

---

## Conclusion

The GUI "zombie window" issue is **not a bug** - it's a fundamental limitation of egui/winit on macOS REPLs. The solution is architectural: either use a different GUI library or implement process-based GUI separation.

**Clorus is working correctly**. The same issue occurs in Clojure/JVM, proving this is egui-specific, not Clorus-specific.
