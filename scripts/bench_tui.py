"""Release PTY timing against the same owned 1000-project scan fixture.

Includes test harness and PTY setup; compare relative results on one machine.
"""
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time

from test_tui import windows_session, posix_session

root = Path(__file__).resolve().parent.parent
label = sys.argv[1] if len(sys.argv) > 1 else "tui"
fixture = root / "target/bench-fixture"
assert fixture.joinpath("ready").exists(), "Run bench_scan.py first"
build = subprocess.run(
    ["cargo", "test", "--locked", "--release", "--bin", "ds", "--no-run", "--message-format=json"],
    cwd=root, text=True, stdout=subprocess.PIPE, check=True,
)
binary = next(item["executable"] for line in build.stdout.splitlines()
              if (item := json.loads(line)).get("reason") == "compiler-artifact"
              and item.get("profile", {}).get("test") and item.get("executable"))
factory = windows_session if os.name == "nt" else posix_session
command = [binary, "--ignored", "--exact", "tui::tests::real_terminal_child", "--nocapture", "--test-threads=1"]
rows = []
for _ in range(12):
    start = time.perf_counter()
    with factory(command, {**os.environ, "DS_TEST_TUI_ROOT": str(fixture)}) as session:
        session.expect(b"quit")
        first = (time.perf_counter() - start) * 1000
        time.sleep(.1)
        start = time.perf_counter()
        session.send(b"rq")
        assert session.wait() == 0
        quit_ms = (time.perf_counter() - start) * 1000
        rows.append({"first_frame_ms": first, "reload_then_quit_ms": quit_ms})
rows = rows[2:]
summary = {key: {"median": statistics.median(row[key] for row in rows),
                 "max": max(row[key] for row in rows)} for key in rows[0]}
report = {"label": label, "projects": 1000, "warmups": 2, "samples": rows, "summary": summary}
out = root / "target/bench-reports"
out.mkdir(parents=True, exist_ok=True)
(out / (label + ".json")).write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps(summary))
