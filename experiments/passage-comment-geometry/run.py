#!/usr/bin/env python3
"""Record reproducible commands; use the installed repository-pinned compiler."""
import datetime
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent / "evidence"
TOOLCHAIN = Path("/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin")
label, *command = sys.argv[1:]
env = dict(os.environ, PATH=f"{TOOLCHAIN}:{os.environ['PATH']}",
           RUSTUP_TOOLCHAIN="1.98.1", CARGO_TARGET_DIR="/private/tmp/brn-passage-geometry-target")
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
processes = subprocess.run(["ps", "-axo", "pid,etime,comm"], text=True, capture_output=True)
builds = [line for line in processes.stdout.splitlines()
          if any(name in line.lower() for name in ("cargo", "rustc", "clang", "rustup"))]
(OUT / f"{label}.processes.txt").write_text("\n".join(builds) + "\n")
tick = time.monotonic()
with (OUT / f"{label}.stdout").open("wb") as stdout, (OUT / f"{label}.stderr").open("wb") as stderr:
    result = subprocess.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
# Cargo's test harness adds blank lines at EOF; trim only those in the
# text evidence so the repository's whitespace check also covers the logs.
for suffix in ("stdout", "stderr"):
    path = OUT / f"{label}.{suffix}"
    data = path.read_bytes()
    if data:
        path.write_bytes(data.rstrip(b"\n") + b"\n")
record = dict(label=label, started_utc=started, seconds=time.monotonic() - tick,
              command=command, cwd=str(ROOT), exit_code=result.returncode,
              env={key: env[key] for key in ("PATH", "RUSTUP_TOOLCHAIN", "CARGO_TARGET_DIR")},
              concurrent_build_processes=builds)
with (OUT / "commands.jsonl").open("a") as output:
    output.write(json.dumps(record) + "\n")
print(json.dumps(record, indent=2))
raise SystemExit(result.returncode)
