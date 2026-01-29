# Clorus Examples

## Basic Arithmetic

```clojure
; Addition
(+ 1 2)              ; => 3
(+ 1 2 3 4)          ; => 10

; Subtraction
(- 10 3)             ; => 7
(- 5)                ; => -5 (negation)

; Multiplication
(* 6 7)              ; => 42
(* 2 3 4)            ; => 24

; Division
(/ 20 4)             ; => 5
(/ 100 2 5)          ; => 10
```

## Nested Expressions

```clojure
; Nested arithmetic
(+ (* 2 3) 4)                ; => 10
(- (* 10 5) (+ 3 7))         ; => 40

; Complex expressions
(/ (+ 10 20) (- 10 5))       ; => 6
(* (+ 1 2) (- 10 5))         ; => 15
```

## What Works Now

- ✅ Numbers (f64)
- ✅ Arithmetic operators: +, -, *, /
- ✅ Nested expressions
- ✅ LLVM optimization (constant folding)
- ✅ JIT execution

## Coming Next

- [ ] Variables: `(def x 10)`
- [ ] Functions: `(defn add [x y] (+ x y))`
- [ ] Control flow: `(if (< x 10) x (* x 2))`
- [ ] Booleans and comparisons
- [ ] Strings and printing
- [ ] Rust FFI calls

## Try It Yourself

```bash
# Run the main demo
cargo run

# Run a specific expression (coming soon)
./test_expr.sh '(+ (* 2 3) 4)'

# Run tests
cargo test
```
