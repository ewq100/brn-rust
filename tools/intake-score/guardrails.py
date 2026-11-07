#!/usr/bin/env python3
"""Combine scorecards with explicit test and executable-bundle size receipts."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
PROTECTED = ["Cargo.toml", "Cargo.lock", "crates/*/Cargo.toml",
             "tools/xberg-intake-helper", "patches/xberg", "scripts/prepare-xberg-adapted.py"]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--scorecard", type=Path, required=True)
    p.add_argument("--tests", type=Path, required=True, help="JSON list of command, exit_code and log_sha256 receipts")
    p.add_argument("--baseline-app", type=Path, required=True)
    p.add_argument("--final-app", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    card = json.loads(args.scorecard.read_text())
    frozen = json.loads((ROOT / "tools/intake-score/frozen.json").read_text())
    source = subprocess.run(["git", "diff", "--exit-code", frozen["base"], "--", *PROTECTED], cwd=ROOT, capture_output=True, text=True)
    size = subprocess.run(["bash", str(ROOT / "scripts/report-sizes.sh"), "--app", str(args.baseline_app), "--app", str(args.final_app)], capture_output=True, text=True)
    sizes = [int(n) for n in re.findall(r"allocated=(\d+) KiB", size.stdout)]
    tests = json.loads(args.tests.read_text())
    import hashlib
    tests_pass = bool(tests) and all(t["exit_code"] == 0 and hashlib.sha256(Path(t["log"]).read_bytes()).hexdigest() == t["log_sha256"] for t in tests)
    gates = {"protected_sources_unchanged": source.returncode == 0,
             "test_receipts_pass": tests_pass,
             "app_size_within_5_percent": size.returncode == 0 and len(sizes) == 2 and sizes[1] <= sizes[0]*1.05,
             "scorecard_guardrails": all(v for v in card["guardrails"].values() if isinstance(v, bool)),
             "metric_eligible": card["decision"]["metric_eligible"]}
    value = {"gates": gates, "kept": all(gates.values()), "sizes_kib": sizes,
             "size_command_output": size.stdout, "source_diff": source.stdout,
             "tests": tests, "scorecard": str(args.scorecard)}
    args.output.write_text(json.dumps(value, indent=2) + "\n")
    print(json.dumps(value, indent=2))
    if not value["kept"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
