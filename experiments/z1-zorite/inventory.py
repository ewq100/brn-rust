#!/usr/bin/env python3
"""Compare the copied lock with BRN and inventory published license metadata."""
import hashlib
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
metadata = json.loads((HERE / "evidence/resolve.stdout").read_text())
baseline = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
resolved = tomllib.loads((HERE / "Cargo.lock").read_text())["package"]
key = lambda p: (p["name"], p["version"], p.get("source") or "")
old = {key(p) for p in baseline}
old_names = {p["name"] for p in baseline}
new = {key(p) for p in resolved} - old
packages = []
for package in sorted(metadata["packages"], key=key):
    item = {k: package[k] for k in ("name", "version", "source", "license", "license_file", "repository")}
    item["new_to_brn_lock"] = key(package) in new
    # SPDX GPL or AGPL tokens; LGPL is separately visible in the full inventory.
    item["gpl_agpl_flag"] = bool(re.search(r"(?<![A-Za-z])(A?GPL)(?:-|\b)", package["license"] or ""))
    item["lgpl_flag"] = "LGPL" in (package["license"] or "")
    if item["new_to_brn_lock"] and package["source"]:
        path = Path(package["manifest_path"])
        item["published_manifest_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        notice = path.parent / "LICENSE"
        if notice.is_file():
            destination = HERE / "evidence/notices" / f"{package['name']}-{package['version']}.txt"
            destination.parent.mkdir(exist_ok=True)
            destination.write_bytes(notice.read_bytes())
            item["license_notice_sha256"] = hashlib.sha256(notice.read_bytes()).hexdigest()
    packages.append(item)
summary = {
    "snapshot": "7e9d0376d677ed2a46bdbcd8933f55f9e1746a02",
    "baseline_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
    "experiment_lock_sha256": hashlib.sha256((HERE / "Cargo.lock").read_bytes()).hexdigest(),
    "new_registry_packages": [p for p in packages if p["new_to_brn_lock"] and p["source"]],
    "new_versions_of_existing_names": sorted(p for p in new if p[0] in old_names),
    "gpui_pre_versions": [p["version"] for p in packages if p["name"] == "gpui-pre"],
    "gpl_agpl_flags": [p for p in packages if p["gpl_agpl_flag"]],
    "lgpl_flags": [p for p in packages if p["lgpl_flag"]],
    "package_count_all_targets": len(packages),
    "packages": packages,
}
(HERE / "evidence/dependencies.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps({k: v for k, v in summary.items() if k != "packages"}, indent=2))
assert summary["gpui_pre_versions"] == ["0.3.9"]
assert not summary["new_versions_of_existing_names"]
assert not any(p["name"] == "zorite" for p in packages)
