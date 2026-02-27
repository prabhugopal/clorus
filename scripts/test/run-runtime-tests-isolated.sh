#!/bin/bash
# Run clorus-runtime lib tests in isolated processes and produce a report.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/test-reports"
REPORT_MD="$REPORT_DIR/runtime-tests.md"
REPORT_TMP="${TMPDIR:-/tmp}/clorus_runtime_tests.$$.md"

mkdir -p "$REPORT_DIR"
chmod u+w "$REPORT_DIR" 2>/dev/null || true
rm -f "$REPORT_MD" 2>/dev/null || true
touch "$REPORT_MD" 2>/dev/null || true
chmod u+w "$REPORT_DIR"/*.md 2>/dev/null || true

echo "Discovering clorus-runtime tests..."
TESTS=()
while IFS= read -r test_name; do
  [ -z "$test_name" ] && continue
  # Strip trailing ':' from cargo --list output
  TESTS+=("${test_name%:}")
done < <(cargo test -p clorus-runtime --lib -- --list | awk '{print $1}' | grep '::tests::' || true)

if [ "${#TESTS[@]}" -eq 0 ]; then
  echo "No tests discovered. Aborting." >&2
  exit 1
fi

total=${#TESTS[@]}
passed=0
failed=0

{
  echo "# Clorus Runtime Test Report"
  echo ""
  echo "Generated: $(date -u +"%Y-%m-%d %H:%M:%SZ")"
  echo ""
  echo "| Test | Status |"
  echo "| --- | --- |"
} > "$REPORT_TMP"

for test_name in "${TESTS[@]}"; do
  echo "Running $test_name..."
  if cargo test -p clorus-runtime --lib -- --nocapture --exact "$test_name" > "${TMPDIR:-/tmp}/runtime_test.$$.log" 2>&1; then
    echo "| \`$test_name\` | PASS |" >> "$REPORT_TMP"
    ((passed++))
  else
    echo "| \`$test_name\` | FAIL |" >> "$REPORT_TMP"
    echo "" >> "$REPORT_TMP"
    echo "## Failure: $test_name" >> "$REPORT_TMP"
    echo "" >> "$REPORT_TMP"
    echo '```text' >> "$REPORT_TMP"
    tail -n 120 "${TMPDIR:-/tmp}/runtime_test.$$.log" >> "$REPORT_TMP"
    echo '```' >> "$REPORT_TMP"
    echo "" >> "$REPORT_TMP"
    ((failed++))
  fi
done

{
  echo ""
  echo "## Summary"
  echo ""
  echo "- Total: $total"
  echo "- Passed: $passed"
  echo "- Failed: $failed"
} >> "$REPORT_TMP"

mv "$REPORT_TMP" "$REPORT_MD"
  echo "Report written to $REPORT_MD"

if [ "$failed" -gt 0 ]; then
  exit 1
fi
