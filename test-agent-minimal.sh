#!/bin/bash

# Minimal agent test
cat > /tmp/test-agent.clr <<'EOF'
(println "Starting agent test")
(def x 42)
(println "Created variable x:" x)
(def a (agent x))
(println "Created agent with value:" x)
EOF

./target/debug/repl < /tmp/test-agent.clr
