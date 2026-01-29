# Clorus Development Roadmap

## ✅ Milestone 1: Working Compiler (COMPLETED!)

You now have a **fully functional** compiler that can:

1. **Parse** Clojure-like syntax into AST
2. **Compile** expressions to LLVM IR
3. **Execute** code with JIT compilation

### What Works Right Now

```clojure
(+ 1 2)                      ; => 3
(- 10 3)                     ; => 7
(* 6 7)                      ; => 42
(/ 20 4)                     ; => 5
(+ (* 2 3) 4)                ; => 10 (nested expressions)
(- (* 10 5) (+ 3 7))         ; => 40 (complex nesting)
```

**Test it:**
```bash
cargo run   # See all examples execute
cargo test  # 14 tests passing
```

---

## 🎯 Milestone 2: Variables & Bindings (Next Step)

Add the ability to define and use variables.

### Target Syntax
```clojure
(let [x 10
      y 20]
  (+ x y))  ; => 30

(def pi 3.14159)
(* pi 2)  ; => 6.28318
```

### Implementation Tasks
- [ ] Add `Let` and `Def` variants to AST
- [ ] Create symbol table for variable lookups
- [ ] Add alloca/store/load in codegen for mutable variables
- [ ] Test with multiple variables and shadowing

---

## 🎯 Milestone 3: Functions

Define and call functions.

### Target Syntax
```clojure
(defn add [x y]
  (+ x y))

(add 10 20)  ; => 30

; Anonymous functions
((fn [x] (* x x)) 5)  ; => 25
```

### Implementation Tasks
- [ ] Add `Defn` and `Fn` to AST
- [ ] Function definition codegen (parameters, body)
- [ ] Function call codegen (argument passing)
- [ ] Support closures (capture environment)

---

## 🎯 Milestone 4: Control Flow

Add if/when/cond for branching.

### Target Syntax
```clojure
(if (< x 10)
  "small"
  "large")

(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))
```

### Implementation Tasks
- [ ] Add `If` to AST
- [ ] Implement phi nodes and basic blocks
- [ ] Support recursion
- [ ] Add boolean operators (and, or, not)

---

## 🎯 Milestone 5: Rust FFI

Call Rust functions from Clorus.

### Target Syntax
```clojure
; Declare external Rust function
(extern "Rust" [calculate_pi [] f64])

; Use it
(def pi (calculate_pi))
(println "Pi is" pi)
```

### Implementation Tasks
- [ ] Add `Extern` declaration syntax
- [ ] Link external symbols at compile time
- [ ] Create example Rust library to call
- [ ] Document FFI conventions

---

## 🎯 Future Milestones

### Type System
- Static typing with inference
- Type annotations: `(defn add [x:i64 y:i64] :i64 (+ x y))`
- Polymorphism/generics

### Standard Library
- Math: sin, cos, sqrt, pow
- IO: println, read-file, write-file
- Collections: map, filter, reduce
- Strings: concat, split, format

### Tooling
- REPL (Read-Eval-Print Loop)
- Compiler to native binary (not just JIT)
- Error messages with line numbers
- Debug info for debuggers

---

## Current Architecture

```
Source Code → Lexer → Parser → AST → CodeGen → LLVM IR → JIT → Execute
```

**Modular Design:**
- Each phase is independent
- Can test each component separately
- Easy to extend with new features

---

## Try It Now!

```bash
# See working examples
cargo run

# Run specific expression (when ready)
./test_expr.sh '(* (+ 1 2) (- 10 5))'

# Run tests
cargo test
```

Want to tackle **Milestone 2** (variables) next? It's the natural progression!
