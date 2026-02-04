# clorus-replx - Extended REPL with Smart Execution

**The REPL that doesn't break itself.**

`clorus-replx` (pronounced "replex") is an enhanced Clorus REPL with smart, adaptive execution that automatically detects and handles operations that would block or hang the REPL.

## Features

✨ **Auto-Detection** - Automatically detects GUI, server, and blocking operations
🧠 **Self-Healing** - Prevents REPL from blocking/hanging
🔧 **Zero Config** - Works out of the box with smart defaults
💡 **Helpful** - Suggests optimizations and best practices
🎯 **Context-Aware** - Understands what you're trying to do

## The Problem

Traditional REPLs block when you call functions that:
- Show GUI windows (event loops)
- Start servers (infinite loops)
- Do long-running computations

```clojure
user> (gui/show-gui "Hello")
;; REPL hangs! Can't type anything!
;; Have to kill the process :(
```

## The Solution

`replx` automatically detects these situations and adapts:

```clojure
user> (gui/show-gui "Hello")
⚡ Detected: GUI function (blocks on main thread)
✓ Auto-adapted: Spawned in detached context
→ Window opened

"Window-1"  ; Returns immediately!

user> (+ 1 2)  ; REPL still responsive!
3
```

## Installation

```bash
# Build from source
cargo build --release -p clorus-replx

# Binary available as 'replx'
./target/release/replx
```

## Usage

### Basic

```bash
# Start extended REPL
replx

# With main-thread mode (for GUI on macOS)
replx --main-thread
```

### Options

```bash
replx [OPTIONS]

OPTIONS:
  --main-thread     Run on main thread (for GUI on macOS)
  --no-adapt        Disable adaptive features
  --warn            Warn about issues but don't auto-fix
  --silent          Auto-adapt silently (no messages)
  --no-gui-detach   Don't auto-detach GUI functions
  -h, --help        Print help
```

## How It Works

### 1. Smart Detection

`replx` analyzes function calls and detects:

- **GUI Functions**: `show-gui`, `window`, `display` → needs main thread
- **Server Functions**: `serve`, `listen`, `server` → infinite loop
- **Compute Functions**: `factorial`, `calculate` → potentially long
- **I/O Functions**: `fetch`, `download`, `read` → may block

### 2. Adaptive Execution

Based on detection, applies strategies:

| Pattern | Strategy | Result |
|---------|----------|--------|
| GUI (blocking) | Detached main-thread | Window opens, REPL continues |
| Server | Warn + suggest async | User decides |
| Long compute | Suggest background | Helpful hint |
| I/O | Normal | Works as expected |

### 3. Learning (Optional)

With `--learn` flag (future):
- Learns from your usage patterns
- Remembers which functions block
- Auto-adapts based on history

## Configuration

Create `~/.clorus/replx.toml`:

```toml
[replx]
enabled = true
level = "auto"  # auto, warn, silent, none

[replx.adaptive]
auto-detach-gui = true
auto-detach-servers = false
suggest-optimizations = true
warn-long-running-ms = 5000

[replx.patterns]
# Custom patterns
detach = ["my-gui-*", "custom-window"]
```

## Adaptive Levels

- **`auto`** (default): Auto-fix with notifications
- **`warn`**: Show warnings, don't fix
- **`silent`**: Auto-fix without messages
- **`none`**: Traditional REPL behavior

## Examples

### GUI Development

```clojure
;; Traditional REPL - hangs!
user> (gui/show-gui "Hello")
;; <stuck>

;; With replx - works!
replx> (gui/show-gui "Hello")
⚡ Auto-adapted: Detached execution
"Window-1"

replx> (gui/update-title "New Title")  ; Still works!
:ok
```

### Server Development

```clojure
replx> (server/start 8080)
⚠️  Warning: Server function detected (runs indefinitely)
💡 Suggestion: Use (async (server/start 8080)) for background
```

### Long Computations

```clojure
replx> (factorial 1000000)
💡 Hint: Large computation detected
   Consider: (async (factorial 1000000))
```

## Architecture

```
clorus-replx/
├── detector.rs    # Smart function analysis
├── strategy.rs    # Execution strategies & config
├── lib.rs         # Core ReplX engine
└── main.rs        # replx binary
```

### Pluggable Design

- Built on top of `clorus-repl` (doesn't modify core)
- Can be disabled completely via feature flags
- Optional at compile time
- Configurable at runtime

## Future Features

- 🔄 **Learning Mode**: Learn from execution history
- ⏮️ **Time Travel**: Rewind/replay REPL sessions
- 🔍 **Enhanced Inspection**: Deep value inspection
- 🎨 **Data Visualization**: Terminal-based viz
- 👥 **Collaborative REPL**: Multi-user sessions

## Philosophy

> **"The REPL should be smart enough not to break itself."**

Good developer tools should:
1. Get out of your way
2. Help when you need it
3. Learn from you
4. Never surprise you negatively

## Comparison

| Feature | Traditional REPL | replx |
|---------|-----------------|-------|
| GUI calls | Hangs | Auto-detaches |
| Server starts | Hangs | Warns + suggests |
| Long compute | Blocks | Suggests async |
| Configuration | None | Optional, smart defaults |
| Learning | No | Yes (optional) |

## Contributing

`clorus-replx` is designed to be extensible:

- Add new detection patterns
- Implement custom strategies
- Contribute learning algorithms
- Improve heuristics

See `CONTRIBUTING.md` for details.

## License

MIT OR Apache-2.0
