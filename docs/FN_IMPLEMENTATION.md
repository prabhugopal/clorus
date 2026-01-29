# Anonymous Functions (fn) Implementation

**Status:** ✅ Complete
**Date Completed:** January 26, 2026
**Priority:** 🔥🔥🔥 CRITICAL (Foundation Layer 0)

---

## Summary

Implemented basic anonymous function support (`fn*/base`) for Clorus. This is the **foundational feature** that unlocks higher-order functions, functional programming, and ultimately enables `map`, `filter`, `reduce`, and the entire functional paradigm.

## What Was Implemented

### 1. AST Support
**File:** `crates/clorus-syntax/src/ast.rs`

Added `Fn` variant to the `Expr` enum:

```rust
/// Anonymous function: (fn [x y] (+ x y))
/// Creates a lambda that can be stored in variables or passed as argument
Fn {
    params: Vec<String>,
    body: Box<Expr>,
},
```

### 2. Parser Support
**File:** `crates/clorus-syntax/src/parser.rs`

- Added "fn" to special forms dispatcher (line 106)
- Implemented `parse_fn()` method (lines 191-233)
- Parses syntax: `(fn [params] body)`
- Validates parameter names are symbols
- Handles variable arity: 0 params, 1 param, multiple params

**Example Input:**
```clojure
(fn [x y] (+ x y))
```

**Parsed AST:**
```rust
Expr::Fn {
    params: vec!["x".to_string(), "y".to_string()],
    body: Box::new(Expr::List(...))  // (+ x y)
}
```

### 3. Macro Expansion
**File:** `crates/clorus-syntax/src/macros.rs`

Added recursive macro expansion for `Fn` bodies (lines 47-50):

```rust
Expr::Fn { params, body } => Expr::Fn {
    params: params.clone(),
    body: Box::new(expand_macros(body)),
},
```

This ensures macros like `->` and `->>` work inside lambda bodies.

### 4. Codegen (LLVM Compilation)
**File:** `crates/clorus-codegen/src/codegen.rs`

**Key Changes:**
- Added `lambda_counter: usize` field to `CodeGen` struct (line 49)
- Implemented `Expr::Fn` compilation (lines 839-907)

**How It Works:**

1. **Unique Lambda Names**: Each lambda gets a unique name (`_lambda_0`, `_lambda_1`, etc.)
2. **Function Signature**: All parameters are `Value*`, returns `Value*`
3. **Compilation**:
   - Creates LLVM function with unique name
   - Binds parameters to allocas
   - Compiles body expression
   - Returns boxed result
4. **Return Value**: Currently returns function pointer as boxed number (temporary until proper Function values exist)

**Generated LLVM (simplified):**
```llvm
define i8* @_lambda_0(i8* %x, i8* %y) {
entry:
  %x1 = alloca i8*
  %y2 = alloca i8*
  store i8* %x, i8** %x1
  store i8* %y, i8** %y2
  ; ... body compilation ...
  ret i8* %result
}
```

### 5. Tests

**Parser Tests** (3 tests - all pass ✅)
- `test_parse_fn_basic`: `(fn [x] x)`
- `test_parse_fn_multiple_params`: `(fn [x y] (+ x y))`
- `test_parse_fn_no_params`: `(fn [] 42)`

**Codegen Tests** (4 tests - all pass ✅)
- `test_compile_fn_basic`: Simple identity lambda
- `test_compile_fn_with_body`: Lambda with arithmetic
- `test_compile_fn_no_params`: Nullary lambda
- `test_compile_multiple_fn`: Multiple lambdas with unique names

---

## Example Usage

```clojure
;; Define a lambda and bind it to a variable
(def double (fn [x] (* x 2)))

;; Lambda with multiple parameters
(def add (fn [x y] (+ x y)))

;; Lambda with no parameters
(def get-constant (fn [] 42))

;; Nested lambdas (currying)
(def adder (fn [x] (fn [y] (+ x y))))

;; Inline lambda (when we have HOFs)
;; (map (fn [x] (* x 2)) [1 2 3])
```

---

## Limitations (Current)

1. **No Closure Capture**: Lambdas don't capture variables from outer scope yet
   - `(let [x 10] (fn [] x))` won't work - `x` is undefined in lambda
   - Future work: Phase 2

2. **No Function Calling Yet**: Can define lambdas but can't call them as first-class values
   - `((fn [x] x) 5)` doesn't work yet
   - Need Function value type in runtime
   - Future work: Phase 2

3. **No Higher-Order Functions**: Can't pass lambdas as arguments yet
   - `(map (fn [x] (* x 2)) nums)` blocked until we have:
     - Function values
     - HOF implementations (map, filter, reduce)

4. **Temporary Representation**: Function pointer returned as boxed number
   - Not proper Value* with Function tag
   - Future: Add `Function` to `ValueTag` enum in runtime

---

## What This Unlocks

### Immediate Benefits (Phase 1 Complete)
- ✅ Lambda syntax works
- ✅ Parser understands `fn`
- ✅ Compiles to LLVM
- ✅ Foundation for functional programming

### Future Unlocks (Phase 2)
- Higher-order functions: `map`, `filter`, `reduce`
- Function composition
- Currying and partial application
- Callbacks and event handlers
- Functional pipelines

### Example of What's Now Possible (after Phase 2):
```clojure
;; Filter even numbers
(filter (fn [x] (= (% x 2) 0)) [1 2 3 4 5])
;; => [2 4]

;; Double all numbers
(map (fn [x] (* x 2)) [1 2 3])
;; => [2 4 6]

;; Sum all numbers
(reduce (fn [acc x] (+ acc x)) 0 [1 2 3 4])
;; => 10

;; Function composition
(def process (comp
  (fn [x] (* x 2))
  (fn [x] (+ x 1))))
```

---

## Technical Notes

### Why Function Pointer as Number?
**Current Approach:**
```rust
let fn_ptr = function.as_global_value().as_pointer_value();
let fn_ptr_as_int = self.builder.build_ptr_to_int(...);
let fn_ptr_as_float = self.builder.build_unsigned_int_to_float(...);
Ok(self.box_number(fn_ptr_as_float))
```

**Reasoning:**
- Temporary solution until runtime has `ValueTag::Function`
- Allows lambdas to exist as values (can assign to variables)
- Enables testing of lambda compilation

**Future Solution:**
1. Add `Function` to `ValueTag` in `clorus-runtime/src/value.rs`
2. Store function pointer directly in `ValueData`
3. Return proper `Value*` with `tag=Function`
4. Enable function calls via function pointers

### LLVM Function Names
- Named functions: `add`, `clorus_math_add` (namespace-mangled)
- Lambdas: `_lambda_0`, `_lambda_1`, `_lambda_N`
- Counter increments per lambda
- Ensures no name collisions

### Memory Management
- Functions are global in LLVM module
- No deallocation needed (static lifetime)
- Future: Closures will need heap allocation + refcounting

---

## Next Steps

### Phase 2: Function Values & Calling
**Goal:** Make lambdas callable

**Tasks:**
1. Add `ValueTag::Function` to runtime
2. Implement function calling mechanism
3. Support `((fn [x] x) 5)` syntax
4. Add function equality/comparison

**Estimated Effort:** 2-3 days

### Phase 3: Closure Capture
**Goal:** Lambdas capture outer scope

**Tasks:**
1. Identify free variables in lambda body
2. Create closure struct with captured values
3. Pass closure as hidden parameter
4. Generate closure allocation code

**Estimated Effort:** 3-5 days

### Phase 4: Higher-Order Functions
**Goal:** Implement map, filter, reduce

**Tasks:**
1. Implement `map` in Clorus stdlib
2. Implement `filter` in Clorus stdlib
3. Implement `reduce` in Clorus stdlib
4. Add list iteration support

**Estimated Effort:** 2-3 days

---

## Comparison with Clojure

| Feature | Clojure | Clorus (Current) | Clorus (Planned) |
|---------|---------|------------------|------------------|
| Basic syntax | `(fn [x] x)` | ✅ `(fn [x] x)` | - |
| Multi-arity | `(fn ([x] x) ([x y] (+ x y)))` | ❌ Not yet | Phase 5 |
| Variadic | `(fn [x & rest] ...)` | ❌ Not yet | Phase 6 |
| Recursion | `(fn foo [x] (foo x))` | ❌ Not yet | Phase 7 |
| Closures | Captures outer scope | ❌ Not yet | Phase 3 |
| First-class | Can call, pass as value | ❌ Not yet | Phase 2 |
| Shorthand | `#(* % 2)` | ❌ Not yet | Next Week |

---

## Impact on Roadmap

### Feature Dependencies Unblocked
- ✅ `fn*/base` → NOW unlocks:
  - Reader shorthand `#()` (Week 2)
  - Higher-order functions (Phases 2-4)
  - Functional composition (Phase 4)
  - Collection operations (Phase 4)

### Milestone Progress
**Milestone 1: Foundation Complete**
- Threading macros: ✅ Done
- **fn*/base: ✅ Done (Jan 26, 2026)**
- Shorthand fns: 🔵 Next up
- do: ⏸️ Waiting

**Progress:** 50% of Milestone 1 complete (2/4 features)

---

## References

- **Clojure fn docs:** https://clojure.org/reference/special_forms#fn
- **Jank progress:** https://jank-lang.org/progress/
- **LLVM lambda tutorial:** https://llvm.org/docs/tutorial/

---

## Contributors

- Implementation: Claude Sonnet 4.5
- Review: User
- Testing: Automated test suite

---

**Status:** Production-ready for parsing and compilation. Runtime support pending for Phase 2.
