# Control Flow Macros Implementation - Complete

## Summary

Successfully implemented all essential Clojure-style control flow macros for Clorus. These macros provide expressive, functional-style control flow that expands to efficient if/let expressions at compile time.

## Implementation Date

January 27, 2026

## What Was Implemented

### 1. Conditional Macros

#### `when` - Execute body when condition is true
**Syntax:** `(when test expr1 expr2 ...)`
**Expansion:** `(if test (do expr1 expr2 ...) nil)`

```clojure
(when (> x 0)
  (println "positive")
  (inc x))
```

**Location:** macros.rs:86-89, 842-866

#### `when-not` - Execute body when condition is false
**Syntax:** `(when-not test expr1 expr2 ...)`
**Expansion:** `(if test nil (do expr1 expr2 ...))`

```clojure
(when-not (empty? list)
  (process list))
```

**Location:** macros.rs:91-94, 868-892

#### `if-not` - Inverted if
**Syntax:** `(if-not test then else)`
**Expansion:** `(if test else then)`

```clojure
(if-not (valid? x)
  (throw "Invalid!")
  (process x))
```

**Location:** macros.rs:106-109, 984-999

### 2. Let-Based Conditional Macros

#### `if-let` - Bind value and test in one form
**Syntax:** `(if-let [binding value] then else)`
**Expansion:** `(let [binding value] (if binding then else))`

```clojure
(if-let [result (find-user id)]
  (greet result)
  (println "User not found"))
```

**Location:** macros.rs:96-99, 894-938

#### `when-let` - Bind value and execute when truthy
**Syntax:** `(when-let [binding value] body ...)`
**Expansion:** `(let [binding value] (when binding body ...))`

```clojure
(when-let [config (load-config)]
  (initialize config)
  (start-server))
```

**Location:** macros.rs:101-104, 940-982

### 3. Logical Macros

#### `and` - Short-circuit logical AND
**Syntax:** `(and expr1 expr2 ...)`
**Expansion:** Nested if expressions that stop at first falsy value
**Special:** `(and)` with no args returns `true`

```clojure
(and (valid? x) (< x 100) (even? x))
```

**Location:** macros.rs:111-114, 1001-1032

#### `or` - Short-circuit logical OR
**Syntax:** `(or expr1 expr2 ...)`
**Expansion:** Nested let+if expressions that stop at first truthy value
**Special:** `(or)` with no args returns `false`

```clojure
(or (find-in-cache key)
    (find-in-db key)
    (default-value))
```

**Location:** macros.rs:116-119, 1034-1076

**Note:** Uses `gensym` to avoid evaluating expressions multiple times while preserving truthy values.

### 4. Threading Macros with Nil Safety

#### `some->` - Thread-first with nil checks
**Syntax:** `(some-> expr (f1 args) (f2 args) ...)`
**Behavior:** Like `->` but stops threading on first nil result

```clojure
(some-> user
  (get :address)
  (get :city)
  (uppercase))  ; Returns nil if any step returns nil
```

**Location:** macros.rs:121-124, 1078-1107

#### `some->>` - Thread-last with nil checks
**Syntax:** `(some->> expr (f1 args) (f2 args) ...)`
**Behavior:** Like `->>` but stops threading on first nil result

```clojure
(some->> numbers
  (filter positive?)
  (map square)
  (reduce +))  ; Returns nil if any step returns nil
```

**Location:** macros.rs:126-129, 1109-1138

### 5. Mutation Chaining

#### `doto` - Thread object through side-effect operations
**Syntax:** `(doto object (method1 args) (method2 args) ...)`
**Expansion:** `(let [obj object] (method1 obj args) (method2 obj args) ... obj)`
**Returns:** The original object (for chaining mutable operations)

```clojure
(doto (new-builder)
  (set-name "Alice")
  (set-age 30)
  (set-active true))  ; Returns the builder with all mutations applied
```

**Location:** macros.rs:131-134, 1140-1182

## Previously Implemented

These macros were already implemented before this session:

### `->` (thread-first)
Thread value as first argument through forms
**Location:** macros.rs:66-69, 592-612

### `->>` (thread-last)
Thread value as last argument through forms
**Location:** macros.rs:71-74, 614-634

### `cond`
Multi-way conditional with test-expr pairs
**Location:** macros.rs:76-79, 636-686

### `case`
Value-based dispatch
**Location:** macros.rs:81-84, 688-753

### `gensym`
Generate unique symbols for macro hygiene
**Location:** macros.rs:138-152, 46-56

## Testing

### Test Coverage

All 11 new control flow macros have comprehensive unit tests:

1. `test_when_macro` - Tests when expansion to if+do
2. `test_when_not_macro` - Tests when-not expansion
3. `test_if_let_macro` - Tests if-let binding and conditional
4. `test_when_let_macro` - Tests when-let binding
5. `test_if_not_macro` - Tests branch inversion
6. `test_and_macro` - Tests and short-circuit behavior
7. `test_and_macro_empty` - Tests (and) => true
8. `test_or_macro` - Tests or short-circuit with let bindings
9. `test_or_macro_empty` - Tests (or) => false
10. `test_some_thread_first` - Tests nil-safe threading first
11. `test_some_thread_last` - Tests nil-safe threading last
12. `test_doto_macro` - Tests mutation chaining

**Location:** macros.rs:1594-1783

### Test Results

```
running 73 tests
...
test macros::tests::test_and_macro ... ok
test macros::tests::test_and_macro_empty ... ok
test macros::tests::test_doto_macro ... ok
test macros::tests::test_if_let_macro ... ok
test macros::tests::test_if_not_macro ... ok
test macros::tests::test_or_macro ... ok
test macros::tests::test_or_macro_empty ... ok
test macros::tests::test_some_thread_first ... ok
test macros::tests::test_some_thread_last ... ok
test macros::tests::test_when_let_macro ... ok
test macros::tests::test_when_macro ... ok
test macros::tests::test_when_not_macro ... ok

test result: ok. 73 passed; 0 failed; 0 ignored
```

## Implementation Details

### Macro Expansion Strategy

All macros follow the same expansion pattern:

1. **Parse Arguments:** Extract test conditions, bindings, and body expressions
2. **Expand Recursively:** Call `expand_macros_with_registry` on sub-expressions
3. **Build AST:** Construct target AST nodes (If, Let, Do, etc.)
4. **Return:** Return expanded expression for further compilation

### Hygiene

- `or` macro uses `gensym` to generate unique symbols, preventing variable capture
- `some->` and `some->>` use `gensym` for temporary bindings
- `doto` uses `gensym` for object binding

### Short-Circuit Behavior

Both `and` and `or` properly short-circuit:

- `and` stops at first falsy value and returns it
- `or` stops at first truthy value and returns it (not just true)
- Both evaluate expressions left-to-right exactly once

Example:
```clojure
(or false nil 42 (throw "never evaluated"))  ; Returns 42
(and true 1 2 false (throw "never evaluated"))  ; Returns false
```

### Nil Safety

`some->` and `some->>` generate code like:

```clojure
;; (some-> x (f a) (g b))
;; Expands to:
(let [tmp__1 x]
  (if tmp__1
    (let [tmp__2 (f tmp__1 a)]
      (if tmp__2
        (g tmp__2 b)
        nil))
    nil))
```

This ensures safe navigation through potentially nil values.

## Examples

### Complex Control Flow

```clojure
;; Find user, validate, and process
(when-let [user (find-user id)]
  (if-not (expired? user)
    (some-> user
      (get :profile)
      (get :settings)
      (update-settings new-values))
    (renew-user user)))
```

### Error Handling

```clojure
(defn safe-divide [x y]
  (when-not (= y 0)
    (/ x y)))

(or (safe-divide 10 2)   ; Try first operation
    (safe-divide 20 4)   ; Try fallback
    (default-value))     ; Final fallback
```

### Builder Pattern

```clojure
(doto (create-request)
  (set-url "https://api.example.com")
  (set-method "POST")
  (add-header "Content-Type" "application/json")
  (set-body payload))
```

## Language Parity Status

With all control flow macros complete, Clorus now has:

### Control Flow - 100% ✅

- ✅ if, if-not
- ✅ when, when-not
- ✅ if-let, when-let
- ✅ cond, case
- ✅ and, or
- ✅ ->, ->>, some->, some->>
- ✅ doto

### Overall Language Parity: ~55%

**Next Priority:** Collections API (as requested by user)

## Related Files

- **Implementation:** `/Users/prabhugopal/Learning/git/clorus/crates/clorus-syntax/src/macros.rs`
- **AST Definitions:** `/Users/prabhugopal/Learning/git/clorus/crates/clorus-syntax/src/ast.rs`
- **Parser:** `/Users/prabhugopal/Learning/git/clorus/crates/clorus-syntax/src/parser.rs`

## Technical Notes

### Why Macros Instead of Built-in Forms?

These are implemented as macros (compile-time transformations) rather than runtime features because:

1. **Zero Runtime Cost:** Expansion happens at compile time
2. **Composition:** Macros can use other macros (e.g., when-let uses when)
3. **Simplicity:** Codegen only needs to handle primitive forms (if, let, do)
4. **Extensibility:** Users can define similar macros with defmacro

### AST Nodes Used

The macros expand to these core AST nodes:
- `Expr::If` - Conditional branching
- `Expr::Let` - Local bindings
- `Expr::Do` - Sequential expressions
- `Expr::Call` - Function calls
- `Expr::Symbol` - Variable references
- `Expr::Bool` - Boolean literals (true/false)
- `Expr::Nil` - Nil value

### Macro Registry

The `MacroRegistry` maintains:
- User-defined macros from `defmacro`
- Gensym counter for unique symbol generation
- Macro lookup during expansion

Built-in macros (like `when`, `and`, etc.) are hardcoded in `expand_macros_with_registry` for efficiency.

## Future Enhancements

While all essential control flow macros are complete, potential additions:

1. **case with guards:** `(case x (guard pred?) val)`
2. **doseq:** Iteration with side effects
3. **for:** List comprehensions
4. **while:** Imperative loops (if needed)

These are lower priority since they can be expressed with existing constructs.

---

**Status:** ✅ Complete - All essential control flow macros implemented and tested

**Next Steps:** Implement Collections API (as per user request)
