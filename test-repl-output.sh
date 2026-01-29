#!/bin/bash
# Test script for REPL output format

echo "(def x 10)
(defn my-fun [x] (+ x 1))
(+ 2 3)
:quit" | cargo run --bin repl 2>&1 | grep -E "^#'|^=>|^nil|^Error"
