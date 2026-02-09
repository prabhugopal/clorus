# Clorus Known Issues - Status Review

Last Updated: 2026-02-09
After: Repository reorganization and multi-source directory implementation

---

## ✅ WORKS (Confirmed):

1. **count** - Function to get collection length
2. **Keyword access** - `(:key map)` works fine
3. **Maps/vectors** - Data structures work
4. **loop/recur** - Recursion works
5. **if, when, do, let** - Control flow works
6. **Multi-source directories** - NEW! Can require from tests/, stdlib/, src/

---

## ❌ CRITICAL MISSING FEATURES:

### 1. nth with loop iteration
**Status:** OPEN - Suspected bug
**Issue:** The combination of using `nth` to access vector elements inside a loop appears to fail.
**Workaround:** Unknown
**Priority:** High

### 2. Integer conversion (int, floor, ceil, round)
**Status:** OPEN - Missing feature
**Issue:** No way to convert float division results to integers for use as indices.
**Workaround:** Use float values directly
**Priority:** High

### 3. Docstrings
**Status:** OPEN - Parser limitation
**Issue:** Cannot put documentation strings in function/namespace definitions after parameter list.
**Example:**
```clojure
(defn create-window [title width height]
  "Create a new window..."  ; ❌ Parser error!
  (gfx-native/create-window title width height))
```
**Workaround:** Use comments above functions
**Priority:** Medium

### 4. String escaping in vectors
**Status:** OPEN - Parser limitation
**Issue:** Cannot use escaped quotes like `\"Hello\"` inside string vectors.
**Workaround:** Remove escaped quotes
**Priority:** Low

### 5. Error messages - Very cryptic
**Status:** OPEN - Quality issue
**Issue:** Errors like "Expected RParen, found LParen" don't point to the actual issue or provide helpful context.
**Example:**
```
Error: Parse error: Expected RParen, found LParen at line 8, column 3
```
Line 8 might be fine, but there's a missing `do` block on line 3!
**Priority:** High - Makes debugging very difficult

### 6. Multiple expressions in defn without do
**Status:** DOCUMENTED - Language design
**Issue:** `defn` requires explicit `do` block for multiple expressions:
```clojure
;; ❌ FAILS
(defn -main []
  (println "Hello")
  (println "World")
  0)

;; ✅ WORKS
(defn -main []
  (do
    (println "Hello")
    (println "World")
    0))
```
**Workaround:** Always use `do` for multi-expression function bodies
**Priority:** Low - Easy workaround once known

---

## 🐛 CRITICAL BUGS:

### 7. Loop/recur parameter binding bug
**Status:** CONFIRMED
**Details:** Loop parameters may not bind correctly in certain scenarios.
**Priority:** Critical

### 8. Atom deref may not read updated values
**Status:** UNDER INVESTIGATION
**Details:** Atom values updated with `swap!` may not be visible on `@atom` dereference.
**Priority:** Critical

### 9. Missing deref function
**Status:** CONFIRMED - Use @ instead
**Workaround:** Use `@atom` instead of `(deref atom)`
**Priority:** Low - Workaround exists

---

## 🚫 MISSING FEATURES:

### 10. No Regex Literal Syntax
**Status:** OPEN
**Issue:** `#"pattern"` not supported
**Workaround:** Use `clojure.string/split` with plain strings
**Priority:** Medium

### 11. swap! Requires Strict Function Syntax
**Status:** DOCUMENTED - Language design difference
**Issue:** `(swap! atom assoc :key val)` doesn't work
**Workaround:** Must wrap: `(swap! atom (fn [s] (assoc s :key val)))`
**Priority:** Low - Clorus doesn't support Clojure's variadic `swap!`

### 12. No update Function
**Status:** OPEN - Missing stdlib function
**Issue:** `(swap! atom update :key fn)` not supported
**Workaround:** Manual assoc with get: `(swap! atom (fn [s] (assoc s :key (fn (get s :key)))))`
**Priority:** Medium

### 13. No Math functions
**Status:** OPEN - Missing stdlib
**Issue:** No `Math/floor`, `Math/ceil`, `Math/round`, `Math/abs`, etc.
**Workaround:** Display float values as-is
**Priority:** Medium

### 14. = function when comparing keywords
**Status:** NEEDS INVESTIGATION
**Issue:** Reported that `=` may not work correctly with keywords
**Priority:** High if confirmed

---

## ✅ DESIGN DECISIONS (Not Bugs):

### 15. Namespace Must Match File Path
**Status:** BY DESIGN - Following Clojure conventions
**Rule:** `examples/gallery/file.clrs` → `(ns examples.gallery.file)`
**Priority:** N/A - This is correct behavior

### 16. Rust FFI dependencies not bundled in .clip
**Status:** DOCUMENTED in IMPROVEMENTS.md P2#6
**Workaround:** Apps must declare Rust dependencies in their Clorus.toml
**Priority:** Medium - Future enhancement planned

---

## 📊 Summary:

- **Total Issues:** 16
- **Works Fine:** 6 features
- **Critical Bugs:** 3 (loop/recur, atom deref, nth)
- **Missing Features:** 9 (int conversion, docstrings, math, regex, update, etc.)
- **Design Decisions:** 2 (namespace matching, defn requires do)
- **Quality Issues:** 1 (cryptic error messages)

**Highest Priority:**
1. Fix loop/recur parameter binding bug
2. Fix atom deref not reading updated values
3. Improve error messages with better context
4. Add integer conversion functions (int, floor, ceil, round)
5. Investigate nth with loop iteration failure
6. Investigate = with keywords

**Medium Priority:**
7. Add update function to stdlib
8. Add Math functions (floor, ceil, round, abs, etc.)
9. Add regex literal syntax support
10. Support docstrings in function definitions

**Low Priority:**
11. Add deref function (@ works fine)
12. String escaping in vectors
13. Variadic swap! support

---

## Next Steps:

This document should be moved to `docs/issues/KNOWN_ISSUES.md` and kept updated as issues are resolved.

For implementation priority, recommend focusing on:
1. Critical bugs first (loop/recur, atom, nth)
2. Error message improvements (huge quality of life)
3. Missing stdlib functions (int, Math.*, update)
