# Long/Double Number System - Compatibility Status

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/reference/LANGUAGE_SPEC.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


## ✅ Already Compatible (No Changes Needed)

### Standard Library Functions
All stdlib functions work correctly with Long/Double types:

**Numeric Predicates** (stdlib/core.clr):
- `zero?` - Works with both Long and Double
- `pos?` - Works with both types via comparison
- `neg?` - Works with both types via comparison
- `even?` - Works with both (mod operator handles type promotion)
- `odd?` - Works with both (delegates to even?)
- `number?` - Returns true for both Long and Double

**Collection Functions**:
- `map`, `filter`, `reduce` - All work unchanged
- `count`, `nth`, `first`, `rest` - No numeric type dependencies
- All collection operations continue working

**Arithmetic Functions**:
- `inc`, `dec` in stdlib - Work with type-aware `+` and `-`
- All math operations now use type-aware runtime functions

### Test Files
All existing test files should work without modification:
- Type predicates test already uses `number?` which handles both types
- Arithmetic tests will benefit from proper Long/Double distinction
- Collection tests have no numeric type dependencies

### Documentation
- FFI guides don't reference old `Number` type
- No breaking changes in documented APIs

## 🆕 New Features Available

### Type-Specific Predicates
Users can now distinguish between integer and floating-point:

**Runtime Functions** (already available via FFI):
- `clorus_is_long` - Check if value is Long (integer)
- `clorus_is_double` - Check if value is Double (float)
- `clorus_is_number` - Check if value is any numeric type

**Recommended Stdlib Additions** (see below):
- `long?` - Clorus wrapper for is_long
- `double?` - Clorus wrapper for is_double
- `integer?` - Alias for long?
- `float?` - Alias for double?

### Type Promotion Rules
Users now get Clojure-compatible numeric behavior:

```clojure
(+ 42 10)      ; => 52 (Long)
(+ 42 3.14)    ; => 45.14 (Double - auto-promoted)
(/ 84 2)       ; => 42.0 (Double - division always returns Double)
(mod 10 3)     ; => 1 (Long)
(mod 10.5 3.0) ; => 1.5 (Double)
```

### Bitwise Operations
Now available (Long only):
- `bit-and`, `bit-or`, `bit-xor`
- `bit-not`
- `bit-shift-left`, `bit-shift-right`

## 📝 Recommended Stdlib Additions

Add type predicates to stdlib/core.clr for user convenience:

```clojure
;; Type predicates for Long/Double distinction
(defn long? [x]
  "Check if value is a Long (integer) type"
  (clorus_is_long x))

(defn double? [x]
  "Check if value is a Double (floating-point) type"
  (clorus_is_double x))

;; Aliases for clarity
(defn integer? [x]
  "Alias for long? - check if value is an integer"
  (long? x))

(defn float? [x]
  "Alias for double? - check if value is floating-point"
  (double? x))
```

## 🔧 No Breaking Changes

**Existing code continues to work** because:

1. **Backward Compatible Literals**:
   - `42` → Long (was f64, now i64)
   - `3.14` → Double (was f64, still f64)
   - Behavior unchanged from user perspective

2. **Auto-Promotion**:
   - Mixed operations promote to Double automatically
   - No manual casting required

3. **Type Predicates**:
   - `number?` returns true for both types
   - Existing code using `number?` works unchanged

4. **Comparison Operators**:
   - Work across types with auto-promotion
   - `(< 5 10.5)` works correctly

## 📊 Summary

| Component | Status | Action Required |
|-----------|--------|-----------------|
| stdlib/core.clr | ✅ Compatible | Optional: Add type predicates |
| stdlib/lazy.clr | ✅ Compatible | None |
| stdlib/transducers.clr | ✅ Compatible | None |
| Test files | ✅ Compatible | None |
| FFI docs | ✅ Up to date | None |
| Examples | ✅ Compatible | None |

**Bottom line**: Everything works out of the box. The type predicates are just nice-to-have additions for users who want fine-grained type control.
