# Passing Components to Clorus GUI

## Summary

**Yes! You can pass complex UI components/data to Clorus, but not as native objects yet.**

### What Works NOW

✅ **Primitives**: String, f64, i32, bool
✅ **JSON serialization**: Complex data as strings
✅ **Builder pattern**: Sequential function calls

### What Doesn't Work YET

❌ **Native maps/vectors**: `{:title "Hello" :buttons ["OK"]}`
❌ **Callbacks**: Passing Clorus functions to Rust
❌ **Objects**: Complex nested structures

---

## Approach 1: JSON Serialization (Works Now!)

**Pass complex UI config as JSON strings:**

### Rust Side
```rust
// egui-hello/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct WindowConfig {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub message: String,
    pub buttons: Vec<String>,  // Array of button labels
}

pub fn show_gui_json(config_json: String) -> f64 {
    let config: WindowConfig = serde_json::from_str(&config_json).unwrap();

    // Build GUI from config
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.width as f32, config.height as f32])
            .with_title(&config.title),
        ..Default::default()
    };

    eframe::run_simple_native(&config.title, options, move |ctx, frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&config.message);

            // Create all configured buttons
            for button_label in &config.buttons {
                if ui.button(button_label).clicked() {
                    // Handle click
                }
            }
        });
    });

    0.0
}
```

### Clorus Side
```clojure
(use rust.egui-hello)

; Define UI components as JSON
(def my-window "{
  \"title\": \"My App\",
  \"width\": 600.0,
  \"height\": 400.0,
  \"message\": \"Welcome to Clorus!\",
  \"buttons\": [\"Start\", \"Stop\", \"Settings\", \"Quit\"]
}")

; Display the configured GUI
(egui-hello/show-gui-json my-window)
=> 3  ; Returns number of clicks before quit
```

**Pros:**
- ✅ Works with current FFI (just String passing)
- ✅ Can pass arbitrarily complex structures
- ✅ Serialization is standard (serde_json)
- ✅ Auto-discovered by FFI system

**Cons:**
- ⚠️ No type safety (runtime errors if JSON wrong)
- ⚠️ Have to manually construct JSON strings
- ⚠️ Verbose syntax

---

## Approach 2: Builder Pattern (Works Now!)

**Build UI with sequential function calls:**

### Rust Side
```rust
// Window handle system
pub fn create_window(title: String, width: f64, height: f64) -> f64 {
    // Create window, return ID
    let id = /* allocate window */;
    id as f64
}

pub fn add_button(window_id: f64, label: String) -> f64 {
    // Add button to window
    0.0
}

pub fn add_label(window_id: f64, text: String) {
    // Add label to window
}

pub fn show_window(window_id: f64) -> f64 {
    // Display the window, block until closed
    0.0
}
```

### Clorus Side
```clojure
(use rust.egui-hello)

; Build UI imperatively
(def win (egui-hello/create-window "My App" 600.0 400.0))
(egui-hello/add-label win "Welcome!")
(egui-hello/add-button win "Start")
(egui-hello/add-button win "Stop")
(egui-hello/add-button win "Quit")
(egui-hello/show-window win)
```

**Pros:**
- ✅ Type-safe (each call checked)
- ✅ Familiar imperative style
- ✅ Easy to conditionally build UI

**Cons:**
- ⚠️ Requires state management (window IDs)
- ⚠️ More verbose than declarative
- ⚠️ Need to design good API

---

## Approach 3: Native Clorus Data (Future!)

**What we WANT eventually:**

```clojure
(use rust.egui-hello)

; Pass native Clorus map
(egui-hello/show-gui {
  :title "My App"
  :width 600
  :height 400
  :message "Welcome!"
  :buttons ["Start" "Stop" "Quit"]
  :on-click (fn [button]
    (println "Clicked:" button))
})
```

**Requires:**
1. ❌ **Value* support for Map/Vector** (Phase 2+)
2. ❌ **Callback marshaling** (pass Clorus functions to Rust)
3. ❌ **Complex type mapping** (nested structures)

**This is the ideal end state!**

---

## Current Demo

**What you can try RIGHT NOW:**

```bash
cd ~/Learning/clorus/gui-test
clorus run
```

**Edit `src/main.clrs`** and uncomment:
```clojure
(egui-hello/show-gui-json my-window)
```

**Result:** Opens a GUI window with:
- Custom title
- Custom size
- Custom message
- 5 buttons (Start, Stop, Settings, About, Quit)
- All configured from Clorus via JSON!

---

## Comparison to Other Systems

### React (JavaScript)
```jsx
<Window title="My App">
  <Button onClick={handleStart}>Start</Button>
  <Button onClick={handleStop}>Stop</Button>
</Window>
```

### SwiftUI
```swift
Window("My App") {
    VStack {
        Button("Start") { handleStart() }
        Button("Stop") { handleStop() }
    }
}
```

### Our Current Approach
```clojure
; Via JSON
(egui-hello/show-gui-json "{\"title\": \"My App\", \"buttons\": [\"Start\", \"Stop\"]}")

; Via Builder (if implemented)
(-> (create-window "My App")
    (add-button "Start")
    (add-button "Stop")
    (show-window))
```

### Our Target (Future)
```clojure
(ui/window {:title "My App"}
  (ui/button "Start" on-click: handle-start)
  (ui/button "Stop" on-click: handle-stop))
```

---

## Implementation Roadmap

### Phase 1 (NOW) ✅
- [x] Pass primitives (String, f64, i32, bool)
- [x] JSON serialization for complex data
- [x] Automatic FFI generation

### Phase 2 (Next)
- [ ] Support Map/Vector in Value*
- [ ] Pass Clorus collections to Rust
- [ ] Better ergonomics

### Phase 3 (Future)
- [ ] Callback marshaling
- [ ] Bidirectional communication
- [ ] Reactive UI bindings

---

## Recommendation

**For now, use JSON serialization:**

1. ✅ Works with current FFI system
2. ✅ Handles arbitrarily complex UI
3. ✅ Automatic function discovery
4. ✅ Type-safe on Rust side (serde)

**Later, we'll add native data structure support!**
