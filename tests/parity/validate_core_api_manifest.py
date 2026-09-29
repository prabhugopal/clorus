#!/usr/bin/env python3
"""Validate and summarize the portable-core beta manifest."""

import json
import sys
from collections import Counter
from pathlib import Path


MANIFEST = Path(__file__).with_name("core_api_manifest.json")
VALID_OWNERS = {"native-kernel", "source-core", "source-transducers", "sequence-runtime"}
VALID_STATUSES = {"implemented", "partial", "planned"}


def fail(message: str) -> int:
    print(f"ERROR: {message}")
    return 1


def main() -> int:
    try:
        manifest = json.loads(MANIFEST.read_text())
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"cannot read {MANIFEST}: {error}")

    if manifest.get("schema_version") != 1:
        return fail("unsupported or missing schema_version")

    entries = manifest.get("apis")
    if not isinstance(entries, list) or not entries:
        return fail("apis must be a non-empty list")

    names = set()
    counts = Counter()
    for entry in entries:
        if not isinstance(entry, dict):
            return fail("every api entry must be an object")
        name = entry.get("name")
        owner = entry.get("owner")
        status = entry.get("status")
        coverage = entry.get("coverage")
        if not isinstance(name, str) or not name:
            return fail("every api entry needs a non-empty name")
        if name in names:
            return fail(f"duplicate API entry: {name}")
        names.add(name)
        if owner not in VALID_OWNERS:
            return fail(f"{name}: invalid owner {owner!r}")
        if status not in VALID_STATUSES:
            return fail(f"{name}: invalid status {status!r}")
        if not isinstance(coverage, str) or not coverage:
            return fail(f"{name}: coverage must be documented")
        if status == "planned" and not entry.get("gap"):
            return fail(f"{name}: planned APIs must state their blocking gap")
        counts[status] += 1

    print("Portable core beta manifest")
    print(f"  total:       {len(entries)}")
    print(f"  implemented: {counts['implemented']}")
    print(f"  partial:     {counts['partial']}")
    print(f"  planned:     {counts['planned']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
