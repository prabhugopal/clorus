#!/bin/bash
# Comprehensive REPL output format demo

echo "Testing Clorus REPL Output Formats"
echo "===================================="
echo ""

if [ -n "$REPL_BIN" ]; then
    REPL_CMD=("$REPL_BIN")
else
    REPL_CMD=(cargo run -p clorus-repl --bin repl-dev --quiet)
fi

echo "Testing def:"
echo '(def x 10)' | "${REPL_CMD[@]}" 2>&1 | grep "^#'"

echo ""
echo "Testing defn:"
echo '(defn add [a b] (+ a b))' | "${REPL_CMD[@]}" 2>&1 | grep "^#'"

echo ""
echo "Testing regular expressions:"
echo '(+ 2 3)
(* 4 5)
(- 10 3)' | "${REPL_CMD[@]}" 2>&1 | grep -E "^[0-9]"

echo ""
echo "Testing namespace switching:"
echo '(ns myapp.core)' | "${REPL_CMD[@]}" 2>&1 | grep -E "^nil|myapp"

echo ""
echo "Testing def in custom namespace:"
echo '(ns myapp.core)
(def my-var 42)' | "${REPL_CMD[@]}" 2>&1 | grep "^#'"

echo ""
echo "All tests complete!"
