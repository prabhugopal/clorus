# Interactive TUI REPL Design Document

**Status:** 📋 PLANNED (Not Yet Implemented)
**Priority:** High (Post 100% Language Parity)
**Estimated Effort:** 5-7 days

---

## Vision

Transform the Clorus REPL from a basic line-based interface into a rich, interactive TUI (Terminal User Interface) similar to:
- OCaml's utop
- IPython
- Fish shell's interactive mode

**Goal:** Make Clorus the most developer-friendly compiled Lisp REPL experience.

---

## Current REPL (Baseline)

```
╔════════════════════════════════════╗
║  Clorus REPL v0.2.0                ║
║  Clojure-inspired systems language ║
╚════════════════════════════════════╝

λ> (+ 1 2)
=> 3

λ> _
```

**Limitations:**
- No autocomplete
- No inline documentation
- No syntax highlighting
- No function discovery
- No multi-line editing hints
- Basic history (up/down only)

---

## Proposed TUI REPL Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ Clorus REPL v0.3.0 | Namespace: user | Memory: 2.1 MB | Uptime: 5m │
├─────────────────────────────────────────────────────────────────────┤
│ History & Output                                                    │
│                                                                     │
│ λ> (def numbers [1 2 3 4 5])                                       │
│ => [1 2 3 4 5]                                                      │
│                                                                     │
│ λ> (map double numbers)                                            │
│ => [2 4 6 8 10]                                                     │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ λ> (fil█                                          ← Cursor here    │
│    ▼ filter (pred coll)                         ← Autocomplete     │
│      Returns elements for which pred is truthy                     │
│      Example: (filter even? [1 2 3 4]) => [2 4]                   │
├─────────────────────────────────────────────────────────────────────┤
│ Available Functions [24]              | Recent History [Ctrl+R]    │
│ • Core (12)                           | • (map double numbers)     │
│   defn, def, let, if, do, fn, quote   | • (def numbers [1 2 3])    │
│ • Collections (6)                      | • (filter even? data)      │
│   get, nth, first, rest, last, count  |                            │
│ • HOFs (3)                            | Quick Help [F1]            │
│   map, filter, reduce                 | • :help <fn>  - Show docs  │
│                                       | • :examples   - Show demos │
│ Press Tab to explore →                | • :quit       - Exit REPL  │
└─────────────────────────────────────────────────────────────────────┘

Keybindings: F1=Help | F2=Functions | F3=History | Ctrl+C=Cancel | Ctrl+D=Exit
```

---

## Core Features

### 1. Live Autocomplete

**Behavior:**
```
λ> (ma█
   ▼ map (f coll)
     Returns new collection with f applied to each element

λ> (map do█
         ▼ double
           (defn double [x] (* x 2))
```

**Features:**
- Fuzzy matching: `fil` matches `filter`
- Context-aware: Only show functions available in current namespace
- Signature display: Show parameter names
- Inline documentation preview
- Press Tab to complete, Enter to accept

**Implementation:**
- Maintain index of all defined functions
- Use fuzzy search algorithm (fzf-style)
- Hook into parser for semantic completion

---

### 2. Syntax Highlighting

**Real-time highlighting as you type:**

```clojure
λ> (defn factorial [n]
     (if (= n 0)
       1
       (* n (factorial (- n 1)))))
```

**Color Scheme:**
- **Special forms** (defn, let, if): Bright yellow/bold
- **Function names**: Cyan
- **Keywords**: Green
- **Strings**: Red
- **Numbers**: Blue
- **Comments**: Gray/dim
- **Matching parens**: Highlight pairs

**Implementation:**
- Use `syntect` for syntax highlighting
- Custom Clorus grammar (TextMate/Sublime syntax)
- Highlight as-you-type with incremental parsing

---

### 3. Inline Documentation

**Press Ctrl+H or hover to see docs:**

```
λ> (filter█ even? numbers)

   ┌─────────────────────────────────────────┐
   │ filter (pred coll)                      │
   │                                         │
   │ Returns a new collection containing     │
   │ only elements for which (pred elem)     │
   │ returns truthy (not nil, not false).    │
   │                                         │
   │ Examples:                               │
   │   (filter odd? [1 2 3 4])  => [1 3]    │
   │   (filter #(> % 5) [1 6 3]) => [6]     │
   │                                         │
   │ See also: map, reduce, remove, keep     │
   │                                         │
   │ Defined in: clorus.core                 │
   │ Type: (fn [fn coll] => coll)           │
   └─────────────────────────────────────────┘
```

**Sources:**
- Docstrings from `defn` (when we add metadata support)
- Built-in documentation database
- User-defined docs via `(doc filter)`

---

### 4. Multi-line Editing

**Smart indentation:**

```
λ> (defn complex-fn [x]
     (let [doubled (* x 2)      ← Auto-indented
           tripled (* x 3)]      ← Aligned
       (if (> doubled 10)        ← Proper nesting
         tripled                 ← Indented
         doubled)))              ← Closing parens aligned
```

**Features:**
- Auto-indent on newline
- Auto-close parens/brackets/braces
- Rainbow parentheses
- Show unmatched paren errors inline
- Ctrl+Enter to evaluate multi-line expression

---

### 5. Searchable History

**Fuzzy search history with Ctrl+R:**

```
┌─────────────────────────────────────┐
│ Search History (Ctrl+R)             │
│                                     │
│ Search: map█                        │
│                                     │
│ Results [3]:                        │
│ > (map double [1 2 3])             │
│   (map #(* % 2) numbers)           │
│   (->> data (map transform))       │
│                                     │
│ ↑↓ Navigate | Enter Select | Esc Cancel │
└─────────────────────────────────────┘
```

**Features:**
- Full-text search across all history
- Recent first, or sort by frequency
- Persist history to `~/.clorus_history`
- Import/export history
- Clear history with `:clear-history`

---

### 6. Function Browser Panel

**Press F2 to open:**

```
┌───────────────────────────────────────────┐
│ Function Browser (24 functions)           │
├───────────────────────────────────────────┤
│ Filter: █                                 │
│                                           │
│ ▼ Core Forms (12)                        │
│   • defn          Define function         │
│   • def           Define variable         │
│   • let           Local bindings         │
│   • if            Conditional            │
│   • do            Sequence expressions    │
│                                           │
│ ▼ Collections (6)                        │
│   • get           Get by key/index       │
│   • nth           Get at index           │
│   • first         First element          │
│                                           │
│ ▼ Higher-Order Functions (3)            │
│   • map           Transform collection    │
│   • filter        Select elements        │
│   • reduce        Accumulate values      │
│                                           │
│ Press Enter to insert | Esc to close     │
└───────────────────────────────────────────┘
```

**Features:**
- Categorized by type
- Search/filter by name or description
- Show signature on selection
- Enter to insert at cursor
- Show user-defined functions separately

---

### 7. Interactive Help System

**`:help <topic>` command:**

```
λ> :help map

┌─────────────────────────────────────────────┐
│ Help: map                                   │
├─────────────────────────────────────────────┤
│                                             │
│ SIGNATURE                                   │
│   (map f coll)                             │
│                                             │
│ DESCRIPTION                                 │
│   Applies function f to each element of    │
│   collection, returning a new collection   │
│   with the results.                        │
│                                             │
│ EXAMPLES                                    │
│   (defn double [x] (* x 2))                │
│   (map double [1 2 3 4 5])                 │
│   => [2 4 6 8 10]                          │
│                                             │
│   (map #(* % 2) [1 2 3])                   │
│   => [2 4 6]                               │
│                                             │
│ PERFORMANCE                                 │
│   Time: O(n)  Space: O(n)                  │
│                                             │
│ SEE ALSO                                    │
│   filter, reduce, mapcat, keep             │
│                                             │
│ Press 'e' to run example | 'q' to close    │
└─────────────────────────────────────────────┘
```

---

### 8. Status Bar

**Real-time information:**

```
┌─────────────────────────────────────────────────────────────┐
│ Namespace: user | Vars: 15 | Memory: 2.1 MB | Uptime: 5m  │
└─────────────────────────────────────────────────────────────┘
```

**Shows:**
- Current namespace
- Number of defined vars/functions
- Memory usage (if `--debug` flag enabled)
- REPL uptime
- Last evaluation time
- Errors/warnings indicator

---

### 9. Error Display

**Rich error messages:**

```
λ> (map double [1 2 "three"])

┌─────────────────────────────────────────────┐
│ ⚠ Runtime Error                             │
├─────────────────────────────────────────────┤
│ Type mismatch in function 'double'          │
│                                             │
│ Expected: Number                            │
│ Got: "three" (String)                       │
│                                             │
│ Stack trace:                                │
│   at double (line 1)                        │
│   in map (clorus.core)                      │
│                                             │
│ Suggestion:                                 │
│   Use (filter number? coll) before mapping  │
│   or add type checking in 'double'          │
│                                             │
│ Press 'h' for help | 'i' to inspect         │
└─────────────────────────────────────────────┘
```

**Features:**
- Syntax errors highlighted inline
- Runtime errors with stack traces
- Suggestions for common mistakes
- Link to documentation
- Colored error levels (warning/error/fatal)

---

### 10. Value Inspector

**Press Ctrl+I to inspect values:**

```
λ> (def data {:users [{:name "Alice" :age 30}
                      {:name "Bob" :age 25}]})

┌─────────────────────────────────────────────┐
│ Value Inspector: data                       │
├─────────────────────────────────────────────┤
│ Type: HashMap                               │
│ Count: 1                                    │
│                                             │
│ ▼ :users (Vector, 2 items)                 │
│   ▶ [0] (HashMap, 2 entries)               │
│   ▼ [1] (HashMap, 2 entries)               │
│       :name  => "Bob" (String)             │
│       :age   => 25 (Number)                │
│                                             │
│ Memory: 312 bytes                           │
│ Retained: 2 references                      │
│                                             │
│ Press ↑↓ to navigate | → to expand         │
└─────────────────────────────────────────────┘
```

**Features:**
- Tree view for nested structures
- Expand/collapse nodes
- Show types and sizes
- Copy path to clipboard
- Export to JSON/EDN

---

## Advanced Features

### 11. Benchmarking

**Built-in benchmarking:**

```
λ> :bench (reduce + 0 (range 10000))

┌─────────────────────────────────────────────┐
│ Benchmark Results                           │
├─────────────────────────────────────────────┤
│ Expression:                                 │
│   (reduce + 0 (range 10000))               │
│                                             │
│ Result: 49995000                            │
│                                             │
│ Performance:                                │
│   Mean:      125.3 μs                      │
│   Median:    124.8 μs                      │
│   Std Dev:   2.1 μs                        │
│   Min:       122.1 μs                      │
│   Max:       131.5 μs                      │
│                                             │
│ Iterations: 1000                            │
│ Total time: 125.3 ms                       │
└─────────────────────────────────────────────┘
```

---

### 12. REPL Commands

**Enhanced command system:**

```
:help              Show help
:help <topic>      Show help for specific function/form
:doc <fn>          Show documentation
:source <fn>       Show source code
:examples          Show example code snippets
:bench <expr>      Benchmark expression
:time <expr>       Time single execution
:inspect <var>     Inspect value structure
:ns <name>         Switch namespace
:vars              List all variables
:fns               List all functions
:clear             Clear screen
:reset             Reset REPL state
:history           Show command history
:save <file>       Save history to file
:load <file>       Load and execute file
:quit              Exit REPL
```

---

### 13. Session Management

**Save/Load REPL sessions:**

```
λ> :save-session my-work.repl

Saving session to my-work.repl...
  - 15 definitions
  - 42 commands
  - Current namespace: user
✓ Session saved

λ> :load-session my-work.repl

Loading session from my-work.repl...
  ✓ Restored 15 definitions
  ✓ Restored namespace: user
  ✓ Loaded history (42 items)
Ready!
```

---

### 14. Code Snippets

**Quick insert common patterns:**

```
λ> :snippet defn

┌─────────────────────────────────────────────┐
│ Snippets: defn                              │
├─────────────────────────────────────────────┤
│ 1. Simple function                          │
│    (defn $name [$args]                      │
│      $body)                                 │
│                                             │
│ 2. Function with docstring                  │
│    (defn $name                              │
│      "$doc"                                 │
│      [$args]                                │
│      $body)                                 │
│                                             │
│ 3. Multi-arity function                     │
│    (defn $name                              │
│      ([$args1] $body1)                      │
│      ([$args2] $body2))                     │
│                                             │
│ Select snippet [1-3]: █                     │
└─────────────────────────────────────────────┘
```

---

### 15. Visual Debugger

**Step through execution:**

```
λ> :debug (factorial 5)

┌─────────────────────────────────────────────┐
│ Debugger: factorial                         │
├─────────────────────────────────────────────┤
│ Current:                                    │
│ > (if (= n 0)                    ← Step 3  │
│     1                                       │
│     (* n (factorial (- n 1))))             │
│                                             │
│ Variables:                                  │
│   n = 5                                     │
│                                             │
│ Stack:                                      │
│   factorial (n=5)                           │
│   main                                      │
│                                             │
│ [s]tep | [n]ext | [c]ontinue | [q]uit     │
└─────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Foundation (2 days)
- **Day 1:** Switch to `ratatui` + `crossterm`
  - Replace rustyline with ratatui layout
  - Basic terminal setup and event loop
  - Render input/output panels

- **Day 2:** Input handling
  - Multi-line editing
  - Proper cursor movement
  - Ctrl+C, Ctrl+D handling

### Phase 2: Core Features (2 days)
- **Day 3:** Autocomplete
  - Build function index
  - Fuzzy matching
  - Popup rendering

- **Day 4:** Syntax highlighting
  - Integrate `syntect`
  - Create Clorus grammar
  - Real-time highlighting

### Phase 3: Advanced (2 days)
- **Day 5:** Help system
  - Documentation database
  - Help panel rendering
  - Command parsing

- **Day 6:** History & Polish
  - History search (Ctrl+R)
  - Status bar
  - Error display

### Phase 4: Extras (1 day)
- **Day 7:** Nice-to-haves
  - Function browser
  - Value inspector
  - Benchmarking

---

## Technical Stack

### Primary Libraries

**Terminal UI:**
```toml
ratatui = "0.28"        # TUI framework
crossterm = "0.27"      # Terminal control
```

**Editing:**
```toml
syntect = "5.2"         # Syntax highlighting
rustyline = "14.0"      # Alternative: keep for history
```

**Search:**
```toml
fuzzy-matcher = "0.3"   # Fuzzy search
nucleo = "0.5"          # Fast fuzzy matching (fzf-style)
```

**Utilities:**
```toml
serde = "1.0"           # Session serialization
chrono = "0.4"          # Timestamps
```

### File Structure

```
crates/clorus-repl/
├── src/
│   ├── main.rs              # Entry point
│   ├── tui/
│   │   ├── mod.rs           # TUI module
│   │   ├── app.rs           # Main app state
│   │   ├── layout.rs        # Panel layouts
│   │   ├── input.rs         # Input handling
│   │   ├── autocomplete.rs  # Autocomplete logic
│   │   ├── highlight.rs     # Syntax highlighting
│   │   ├── help.rs          # Help system
│   │   ├── history.rs       # History search
│   │   └── inspector.rs     # Value inspector
│   ├── docs/
│   │   └── builtin_docs.rs  # Documentation database
│   └── config.rs            # User preferences
└── assets/
    └── clorus.sublime-syntax # Syntax definition
```

---

## Configuration

**User config file:** `~/.clorus/config.toml`

```toml
[repl]
theme = "dark"              # dark, light, solarized
show_line_numbers = true
auto_indent = true
rainbow_parens = true
vim_mode = false            # Future: vim keybindings

[autocomplete]
enabled = true
min_chars = 2               # Start after 2 chars
max_suggestions = 10

[history]
max_entries = 10000
persist = true
file = "~/.clorus_history"

[colors]
special_form = "yellow"
function = "cyan"
keyword = "green"
string = "red"
number = "blue"
comment = "gray"
```

---

## Keybindings

### Navigation
- `↑/↓` - History navigation
- `Ctrl+A` - Start of line
- `Ctrl+E` - End of line
- `Ctrl+←/→` - Word jump
- `Home/End` - Line start/end

### Editing
- `Tab` - Autocomplete / Indent
- `Shift+Tab` - De-indent
- `Ctrl+W` - Delete word
- `Ctrl+K` - Kill to end
- `Ctrl+U` - Kill to start
- `Ctrl+Enter` - Eval multi-line

### Features
- `Ctrl+R` - Search history
- `Ctrl+H` - Show help
- `Ctrl+I` - Inspect value
- `F1` - Help
- `F2` - Function browser
- `F3` - History panel

### REPL Control
- `Ctrl+C` - Cancel current input
- `Ctrl+D` - Exit REPL
- `Ctrl+L` - Clear screen

---

## Mockup Screenshots

### 1. Main View
```
┌─────────────────────────────────────────────────────────────────────┐
│ Clorus REPL v0.3.0 | user | 2.1 MB | 5m                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│ λ> (defn factorial [n]                                             │
│      (if (= n 0)                                                    │
│        1                                                            │
│        (* n (factorial (- n 1)))))                                 │
│ => #'user/factorial                                                 │
│                                                                     │
│ λ> (factorial 5)                                                    │
│ => 120                                                              │
│                                                                     │
│ λ> _█                                                               │
└─────────────────────────────────────────────────────────────────────┘
F1=Help | F2=Functions | Ctrl+R=Search | Ctrl+C=Cancel | Ctrl+D=Exit
```

### 2. Autocomplete Active
```
│ λ> (ma█                                                             │
│    ┌─────────────────────────────────────┐                         │
│    │ map (f coll)                        │                         │
│    │ Returns new collection with f       │                         │
│    │ applied to each element             │                         │
│    │                                     │                         │
│    │ Example:                            │                         │
│    │   (map double [1 2 3])             │                         │
│    └─────────────────────────────────────┘                         │
```

### 3. Error Display
```
│ λ> (+ 1 "two")                                                      │
│ ⚠ Type Error: Cannot add Number and String                         │
│   Expected: Number                                                  │
│   Got: "two" (String) at argument 2                                │
│                                                                     │
│   Suggestion: Use (str 1 "two") to concatenate                     │
```

---

## Success Metrics

**User Experience:**
- ✅ New users can discover functions without docs
- ✅ Errors are clear and actionable
- ✅ Common tasks take < 3 keystrokes
- ✅ REPL feels responsive (< 50ms feedback)

**Developer Productivity:**
- ✅ 50% faster function discovery
- ✅ 80% fewer doc lookups needed
- ✅ 3x faster multi-line editing
- ✅ Zero copy-paste from external docs

**Comparison:**
- Match or exceed OCaml's utop UX
- Better than standard Clojure REPL
- Competitive with IPython for Python

---

## Future Enhancements

### Post-MVP Features
1. **Notebook mode** - Save sessions as executable notebooks
2. **Remote REPL** - Connect to running Clorus processes
3. **Graphing** - Plot data inline (like R)
4. **Profiler integration** - Visual flame graphs
5. **Git integration** - Show dirty state, commit from REPL
6. **Package manager** - Browse/install libraries
7. **AI assistance** - Code suggestions via LLM
8. **Collaborative REPL** - Multiple users, shared session

---

## Comparison to Other REPLs

| Feature | Clorus TUI | OCaml utop | Clojure REPL | IPython |
|---------|------------|------------|--------------|---------|
| Autocomplete | ✅ Fuzzy | ✅ | ❌ | ✅ |
| Syntax Highlight | ✅ | ✅ | ❌ | ✅ |
| Inline Docs | ✅ | ✅ | ❌ | ✅ |
| Multi-line Edit | ✅ | ✅ | ❌ | ✅ |
| History Search | ✅ | ✅ | ❌ | ✅ |
| Value Inspector | ✅ | ❌ | ❌ | ✅ |
| Function Browser | ✅ | ❌ | ❌ | ❌ |
| Benchmarking | ✅ | ❌ | ❌ | ✅ (%%timeit) |
| Debugger | 🔜 | ❌ | ❌ | ✅ |

---

## Next Steps

**When to implement:** After reaching 100% language parity

**Priority order:**
1. Basic TUI layout with ratatui
2. Autocomplete (biggest win)
3. Syntax highlighting
4. Inline help
5. History search
6. Everything else

**Estimated timeline:** 1-2 weeks for MVP, 1 month for full feature set

---

✅ **TUI REPL Design Complete!**

This will make Clorus one of the most pleasant compiled Lisps to work with! 🚀

Now let's get back to language parity...
