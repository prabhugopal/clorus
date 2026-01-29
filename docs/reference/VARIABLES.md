# Clorus Variable Examples

## Let Bindings (Local Variables)

```clojure
; Simple binding
(let [x 10]
  x)
; => 10

; Multiple bindings
(let [x 5
      y 10]
  (+ x y))
; => 15

; Use in arithmetic
(let [a 2
      b 3]
  (* a b))
; => 6

; Nested bindings
(let [x 10]
  (let [y 20]
    (+ x y)))
; => 30

; Calculate area of circle
(let [r 10
      pi 3.14159]
  (* pi (* r r)))
; => 314.159

; Compound expressions
(let [x 5
      y (+ x 3)]  ; y depends on x
  (* x y))
; => 40
```

## Variable Scoping

```clojure
; Local scope - x only exists in let body
(let [x 10]
  (+ x 5))
; => 15

; Shadowing
(let [x 10]
  (let [x 20]  ; Inner x shadows outer x
    x))
; => 20

; Multiple uses
(let [x 3]
  (+ x x x))
; => 9
```

## Practical Examples

```clojure
; Pythagorean theorem: a² + b² = c²
(let [a 3
      b 4
      a_sq (* a a)
      b_sq (* b b)]
  (+ a_sq b_sq))
; => 25 (c would be 5)

; Convert Fahrenheit to Celsius: (F - 32) * 5/9
(let [f 98.6
      minus_32 (- f 32)]
  (* minus_32 (/ 5 9)))
; => 37 (approximately)

; Calculate compound interest
(let [principal 1000
      rate 0.05
      time 2
      amount (* principal (+ 1 (* rate time)))]
  amount)
; => 1100
```

## Coming Soon

```clojure
; Def (global variables) - not yet in REPL
(def pi 3.14159)
(* pi 2)
; => 6.28318

; Function parameters will use the same mechanism
(defn square [x]
  (* x x))
```

## Try It!

Start the REPL and try these:

```bash
cargo run --bin repl
```

```clojure
λ> (let [x 10] x)
=> 10

λ> (let [x 5 y 10] (+ x y))
=> 15

λ> (let [r 10] (* 3.14 (* r r)))
=> 314
```
