# Clorus Language Issues
Issues discovered during CORAL GUI framework development

## Status Legend
- ✅ FIXED - Issue has been resolved
- 🔴 CRITICAL - Blocks production applications
- 🟡 HIGH - Workarounds exist but painful
- 🟢 LOW - Minor inconvenience

---

## Issue #1: Loop/Recur Parameters Don't Update ✅ FIXED
**Status:** ✅ FIXED in previous session
**Severity:** 🔴 CRITICAL
**Impact:** Made loops completely unusable

**Problem:**
Loop parameters never updated when `recur` was called, causing infinite loops.

**Root Cause:**
Loop parameters stored in allocas were loaded once at loop start. LLVM optimizer cached the load, so recur's stores never took effect.

**Solution:**
Implemented phi nodes at loop_start to properly merge control flow from entry and recur branches.

---

## Issue #2: String Use-After-Free in Atom Updates ✅ FIXED
**Status:** ✅ FIXED in previous session
**Severity:** 🔴 CRITICAL
**Impact:** Only ONE text field could hold text at a time

**Problem:**
Multiple string atoms caused memory corruption due to use-after-free.

**Root Cause:**
`rust_string_to_value()` moved String, then borrowed it, then deallocated it while Value still pointed to freed memory.

**Solution:**
Box the String before creating Value to keep it on heap with stable pointer.

---

## Issue #3: Mutual Recursion with `declare` ✅ FIXED
**Status:** ✅ FIXED in commit cc04caa
**Severity:** 🔴 CRITICAL (was blocking)
**Impact:** Can now implement tree-walking renderers, visitors, and recursive data processors

### Problem
The `declare` form had a namespace mangling mismatch - it was missing `.replace('-', "_")` on the namespace name, causing forward-declared functions to be stored under different mangled names than what function call resolution looked up.

### Expected Behavior (Clojure)
```clojure
(declare render-component)

(defn render-children [children]
  (doseq [child children]
    (render-component child)))

(defn render-component [comp]
  (println "Rendering" (:tag comp))
  (render-children (:children comp)))
```
This should work - `declare` tells compiler "this function exists, trust me."

### Actual Behavior (Clorus)
```clojure
(declare render-component)

(defn render-children [children]
  (render-component (first children)))  ; ERROR: Undefined function

(defn render-component [comp]
  (render-children (:children comp)))
```
**Compile Error:** `Undefined function: render-component`

### Workarounds Attempted

**1. Using `declare`** ❌ Doesn't work at all
```clojure
(declare foo)
(defn bar [] (foo))
(defn foo [] (bar))
; Error: Undefined function: foo
```

**2. Code inlining** ✅ Works but unmaintainable
```clojure
(defn render-element [window tag props children]
  (cond
    (= tag :column)
    ; Inline the child rendering logic instead of calling render-component
    (loop [remaining children]
      (when (> (count remaining) 0)
        (let [child (first remaining)
              child-tag (first child)
              child-props ...]
          ; Manually duplicate render-element logic here
          (cond
            (= child-tag :text) ...
            (= child-tag :button) ...
            (= child-tag :column) ...  ; Can't recurse! Must inline AGAIN
            ))))

    (= tag :text) ...))
```
This becomes unmaintainable for deep trees.

**3. Restructuring to avoid recursion** ⚠️ Sometimes possible
- Flatten data structures
- Use iteration instead of recursion
- Limited to specific cases

### Impact on CORAL Development

**Blocked Features:**
1. **Tree renderers** - Can't walk Hiccup component trees naturally
2. **Nested layouts** - Column/Row can't contain Column/Row recursively
3. **Component composition** - Can't implement HOCs or component wrappers
4. **AST walkers** - Any code analysis/transformation tools

**Current State:**
We're forced to inline column child rendering, which means:
- ❌ Column can't contain nested columns
- ❌ Code duplication for every component type
- ❌ Hard to add new component types
- ❌ Can't implement proper Reagent-style architecture

### Files Affected
- `/Users/prabhugopal/Learning/clorus/coral/examples/gallery/coral-gallery-simple.clrs` (lines 169-222)
  - Had to inline child rendering in `:column` component
  - Can't properly implement recursive layouts

### What Needs to Be Fixed

**In the Compiler:**
Location: `/Users/prabhugopal/Learning/git/clorus/crates/clorus-codegen/src/codegen.rs`

1. **Handle `declare` form** - Currently might be ignored or not implemented
2. **Create forward declarations** - When processing `declare`, add function stub to symbol table
3. **Allow forward references** - When compiling function calls, check both:
   - Already compiled functions
   - Forward-declared functions
4. **Link at end** - After all functions compiled, verify all forward declarations were defined

**Possible Implementation:**
```rust
// In codegen.rs
struct Codegen {
    // Existing fields...
    forward_declared_functions: HashSet<String>,
}

fn compile_declare(&mut self, names: &[String]) {
    for name in names {
        // Create function signature in LLVM
        let fn_type = // Placeholder type
        self.module.add_function(name, fn_type, None);
        self.forward_declared_functions.insert(name.clone());
    }
}

fn compile_function_call(&mut self, name: &str) {
    // Check if function exists OR is forward-declared
    if self.module.get_function(name).is_some()
       || self.forward_declared_functions.contains(name) {
        // Allow the call
    } else {
        return Err("Undefined function");
    }
}
```

### Test Cases

**Test 1: Simple mutual recursion**
```clojure
(declare is-odd?)

(defn is-even? [n]
  (if (= n 0)
    true
    (is-odd? (- n 1))))

(defn is-odd? [n]
  (if (= n 0)
    false
    (is-even? (- n 1))))

(println (is-even? 4))  ; Should print true
(println (is-odd? 4))   ; Should print false
```

**Test 2: Component tree walking**
```clojure
(declare render-component)

(defn render-children [window children]
  (doseq [child children]
    (render-component window child)))

(defn render-component [window component]
  (let [tag (first component)
        props (second component)
        children (rest (rest component))]
    (cond
      (= tag :div)
      (do
        (draw-box window)
        (render-children window children))

      (= tag :text)
      (draw-text window (get props :label "")))))

(render-component window [:div {}
                            [:text {:label "Hello"}]
                            [:div {}
                              [:text {:label "Nested"}]]])
```

**Test 3: Multiple forward declarations**
```clojure
(declare foo bar baz)

(defn foo [] (bar))
(defn bar [] (baz))
(defn baz [] (println "All work!"))

(foo)  ; Should print "All work!"
```

---

## Issue #4: Docstrings in `defn` Cause Parse Errors 🟡 HIGH
**Status:** 🟡 OPEN - WORKAROUND: Remove docstrings
**Severity:** 🟡 HIGH
**Impact:** Cannot document functions inline

### Problem
Any docstring in a function definition causes parse error.

### Example
```clojure
(defn my-function [x]
  "This is a docstring"
  (+ x 1))
; Parse error: Expected RParen, found Nil
```

### Workaround
Use comments instead:
```clojure
(defn my-function [x]
  ; This function adds 1 to x
  (+ x 1))
```

### Files Affected
- All CORAL code had docstrings removed
- See commit history for examples

---

## Issue #5: Missing Type Predicates 🟢 LOW ✅ FIXED
**Status:** ✅ FIXED
**Severity:** 🟢 LOW
**Impact:** Had to use workarounds for type checking

### Problem
Missing standard predicates:
- ❌ `vector?` - Check if value is vector
- ❌ `map?` - Check if value is map
- ❌ `nil?` - Check if value is nil
- ❌ `keyword?` - Check if value is keyword
- ❌ `list?` - Check if value is list

### Solution
Added `vector?` and `map?` in commit a9ec790.

### Still Missing
- `nil?`
- `keyword?`
- `list?`
- `string?` (exists but should be documented)
- `number?`
- `fn?`

---

## Issue #6: Missing Core Functions 🟢 LOW
**Status:** 🟢 OPEN - WORKAROUND: Use alternatives
**Severity:** 🟢 LOW
**Impact:** Minor inconvenience, workarounds exist

### Missing Functions

**Sequence Operations:**
- `second` - Use `(first (rest coll))` instead
- `nth` with default - Currently `nth` doesn't support default value
- `take` - Take first n elements
- `drop` - Drop first n elements
- `filter` - Filter collection by predicate
- `map` - Map function over collection
- `reduce` - Already exists but might need testing

**Numeric:**
- `inc` - Use `(+ x 1)` instead
- `dec` - Use `(- x 1)` instead
- `max` - Maximum of numbers
- `min` - Minimum of numbers
- `abs` - Absolute value

**Boolean:**
- `not` - Use `(if x false true)` instead
- `boolean` - Convert to boolean

**Map Operations:**
- `assoc-in` - Deep associative update
- `update-in` - Deep update with function
- `get-in` - Deep get with path

### Priority
Low - workarounds are simple and don't block development.

---

## Issue #7: Keyword Calls in Shorthand Functions ✅ FIXED
**Status:** ✅ FIXED in commit 3307b52
**Severity:** 🟡 HIGH (was blocking Reagent patterns)

### Problem
Pattern `#((:key %))` failed to parse.

### Solution
Fixed parser to handle keyword calls inside shorthand functions by creating `Expr::Call` nodes and transforming to `get` calls in codegen.

---

## Issue #8: Compiler Hangs on Parse Errors 🔴 CRITICAL
**Status:** 🔴 OPEN - NO WORKAROUND
**Severity:** 🔴 CRITICAL
**Impact:** Wastes developer time, requires force-killing processes

### Problem
When there's a paren mismatch or other parse error, the compiler hangs indefinitely instead of reporting the error and exiting. The process must be killed with SIGKILL (signal 137).

### Expected Behavior
```bash
$ clorus build
Error: Parse error in file.clrs:42: Unexpected closing delimiter: RParen
$ echo $?
1
```
Fast failure with clear error message.

### Actual Behavior
```bash
$ clorus build
Processing 1 Rust dependencies...
...
Compiling coral v0.1.0
[HANGS FOREVER - must kill with Ctrl+C or kill -9]
```

### Examples Encountered

**Case 1: Paren mismatch**
- File had one extra `)` closing paren
- Build hung during compilation phase
- Had to kill process
- After fixing, got "Unexpected closing delimiter" error immediately

**Case 2: Complex nesting**
- Editing large file with nested cond/let/when
- Accidentally added extra `)  `
- Build hung for several minutes before being killed
- Error only appeared after manually fixing paren count

### Impact
- Slows development significantly
- Forces context switching (start build, wait, realize it hung, kill, fix, retry)
- Makes trial-and-error development painful
- Especially bad for beginners who make more syntax errors

### What Should Happen
1. Parser should detect mismatched delimiters immediately
2. Report clear error with line number
3. Exit with non-zero status code
4. **Never** hang or enter infinite loop

### Files Affected
Likely in parser or compiler driver:
- `/Users/prabhugopal/Learning/git/clorus/crates/clorus-syntax/src/parser.rs`
- `/Users/prabhugopal/Learning/git/clorus/crates/clorus-cli/src/main.rs`

### Possible Root Cause
- Parser may loop infinitely trying to recover from error
- Missing timeout or error propagation
- Infinite recursion when parsing fails

---

## Summary

### Critical Issues Blocking CORAL
1. ❌ **Mutual recursion with `declare`** - No workaround, blocks tree structures
2. ❌ **Compiler hangs on parse errors** - Should report error and exit, not hang
3. ✅ **Loop/recur** - FIXED
4. ✅ **String UAF** - FIXED

### High Priority
1. 🟡 **Docstrings** - Workaround: use comments
2. ✅ **Type predicates** - FIXED (vector?, map?)
3. ✅ **Keyword shorthand** - FIXED

### Low Priority
1. 🟢 **Missing core functions** - Workarounds exist
2. 🟢 **More type predicates** - Can add as needed

### Next Steps
1. **Fix `declare`** - This is the biggest blocker for advanced CORAL features
2. Add remaining type predicates (`nil?`, `keyword?`, etc.)
3. Fix docstring parsing
4. Add missing core functions as needed
