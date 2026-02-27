#!/bin/bash
# Generate comprehensive test metrics and coverage report
# Compatible with bash 3.2+ (macOS compatible)
# Updated for feature-based test organization

set -e

CLORUS_BIN="${CLORUS_BIN:-./target/release/clorus}"
TEST_DIR="./tests"
METRICS_DIR="./test-metrics"
TIMESTAMP=$(date +%Y-%m-%d_%H-%M-%S)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Create metrics directory
mkdir -p "$METRICS_DIR"

echo "================================================"
echo "  CLORUS TEST METRICS GENERATOR"
echo "================================================"
echo ""

# Check if clorus binary exists
if [ ! -f "$CLORUS_BIN" ]; then
    echo -e "${RED}Error: Clorus binary not found at $CLORUS_BIN${NC}"
    echo "Please run: cargo build --release"
    exit 1
fi

# Initialize counters (using indexed arrays for bash 3.2 compatibility)
CATEGORY_NAMES=()
CATEGORY_PASSED=()
CATEGORY_FAILED=()
CATEGORY_TOTAL=()

TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_TESTS=0

# Detect timeout command
TIMEOUT_CMD=""
if command -v timeout &> /dev/null; then
    TIMEOUT_CMD="timeout 5s"
elif command -v gtimeout &> /dev/null; then
    TIMEOUT_CMD="gtimeout 5s"
fi

# Function to run a single test
run_test() {
    local test_file=$1

    # Run the test
    if [ -n "$TIMEOUT_CMD" ]; then
        if $TIMEOUT_CMD "$CLORUS_BIN" run "$test_file" > /dev/null 2>&1; then
            return 0
        else
            return 1
        fi
    else
        if "$CLORUS_BIN" run "$test_file" > /dev/null 2>&1; then
            return 0
        else
            return 1
        fi
    fi
}

# Function to analyze a category
analyze_category() {
    local category=$1
    local category_path="$TEST_DIR/$category"

    if [ ! -d "$category_path" ]; then
        return
    fi

    local passed=0
    local failed=0
    local total=0

    # Find all test files
    local test_files=()
    while IFS= read -r -d '' file; do
        test_files+=("$file")
    done < <(find "$category_path" \( -name "*.clr" -o -name "*.clrs" \) -print0 2>/dev/null)

    # Run each test
    for test_file in "${test_files[@]}"; do
        ((total++)) || true
        if run_test "$test_file"; then
            ((passed++)) || true
        else
            ((failed++)) || true
        fi
    done

    # Store results
    CATEGORY_NAMES+=("$category")
    CATEGORY_PASSED+=($passed)
    CATEGORY_FAILED+=($failed)
    CATEGORY_TOTAL+=($total)

    ((TOTAL_PASSED += passed)) || true
    ((TOTAL_FAILED += failed)) || true
    ((TOTAL_TESTS += total)) || true
}

echo -e "${BLUE}Analyzing test categories...${NC}\n"

# Analyze feature tests (main test suite)
echo -e "${CYAN}=== Language Features ===${NC}"
if [ -d "$TEST_DIR/features" ]; then
    for feature_dir in "$TEST_DIR/features"/*; do
        if [ -d "$feature_dir" ]; then
            feature_name=$(basename "$feature_dir")
            echo -ne "  ${feature_name}..."
            analyze_category "features/${feature_name}"
            echo -e " ${GREEN}✓${NC}"
        fi
    done
fi

# Analyze integration tests
echo -e "\n${CYAN}=== Integration Tests ===${NC}"
if [ -d "$TEST_DIR/integration" ]; then
    echo -ne "  integration..."
    analyze_category "integration"
    echo -e " ${GREEN}✓${NC}"
fi

# Analyze compiler tests (optional - not counted toward main metrics by default)
if [ -d "$TEST_DIR/compiler" ]; then
    echo -e "\n${CYAN}=== Compiler Tests ===${NC}"
    echo -ne "  compiler..."
    analyze_category "compiler"
    echo -e " ${GREEN}✓${NC}"
fi

# Calculate percentages
if [ $TOTAL_TESTS -gt 0 ]; then
    PASS_RATE=$(awk "BEGIN {printf \"%.1f\", ($TOTAL_PASSED/$TOTAL_TESTS)*100}")
    FAIL_RATE=$(awk "BEGIN {printf \"%.1f\", ($TOTAL_FAILED/$TOTAL_TESTS)*100}")
else
    PASS_RATE="0.0"
    FAIL_RATE="0.0"
fi

echo ""
echo "================================================"
echo "  TEST METRICS SUMMARY"
echo "================================================"
echo ""
echo -e "${CYAN}Total Tests:${NC}     $TOTAL_TESTS"
echo -e "${GREEN}Passed:${NC}          $TOTAL_PASSED (${PASS_RATE}%)"
echo -e "${RED}Failed:${NC}          $TOTAL_FAILED (${FAIL_RATE}%)"
echo ""

# Display category breakdown
echo "================================================"
echo "  CATEGORY BREAKDOWN"
echo "================================================"
echo ""
printf "%-25s %8s %8s %8s %8s\n" "Category" "Total" "Passed" "Failed" "Rate"
echo "----------------------------------------------------------------"

for i in "${!CATEGORY_NAMES[@]}"; do
    category="${CATEGORY_NAMES[$i]}"
    total=${CATEGORY_TOTAL[$i]}

    if [ $total -gt 0 ]; then
        passed=${CATEGORY_PASSED[$i]}
        failed=${CATEGORY_FAILED[$i]}
        rate=$(awk "BEGIN {printf \"%.1f\", ($passed/$total)*100}")

        # Color code the rate
        if awk "BEGIN {exit !($rate >= 90)}"; then
            color=$GREEN
        elif awk "BEGIN {exit !($rate >= 70)}"; then
            color=$YELLOW
        else
            color=$RED
        fi

        # Format category name
        display_name=$(echo "$category" | sed 's/features\///')

        printf "%-25s %8d %8d %8d ${color}%7.1f%%${NC}\n" \
            "$display_name" "$total" "$passed" "$failed" "$rate"
    fi
done

echo ""

# Generate JSON metrics file
METRICS_FILE="$METRICS_DIR/metrics_${TIMESTAMP}.json"
echo "{" > "$METRICS_FILE"
echo "  \"timestamp\": \"$TIMESTAMP\"," >> "$METRICS_FILE"
echo "  \"summary\": {" >> "$METRICS_FILE"
echo "    \"total_tests\": $TOTAL_TESTS," >> "$METRICS_FILE"
echo "    \"passed\": $TOTAL_PASSED," >> "$METRICS_FILE"
echo "    \"failed\": $TOTAL_FAILED," >> "$METRICS_FILE"
echo "    \"pass_rate\": $PASS_RATE" >> "$METRICS_FILE"
echo "  }," >> "$METRICS_FILE"
echo "  \"categories\": {" >> "$METRICS_FILE"

first=true
for i in "${!CATEGORY_NAMES[@]}"; do
    category="${CATEGORY_NAMES[$i]}"
    total=${CATEGORY_TOTAL[$i]}

    if [ $total -gt 0 ]; then
        if [ "$first" = false ]; then
            echo "," >> "$METRICS_FILE"
        fi
        first=false

        passed=${CATEGORY_PASSED[$i]}
        failed=${CATEGORY_FAILED[$i]}
        rate=$(awk "BEGIN {printf \"%.1f\", ($passed/$total)*100}")

        # Clean category name for JSON
        json_name=$(echo "$category" | sed 's/features\///')

        echo -n "    \"$json_name\": {" >> "$METRICS_FILE"
        echo -n "\"total\": $total, " >> "$METRICS_FILE"
        echo -n "\"passed\": $passed, " >> "$METRICS_FILE"
        echo -n "\"failed\": $failed, " >> "$METRICS_FILE"
        echo -n "\"pass_rate\": $rate" >> "$METRICS_FILE"
        echo -n "}" >> "$METRICS_FILE"
    fi
done

echo "" >> "$METRICS_FILE"
echo "  }" >> "$METRICS_FILE"
echo "}" >> "$METRICS_FILE"

echo -e "${GREEN}Metrics saved to: $METRICS_FILE${NC}"
echo ""

# Create symlink to latest
ln -sf "metrics_${TIMESTAMP}.json" "$METRICS_DIR/latest.json"

# Generate visual dashboard
echo "================================================"
echo "  VISUAL TEST DASHBOARD"
echo "================================================"
echo ""

# Function to draw progress bar
draw_bar() {
    local percentage=$1
    local width=50
    local filled=$(awk "BEGIN {printf \"%.0f\", ($percentage/100)*$width}")
    local empty=$((width - filled))

    echo -n "["
    for ((i=0; i<filled; i++)); do echo -n "█"; done
    for ((i=0; i<empty; i++)); do echo -n "░"; done
    echo -n "]"
}

echo -e "${CYAN}Overall Test Pass Rate:${NC}"
echo -n "  "
draw_bar "$PASS_RATE"
echo -e " ${GREEN}${PASS_RATE}%${NC}"
echo ""

echo -e "${CYAN}Category Pass Rates:${NC}"
for i in "${!CATEGORY_NAMES[@]}"; do
    category="${CATEGORY_NAMES[$i]}"
    total=${CATEGORY_TOTAL[$i]}

    if [ $total -gt 0 ]; then
        passed=${CATEGORY_PASSED[$i]}
        rate=$(awk "BEGIN {printf \"%.1f\", ($passed/$total)*100}")

        # Clean display name
        display_name=$(echo "$category" | sed 's/features\///')

        printf "  %-20s " "$display_name"
        draw_bar "$rate"
        printf " %.1f%%\n" "$rate"
    fi
done

echo ""
echo -e "${GREEN}✓ Metrics generation complete!${NC}"
echo ""
