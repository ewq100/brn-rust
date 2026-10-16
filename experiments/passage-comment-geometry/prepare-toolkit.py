#!/usr/bin/env python3
"""Materialize only this experiment's ignored toolkit copy from its pinned archive."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request

HERE = Path(__file__).resolve().parent
NAME = "gpui-base-0.7.3"
SHA256 = "2e9955b6f4b048fd50ca15fc8e225d80bb00ba9ebc8fecdddde5ad26dbef9270"
PATCH = HERE / "patches" / f"{NAME}-range-start.patch"
VENDOR = HERE / ".vendor"
DEST = VENDOR / "gpui-base"
STAMP = VENDOR / "prepared-patch.sha256"
patch_hash = hashlib.sha256(PATCH.read_bytes()).hexdigest()
if DEST.exists():
    if STAMP.exists() and STAMP.read_text().strip() == patch_hash:
        print(f"Ready: {DEST} (patch {patch_hash})")
        raise SystemExit(0)
    raise SystemExit("Generated .vendor exists with a different/unrecorded patch. Remove only this experiment's .vendor and rerun.")
VENDOR.mkdir(exist_ok=True)
archives = sorted((Path.home() / ".cargo" / "registry" / "cache").glob(f"*/{NAME}.crate"))
archive = archives[0] if archives else VENDOR / f"{NAME}.crate"
if not archive.exists():
    urllib.request.urlretrieve(f"https://static.crates.io/crates/gpui-base/{NAME}.crate", archive)
if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
    raise SystemExit("Pinned crate checksum mismatch; nothing extracted")
with tarfile.open(archive) as bundle:
    for member in bundle.getmembers():
        if not member.name.startswith(f"{NAME}/") or member.issym() or member.islnk() or ".." in Path(member.name).parts:
            raise SystemExit("Unexpected crate member")
    bundle.extractall(VENDOR, filter="data")
shutil.move(VENDOR / NAME, DEST)
subprocess.run(["patch", "--batch", "-p1", "-i", str(PATCH)], cwd=DEST, check=True)
STAMP.write_text(patch_hash + "\n")
print(f"Prepared {NAME}, archive SHA256={SHA256}; patch SHA256={patch_hash}")
