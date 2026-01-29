#!/bin/bash
# Comprehensive REPL output format demo

echo "Testing Clorus REPL Output Formats"
echo "===================================="
echo ""

echo "Testing def:"
echo '(def x 10)' | cargo run --bin repl --quiet 2>&1 | grep "^#'"

echo ""
echo "Testing defn:"
echo '(defn add [a b] (+ a b))' | cargo run --bin repl --quiet 2>&1 | grep "^#'"

echo ""
echo "Testing regular expressions:"
echo '(+ 2 3)
(* 4 5)
(- 10 3)' | cargo run --bin repl --quiet 2>&1 | grep -E "^[0-9]"

echo ""
echo "Testing namespace switching:"
echo '(ns myapp.core)' | cargo run --bin repl --quiet 2>&1 | grep -E "^nil|myapp"

echo ""
echo "Testing def in custom namespace:"
echo '(ns myapp.core)
(def my-var 42)' | cargo run --bin repl --quiet 2>&1 | grep "^#'"

echo ""
echo "All tests complete!"
