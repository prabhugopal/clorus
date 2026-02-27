#!/bin/bash
# Test script for REPL output format

if [ -n "$REPL_BIN" ]; then
    REPL_CMD=("$REPL_BIN")
else
    REPL_CMD=(cargo run -p clorus-repl --bin repl-dev --quiet)
fi

echo "(def x 10)
(defn my-fun [x] (+ x 1))
(+ 2 3)
:quit" | "${REPL_CMD[@]}" 2>&1 | grep -E "^#'|^=>|^nil|^Error"
