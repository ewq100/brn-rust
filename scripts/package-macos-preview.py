#!/usr/bin/env python3
"""Package an Apple Silicon preview without developer credentials or user data."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = re.compile(r"v\d+\.\d+\.\d+-preview\.[1-9]\d*")


def read_manifest(path):
    manifest = json.loads(Path(path).read_text())
    if (manifest.get("schema") != 1 or manifest.get("enabled") is not True
            or not VERSION.fullmatch(manifest.get("version", ""))
            or type(manifest.get("build_number")) is not int
            or not 1 <= manifest["build_number"] <= 999999
            or manifest.get("platform") != "macos-arm64"
            or manifest.get("minimum_macos") != "15.0"
            or manifest.get("features") != ["native-ui"]
            or manifest.get("notes_file") != "docs/development/preview-release.md"):
        raise ValueError("Invalid or unapproved preview manifest")
    return manifest


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def run(*args):
    return subprocess.run(args, check=True, text=True, capture_output=True).stdout


def inspect_binary(path):
    path = Path(path)
    if not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
        raise ValueError(f"Missing absolute executable: {path}")
    if run("/usr/bin/lipo", "-archs", str(path)).strip() != "arm64":
        raise ValueError(f"Preview requires an arm64-only Mach-O: {path}")
    libraries = []
    for line in run("/usr/bin/otool", "-L", str(path)).splitlines()[1:]:
        dependency = line.strip().split(" (", 1)[0]
        if not dependency.startswith(("/usr/lib/", "/System/Library/")):
            raise ValueError(f"Nonportable linked library in {path.name}: {dependency}")
        libraries.append(dependency)
    if not libraries:
        raise ValueError(f"No linked-library inventory for {path}")
    deployment = re.findall(r"^\s*minos (\S+)\s*$", run("/usr/bin/otool", "-l", str(path)), re.MULTILINE)
    if deployment != ["15.0"]:
        raise ValueError(f"Preview requires an explicit macOS 15.0 deployment target: {path}")
    return {"sha256": digest(path), "architecture": "arm64", "minimum_macos": deployment[0], "libraries": libraries}


def package(binary, helper, manifest_path, source_sha, output_dir):
    manifest = read_manifest(manifest_path)
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha):
        raise ValueError("Source commit must be an exact SHA")
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("Preview packaging requires macOS Apple Silicon")
    binaries = {"brn-desktop": inspect_binary(binary),
                "brn-intake-helper": inspect_binary(helper)}
    output = Path(output_dir)
    if not output.is_absolute() or output.exists() or output.is_symlink():
        raise ValueError("Output must be an absent absolute directory")
    output.parent.mkdir(parents=True, exist_ok=True)
    name = f"BRN-Threads-{manifest['version']}-macos-arm64"
    build = dict(manifest, source_sha=source_sha, signing="ad-hoc; not notarized",
                 binaries=binaries, data_directory="~/Library/Application Support/BRN Threads Preview/workspace",
                 qualification={"native_interaction": "deferred", "live_provider": "deferred",
                                "recovery_acceptance": "pending"},
                 external_pdf_tools=["pdftotext", "pdfimages"],
                 bundled_embeddings=False, bundled_provider_credentials=False,
                 cargo_lock_sha256=digest(ROOT / "Cargo.lock"),
                 rust_toolchain=(ROOT / "rust-toolchain.toml").read_text())
    with tempfile.TemporaryDirectory(prefix="brn-preview-", dir=output.parent) as scratch:
        stage = Path(scratch) / "distribution"
        stage.mkdir()
        app = stage / "BRN Threads.app"
        run("bash", str(ROOT / "scripts/make-macos-app.sh"), "--output", str(app),
            "--binary", str(binary), "--intake-helper", str(helper),
            "--preview-version", manifest["version"], "--build-number", str(manifest["build_number"]))
        for executable in ("brn-desktop", "brn-intake-helper"):
            run("/usr/bin/codesign", "--force", "--sign", "-",
                str(app / "Contents/Resources/bin" / executable))
            installed = inspect_binary(app / "Contents/Resources/bin" / executable)
            installed["source_binary_sha256"] = binaries[executable]["sha256"]
            build["binaries"][executable] = installed
        (app / "Contents/Resources/build-manifest.json").write_text(json.dumps(build, indent=2) + "\n")
        run("/usr/bin/codesign", "--force", "--sign", "-", str(app))
        run("/usr/bin/codesign", "--verify", "--strict", "--deep", str(app))
        archive = stage / f"{name}.zip"
        run("/usr/bin/ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(app), str(archive))
        relocated = Path(scratch) / "relocated preview"
        run("/usr/bin/ditto", "-x", "-k", str(archive), str(relocated))
        run("/usr/bin/codesign", "--verify", "--strict", "--deep", str(relocated / app.name))
        delivery = dict(build, archive={"name": archive.name, "sha256": digest(archive),
                                       "bytes": archive.stat().st_size})
        receipt = stage / f"{name}.manifest.json"
        receipt.write_text(json.dumps(delivery, indent=2) + "\n")
        (stage / "SHA256SUMS").write_text("".join(f"{digest(path)}  {path.name}\n" for path in (archive, receipt)))
        # Claim exclusively after preparation. Never replace even an empty collision.
        # Interrupted publication stays visibly incomplete and is never releasable.
        output.mkdir(mode=0o700)
        incomplete = output / ".incomplete"
        incomplete.write_text(source_sha + "\n")
        for path in stage.iterdir():
            path.rename(output / path.name)
        incomplete.unlink()
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--intake-helper", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=ROOT / "release/preview.json")
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        print(package(args.binary, args.intake_helper, args.manifest, args.source_sha, args.output_dir))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Preview packaging failed: {error}\n")


if __name__ == "__main__":
    main()
