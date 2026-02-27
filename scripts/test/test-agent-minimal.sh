#!/bin/bash

# Minimal agent test
cat > /tmp/test-agent.clr <<'EOF'
(println "Starting agent test")
(def x 42)
(println "Created variable x:" x)
(def a (agent x))
(println "Created agent with value:" x)
EOF

if [ -n "$REPL_BIN" ]; then
    REPL_CMD=("$REPL_BIN")
else
    REPL_CMD=(cargo run -p clorus-repl --bin repl-dev --quiet)
fi

"${REPL_CMD[@]}" < /tmp/test-agent.clr
