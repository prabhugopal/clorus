#!/bin/bash
# Clorus Comprehensive Test Suite Runner
# Run this before any build to ensure all features work
# Updated for reorganized test structure:
#   - tests/language/ - Language feature tests (conditionals, loops, functions)
#   - tests/stdlib/ - Standard library tests
#   - tests/compiler/ - Compiler-specific tests
#   - tests/integration/ - Integration and system tests

set -e  # Exit on first failure

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

CLORUS_BIN="${CLORUS_BIN:-./target/release/clorus}"
TEST_DIR="./tests"
PASSED=0
FAILED=0
SKIPPED=0
# Engines for semantics parity checks. Values: "jit", "legacy"
CLORUS_TEST_ENGINES="${CLORUS_TEST_ENGINES:-jit legacy}"

# Check if clorus binary exists
if [ ! -f "$CLORUS_BIN" ]; then
    echo -e "${RED}Error: Clorus binary not found at $CLORUS_BIN${NC}"
    echo "Please run: cargo build --release"
    exit 1
fi

# Detect timeout command (timeout on Linux, gtimeout on macOS with coreutils)
TIMEOUT_CMD=""
if command -v timeout &> /dev/null; then
    TIMEOUT_CMD="timeout 5s"
elif command -v gtimeout &> /dev/null; then
    TIMEOUT_CMD="gtimeout 5s"
else
    # No timeout available - tests will run without timeout
    TIMEOUT_CMD=""
fi

echo "================================================"
echo "  CLORUS COMPREHENSIVE TEST SUITE"
echo "================================================"
echo "  Engines: $CLORUS_TEST_ENGINES"
echo ""

run_one() {
    local mode=$1
    local test_file=$2

    if [ "$mode" = "legacy" ]; then
        if [ -n "$TIMEOUT_CMD" ]; then
            CLORUS_ENTRY_FILE="$test_file" $TIMEOUT_CMD "$CLORUS_BIN" run --legacy-run > /dev/null 2>&1
        else
            CLORUS_ENTRY_FILE="$test_file" "$CLORUS_BIN" run --legacy-run > /dev/null 2>&1
        fi
        return $?
    fi

    # Default mode: JIT execution path (`clorus run`).
    if [ "$mode" = "jit" ]; then
        if [ -n "$TIMEOUT_CMD" ]; then
            CLORUS_ENTRY_FILE="$test_file" $TIMEOUT_CMD "$CLORUS_BIN" run > /dev/null 2>&1
        else
            CLORUS_ENTRY_FILE="$test_file" "$CLORUS_BIN" run > /dev/null 2>&1
        fi
        return $?
    fi

    return 99
}

# Function to run a single test file across configured engines
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .clr)
    local category=$(dirname "$test_file" | xargs basename)
    local mode

    for mode in $CLORUS_TEST_ENGINES; do
        printf "%-50s" "Testing $category/$test_name [$mode]..."

        if run_one "$mode" "$test_file"; then
            echo -e "${GREEN}✓ PASS${NC}"
            ((PASSED++))
        else
            local code=$?
            if [ "$code" -eq 99 ]; then
                echo -e "${YELLOW}⚠ SKIP${NC}"
                ((SKIPPED++))
                echo "  File: $test_file (mode=$mode, reason=unknown-engine)" >> test_failures.log
                continue
            fi

            echo -e "${RED}✗ FAIL${NC}"
            ((FAILED++))
            echo "  File: $test_file (mode=$mode)" >> test_failures.log
            return 1
        fi
    done
}

# Function to run tests in a category
run_category() {
    local category=$1
    local category_path="$TEST_DIR/$category"

    if [ ! -d "$category_path" ]; then
        echo -e "${YELLOW}⚠ Category $category not found${NC}"
        return
    fi

    echo -e "\n${BLUE}=== Testing: $category ===${NC}"

    # Find all .clr and .clrs files and store in array to avoid subshell issues
    local test_files=()
    while IFS= read -r -d '' file; do
        test_files+=("$file")
    done < <(find "$category_path" \( -name "*.clr" -o -name "*.clrs" \) -print0)

    # Run each test
    for test_file in "${test_files[@]}"; do
        run_test "$test_file"
    done
}

# Clear previous failure log
rm -f test_failures.log

# Run tests by category - Updated for new structure
echo -e "${BLUE}1. LANGUAGE FEATURE TESTS${NC}"
run_category "language"

echo -e "\n${BLUE}2. STANDARD LIBRARY TESTS${NC}"
run_category "stdlib"

echo -e "\n${BLUE}3. COMPILER TESTS${NC}"
run_category "compiler"

echo -e "\n${BLUE}4. INTEGRATION TESTS${NC}"
run_category "integration"

# Summary
echo ""
echo "================================================"
echo "  TEST SUMMARY"
echo "================================================"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo -e "${YELLOW}Skipped: $SKIPPED${NC}"

if [ $FAILED -gt 0 ]; then
    echo ""
    echo -e "${RED}Failed tests logged to: test_failures.log${NC}"
    echo ""
    exit 1
else
    echo ""
    echo -e "${GREEN}All tests passed! ✓${NC}"
    echo ""
    exit 0
fi
