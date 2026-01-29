# Testing Autocomplete in Clorus REPL

To test autocomplete, you'll need to run the REPL interactively. Here's what should work:

## Test 1: Basic Autocomplete
```bash
$ cargo run --bin repl

λ> (d<TAB>
# Should show: def  defn

λ> (def<TAB>
# Should show: def  defn

λ> (defn<TAB>
# Should complete to: defn
```

## Test 2: Function Completions
```bash
λ> (fs/<TAB>
# Should show all fs/ functions:
# fs/read  fs/write  fs/append  fs/exists?  fs/is-file?
# fs/is-dir?  fs/copy  fs/rename  fs/remove
# fs/create-dir  fs/create-dir-all

λ> (fs/w<TAB>
# Should show: fs/write

λ> (fs/wr<TAB>
# Should complete to: fs/write
```

## Test 3: Module Names
```bash
λ> (use <TAB>
# Should show: rust.fs  clorus.core

λ> (use rust.<TAB>
# Should show: rust.fs

λ> (use clorus.<TAB>
# Should show: clorus.core
```

## Test 4: clorus.core Functions
```bash
λ> (use clorus.core)
=> 0

λ> (sl<TAB>
# Should complete to: slurp

λ> (sp<TAB>
# Should complete to: spit
```

## Test 5: History (Arrow Keys)
```bash
λ> (+ 1 2)
=> 3

λ> (+ 3 4)
=> 7

# Press UP arrow
# Should show: (+ 3 4)

# Press UP arrow again
# Should show: (+ 1 2)

# Press DOWN arrow
# Should show: (+ 3 4)
```

## Test 6: Line Editing
```bash
λ> (+ 1 2 3 4 5)
# Press Ctrl-A (should move cursor to beginning)
# Press Ctrl-E (should move cursor to end)
# Press ← → arrows (should move cursor left/right)
```

## Test 7: History Persistence
```bash
λ> (+ 1 2)
=> 3
λ> :quit
Goodbye!

# Start REPL again
$ cargo run --bin repl

# Press UP arrow - should show: (+ 1 2)
# History is saved to ~/.clorus_history
```

## Expected Completions List

**Operators**: +, -, *, /, <, >, =

**Special Forms**: def, defn, let, if, use

**rust.fs Functions**:
- fs/read
- fs/write
- fs/append
- fs/exists?
- fs/is-file?
- fs/is-dir?
- fs/remove
- fs/copy
- fs/rename
- fs/create-dir
- fs/create-dir-all

**clorus.core Functions**:
- slurp
- spit

**Module Names**:
- rust.fs
- clorus.core

Run the REPL and try these tests! 🎯
