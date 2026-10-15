#!/usr/bin/env python3
"""Record immutable prototype/current-main reconciliation; no checkout changes."""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BASE = "7e9d0376d677ed2a46bdbcd8933f55f9e1746a02"
PROTOTYPE = "26b029756e3f20a3d9f0a7273230ab6cecee756f"
CURRENT = "8bef439b133e91ad68b0b76792bcd155d3d06237"
Z1 = "b9bd1a14862fe2efe246790e887797c949f13c50"
def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()
paths = ["AGENTS.md", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
         "crates/brn-desktop", "crates/brn-threads-core", "crates/brn-threads-app",
         "docs/work/active/modular-foundation", "docs/work/active/writing-polish"]
identity = {}
for path in paths:
    old = git("rev-parse", f"{BASE}:{path}")
    current = git("rev-parse", f"{CURRENT}:{path}")
    assert old == current, path
    identity[path] = current
source_hashes = json.loads((HERE / "evidence/qualified-toolkit-sources.json").read_text())
for path, expected in source_hashes.items():
    assert hashlib.sha256((HERE / ".vendor/gpui-base" / path).read_bytes()).hexdigest() == expected, path
assert git("rev-parse", "work/z1-zorite") == Z1
assert git("ls-files", "experiments/passage-comment-geometry/.vendor") == ""
changed = git("diff", "--name-only", BASE).splitlines()
untracked = git("ls-files", "--others", "--exclude-standard").splitlines()
assert all(p.startswith("experiments/passage-comment-geometry/") for p in changed + untracked)
assert subprocess.run(["git", "cat-file", "-e", f"{CURRENT}:docs/work/active/modular-foundation/plan.md"],
                      cwd=ROOT, capture_output=True).returncode != 0
patch = HERE / "patches/gpui-base-0.7.3-range-start.patch"
print(json.dumps({
    "original_z1_commit_unchanged": Z1,
    "followup_initial_prototype_commit": PROTOTYPE,
    "followup_baseline": BASE,
    "fetched_main_for_handoff": CURRENT,
    "origin_main_at_check": git("rev-parse", "origin/main"),
    "main_changes_since_baseline": git("diff", "--name-only", BASE, CURRENT).splitlines(),
    "identical_production_and_guidance_objects": identity,
    "toolkit_patch_sha256": hashlib.sha256(patch.read_bytes()).hexdigest(),
    "qualified_toolkit_source_hashes_unchanged": source_hashes,
    "tracked_vendor_files": [],
    "new_root_dependency_changes": [],
    "c0_active_plan_present": False,
    "reconciliation": "production code/pins unchanged; no rebase or production integration performed",
    "scope": "experiment only"
}, indent=2))
