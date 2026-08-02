#!/usr/bin/env python3
"""
Generate canonical status docs from a code/test-backed inventory.

This script does not read prior parity Markdown as input. Instead it uses a
curated feature inventory with direct source/test references, verifies those
references exist, and renders generated status pages for MkDocs.
"""

from __future__ import annotations

import json
from collections import Counter, defaultdict
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[2]
DOCS_DIR = ROOT / "docs"
GENERATED_DIR = DOCS_DIR / "generated"
TESTS_DIR = ROOT / "tests"


STATUS_ORDER = ["implemented", "partial", "missing", "out_of_scope"]
STATUS_LABELS = {
    "implemented": "Implemented",
    "partial": "Partial",
    "missing": "Missing",
    "out_of_scope": "Out of Scope",
}
STATUS_ICONS = {
    "implemented": "✅",
    "partial": "🟡",
    "missing": "❌",
    "out_of_scope": "⛔",
}


@dataclass(frozen=True)
class Feature:
    area: str
    name: str
    status: str
    summary: str
    code_refs: tuple[str, ...]
    test_refs: tuple[str, ...]
    notes: str = ""


FEATURES: tuple[Feature, ...] = (
    Feature(
        area="Reader & Syntax",
        name="Reader macros and quoting forms",
        status="implemented",
        summary="Quote, syntax-quote, unquote, deref, var-quote, discard, and regex literal lowering are implemented.",
        code_refs=(
            "crates/clorus-syntax/src/parser.rs",
            "crates/clorus-syntax/src/macros.rs",
        ),
        test_refs=(
            "tests/language/test-reader-discard.clr",
            "tests/language/test-reader-discard-edge.clr",
            "tests/language/test-reader-macro-forms.clr",
            "tests/language/test-reader-regex-literal.clr",
        ),
    ),
    Feature(
        area="Reader & Syntax",
        name="Regex literal runtime APIs",
        status="implemented",
        summary="Reader lowering and stdlib/runtime regex helpers are present.",
        code_refs=(
            "crates/clorus-runtime/src/string.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=("tests/stdlib/test-core-regex.clr",),
    ),
    Feature(
        area="Reader & Syntax",
        name="Characters and ratio literals",
        status="missing",
        summary="Character and ratio support is not evidenced in parser/runtime.",
        code_refs=(),
        test_refs=(),
        notes="These remain feature gaps for closer Clojure parity.",
    ),
    Feature(
        area="Functions & Evaluation",
        name="Functions, closures, multi-arity, variadics",
        status="implemented",
        summary="Core function forms, closure capture, and variadic dispatch are implemented and tested.",
        code_refs=(
            "crates/clorus-syntax/src/ast.rs",
            "crates/clorus-codegen/src/codegen/mod.rs",
        ),
        test_refs=(
            "tests/language/test-simple-closure.clr",
            "tests/language/test-multi-arity-hof.clr",
            "tests/language/test-fixed-vs-variadic-dispatch.clr",
            "tests/language/test-variadic-rest.clr",
        ),
    ),
    Feature(
        area="Functions & Evaluation",
        name="Tail-position recur in functions and loops",
        status="implemented",
        summary="Loop/recur and fn-tail recur paths are covered.",
        code_refs=("crates/clorus-codegen/src/codegen/mod.rs",),
        test_refs=(
            "tests/language/test-recur-fn-tail.clr",
            "tests/stdlib/test-loop-recur.clr",
        ),
    ),
    Feature(
        area="Macros & Metadata",
        name="User macros and macroexpansion",
        status="implemented",
        summary="defmacro, macroexpand, &env, and &form baseline behavior is present.",
        code_refs=(
            "crates/clorus-syntax/src/parser.rs",
            "crates/clorus-syntax/src/macros.rs",
        ),
        test_refs=(
            "tests/language/test-macro-env-form.clr",
            "tests/language/test-macroexpand.clr",
            "tests/language/test-gensym.clr",
        ),
    ),
    Feature(
        area="Macros & Metadata",
        name="Macro hygiene edge cases",
        status="partial",
        summary="Auto-gensym baseline exists, but deeper hygiene semantics are still treated as partial.",
        code_refs=(
            "crates/clorus-syntax/src/macros.rs",
            "crates/clorus-syntax/src/parser.rs",
        ),
        test_refs=("tests/language/test-macro-hygiene.clr",),
        notes="Practical baseline is in place; full Clojure macro hygiene remains broader.",
    ),
    Feature(
        area="Macros & Metadata",
        name="Metadata primitives and var metadata",
        status="implemented",
        summary="meta, with-meta, vary-meta, def/defn metadata, and merge order behavior are covered.",
        code_refs=(
            "crates/clorus-syntax/src/parser.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(
            "tests/language/test-meta-contract.clr",
            "tests/language/test-with-meta-vary-meta.clr",
            "tests/language/test-var-metadata.clr",
            "tests/language/test-metadata-merge-order.clr",
        ),
    ),
    Feature(
        area="Namespaces",
        name="Require/refer/rename baseline",
        status="implemented",
        summary="Namespace resolution baseline, including refer/rename matrices and conflict detection, is covered.",
        code_refs=(
            "crates/clorus-cli/src/commands.rs",
            "crates/clorus-cli/src/manifest.rs",
        ),
        test_refs=(
            "tests/language/test-require-refer-rename.clr",
            "tests/language/test-namespace-refer-rename-matrix.clr",
            "tests/language/test-namespace-resolution-edge.clr",
            "tests/compiler/test-namespace-alias-conflict.clr",
        ),
    ),
    Feature(
        area="Namespaces",
        name="Namespace ergonomics long-tail",
        status="partial",
        summary="Baseline works, but alias/import edge semantics are still tracked as partial.",
        code_refs=(
            "crates/clorus-cli/src/commands.rs",
            "crates/clorus-cli/src/manifest.rs",
        ),
        test_refs=(
            "tests/compiler/test-namespace-import-conflict.clr",
            "tests/compiler/test-namespace-refer-conflict.clr",
            "tests/compiler/test-namespace-rename-conflict.clr",
        ),
    ),
    Feature(
        area="Collections & Seqs",
        name="Core collections and nested map ops",
        status="implemented",
        summary="Vectors, maps, sets, nested get/assoc/update helpers, and many long-tail map utilities are covered.",
        code_refs=(
            "crates/clorus-runtime/src/value.rs",
            "crates/clorus-runtime/src/collections/map_ops.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(
            "tests/language/test-set.clr",
            "tests/language/test-nested-map-ops-parity.clr",
            "tests/stdlib/test-core-map-utils.clr",
            "tests/stdlib/test-core-nested-access-semantics.clr",
        ),
    ),
    Feature(
        area="Collections & Seqs",
        name="Ordering, sort, compare, mapv",
        status="implemented",
        summary="Ordering semantics, sort/sort-by, and mapv are covered in stdlib tests.",
        code_refs=(
            "stdlib/clorus/core.clr",
            "crates/clorus-runtime/src/value.rs",
        ),
        test_refs=(
            "tests/stdlib/test-core-ordering.clr",
            "tests/stdlib/test-core-sort-behavior.clr",
            "tests/stdlib/test-core-mapv.clr",
        ),
    ),
    Feature(
        area="Collections & Seqs",
        name="Persistent collection internals",
        status="partial",
        summary="Collection semantics are usable, but internals are not yet a full persistent HAMT/vector story.",
        code_refs=(
            "crates/clorus-runtime/src/collections/map_ops.rs",
            "crates/clorus-runtime/src/value.rs",
        ),
        test_refs=(),
        notes="Usable parity exists at API level for many cases; data-structure implementation depth is still evolving.",
    ),
    Feature(
        area="Exceptions & Dynamic Vars",
        name="throw, try/catch, ex-info, ex-data",
        status="implemented",
        summary="Throw/catch syntax and exception data propagation are covered.",
        code_refs=(
            "crates/clorus-syntax/src/parser.rs",
            "crates/clorus-runtime/src/value.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(
            "tests/language/test-try-catch-typed.clr",
            "tests/language/test-ex-info.clr",
            "tests/language/test-exception-data-propagation.clr",
        ),
    ),
    Feature(
        area="Exceptions & Dynamic Vars",
        name="Dynamic vars, binding, set!",
        status="implemented",
        summary="Dynamic var binding, restoration, and throw-path coverage are present.",
        code_refs=(
            "crates/clorus-runtime/src/var.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(
            "tests/language/test-binding-edge.clr",
            "tests/language/test-dynamic-var-restoration.clr",
            "tests/language/test-dynamic-var-throw-paths.clr",
            "tests/language/test-set-bang.clr",
        ),
    ),
    Feature(
        area="Exceptions & Dynamic Vars",
        name="Deep STM-grade ref semantics",
        status="partial",
        summary="Refs/dosync/alter surface exists, but full retry semantics and production confidence are not complete.",
        code_refs=(
            "crates/clorus-runtime/src/ref_type.rs",
            "crates/clorus-runtime/src/transaction.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=("tests/language/test-alter-reset-meta.clr",),
        notes="This is one of the remaining bigger parity gaps.",
    ),
    Feature(
        area="Polymorphism",
        name="Protocols, satisfies?, introspection",
        status="implemented",
        summary="Protocol definition, dispatch, and introspection helpers are implemented and tested.",
        code_refs=(
            "crates/clorus-syntax/src/parser.rs",
            "crates/clorus-runtime/src/protocols.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(
            "tests/language/test-protocol-introspection.clr",
            "tests/language/test-satisfies.clr",
        ),
    ),
    Feature(
        area="Polymorphism",
        name="Multimethods and hierarchy",
        status="implemented",
        summary="defmulti/defmethod, hierarchy, and multimethod administration/introspection are covered.",
        code_refs=(
            "crates/clorus-syntax/src/parser.rs",
            "crates/clorus-runtime/src/hierarchy.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(
            "tests/language/test-multimethod-dispatch.clr",
            "tests/language/test-multimethod-hierarchy.clr",
            "tests/language/test-multimethod-admin.clr",
            "tests/language/test-hierarchy-queries.clr",
        ),
    ),
    Feature(
        area="Interop & Tooling",
        name="Rust interop and package workflow",
        status="implemented",
        summary="Rust interop generation and package workflows are implemented and exercised through build paths.",
        code_refs=(
            "crates/clorus-ffi-gen/src/main.rs",
            "crates/clorus-cli/src/modern_ffi.rs",
            "crates/clorus-cli/src/commands.rs",
        ),
        test_refs=(
            "tests/integration/basic.clr",
            "tests/integration/stdlib-minimal.clr",
        ),
        notes="Interop breadth still depends on interface coverage and bridge crates.",
    ),
    Feature(
        area="Interop & Tooling",
        name="REPL/run/build dual-engine execution",
        status="implemented",
        summary="JIT and legacy execution paths both exist and are exercised by the test runner.",
        code_refs=(
            "crates/clorus-cli/src/commands.rs",
            "crates/clorus-repl/src/lib.rs",
        ),
        test_refs=(
            "tests/run_all_tests.sh",
            "tests/integration/binding.clr",
        ),
    ),
    Feature(
        area="Concurrency",
        name="Atoms, refs, channels baseline",
        status="partial",
        summary="Concurrency primitives exist, but parity and hardening for full Clojure semantics remain open.",
        code_refs=(
            "crates/clorus-runtime/src/atom.rs",
            "crates/clorus-runtime/src/channel.rs",
            "crates/clorus-runtime/src/ref_type.rs",
            "crates/clorus-runtime/src/go_block.rs",
            "stdlib/clorus/core.clr",
        ),
        test_refs=(),
        notes="This is the main gap for Brave Clojure-style concurrency chapter parity.",
    ),
    Feature(
        area="Concurrency",
        name="core.async-style go/alts/parking semantics",
        status="missing",
        summary="No complete, code-evidenced core.async-compatible runtime model is present yet.",
        code_refs=(),
        test_refs=(),
        notes="Channels/go-style pieces exist, but not the full API/semantic contract.",
    ),
    Feature(
        area="Host Scope",
        name="JVM / Java interop",
        status="out_of_scope",
        summary="Java host interop is not a target for Clorus.",
        code_refs=(),
        test_refs=(),
    ),
)


def file_count(path: Path, pattern: str) -> int:
    return len(list(path.glob(pattern)))


def normalize_ref(ref: str) -> str:
    return ref.replace("\\", "/")


def ref_exists(ref: str) -> bool:
    return (ROOT / ref).exists()


def render_ref_list(refs: Iterable[str]) -> str:
    refs = list(refs)
    if not refs:
        return "-"
    return "<br>".join(f"`{normalize_ref(ref)}`" for ref in refs)


def test_inventory() -> dict[str, int]:
    return {
        "language": file_count(TESTS_DIR / "language", "*.clr"),
        "stdlib": file_count(TESTS_DIR / "stdlib", "*.clr"),
        "integration": file_count(TESTS_DIR / "integration", "*.clr"),
        "compiler": file_count(TESTS_DIR / "compiler", "*.clr"),
        "parity_harness": len(list((TESTS_DIR / "parity").glob("*"))),
    }


def validate_inventory(features: Iterable[Feature]) -> list[str]:
    errors: list[str] = []
    for feature in features:
        for ref in feature.code_refs + feature.test_refs:
            if not ref_exists(ref):
                errors.append(f"{feature.name}: missing reference `{ref}`")
    return errors


def build_status_json(features: Iterable[Feature]) -> dict:
    inventory = []
    for feature in features:
        inventory.append(
            {
                **asdict(feature),
                "code_refs_present": all(ref_exists(ref) for ref in feature.code_refs),
                "test_refs_present": all(ref_exists(ref) for ref in feature.test_refs),
            }
        )

    by_area = defaultdict(Counter)
    overall = Counter()
    for feature in features:
        by_area[feature.area][feature.status] += 1
        overall[feature.status] += 1

    return {
        "generated_from": "scripts/docs/build_status.py",
        "features": inventory,
        "summary": {
            "overall": dict(overall),
            "by_area": {area: dict(counter) for area, counter in by_area.items()},
            "test_inventory": test_inventory(),
        },
    }


def render_parity_markdown(features: Iterable[Feature]) -> str:
    features = list(features)
    overall = Counter(feature.status for feature in features)
    area_buckets: dict[str, list[Feature]] = defaultdict(list)
    for feature in features:
        area_buckets[feature.area].append(feature)

    lines = [
        "# Generated Parity Status",
        "",
        "> Generated by `scripts/docs/build_status.py`.",
        "> This page is derived from a curated code/test inventory, not from older parity prose docs.",
        "",
        "## Evidence Rules",
        "",
        "- A feature entry points to concrete source files and tests when available.",
        "- `Implemented` means the feature has code evidence and direct tests in the repo.",
        "- `Partial` means code exists, but semantics, breadth, or hardening are still incomplete.",
        "- `Missing` means the code/test inventory does not support claiming the feature today.",
        "- `Out of Scope` means intentionally not targeted (for example JVM interop).",
        "",
        "## Snapshot",
        "",
        f"- `{STATUS_ICONS['implemented']}` Implemented: {overall['implemented']}",
        f"- `{STATUS_ICONS['partial']}` Partial: {overall['partial']}",
        f"- `{STATUS_ICONS['missing']}` Missing: {overall['missing']}",
        f"- `{STATUS_ICONS['out_of_scope']}` Out of Scope: {overall['out_of_scope']}",
        "",
    ]

    for area in sorted(area_buckets):
        lines.extend(
            [
                f"## {area}",
                "",
                "| Feature | Status | Summary | Code Evidence | Test Evidence | Notes |",
                "|---|---|---|---|---|---|",
            ]
        )
        for feature in area_buckets[area]:
            lines.append(
                "| {name} | {status} | {summary} | {code} | {tests} | {notes} |".format(
                    name=feature.name,
                    status=f"{STATUS_ICONS[feature.status]} {STATUS_LABELS[feature.status]}",
                    summary=feature.summary,
                    code=render_ref_list(feature.code_refs),
                    tests=render_ref_list(feature.test_refs),
                    notes=feature.notes or "-",
                )
            )
        lines.append("")

    lines.extend(
        [
            "## Immediate Gaps",
            "",
            "- Full `core.async`-style API and parking semantics",
            "- Stronger STM / `dosync` retry semantics",
            "- Character and ratio literals",
            "- Remaining namespace and macro long-tail edge cases",
            "- Production-grade concurrency hardening",
            "",
        ]
    )
    return "\n".join(lines)


def render_coverage_markdown(features: Iterable[Feature]) -> str:
    features = list(features)
    tests = test_inventory()
    by_area = defaultdict(Counter)
    for feature in features:
        by_area[feature.area][feature.status] += 1

    lines = [
        "# Generated Coverage Summary",
        "",
        "> Generated by `scripts/docs/build_status.py`.",
        "> This is the canonical coverage/status entrypoint. Manual status docs should point here.",
        "",
        "## Test Inventory",
        "",
        "| Suite | File Count |",
        "|---|---:|",
        f"| `tests/language` | {tests['language']} |",
        f"| `tests/stdlib` | {tests['stdlib']} |",
        f"| `tests/integration` | {tests['integration']} |",
        f"| `tests/compiler` | {tests['compiler']} |",
        f"| `tests/parity` harness files | {tests['parity_harness']} |",
        "",
        "## Feature Status by Area",
        "",
        "| Area | Implemented | Partial | Missing | Out of Scope |",
        "|---|---:|---:|---:|---:|",
    ]

    for area in sorted(by_area):
        counts = by_area[area]
        lines.append(
            f"| {area} | {counts['implemented']} | {counts['partial']} | {counts['missing']} | {counts['out_of_scope']} |"
        )

    lines.extend(
        [
            "",
            "## Policy",
            "",
            "- New language/runtime work should add direct tests under `tests/language`, `tests/stdlib`, `tests/integration`, or `tests/compiler`.",
            "- Status pages should be regenerated after meaningful parity or stdlib work.",
            "- Historical docs can remain for context, but status claims should resolve to generated pages.",
            "",
            "## Generation",
            "",
            "```bash",
            "python3 scripts/docs/build_status.py",
            "```",
            "",
        ]
    )

    return "\n".join(lines)


def write_file(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content + "\n", encoding="utf-8")


def main() -> int:
    errors = validate_inventory(FEATURES)
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    GENERATED_DIR.mkdir(parents=True, exist_ok=True)
    status_json = build_status_json(FEATURES)

    write_file(GENERATED_DIR / "PARITY_STATUS.md", render_parity_markdown(FEATURES))
    write_file(GENERATED_DIR / "COVERAGE_SUMMARY.md", render_coverage_markdown(FEATURES))
    write_file(
        GENERATED_DIR / "STATUS.json",
        json.dumps(status_json, indent=2, sort_keys=True),
    )
    print(f"Wrote generated status docs to {GENERATED_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
