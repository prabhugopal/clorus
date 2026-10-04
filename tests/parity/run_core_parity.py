#!/usr/bin/env python3
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CASES_FILE = Path(__file__).resolve().parent / "core_cases.json"
CLORUS_BIN = os.environ.get("CLORUS_BIN", str(ROOT / "target" / "debug" / "clorus"))


def reference_clojure_command():
    """Find a supported Clojure launcher without tying parity to one installer.

    The official Clojure CLI exposes `clj`; distro and Homebrew packages often
    expose only `clojure`.  The parity corpus is a pure-core file, so either
    launcher is sufficient and keeps the CI setup portable across platforms.
    """
    for command in ("clj", "clojure"):
        if shutil.which(command):
            return command
    return None


def run_cmd(cmd, env=None):
    p = subprocess.run(cmd, capture_output=True, text=True, env=env, cwd=ROOT)
    return p.returncode, p.stdout, p.stderr


def clj_eval_all(cases):
    """Evaluate all reference expressions in one Clojure process.

    The parity corpus intentionally contains only independent, pure expressions.
    Batching them keeps the smoke test fast enough to run routinely while the
    marker makes every reference value unambiguous, including nil and strings.
    """
    launcher = reference_clojure_command()
    if launcher is None:
        return False, "Clojure launcher not found (expected `clj` or `clojure`)"

    with tempfile.NamedTemporaryFile("w", suffix=".clj", delete=False) as f:
        for index, case in enumerate(cases):
            f.write(
                f'(println "__CLORUS_PARITY_EXPECT_{index}__" '
                f'(pr-str {case["expr"]}))\n'
            )
        path = f.name
    try:
        command = [launcher, "-M", path] if launcher == "clj" else [launcher, path]
        code, out, err = run_cmd(command)
        if code != 0:
            return False, f"Clojure reference launcher failed: {err.strip() or out.strip()}"

        values = {}
        for line in out.splitlines():
            match = re.match(r"^__CLORUS_PARITY_EXPECT_(\d+)__\s+(.*)$", line)
            if match:
                values[int(match.group(1))] = match.group(2)

        missing = [str(index) for index in range(len(cases)) if index not in values]
        if missing:
            return False, "clj output missing reference values for case indexes: " + ", ".join(missing)
        return True, values
    finally:
        Path(path).unlink(missing_ok=True)


def clorus_eval_all(cases, expected_literals, engine: str):
    """Check all expressions in one Clorus process for the selected engine."""
    with tempfile.NamedTemporaryFile("w", suffix=".clr", delete=False) as f:
        for index, case in enumerate(cases):
            expected = expected_literals[index]
            # Clojure's printed list values are forms in Clorus source, so
            # they need quoting to remain data. Other pr-str literals in this
            # corpus (numbers, booleans, strings, keywords, maps, vectors,
            # and sets) are already self-evaluating; quoting booleans changes
            # their meaning in the current reader.
            if expected.startswith("("):
                expected = f"(quote {expected})"
            f.write(
                f'(println "__CLORUS_PARITY_RESULT_{index}__" '
                f'(= {case["expr"]} {expected}))\n'
            )
        path = f.name
    try:
        env = os.environ.copy()
        env["CLORUS_ENTRY_FILE"] = path
        env.setdefault("CLORUS_HOME", str(ROOT))
        cmd = [CLORUS_BIN, "run"]
        if engine == "aot":
            cmd.append("--legacy-run")
        code, out, err = run_cmd(cmd, env=env)
        if code != 0:
            msg = (out + "\n" + err).strip()
            return False, f"clorus failed: {msg}"

        combined = (out + "\n" + err).splitlines()
        values = {}
        for line in combined:
            match = re.match(r"^__CLORUS_PARITY_RESULT_(\d+)__\s+(.*)$", line)
            if match:
                values[int(match.group(1))] = match.group(2).strip()

        missing = [str(index) for index in range(len(cases)) if index not in values]
        if missing:
            return False, (
                "clorus output missing result values for case indexes: "
                + ", ".join(missing)
                + f". output={out!r} err={err!r}"
            )
        return True, values
    finally:
        Path(path).unlink(missing_ok=True)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Compare pure core expressions across Clojure, Clorus JIT, and Clorus AOT."
    )
    parser.add_argument(
        "--engines",
        default="jit,aot",
        help="Comma-separated Clorus engines to run: jit,aot (default: jit,aot)",
    )
    return parser.parse_args()


def main():
    args = parse_args()
    engines = [engine.strip() for engine in args.engines.split(",") if engine.strip()]
    invalid = set(engines) - {"jit", "aot"}
    if invalid or not engines:
        print("ERROR: --engines must contain one or both of: jit,aot")
        return 2
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
    print(f"engines: {', '.join(engines)}")
    print("")

    ok_clj, clj_values = clj_eval_all(cases)
    if not ok_clj:
        print(f"[FAIL] Clojure reference evaluation: {clj_values}")
        return 1

    engine_values = {}
    engine_errors = {}
    for engine in engines:
        ok_clorus, values = clorus_eval_all(cases, clj_values, engine)
        if ok_clorus:
            engine_values[engine] = values
        else:
            engine_errors[engine] = values

    passed = 0
    failed = 0
    for index, case in enumerate(cases):
        name = case["name"]
        expr = case["expr"]
        case_failed = False

        for engine in engines:
            if engine in engine_errors:
                print(f"[FAIL] {name} [{engine}]: {engine_errors[engine]}")
                case_failed = True
                continue

            clorus_value = engine_values[engine][index]
            if clorus_value != "true":
                print(f"[FAIL] {name} [{engine}]")
                print(f"  expr:   {expr}")
                print(f"  expect: {clj_values[index]}")
                print(f"  clorus: (= expr expect) => {clorus_value}")
                case_failed = True

        if case_failed:
            failed += 1
        else:
            print(f"[PASS] {name} [{'/'.join(engines)}]")
            passed += 1

    print("")
    print("-----------")
    print(f"passed: {passed}")
    print(f"failed: {failed}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
