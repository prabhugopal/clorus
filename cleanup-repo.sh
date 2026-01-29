#!/bin/bash
# Clorus Repository Cleanup Script
# Organizes test files and documentation into proper directories

set -e

echo "🧹 Cleaning up Clorus repository..."

# Create target directories if they don't exist
mkdir -p tests/polymorphism
mkdir -p tests/loops
mkdir -p tests/concurrency
mkdir -p docs/implementation

# Move polymorphism tests
echo "📦 Moving polymorphism tests..."
mv -f test-records.clr tests/polymorphism/ 2>/dev/null || true
mv -f test-record-*.clr tests/polymorphism/ 2>/dev/null || true
mv -f test-records-comprehensive.clr tests/polymorphism/ 2>/dev/null || true
mv -f test-protocol-*.clr tests/polymorphism/ 2>/dev/null || true
mv -f test-protocols-comprehensive.clr tests/polymorphism/ 2>/dev/null || true
mv -f test-multimethod-*.clr tests/polymorphism/ 2>/dev/null || true

# Move loop tests
echo "📦 Moving loop tests..."
mv -f test-while.clr tests/loops/ 2>/dev/null || true
mv -f test-simple-while.clr tests/loops/ 2>/dev/null || true
mv -f test-dotimes.clr tests/loops/ 2>/dev/null || true
mv -f test-doseq.clr tests/loops/ 2>/dev/null || true

# Move concurrency tests
echo "📦 Moving concurrency tests..."
mv -f test-atoms.clr tests/concurrency/ 2>/dev/null || true
mv -f test-ref-*.clr tests/concurrency/ 2>/dev/null || true
mv -f test-go-closure.clr tests/concurrency/ 2>/dev/null || true

# Move string tests
echo "📦 Moving string tests..."
mv -f test-simple-string.clr tests/strings/ 2>/dev/null || true
mv -f test-string*.clr tests/strings/ 2>/dev/null || true

# Move collection tests
echo "📦 Moving collection tests..."
mv -f test-collections.clr tests/collections/ 2>/dev/null || true

# Move implementation documentation
echo "📝 Moving implementation docs..."
mv -f RECORDS_IMPLEMENTATION.md docs/implementation/ 2>/dev/null || true
mv -f PROTOCOLS_IMPLEMENTATION.md docs/implementation/ 2>/dev/null || true
mv -f MULTIMETHODS_IMPLEMENTATION.md docs/implementation/ 2>/dev/null || true
mv -f CSP_IMPLEMENTATION.md docs/implementation/ 2>/dev/null || true
mv -f LOOP_AND_ATOMS_COMPLETE.md docs/implementation/ 2>/dev/null || true
mv -f COVERAGE_ASSESSMENT.md docs/ 2>/dev/null || true

# Move any remaining test files to tests/misc
echo "📦 Moving remaining tests..."
mkdir -p tests/misc
mv -f test-*.clr tests/misc/ 2>/dev/null || true

echo "✅ Cleanup complete!"
echo ""
echo "📊 Summary:"
echo "  - tests/polymorphism/: $(ls tests/polymorphism/*.clr 2>/dev/null | wc -l | tr -d ' ') files"
echo "  - tests/loops/: $(ls tests/loops/*.clr 2>/dev/null | wc -l | tr -d ' ') files"
echo "  - tests/concurrency/: $(ls tests/concurrency/*.clr 2>/dev/null | wc -l | tr -d ' ') files"
echo "  - tests/strings/: $(ls tests/strings/*.clr 2>/dev/null | wc -l | tr -d ' ') files"
echo "  - docs/implementation/: $(ls docs/implementation/*.md 2>/dev/null | wc -l | tr -d ' ') files"
echo ""
echo "🎉 Repository is now organized!"
