"""Collect unit, CLI and real-PTY coverage; keep reports under ignored target/."""
import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--minimum-lines", type=float, default=75)
    args = parser.parse_args()
    report_dir = Path("target/coverage")
    report_dir.mkdir(parents=True, exist_ok=True)
    run("cargo", "llvm-cov", "clean", "--workspace")
    run("cargo", "llvm-cov", "--locked", "--all-targets", "--no-report")
    instrumented_target = str(Path("target/llvm-cov-target").resolve())
    text = run("cargo", "llvm-cov", "show-env", "--sh", env={**os.environ, "CARGO_TARGET_DIR": instrumented_target}, capture_output=True, text=True).stdout
    env = os.environ.copy()
    for line in text.splitlines():
        # cargo emits shell-quoted NAME=value assignments. Parse values as data;
        # never evaluate the output as code or mutate persistent environment.
        tokens = shlex.split(line)
        assignment, = tokens[1:] if tokens[0] == "export" else tokens
        name, value = assignment.split("=", 1)
        env[name] = value
    result = run("cargo", "test", "--locked", "--bin", "ds", "--no-run", "--message-format=json", "--target-dir", instrumented_target, env=env, capture_output=True, text=True)
    binaries = [item["executable"] for line in result.stdout.splitlines()
                if (item := json.loads(line)).get("reason") == "compiler-artifact"
                and item.get("profile", {}).get("test") and item.get("executable")]
    if len(binaries) != 1:
        raise RuntimeError(f"expected one ds unit-test binary, found {len(binaries)}")
    run(sys.executable, "scripts/test_tui.py", "--binary", binaries[0], env=env)
    ignored = r"regression_tests.rs|tests[/\\]"
    run("cargo", "llvm-cov", "report", "--json", "--summary-only", "--output-path", str(report_dir / "summary.json"), "--ignore-filename-regex", ignored)
    run("cargo", "llvm-cov", "report", "--lcov", "--output-path", str(report_dir / "lcov.info"), "--ignore-filename-regex", ignored)
    run("cargo", "llvm-cov", "report", "--html", "--output-dir", str(report_dir / "html"), "--ignore-filename-regex", ignored, "--fail-under-lines", str(args.minimum_lines))
    summary = json.loads((report_dir / "summary.json").read_text(encoding="utf-8"))["data"][0]["totals"]["lines"]
    print(f"Line coverage: {summary['percent']:.2f}% ({summary['covered']}/{summary['count']})")


if __name__ == "__main__":
    main()
