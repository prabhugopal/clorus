#!/usr/bin/env python3
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CASES_FILE = Path(__file__).resolve().parent / "core_cases.json"
CLORUS_BIN = os.environ.get("CLORUS_BIN", str(ROOT / "target" / "debug" / "clorus"))


def run_cmd(cmd, env=None):
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    return p.returncode, p.stdout, p.stderr


def clj_eval(expr: str):
    with tempfile.NamedTemporaryFile("w", suffix=".clj", delete=False) as f:
        f.write(f"(println (pr-str {expr}))\n")
        path = f.name
    try:
        code, out, err = run_cmd(["clj", "-M", path])
        if code != 0:
            return False, f"clj failed: {err.strip() or out.strip()}"
        return True, out.strip().splitlines()[-1] if out.strip() else ""
    finally:
        Path(path).unlink(missing_ok=True)


def clorus_eval(expr: str, expected_literal: str):
    wrapped_expr = f"(= {expr} {expected_literal})"
    with tempfile.NamedTemporaryFile("w", suffix=".clr", delete=False) as f:
        f.write(wrapped_expr + "\n")
        path = f.name
    try:
        env = os.environ.copy()
        env["CLORUS_ENTRY_FILE"] = path
        code, out, err = run_cmd([CLORUS_BIN, "run"], env=env)
        if code != 0:
            msg = (out + "\n" + err).strip()
            return False, f"clorus failed: {msg}"
        combined = (out + "\n" + err).splitlines()
        value = None
        for line in reversed(combined):
            m = re.match(r"^\s*=>\s*(.*)$", line)
            if m:
                value = m.group(1).strip()
                break
        if value is None:
            return False, f"clorus output missing result marker. output={out!r} err={err!r}"
        return True, value
    finally:
        Path(path).unlink(missing_ok=True)


def main():
    if not Path(CLORUS_BIN).exists():
        print(f"ERROR: CLORUS_BIN not found: {CLORUS_BIN}")
        return 2
    if not CASES_FILE.exists():
        print(f"ERROR: cases file not found: {CASES_FILE}")
        return 2

    with CASES_FILE.open() as f:
        cases = json.load(f)

    print("====================================")
    print("  CLOJURE CORE PARITY (SMOKE SET)")
    print("====================================")
    print(f"clorus: {CLORUS_BIN}")
    print(f"cases:  {CASES_FILE}")
    print("")

    passed = 0
    failed = 0

    for case in cases:
        name = case["name"]
        expr = case["expr"]

        ok_clj, clj_val = clj_eval(expr)
        if not ok_clj:
            print(f"[FAIL] {name}: {clj_val}")
            failed += 1
            continue

        ok_clorus, clorus_val = clorus_eval(expr, clj_val)
        if not ok_clorus:
            print(f"[FAIL] {name}: {clorus_val}")
            failed += 1
            continue

        if clorus_val == "true":
            print(f"[PASS] {name}")
            passed += 1
        else:
            print(f"[FAIL] {name}")
            print(f"  expr:   {expr}")
            print(f"  expect: {clj_val}")
            print(f"  clorus: (= expr expect) => {clorus_val}")
            failed += 1

    print("")
    print("-----------")
    print(f"passed: {passed}")
    print(f"failed: {failed}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
