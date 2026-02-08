#!/usr/bin/env python3
"""
Update FEATURE_COVERAGE.md with live test results
Matches test files to feature matrix and updates status
"""

import os
import json
import re
from pathlib import Path

# Feature categories and their test mappings
FEATURE_TEST_MAP = {
    # Primitives & Literals
    "Long integers": ["lang/types/numbers", "lang/arithmetic/"],
    "Doubles": ["lang/types/numbers", "lang/arithmetic/"],
    "Strings": ["strings/string-operations-test"],
    "Keywords": ["lang/types/keywords"],
    "Booleans": ["lang/types/booleans"],
    "Nil": ["lang/types/nil"],
    "Vectors": ["collections/vectors", "collections/api-test"],
    "Lists": ["collections/lists", "collections/api-test"],
    "Maps": ["collections/maps", "collections/api-test", "collections/nested-map-ops"],
    "Sets": ["collections/sets"],

    # Arithmetic Operators
    "+": ["lang/arithmetic/addition"],
    "-": ["lang/arithmetic/subtraction"],
    "*": ["lang/arithmetic/multiplication"],
    "/": ["lang/arithmetic/division"],
    "mod": ["lang/arithmetic/modulo", "lang/arithmetic/comparison"],
    "<": ["lang/arithmetic/comparison"],
    ">": ["lang/arithmetic/comparison"],
    "<=": ["lang/arithmetic/comparison"],
    ">=": ["lang/arithmetic/comparison"],
    "=": ["lang/arithmetic/comparison"],

    # Control Flow
    "if": ["lang/control/if"],
    "do": ["lang/control/do"],
    "let": ["core/destructuring-test"],
    "loop/recur": ["core/loop-recur-test"],
    "when": ["macros/control-flow-macros"],
    "when-not": ["macros/control-flow-macros"],
    "cond": ["macros/control-flow-macros"],
    "case": ["lang/control/case"],

    # Functions
    "defn": ["lang/core/functions"],
    "fn": ["lang/core/functions"],
    "Multi-arity": ["core/multi-arity-test"],
    "Variadic": ["core/variadic-test"],
    "Closures": ["lang/core/closures"],
    "Recursion": ["core/loop-recur-test", "lang/core/recursion"],

    # Collections
    "range": ["stdlib/sequences/range-test"],
    "map": ["stdlib/core/map"],
    "filter": ["stdlib/core/filter"],
    "reduce": ["stdlib/core/reduce"],
    "apply": ["stdlib/core/apply"],
    "take": ["stdlib/sequences"],
    "drop": ["stdlib/sequences"],

    # Type predicates
    "seq?": ["types/type-predicates"],
    "coll?": ["types/type-predicates"],
    "fn?": ["types/type-predicates"],

    # Concurrency
    "atom": ["atoms/atoms-test"],
    "deref/@": ["atoms/atoms-test"],
    "reset!": ["atoms/atoms-test"],
    "swap!": ["atoms/atoms-test"],
    "ref": ["refs/stm"],
    "dosync": ["refs/stm"],
    "alter": ["refs/stm"],
    "commute": ["refs/stm"],

    # Macros
    "quote '": ["macros/quote-test"],
    "syntax-quote `": ["macros/syntax-quote-test"],
    "unquote ~": ["macros/unquote-test"],
    "defmacro": ["macros/defmacro-test"],

    # Exceptions
    "try": ["exceptions/try-catch-test"],
    "catch": ["exceptions/try-catch-test"],
    "finally": ["exceptions/try-catch-test"],
    "throw": ["exceptions/try-catch-test"],
}

def find_test_files(test_dir):
    """Find all test files in the test directory"""
    test_files = []
    for root, dirs, files in os.walk(test_dir):
        for file in files:
            if file.endswith('.clr') or file.endswith('.clrs'):
                rel_path = os.path.relpath(os.path.join(root, file), test_dir)
                test_files.append(rel_path)
    return test_files

def check_feature_tested(feature, test_files):
    """Check if a feature has corresponding test files"""
    if feature not in FEATURE_TEST_MAP:
        return False, []

    patterns = FEATURE_TEST_MAP[feature]
    matching_tests = []

    for pattern in patterns:
        for test_file in test_files:
            if pattern in test_file:
                matching_tests.append(test_file)

    return len(matching_tests) > 0, matching_tests

def generate_coverage_badge(percentage):
    """Generate a coverage badge with color coding"""
    if percentage >= 90:
        return f"![Coverage](https://img.shields.io/badge/coverage-{percentage:.0f}%25-brightgreen)"
    elif percentage >= 70:
        return f"![Coverage](https://img.shields.io/badge/coverage-{percentage:.0f}%25-green)"
    elif percentage >= 50:
        return f"![Coverage](https://img.shields.io/badge/coverage-{percentage:.0f}%25-yellow)"
    else:
        return f"![Coverage](https://img.shields.io/badge/coverage-{percentage:.0f}%25-red)"

def update_coverage_report(test_dir, metrics_file, output_file):
    """Generate updated coverage report"""

    # Find all test files
    test_files = find_test_files(test_dir)
    print(f"Found {len(test_files)} test files")

    # Load metrics if available
    metrics = None
    if os.path.exists(metrics_file):
        with open(metrics_file, 'r') as f:
            metrics = json.load(f)

    # Generate report
    report = []
    report.append("# Clorus Test Coverage Report")
    report.append("")
    report.append(f"**Generated:** {metrics['timestamp'] if metrics else 'N/A'}")
    report.append("")

    if metrics:
        summary = metrics['summary']
        pass_rate = summary['pass_rate']
        report.append(f"## Overall Status")
        report.append("")
        report.append(f"- **Total Tests:** {summary['total_tests']}")
        report.append(f"- **Passed:** {summary['passed']} ✅")
        report.append(f"- **Failed:** {summary['failed']} ❌")
        report.append(f"- **Pass Rate:** {pass_rate:.1f}% {generate_coverage_badge(pass_rate)}")
        report.append("")

        # Category breakdown
        report.append("## Category Breakdown")
        report.append("")
        report.append("| Category | Tests | Passed | Failed | Pass Rate |")
        report.append("|----------|-------|--------|--------|-----------|")

        for category, data in sorted(metrics['categories'].items()):
            total = data['total']
            passed = data['passed']
            failed = data['failed']
            rate = data['pass_rate']

            # Status emoji
            if rate >= 90:
                emoji = "🟢"
            elif rate >= 70:
                emoji = "🟡"
            else:
                emoji = "🔴"

            report.append(f"| {category.title()} | {total} | {passed} | {failed} | {rate:.1f}% {emoji} |")

        report.append("")

    # Feature test mapping
    report.append("## Feature Test Coverage")
    report.append("")
    report.append("Features mapped to their test files:")
    report.append("")

    for feature, patterns in sorted(FEATURE_TEST_MAP.items()):
        tested, matching = check_feature_tested(feature, test_files)
        status = "✅" if tested else "❌"

        report.append(f"### {status} {feature}")
        if matching:
            report.append("")
            for test in matching:
                report.append(f"- `tests/{test}`")
        else:
            report.append("")
            report.append("*No test files found*")
        report.append("")

    # Write report
    with open(output_file, 'w') as f:
        f.write('\n'.join(report))

    print(f"✓ Coverage report generated: {output_file}")

if __name__ == '__main__':
    test_dir = 'tests'
    metrics_file = 'test-metrics/latest.json'
    output_file = 'test-metrics/COVERAGE_REPORT.md'

    update_coverage_report(test_dir, metrics_file, output_file)
