#!/bin/bash
# Clorus Comprehensive Test Suite Runner
# Run this before any build to ensure all features work
# Updated for reorganized test structure:
#   - tests/language/ - Language feature tests (conditionals, loops, functions)
#   - tests/stdlib/ - Standard library tests
#   - tests/compiler/ - Compiler-specific tests
#   - tests/integration/ - Integration and system tests

# NOTE: deliberately no `set -e`. Every test failure is captured via
# PASSED/FAILED/SKIPPED counters and test_failures.log; the script must run
# the full suite and report a complete summary, not stop at the first
# failure it hits.

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
export CLORUS_HOME="${CLORUS_HOME:-$(pwd)}"
# Engines for semantics parity checks. Values: "jit", "legacy"
CLORUS_TEST_ENGINES="${CLORUS_TEST_ENGINES:-jit legacy}"
CLORUS_TEST_TIMEOUT_SECONDS="${CLORUS_TEST_TIMEOUT_SECONDS:-20}"
CLORUS_TEST_JOBS="${CLORUS_TEST_JOBS:-2}"
CLORUS_TEST_ISOLATE="${CLORUS_TEST_ISOLATE:-1}"
CLORUS_TEST_ISOLATION_ROOT="${CLORUS_TEST_ISOLATION_ROOT:-/tmp/clorus-test-isolation}"

# Check if clorus binary exists
if [ ! -f "$CLORUS_BIN" ]; then
    echo -e "${RED}Error: Clorus binary not found at $CLORUS_BIN${NC}"
    echo "Please run: cargo build --release"
    exit 1
fi

# Detect timeout command (timeout on Linux, gtimeout on macOS with coreutils)
TIMEOUT_CMD=""
if command -v timeout &> /dev/null; then
    TIMEOUT_CMD="timeout ${CLORUS_TEST_TIMEOUT_SECONDS}s"
elif command -v gtimeout &> /dev/null; then
    TIMEOUT_CMD="gtimeout ${CLORUS_TEST_TIMEOUT_SECONDS}s"
else
    # No timeout available - tests will run without timeout
    TIMEOUT_CMD=""
fi

echo "================================================"
echo "  CLORUS COMPREHENSIVE TEST SUITE"
echo "================================================"
echo "  Engines: $CLORUS_TEST_ENGINES"
echo "  Parallel jobs: $CLORUS_TEST_JOBS"
echo "  Isolated artifacts: $CLORUS_TEST_ISOLATE"
if [ -n "$TIMEOUT_CMD" ]; then
    echo "  Timeout: ${CLORUS_TEST_TIMEOUT_SECONDS}s per test"
else
    echo "  Timeout: disabled (timeout command not found)"
fi
echo ""

run_one() {
    local mode=$1
    local test_file=$2
    local run_flag=""

    if [ "$mode" = "legacy" ]; then
        run_flag="--legacy-run"
    elif [ "$mode" != "jit" ]; then
        return 99
    fi

    local cargo_target_dir=""
    local tmp_dir=""
    if [ "$CLORUS_TEST_ISOLATE" = "1" ]; then
        cargo_target_dir="${CLORUS_TEST_ISOLATION_ROOT}/${mode}/cargo-target"
        tmp_dir="${CLORUS_TEST_ISOLATION_ROOT}/${mode}/tmp"
        mkdir -p "$cargo_target_dir" "$tmp_dir"
    fi

    if [ -n "$TIMEOUT_CMD" ]; then
        if [ "$CLORUS_TEST_ISOLATE" = "1" ]; then
            CLORUS_ENTRY_FILE="$test_file" CARGO_TARGET_DIR="$cargo_target_dir" TMPDIR="$tmp_dir" \
                $TIMEOUT_CMD "$CLORUS_BIN" run $run_flag > /dev/null 2>&1
        else
            CLORUS_ENTRY_FILE="$test_file" $TIMEOUT_CMD "$CLORUS_BIN" run $run_flag > /dev/null 2>&1
        fi
    else
        if [ "$CLORUS_TEST_ISOLATE" = "1" ]; then
            CLORUS_ENTRY_FILE="$test_file" CARGO_TARGET_DIR="$cargo_target_dir" TMPDIR="$tmp_dir" \
                "$CLORUS_BIN" run $run_flag > /dev/null 2>&1
        else
            CLORUS_ENTRY_FILE="$test_file" "$CLORUS_BIN" run $run_flag > /dev/null 2>&1
        fi
    fi

    return $?
}

expected_compile_error() {
    local test_file=$1
    rg "^;\\s*EXPECT_COMPILE_ERROR:\\s*(.+)$" "$test_file" -r '$1' | head -n 1
}

run_test_mode() {
    local mode=$1
    local test_file=$2
    local expected_error=$3
    local result_file=$4
    local output_file
    local exit_code=0

    output_file=$(mktemp "/tmp/clorus-test-output.XXXXXX")

    if [ -n "$expected_error" ]; then
        local run_flag=""
        if [ "$mode" = "legacy" ]; then
            run_flag="--legacy-run"
        elif [ "$mode" != "jit" ]; then
            echo "SKIP" > "$result_file"
            echo "  File: $test_file (mode=$mode, reason=unknown-engine)" > "${result_file}.details"
            rm -f "$output_file"
            return 0
        fi

        local cargo_target_dir=""
        local tmp_dir=""
        if [ "$CLORUS_TEST_ISOLATE" = "1" ]; then
            cargo_target_dir="${CLORUS_TEST_ISOLATION_ROOT}/${mode}/cargo-target"
            tmp_dir="${CLORUS_TEST_ISOLATION_ROOT}/${mode}/tmp"
            mkdir -p "$cargo_target_dir" "$tmp_dir"
        fi

        if [ -n "$TIMEOUT_CMD" ]; then
            if [ "$CLORUS_TEST_ISOLATE" = "1" ]; then
                CLORUS_ENTRY_FILE="$test_file" CARGO_TARGET_DIR="$cargo_target_dir" TMPDIR="$tmp_dir" \
                    $TIMEOUT_CMD "$CLORUS_BIN" run $run_flag >"$output_file" 2>&1 || exit_code=$?
            else
                CLORUS_ENTRY_FILE="$test_file" $TIMEOUT_CMD "$CLORUS_BIN" run $run_flag >"$output_file" 2>&1 || exit_code=$?
            fi
        else
            if [ "$CLORUS_TEST_ISOLATE" = "1" ]; then
                CLORUS_ENTRY_FILE="$test_file" CARGO_TARGET_DIR="$cargo_target_dir" TMPDIR="$tmp_dir" \
                    "$CLORUS_BIN" run $run_flag >"$output_file" 2>&1 || exit_code=$?
            else
                CLORUS_ENTRY_FILE="$test_file" "$CLORUS_BIN" run $run_flag >"$output_file" 2>&1 || exit_code=$?
            fi
        fi

        if [ "$exit_code" -ne 0 ] && rg -q --fixed-strings "$expected_error" "$output_file"; then
            echo "PASS" > "$result_file"
        else
            echo "FAIL" > "$result_file"
            {
                echo "  File: $test_file (mode=$mode, expected_compile_error=$expected_error)"
                echo "  Output: $output_file"
            } > "${result_file}.details"
        fi
    else
        if run_one "$mode" "$test_file"; then
            echo "PASS" > "$result_file"
        else
            local code=$?
            if [ "$code" -eq 99 ]; then
                echo "SKIP" > "$result_file"
                echo "  File: $test_file (mode=$mode, reason=unknown-engine)" > "${result_file}.details"
            else
                echo "FAIL" > "$result_file"
                echo "  File: $test_file (mode=$mode)" > "${result_file}.details"
            fi
        fi
    fi

    rm -f "$output_file"
}

# Function to run a single test file across configured engines
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .clr)
    local category=$(dirname "$test_file" | xargs basename)
    local mode
    local expected_error
    expected_error="$(expected_compile_error "$test_file")"

    local modes=()
    while read -r mode; do
        [ -n "$mode" ] && modes+=("$mode")
    done < <(printf "%s\n" $CLORUS_TEST_ENGINES)

    if [ "$CLORUS_TEST_JOBS" -gt 1 ] && [ "${#modes[@]}" -gt 1 ]; then
        local pids=()
        local result_files=()
        local i
        local mode

        for mode in "${modes[@]}"; do
            local result_file
            result_file=$(mktemp "/tmp/clorus-test-result.XXXXXX")
            result_files+=("$result_file")
            run_test_mode "$mode" "$test_file" "$expected_error" "$result_file" &
            pids+=($!)
        done

        for i in "${!pids[@]}"; do
            if wait "${pids[$i]}"; then
                :
            else
                # Result file still carries PASS/FAIL/SKIP state; continue processing.
                :
            fi
        done

        for i in "${!modes[@]}"; do
            mode="${modes[$i]}"
            printf "%-50s" "Testing $category/$test_name [$mode]..."
            local status
            status=$(cat "${result_files[$i]}")
            case "$status" in
                PASS)
                    echo -e "${GREEN}✓ PASS${NC}"
                    ((PASSED+=1))
                    ;;
                SKIP)
                    echo -e "${YELLOW}⚠ SKIP${NC}"
                    ((SKIPPED+=1))
                    [ -f "${result_files[$i]}.details" ] && cat "${result_files[$i]}.details" >> test_failures.log
                    ;;
                *)
                    echo -e "${RED}✗ FAIL${NC}"
                    ((FAILED+=1))
                    [ -f "${result_files[$i]}.details" ] && cat "${result_files[$i]}.details" >> test_failures.log
                    rm -f "${result_files[@]}" "${result_files[@]/%/.details}"
                    return 1
                    ;;
            esac
        done

        rm -f "${result_files[@]}" "${result_files[@]/%/.details}"
    else
        for mode in "${modes[@]}"; do
            printf "%-50s" "Testing $category/$test_name [$mode]..."
            local result_file
            result_file=$(mktemp "/tmp/clorus-test-result.XXXXXX")
            run_test_mode "$mode" "$test_file" "$expected_error" "$result_file"
            local status
            status=$(cat "$result_file")
            case "$status" in
                PASS)
                    echo -e "${GREEN}✓ PASS${NC}"
                    ((PASSED+=1))
                    ;;
                SKIP)
                    echo -e "${YELLOW}⚠ SKIP${NC}"
                    ((SKIPPED+=1))
                    [ -f "${result_file}.details" ] && cat "${result_file}.details" >> test_failures.log
                    ;;
                *)
                    echo -e "${RED}✗ FAIL${NC}"
                    ((FAILED+=1))
                    [ -f "${result_file}.details" ] && cat "${result_file}.details" >> test_failures.log
                    rm -f "$result_file" "${result_file}.details"
                    return 1
                    ;;
            esac
            rm -f "$result_file" "${result_file}.details"
        done
    fi
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

run_repl_regressions() {
    echo -e "\n${BLUE}5. REPL REGRESSION TESTS${NC}"

    local repl_output_file="/tmp/clorus_repl_regression.log"
    local clorus_bin_abs
    clorus_bin_abs="$(cd "$(dirname "$CLORUS_BIN")" && pwd)/$(basename "$CLORUS_BIN")"
    local clorus_home_for_tests
    clorus_home_for_tests="$(cd "$(dirname "$clorus_bin_abs")/../.." && pwd)"
    local mode

    for mode in $CLORUS_TEST_ENGINES; do
        printf "%-50s" "Testing repl/core-map-defmacro [$mode]..."

        if [ "$mode" = "legacy" ]; then
            # REPL exercises JIT path; legacy mode is not applicable here.
            echo -e "${YELLOW}⚠ SKIP${NC}"
            ((SKIPPED+=1))
            continue
        fi

        # Pipe scripted input to REPL and validate key behaviors:
        # - defmacro returns var-ish symbol name
        # - macro expansion has usable env form
        # - core map is callable
        if (
            cd /tmp && cat <<'EOF' | CLORUS_HOME="$clorus_home_for_tests" CLORUS_REPL_NO_AUTO_LOAD=1 CLORUS_REPL_LOAD_STDLIB=1 "$clorus_bin_abs" repl > "$repl_output_file" 2>&1
(defmacro show-env [] &env)
(show-env)
(map #(* % 2) [1 2 3 4])
(map (constantly 9) [1 2 3])
(filter odd? [1 2 3 4 5])
(take 3 [10 20 30 40])
(drop 2 [10 20 30 40])
:q
EOF
        )
        then
            :
        else
            echo -e "${RED}✗ FAIL${NC}"
            ((FAILED+=1))
            echo "  File: repl/core-map-defmacro (mode=$mode, reason=repl-exit-nonzero)" >> test_failures.log
            continue
        fi

        if grep -qE "#'.+/show-env" "$repl_output_file" \
            && grep -q "\[2 4 6 8\]" "$repl_output_file" \
            && grep -q "\[9 9 9\]" "$repl_output_file" \
            && grep -q "\[1 3 5\]" "$repl_output_file" \
            && grep -q "\[10 20 30\]" "$repl_output_file" \
            && grep -q "\[30 40\]" "$repl_output_file" \
            && ! grep -q "Undefined function: map" "$repl_output_file"; then
            echo -e "${GREEN}✓ PASS${NC}"
            ((PASSED+=1))
        else
            echo -e "${RED}✗ FAIL${NC}"
            ((FAILED+=1))
            echo "  File: repl/core-map-defmacro (mode=$mode)" >> test_failures.log
            echo "  Output: $repl_output_file" >> test_failures.log
        fi
    done
}

run_runtime_noise_regressions() {
    echo -e "\n${BLUE}6. RUNTIME NOISE REGRESSION TESTS${NC}"

    local mode
    local output_file
    local noise_pattern="Attempted to call non-function|Attempted to call null as function|Attempted to call unresolved var as function|Arity mismatch:|Attempted to deref invalid value pointer"
    local test_file="tests/stdlib/test-core-seq-builder-error-paths.clr"

    for mode in $CLORUS_TEST_ENGINES; do
        printf "%-50s" "Testing runtime/no-call-noise [$mode]..."
        output_file=$(mktemp "/tmp/clorus-runtime-noise.XXXXXX")

        if [ "$mode" = "legacy" ]; then
            if [ -n "$TIMEOUT_CMD" ]; then
                CLORUS_ENTRY_FILE="$test_file" $TIMEOUT_CMD "$CLORUS_BIN" run --legacy-run >"$output_file" 2>&1
            else
                CLORUS_ENTRY_FILE="$test_file" "$CLORUS_BIN" run --legacy-run >"$output_file" 2>&1
            fi
        elif [ "$mode" = "jit" ]; then
            if [ -n "$TIMEOUT_CMD" ]; then
                CLORUS_ENTRY_FILE="$test_file" $TIMEOUT_CMD "$CLORUS_BIN" run >"$output_file" 2>&1
            else
                CLORUS_ENTRY_FILE="$test_file" "$CLORUS_BIN" run >"$output_file" 2>&1
            fi
        else
            echo -e "${YELLOW}⚠ SKIP${NC}"
            ((SKIPPED+=1))
            rm -f "$output_file"
            continue
        fi

        if rg -q "$noise_pattern" "$output_file"; then
            echo -e "${RED}✗ FAIL${NC}"
            ((FAILED+=1))
            {
                echo "  File: runtime/no-call-noise (mode=$mode)"
                echo "  Output: $output_file"
            } >> test_failures.log
            return 1
        else
            echo -e "${GREEN}✓ PASS${NC}"
            ((PASSED+=1))
            rm -f "$output_file"
        fi
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

run_repl_regressions
run_runtime_noise_regressions

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
