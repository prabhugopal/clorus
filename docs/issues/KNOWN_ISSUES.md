# ✅ FIXED: Segfault With Let Expressions and Variable Shadowing

## Symptom (RESOLVED)
Program crashed with segfault (exit code 139) when let expressions used variable names that matched existing global definitions.

## Root Cause (IDENTIFIED)
The `Expr::Symbol` variable lookup in `codegen.rs` (line 1759) checked **globals FIRST**, then locals. This prevented local bindings from properly shadowing globals with the same name. When a `let` binding used the same variable name as an existing `def`, the code would load the stale global pointer instead of the fresh local value, causing memory corruption and segfault.

## Example That Was Crashing
```clojure
(defn test1 []
  (def v1 [1 2 3]))  ; Creates global v1

(defn test-with-let []
  (let [v1 [4 5 6]]  ; Should shadow global v1, but loaded global instead
    (first v1)))      ; CRASH - loaded corrupted global pointer
```

## The Fix ✅
**File:** `crates/clorus-codegen/src/codegen.rs` lines 1759-1806

**Changed variable lookup order** to check **locals FIRST**, then globals:

```rust
// Before (BUGGY - checked globals first):
if let Some(global) = self.globals.get(&resolved_name) {
    // Load from global
} else if let Some(ptr) = self.variables.get(name) {
    // Load from local
}

// After (FIXED - checks locals first):
if let Some(ptr) = self.variables.get(name) {
    // Load from local - allows shadowing
} else if let Some(global) = self.globals.get(&resolved_name) {
    // Load from global
}
```

This allows proper **variable shadowing** - local bindings now correctly override globals with the same name.

## Verification
All tests now pass without segfault:
- ✓ Multiple def statements followed by let expressions
- ✓ Variable name conflicts between globals and locals
- ✓ test-code-samples.clr runs successfully
- ✓ test-vector-debug.clr runs successfully

## Status: FIXED (2026-02-08)
