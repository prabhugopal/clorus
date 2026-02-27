#!/bin/bash
# Unified test runner (Rust + Clorus + REPL + Runtime Isolated)

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLORUS_BIN="$ROOT_DIR/target/release/clorus"
REPORT_DIR="$ROOT_DIR/test-reports"
REPORT_MD="$REPORT_DIR/all-tests.md"
REPORT_TMP="${TMPDIR:-/tmp}/clorus_all_tests.$$.md"

mkdir -p "$REPORT_DIR"
chmod u+w "$REPORT_DIR" 2>/dev/null || true
rm -f "$REPORT_MD" "$REPORT_DIR/runtime-tests.md" 2>/dev/null || true
touch "$REPORT_MD" "$REPORT_DIR/runtime-tests.md" 2>/dev/null || true
chmod u+w "$REPORT_DIR"/*.md 2>/dev/null || true

timestamp="$(date -u +"%Y-%m-%d %H:%M:%SZ")"
{
  echo "# Clorus Full Test Report"
  echo ""
  echo "Generated: $timestamp"
  echo ""
  echo "| Suite | Status |"
  echo "| --- | --- |"
} > "$REPORT_TMP"

passed=0
failed=0
total=0

run_step() {
  local name="$1"
  local cmd="$2"
  local log="$3"

  total=$((total + 1))
  echo "Running: $name"
  if bash -c "$cmd" > "$log" 2>&1; then
    echo "| $name | PASS |" >> "$REPORT_TMP"
    passed=$((passed + 1))
    return 0
  else
    echo "| $name | FAIL |" >> "$REPORT_TMP"
    echo "" >> "$REPORT_TMP"
    echo "## Failure: $name" >> "$REPORT_TMP"
    echo "" >> "$REPORT_TMP"
    echo '```text' >> "$REPORT_TMP"
    tail -n 120 "$log" >> "$REPORT_TMP"
    echo '```' >> "$REPORT_TMP"
    echo "" >> "$REPORT_TMP"
    failed=$((failed + 1))
    return 1
  fi
}

echo "================================================"
echo "  CLORUS FULL TEST SUITE"
echo "================================================"
echo ""

run_step "Rust tests (cargo test, excluding clorus-runtime)" \
  "cargo test --workspace --exclude clorus-runtime" \
  "/tmp/clorus_cargo_test.log"

echo ""
echo "Clorus language tests"
if [ ! -f "$CLORUS_BIN" ]; then
  echo "Error: $CLORUS_BIN not found. Build with: cargo build --release"
  echo "| Clorus language tests | FAIL |" >> "$REPORT_TMP"
  failed=$((failed + 1))
else
  if [ -x "$ROOT_DIR/tests/run_all_tests.sh" ]; then
    run_step "Clorus language tests" "(cd \"$ROOT_DIR\" && ./tests/run_all_tests.sh)" "/tmp/clorus_lang_tests.log"
  else
    echo "Error: $ROOT_DIR/tests/run_all_tests.sh not found or not executable"
    echo "| Clorus language tests | FAIL |" >> "$REPORT_TMP"
    failed=$((failed + 1))
  fi
fi

echo ""
echo "REPL output smoke tests"
run_step "REPL output smoke" \
  "REPL_BIN=\"$ROOT_DIR/target/release/repl-dev\" bash \"$ROOT_DIR/scripts/test/test-repl-output.sh\" && REPL_BIN=\"$ROOT_DIR/target/release/repl-dev\" bash \"$ROOT_DIR/scripts/test/test-repl-formats.sh\"" \
  "/tmp/clorus_repl_tests.log"

echo ""
echo "Runtime tests (isolated processes)"
run_step "Runtime tests (isolated)" \
  "bash \"$ROOT_DIR/scripts/test/run-runtime-tests-isolated.sh\"" \
  "/tmp/clorus_runtime_isolated.log"

{
  echo ""
  echo "## Summary"
  echo ""
  echo "- Total suites: $total"
  echo "- Passed: $passed"
  echo "- Failed: $failed"
  echo ""
  echo "## Reports"
  echo ""
  echo "- test-reports/all-tests.md"
  echo "- test-reports/runtime-tests.md"
} >> "$REPORT_TMP"

mv "$REPORT_TMP" "$REPORT_MD"

echo ""
echo "Report written to $REPORT_MD"

if [ "$failed" -gt 0 ]; then
  exit 1
fi
