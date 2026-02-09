# Clorus Issues - Fix Plan (No Workarounds)

**Principle:** Fix issues properly, don't document workarounds. Maintain backward compatibility.

---

## 🔥 Priority 1: Critical Parser/Compiler Fixes

### 1. defn Should Allow Multiple Expressions Without do
**Current:** Must wrap multiple expressions in `do`
```clojure
;; Currently FAILS
(defn -main []
  (println "Hello")
  (println "World"))

;; Must use do
(defn -main []
  (do
    (println "Hello")
    (println "World")))
```

**Fix:** Update parser to treat defn body as implicit do
- Location: `crates/clorus-syntax/src/parser.rs` - `parse_defn()`
- Wrap body expressions in implicit do if more than one expression
- This matches Clojure behavior

### 2. Improve Error Messages with Context
**Current:** "Expected RParen, found LParen at line 8, column 3" (unhelpful)

**Fix:**
- Add context to parse errors (show surrounding code)
- Better error messages for common mistakes
- Location: `crates/clorus-syntax/src/parser.rs` error handling

### 3. Support Docstrings in defn
**Current:** Parse error if docstring after params
```clojure
(defn create-window [title width height]
  "Creates a new window"  ; ❌ Parse error!
  (gfx-native/create-window title width height))
```

**Fix:** Update parser to accept optional docstring between params and body
- Location: `crates/clorus-syntax/src/parser.rs` - `parse_defn()`
- Store docstring in AST
- This matches Clojure syntax

---

## 🐛 Priority 2: Critical Runtime Bugs

### 4. Loop/Recur Parameter Binding Bug
**Status:** CONFIRMED - parameters don't bind correctly
**Fix:** Debug and fix parameter binding in loop/recur
- Location: `crates/clorus-codegen/src/codegen.rs` - loop/recur codegen
- Verify parameter order and binding
- Add tests

### 5. Atom deref Not Reading Updated Values
**Status:** UNDER INVESTIGATION
**Fix:** Debug swap!/deref interaction
- Location: `crates/clorus-runtime/src/atom.rs`
- Verify thread safety and memory ordering
- Add tests

### 6. nth with Loop Iteration Fails
**Status:** SUSPECTED
**Fix:** Test and debug nth inside loop
- May be related to issue #4 (loop/recur bug)
- Add comprehensive tests
- Fix if confirmed

---

## 📚 Priority 3: Missing Stdlib Functions

### 7. Integer Conversion Functions
**Fix:** Add to stdlib/core.clr
```clojure
(defn int [x]
  (clorus-runtime-floor x))  ; Need runtime support

(defn floor [x] ...)
(defn ceil [x] ...)
(defn round [x] ...)
```
**Implementation:**
- Add runtime functions in `crates/clorus-runtime/src/math.rs` (NEW FILE)
- Expose via FFI
- Add to stdlib/core.clr

### 8. Math Functions
**Fix:** Add to stdlib/core.clr
```clojure
(defn abs [x] ...)
(defn sqrt [x] ...)
(defn pow [x y] ...)
(defn min [x y] ...)
(defn max [x y] ...)
```
**Implementation:**
- Use existing Rust std::f64 functions
- Add runtime wrappers
- Add to stdlib

### 9. update Function
**Fix:** Add to stdlib/core.clr
```clojure
(defn update [m k f & args]
  (assoc m k (apply f (get m k) args)))
```
**Implementation:**
- Pure Clorus implementation
- No runtime changes needed

### 10. deref Function
**Fix:** Add to stdlib/core.clr as alias to @
```clojure
;; Runtime already supports @atom
;; Just add function form
(defn deref [atom-val]
  @atom-val)
```

---

## 🔧 Priority 4: Language Features

### 11. Regex Literal Syntax #"pattern"
**Fix:** Add to lexer and parser
- Location: `crates/clorus-syntax/src/lexer.rs` - recognize `#"`
- Location: `crates/clorus-syntax/src/parser.rs` - parse regex literals
- Add to AST: `Expr::Regex { pattern: String }`
- Codegen: Compile to regex runtime type

### 12. Variadic swap! Support
**Current:** `(swap! atom assoc :key val)` doesn't work

**Fix:** Make swap! accept additional args to pass to function
```clojure
;; Should work like Clojure
(swap! atom assoc :key val)
;; Equivalent to:
(swap! atom (fn [state] (assoc state :key val)))
```
**Implementation:**
- Update swap! in runtime to accept variadic args
- Pass extra args to update function
- Location: `crates/clorus-runtime/src/atom.rs`

### 13. String Escaping in Vectors
**Fix:** Improve string parsing to handle escapes correctly
- Location: `crates/clorus-syntax/src/lexer.rs` - string parsing
- Handle `\"`, `\\`, `\n`, `\t` etc.

---

## ✅ Not Issues (By Design)

### 14. Namespace Must Match File Path
**Status:** CORRECT - Matches Clojure conventions
**No action needed.**

### 15. Rust FFI Dependencies in .clip
**Status:** DOCUMENTED - Future enhancement in IMPROVEMENTS.md
**No action now - keep on roadmap.**

---

## 🎯 Implementation Order

**Week 1: Critical Fixes**
1. defn implicit do (High impact, low risk)
2. Better error messages (Huge QoL improvement)
3. Docstring support (matches Clojure)
4. Loop/recur bug (Critical)

**Week 2: Runtime Bugs**
5. Atom deref investigation and fix
6. nth with loop testing and fix

**Week 3: Stdlib Functions**
7. Integer conversion (int, floor, ceil, round)
8. Math functions (abs, sqrt, pow, min, max)
9. update function
10. deref function

**Week 4: Advanced Features**
11. Regex literals
12. Variadic swap!
13. String escaping improvements

---

## Testing Strategy

For each fix:
1. Write failing test first
2. Implement fix
3. Verify test passes
4. Check backward compatibility
5. Update documentation

No workarounds, only proper fixes.
