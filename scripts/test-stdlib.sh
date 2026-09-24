#!/usr/bin/env bash
# Clorus Standard Library Test Script
# Runs stdlib tests and provides detailed metrics

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
RUN_TESTS=true
VERBOSE=false
TEST_PATTERN=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-tests)
            RUN_TESTS=false
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --pattern)
            TEST_PATTERN="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --no-tests     Skip running tests"
            echo "  --verbose      Show detailed test output"
            echo "  --pattern PAT  Run only tests matching pattern"
            echo "  --help         Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                          # Run all tests"
            echo "  $0 --pattern lang           # Run only lang tests"
            echo "  $0 --pattern concurrency    # Run only concurrency tests"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run with --help for usage"
            exit 1
            ;;
    esac
done

# Print header
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}${BLUE}  Clorus Standard Library Test Script${NC}"
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

START_TIME=$(date +%s)

# Check if clorus binary exists
BINARY="target/debug/clorus"
if [ ! -f "$BINARY" ]; then
    BINARY="target/release/clorus"
fi

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}✗ Clorus binary not found${NC}"
    echo -e "  Run ${BLUE}./scripts/build-compiler.sh${NC} first"
    exit 1
fi

echo -e "${YELLOW}Binary:${NC} $BINARY"
echo -e "${YELLOW}Tests:${NC} $([ "$RUN_TESTS" = true ] && echo "Enabled" || echo "Disabled")"
echo -e "${YELLOW}Pattern:${NC} $([ -z "$TEST_PATTERN" ] && echo "(all)" || echo "$TEST_PATTERN")"
echo ""

if [ "$RUN_TESTS" = false ]; then
    echo -e "${YELLOW}Tests disabled (--no-tests flag)${NC}"
    exit 0
fi

# Find test files
echo -e "${BOLD}[1/2] Discovering test files...${NC}"

TEST_DIRS=(
    "tests/language"
    "tests/stdlib"
    "tests/compiler"
    "tests/integration"
    "tests/parity"
)

TEST_FILES=()
for dir in "${TEST_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        while IFS= read -r -d '' file; do
            if [[ "$file" == *"$TEST_PATTERN"* ]]; then
                TEST_FILES+=("$file")
            fi
        done < <(find "$dir" -name "*.clr" -type f -print0)
    fi
done

TOTAL_FILES=${#TEST_FILES[@]}
echo -e "  Found ${BOLD}$TOTAL_FILES${NC} test files"
echo ""

# Run tests
echo -e "${BOLD}[2/2] Running tests...${NC}"
echo ""

PASSED_FILES=0
FAILED_FILES=0
PASSED_TESTS=0
FAILED_TESTS=0

# Portable category-stats tracking (no associative arrays -- macOS ships
# bash 3.2 by default, which has no `declare -A`). CATEGORY_NAMES holds each
# category seen so far in first-seen order; CATEGORY_PASS/CATEGORY_FAIL are
# parallel indexed arrays of counts, looked up by matching index.
CATEGORY_NAMES=()
CATEGORY_PASS=()
CATEGORY_FAIL=()

# Sets CATEGORY_IDX as a side effect. Must be called as a plain statement
# (not via command substitution) so its array/variable mutations happen in
# this shell rather than in a disposable subshell.
category_index() {
    local name="$1"
    local i
    for i in "${!CATEGORY_NAMES[@]}"; do
        if [ "${CATEGORY_NAMES[$i]}" = "$name" ]; then
            CATEGORY_IDX="$i"
            return 0
        fi
    done
    CATEGORY_NAMES+=("$name")
    CATEGORY_PASS+=(0)
    CATEGORY_FAIL+=(0)
    CATEGORY_IDX=$((${#CATEGORY_NAMES[@]} - 1))
}

for test_file in "${TEST_FILES[@]}"; do
    # Extract category from path
    CATEGORY=$(echo "$test_file" | cut -d'/' -f2)
    category_index "$CATEGORY"
    idx="$CATEGORY_IDX"

    # Get relative filename
    FILENAME=$(basename "$test_file")

    # Run test
    if [ "$VERBOSE" = true ]; then
        echo -e "${BLUE}Running:${NC} $test_file"
    fi

    if $BINARY run "$test_file" >/dev/null 2>&1; then
        if [ "$VERBOSE" = true ]; then
            echo -e "  ${GREEN}✓ PASS${NC}"
        else
            echo -n "."
        fi
        PASSED_FILES=$((PASSED_FILES + 1))
        PASSED_TESTS=$((PASSED_TESTS + 1))
        CATEGORY_PASS[$idx]=$((CATEGORY_PASS[$idx] + 1))
    else
        if [ "$VERBOSE" = true ]; then
            echo -e "  ${RED}✗ FAIL${NC}"
        else
            echo -n "F"
        fi
        FAILED_FILES=$((FAILED_FILES + 1))
        FAILED_TESTS=$((FAILED_TESTS + 1))
        CATEGORY_FAIL[$idx]=$((CATEGORY_FAIL[$idx] + 1))

        echo -e "\n${RED}Failed: $test_file${NC}"
    fi
done

if [ "$VERBOSE" = false ]; then
    echo ""  # Newline after dots
fi
echo ""

# Calculate metrics
END_TIME=$(date +%s)
TEST_TIME=$((END_TIME - START_TIME))
SUCCESS_RATE=0
if [ $TOTAL_FILES -gt 0 ]; then
    SUCCESS_RATE=$((PASSED_FILES * 100 / TOTAL_FILES))
fi

# Print summary
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}  Test Summary${NC}"
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  Total Files:    ${BOLD}$TOTAL_FILES${NC}"
echo -e "  Passed Files:   ${GREEN}$PASSED_FILES${NC}"
echo -e "  Failed Files:   ${RED}$FAILED_FILES${NC}"
echo -e "  Success Rate:   ${BOLD}$SUCCESS_RATE%${NC}"
echo -e "  Test Time:      ${TEST_TIME}s"
echo ""

# Print category breakdown
echo -e "${BOLD}Category Breakdown:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for category in $(printf '%s\n' "${CATEGORY_NAMES[@]}" | sort); do
    category_index "$category"
    idx="$CATEGORY_IDX"
    cat_pass=${CATEGORY_PASS[$idx]}
    cat_fail=${CATEGORY_FAIL[$idx]}
    cat_total=$((cat_pass + cat_fail))
    cat_rate=0
    if [ $cat_total -gt 0 ]; then
        cat_rate=$((cat_pass * 100 / cat_total))
    fi

    printf "  %-15s ${GREEN}%3d${NC} passed, ${RED}%3d${NC} failed  [${BOLD}%3d%%${NC}]\n" \
        "$category:" "$cat_pass" "$cat_fail" "$cat_rate"
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Final status
if [ $FAILED_FILES -eq 0 ]; then
    echo -e "${BOLD}${GREEN}✓ All tests passed!${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    exit 0
else
    echo -e "${BOLD}${RED}✗ Some tests failed${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    exit 1
fi
