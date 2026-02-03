# Clorus REPL - Features & Usage

## REPL Persistence ✅

**Status:** Fully supported!

Variables and functions now persist across REPL lines. The REPL maintains history and recompiles definitions as needed.

### Global Variables (def)
```clojure
λ> (def x 10)
=> 10
λ> (def y 20)
=> 20
λ> (+ x y)
=> 30
```

### Functions (defn)
```clojure
λ> (defn square [x] (* x x))
=> 0
λ> (square 5)
=> 25
λ> (square 10)
=> 100
```

### Mixed Usage
```clojure
λ> (def radius 10)
=> 10
λ> (defn circle-area [r] (* 3.14 (* r r)))
=> 0
λ> (circle-area radius)
=> 314
```

## What Works Well

✅ **Arithmetic expressions**
```clojure
λ> (+ 1 2 3)
=> 6
λ> (* 2 (+ 3 4))
=> 14
```

✅ **Global variables with def**
```clojure
λ> (def pi 3.14159)
=> 3.14159
λ> (* pi 2)
=> 6.28318
```

✅ **Functions with defn**
```clojure
λ> (defn add [a b] (+ a b))
=> 0
λ> (add 10 20)
=> 30
```

✅ **Let bindings (local variables)**
```clojure
λ> (let [x 5] (* x x))
=> 25
λ> (let [a 2 b 3] (+ a b))
=> 5
```

✅ **Control flow (if expressions)**
```clojure
λ> (if (< 5 10) 100 200)
=> 100
λ> (def x 7)
=> 7
λ> (if (< x 10) (* x 2) (* x 3))
=> 14
```

✅ **Nested expressions**
```clojure
λ> (let [r 10] (* 3.14 (* r r)))
=> 314
```

✅ **Comparisons**
```clojure
λ> (< 5 10)
=> 1.0
λ> (> 5 10)
=> 0.0
λ> (= 42 42)
=> 1.0
```

## Implementation Details

The REPL maintains a history of all expressions entered. For each new line:
1. All previous expressions are recompiled to build up the symbol table
2. Historical `def` and `defn` statements are executed to initialize globals
3. Only the latest expression is executed for output

This approach ensures persistence while avoiding unnecessary re-execution of side-effect-free expressions.

## Commands

- `:quit` or `:q` - Exit REPL
- `:help` or `:h` - Show help message
- `:examples` or `:e` - Show example expressions

## CLI Tool for Scripts

For multi-line programs or scripts, use the CLI tool:
```bash
cargo run --bin clorus < script.clorus
```

Example script:
```clojure
(def radius 10)
(defn circle-area [r] (* 3.14 (* r r)))
(circle-area radius)
```
