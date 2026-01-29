# 🎉 Clorus Milestone: Variables!

## What You Can Do Now

### 1. Use Variables with `let`

```clojure
; Simple binding
(let [x 10] x)
=> 10

; Multiple variables
(let [x 5 y 10] (+ x y))
=> 15

; Nested calculations
(let [r 10] (* 3.14 (* r r)))
=> 314
```

### 2. Arithmetic with Variables

```clojure
; Pythagorean theorem
(let [x 3 y 4]
  (+ (* x x) (* y y)))
=> 25

; Area calculations
(let [width 10 height 20]
  (* width height))
=> 200
```

### 3. Variable Scoping

```clojure
; Local scope
(let [x 10]
  (let [y 20]
    (+ x y)))
=> 30

; Shadowing
(let [x 10]
  (let [x 20]
    x))
=> 20
```

## Try It Now!

```bash
# Start the REPL
./repl.sh

# Or
cargo run --bin repl
```

```clojure
λ> (let [x 10 y 20] (+ x y))
=> 30

λ> (let [r 5] (* 3.14 (* r r)))
=> 78.5

λ> :examples
[see more examples]
```

## Implementation Details

### What We Built

1. **Extended AST** - Added `Let` and `Def` variants
2. **Parser Updates** - Parse `(let [x 10] body)` syntax
3. **Symbol Table** - HashMap tracking variables → memory locations
4. **LLVM Codegen**:
   - `alloca` - allocate stack space for variables
   - `store` - save values to memory
   - `load` - retrieve values from memory
5. **Scope Management** - Local scopes with proper shadowing

### Architecture

```
(let [x 10] (+ x 5))
        ↓ Parser
    Let { bindings: [(x, 10)], body: (+ x 5) }
        ↓ CodeGen
    1. alloca %x on stack
    2. store 10.0 into %x
    3. load %x for use in addition
    4. build_float_add
        ↓ LLVM
    Native machine code
```

## What's Next?

See `ROADMAP.md` for the full plan. Next milestones:

1. **Functions** - `(defn square [x] (* x x))`
2. **Control Flow** - `(if (< x 10) x (* x 2))`
3. **Rust FFI** - Call Rust functions from Clorus

## Files Added/Modified

- `src/ast.rs` - Added `Let` and `Def` variants
- `src/parser.rs` - Parse let/def syntax
- `src/codegen.rs` - Variable codegen with symbol table
- `src/bin/repl.rs` - Updated help and examples
- `VARIABLES.md` - Comprehensive variable guide

## Test It

```bash
# Run all tests (14 passing)
cargo test

# Try the REPL
cargo run --bin repl

# Run demo
cargo run
```

## Keep It Modular

Current structure remains modular:
- `ast.rs` - Clean AST definitions
- `lexer.rs` - Self-contained tokenizer
- `parser.rs` - Independent parser
- `codegen.rs` - LLVM backend

Ready to split into separate crates when needed (see `ARCHITECTURE.md`).

---

**Performance Note:** LLVM optimizes variables at compile time. Simple constant expressions are completely eliminated!

```clojure
(let [x 10 y 20] (+ x y))
```

Compiles to:
```llvm
ret double 30.0  ; LLVM folded everything!
```

Want to add functions next? That's the natural progression! 🚀
