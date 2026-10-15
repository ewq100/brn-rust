#!/usr/bin/env python3
"""Check experiment version/scope assumptions without modifying the checkout."""
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()
root = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
experiment = tomllib.loads((HERE / "Cargo.lock").read_text())["package"]
root_keys = {(p["name"], p["version"]) for p in root}
new = sorted({(p["name"], p["version"]) for p in experiment} - root_keys)
assert new == [("brn-passage-comment-geometry", "0.1.0")], new
for name, version in [("gpui-kit", "0.7.3"), ("gpui-base", "0.7.3"), ("gpui-pre", "0.3.9")]:
    assert [p["version"] for p in experiment if p["name"] == name] == [version]
assert not any(p["name"].startswith("zorite") for p in experiment)
changed = git("diff", "--name-only", "7e9d0376d677ed2a46bdbcd8933f55f9e1746a02").splitlines()
untracked = git("ls-files", "--others", "--exclude-standard").splitlines()
assert all(p.startswith("experiments/passage-comment-geometry/") for p in changed + untracked)
patch = HERE / "patches/gpui-base-0.7.3-range-start.patch"
print(json.dumps({
    "branch": git("branch", "--show-current"),
    "baseline": git("merge-base", "HEAD", "origin/main"),
    "origin_main": git("rev-parse", "origin/main"),
    "remote": git("remote", "get-url", "origin"),
    "versions": {name: [p["version"] for p in experiment if p["name"] == name] for name in ["gpui-kit", "gpui-base", "gpui-pre", "markdown", "ropey"]},
    "new_external_crates": [],
    "new_local_package": new,
    "gpui_base_source": "isolated path copy with reviewable patch",
    "patch_sha256": hashlib.sha256(patch.read_bytes()).hexdigest(),
    "production_changes": [],
    "scope_check": "passed"
}, indent=2))
