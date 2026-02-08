#!/bin/bash
# Comprehensive test metrics and dashboard generator
# Runs tests, generates metrics, and creates HTML dashboard

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║                                                ║"
echo "║     🧪 CLORUS TEST METRICS GENERATOR 🧪       ║"
echo "║                                                ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Step 1: Run tests with metrics
echo -e "${BLUE}Step 1: Running test suite and collecting metrics...${NC}"
echo ""
bash tests/generate_metrics.sh

echo ""
echo -e "${BLUE}Step 2: Generating HTML dashboard...${NC}"
echo ""

# Step 2: Generate HTML dashboard
if [ -f "test-metrics/latest.json" ]; then
    python3 tests/generate_dashboard.py test-metrics/latest.json test-metrics/dashboard.html
else
    echo "Error: Metrics file not found"
    exit 1
fi

echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║                                                ║"
echo "║            ✅ METRICS COMPLETE ✅             ║"
echo "║                                                ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Display quick summary
echo -e "${CYAN}Quick Access:${NC}"
echo ""
echo "  📊 HTML Dashboard:"
echo "     file://$(pwd)/test-metrics/dashboard.html"
echo ""
echo "  📁 Metrics JSON:"
echo "     $(pwd)/test-metrics/latest.json"
echo ""
echo "  🔍 Open dashboard in browser:"
echo "     open test-metrics/dashboard.html  (macOS)"
echo "     xdg-open test-metrics/dashboard.html  (Linux)"
echo ""

# Offer to open dashboard
if [[ "$OSTYPE" == "darwin"* ]]; then
    read -p "Open dashboard in browser now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        open test-metrics/dashboard.html
        echo -e "${GREEN}✓ Dashboard opened in browser${NC}"
    fi
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    read -p "Open dashboard in browser now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        xdg-open test-metrics/dashboard.html
        echo -e "${GREEN}✓ Dashboard opened in browser${NC}"
    fi
fi

echo ""
