#!/usr/bin/env bash
# Clorus Compiler/Runtime Build & Test Script
# Builds the core language and optionally runs tests with metrics

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
BUILD_RELEASE=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-tests)
            RUN_TESTS=false
            shift
            ;;
        --release)
            BUILD_RELEASE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --no-tests    Build without running tests"
            echo "  --release     Build in release mode (optimized)"
            echo "  --verbose     Show detailed build output"
            echo "  --help        Show this help message"
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
echo -e "${BOLD}${BLUE}  Clorus Compiler/Runtime Build Script${NC}"
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Start timer
START_TIME=$(date +%s)

# Determine build mode
if [ "$BUILD_RELEASE" = true ]; then
    BUILD_MODE="release"
    BUILD_FLAGS="--release"
    echo -e "${YELLOW}Build Mode:${NC} Release (optimized)"
else
    BUILD_MODE="debug"
    BUILD_FLAGS=""
    echo -e "${YELLOW}Build Mode:${NC} Debug (fast compile)"
fi

echo -e "${YELLOW}Tests:${NC} $([ "$RUN_TESTS" = true ] && echo "Enabled" || echo "Disabled")"
echo ""

# Build the project
echo -e "${BOLD}[1/3] Building Clorus...${NC}"
if [ "$VERBOSE" = true ]; then
    cargo build $BUILD_FLAGS
else
    cargo build $BUILD_FLAGS --quiet
fi

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi
echo ""

# Run tests if enabled
if [ "$RUN_TESTS" = true ]; then
    echo -e "${BOLD}[2/3] Running Rust tests...${NC}"

    # Capture test output
    TEST_OUTPUT=$(cargo test $BUILD_FLAGS 2>&1)
    TEST_EXIT_CODE=$?

    # Parse test results
    TOTAL_TESTS=$(echo "$TEST_OUTPUT" | grep -o "running [0-9]* test" | grep -o "[0-9]*" | head -1 || echo "0")
    PASSED_TESTS=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -o "[0-9]* passed" | grep -o "[0-9]*" || echo "0")
    FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -o "[0-9]* failed" | grep -o "[0-9]*" || echo "0")

    # Show results
    if [ "$VERBOSE" = true ]; then
        echo "$TEST_OUTPUT"
        echo ""
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Test Summary:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "  Total:   ${BOLD}$TOTAL_TESTS${NC} tests"
    echo -e "  Passed:  ${GREEN}$PASSED_TESTS${NC}"
    echo -e "  Failed:  ${RED}$FAILED_TESTS${NC}"

    if [ $TEST_EXIT_CODE -eq 0 ]; then
        echo -e "  ${GREEN}✓ All tests passed${NC}"
    else
        echo -e "  ${RED}✗ Some tests failed${NC}"
    fi
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
else
    echo -e "${YELLOW}[2/3] Skipping tests (--no-tests flag)${NC}"
    echo ""
fi

# Binary location
echo -e "${BOLD}[3/3] Build artifacts:${NC}"
if [ "$BUILD_MODE" = "release" ]; then
    BINARY_PATH="target/release/clorus"
else
    BINARY_PATH="target/debug/clorus"
fi

if [ -f "$BINARY_PATH" ]; then
    BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    echo -e "  Binary: ${GREEN}$BINARY_PATH${NC} (${BINARY_SIZE})"
else
    echo -e "  ${RED}Binary not found at $BINARY_PATH${NC}"
fi
echo ""

# Calculate build time
END_TIME=$(date +%s)
BUILD_TIME=$((END_TIME - START_TIME))

# Print summary
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}${GREEN}✓ Build Complete${NC}"
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "  Build time: ${BUILD_TIME}s"
echo -e "  Mode: ${BUILD_MODE}"
if [ "$RUN_TESTS" = true ]; then
    if [ $TEST_EXIT_CODE -eq 0 ]; then
        echo -e "  Tests: ${GREEN}✓ $PASSED_TESTS/$TOTAL_TESTS passed${NC}"
    else
        echo -e "  Tests: ${RED}✗ $FAILED_TESTS/$TOTAL_TESTS failed${NC}"
        exit 1
    fi
else
    echo -e "  Tests: ${YELLOW}Skipped${NC}"
fi
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Quick start instructions
echo -e "${BOLD}Quick Start:${NC}"
echo -e "  Run REPL:  ${BLUE}$BINARY_PATH repl${NC}"
echo -e "  Run file:  ${BLUE}$BINARY_PATH run <file.clr>${NC}"
echo -e "  Help:      ${BLUE}$BINARY_PATH --help${NC}"
echo ""

exit 0
