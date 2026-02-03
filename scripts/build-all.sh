#!/usr/bin/env bash
# Clorus Full Build & Test Script
# Builds compiler/runtime and stdlib, runs all tests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
BUILD_COMPILER=true
TEST_COMPILER=true
TEST_STDLIB=true
BUILD_RELEASE=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-compiler-tests)
            TEST_COMPILER=false
            shift
            ;;
        --no-stdlib-tests)
            TEST_STDLIB=false
            shift
            ;;
        --no-tests)
            TEST_COMPILER=false
            TEST_STDLIB=false
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
            echo "  --no-compiler-tests   Skip Rust/compiler tests"
            echo "  --no-stdlib-tests     Skip stdlib .clr tests"
            echo "  --no-tests            Skip all tests"
            echo "  --release             Build in release mode"
            echo "  --verbose             Show detailed output"
            echo "  --help                Show this help message"
            echo ""
            echo "This script runs:"
            echo "  1. Compiler/runtime build + Rust tests"
            echo "  2. Standard library .clr tests"
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
echo -e "${BOLD}${BLUE}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║        Clorus Full Build & Test Suite           ║${NC}"
echo -e "${BOLD}${BLUE}╚══════════════════════════════════════════════════╝${NC}"
echo ""

START_TIME=$(date +%s)

# Step 1: Build compiler/runtime
echo -e "${BOLD}${BLUE}[Step 1/2] Compiler & Runtime${NC}"
echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

COMPILER_FLAGS=""
[ "$BUILD_RELEASE" = true ] && COMPILER_FLAGS="$COMPILER_FLAGS --release"
[ "$TEST_COMPILER" = false ] && COMPILER_FLAGS="$COMPILER_FLAGS --no-tests"
[ "$VERBOSE" = true ] && COMPILER_FLAGS="$COMPILER_FLAGS --verbose"

./scripts/build-compiler.sh $COMPILER_FLAGS
COMPILER_EXIT=$?

echo ""

# Step 2: Test stdlib (if compiler succeeded and tests enabled)
if [ $COMPILER_EXIT -eq 0 ] && [ "$TEST_STDLIB" = true ]; then
    echo -e "${BOLD}${BLUE}[Step 2/2] Standard Library Tests${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    STDLIB_FLAGS=""
    [ "$VERBOSE" = true ] && STDLIB_FLAGS="$STDLIB_FLAGS --verbose"

    ./scripts/test-stdlib.sh $STDLIB_FLAGS
    STDLIB_EXIT=$?
else
    STDLIB_EXIT=0
    if [ "$TEST_STDLIB" = false ]; then
        echo -e "${YELLOW}[Step 2/2] Stdlib tests skipped (--no-stdlib-tests)${NC}"
    else
        echo -e "${RED}[Step 2/2] Stdlib tests skipped (compiler build failed)${NC}"
    fi
    echo ""
fi

# Calculate total time
END_TIME=$(date +%s)
TOTAL_TIME=$((END_TIME - START_TIME))

# Print final summary
echo -e "${BOLD}${BLUE}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║              Build Summary                       ║${NC}"
echo -e "${BOLD}${BLUE}╚══════════════════════════════════════════════════╝${NC}"
echo ""

# Compiler status
if [ $COMPILER_EXIT -eq 0 ]; then
    echo -e "  Compiler:       ${GREEN}✓ Success${NC}"
else
    echo -e "  Compiler:       ${RED}✗ Failed${NC}"
fi

# Stdlib status
if [ "$TEST_STDLIB" = true ]; then
    if [ $STDLIB_EXIT -eq 0 ]; then
        echo -e "  Stdlib Tests:   ${GREEN}✓ Success${NC}"
    else
        echo -e "  Stdlib Tests:   ${RED}✗ Failed${NC}"
    fi
else
    echo -e "  Stdlib Tests:   ${YELLOW}Skipped${NC}"
fi

# Total time
echo -e "  Total Time:     ${TOTAL_TIME}s"
echo ""

# Exit with failure if any component failed
if [ $COMPILER_EXIT -ne 0 ] || [ $STDLIB_EXIT -ne 0 ]; then
    echo -e "${BOLD}${RED}✗ Build failed${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    exit 1
else
    echo -e "${BOLD}${GREEN}✓ All builds and tests passed!${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    exit 0
fi
