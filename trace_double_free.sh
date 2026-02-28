#!/usr/bin/env bash
set -e
set -o pipefail

CLORUS_BIN="/Users/prabhugopal/Workspace/github/clorus/target/debug/clorus"
LOG="/tmp/clorus.log"
APP_DIR="${CLORUS_APP_DIR:-/Users/prabhugopal/Workspace/github/coral/coral-examples/gallery}"

if [ ! -d "$APP_DIR" ]; then
  echo "error: APP_DIR not found: $APP_DIR" >&2
  exit 1
fi

run_clorus() {
  (cd "$APP_DIR" && "$CLORUS_BIN" run --jit)
}

set +e
run_clorus 2>&1 | tee "$LOG"
RUN_RC=${PIPESTATUS[0]}
set -e
if [ "$RUN_RC" -ne 0 ]; then
  echo "first run exited with code $RUN_RC" >&2
fi

PTR=$(rg -n "double free detected" "$LOG" | head -n 1 | rg -o "0x[0-9a-fA-F]+" | head -n 1 || true)

if [ -z "$PTR" ]; then
  echo "no double free detected in first run" >&2
  exit 2
fi

echo "PTR=$PTR"

CLORUS_DEBUG_PTR="$PTR" CLORUS_DEBUG_RELEASE=1 CLORUS_GUARD_RELEASE=1 run_clorus 2>&1 | tee "$LOG"

rg -n "$PTR" "$LOG" || true
