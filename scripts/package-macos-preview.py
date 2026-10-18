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
import shutil
import tarfile
import tomllib

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


def prepare_notices(output, graphs):
    """Retain the exact cached source archives and their embedded legal notices.

    Graphs are target-specific locked normal/build cargo-tree receipts from the
    actual desktop and both helper workspaces, not the whole platform lock set.
    """
    output = Path(output)
    if not output.is_absolute() or output.exists():
        raise ValueError("Notices output must be absent and absolute")
    locks = [ROOT / "Cargo.lock", ROOT / "tools/xberg-intake-helper/Cargo.lock",
             ROOT / "tools/xberg-intake-helper/adapted.Cargo.lock"]
    packages = {}
    for lock in locks:
        for item in tomllib.loads(lock.read_text())["package"]:
            if item.get("source", "").startswith("registry+"):
                key = (item["name"], item["version"])
                if key in packages and packages[key]["checksum"] != item["checksum"]:
                    raise ValueError("Conflicting source archive identity")
                packages[key] = item
    selected = set()
    for graph in graphs:
        for line in Path(graph).read_text().splitlines():
            match = re.fullmatch(r"([A-Za-z0-9_-]+) v([^ ]+)(?: \((.*?)\))?(?: \(\*\))?", line)
            if not match:
                raise ValueError(f"Unrecognized graph row: {line}")
            if (match[1], match[2]) in packages:
                selected.add((match[1], match[2]))
            elif not match[3] or not Path(match[3]).is_absolute():
                raise ValueError(f"Unknown registry graph identity: {line}")
    cache = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "registry/cache/index.crates.io-1949cf8c6b5b557f"
    # Validate every archive before creating the output. No source acquisition.
    for name, version in selected:
        archive = cache / f"{name}-{version}.crate"
        if digest(archive) != packages[name, version]["checksum"]:
            raise ValueError(f"Cached archive checksum differs: {name}-{version}")
    output.mkdir(mode=0o700, parents=True)
    records = []
    for name, version in sorted(selected):
        archive = cache / f"{name}-{version}.crate"
        destination = output / "registry-source" / archive.name
        destination.parent.mkdir(exist_ok=True)
        shutil.copyfile(archive, destination)
        with tarfile.open(archive, "r:gz") as source:
            manifest = tomllib.loads(source.extractfile(f"{name}-{version}/Cargo.toml").read().decode())
        package = manifest["package"]
        if not package.get("license") and not package.get("license-file"):
            raise ValueError(f"Missing license declaration: {name}-{version}")
        records.append(dict(name=name, version=version, license=package.get("license"),
                            authors=package.get("authors", []), repository=package.get("repository"),
                            license_file=package.get("license-file"), source=packages[name, version]["source"],
                            sha256=digest(archive), archive=str(destination.relative_to(output))))
    for number, graph in enumerate(graphs):
        shutil.copyfile(graph, output / f"target-graph-{number}.txt")
    for lock in locks:
        relative = lock.relative_to(ROOT)
        destination = output / "locks" / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(lock, destination)
    # Include the modified source delta and all attribution/provenance documents.
    patches = output / "xberg-adaptations"
    patches.mkdir()
    for prefix in ["0001", "0003", "0004", "0005"]:
        for source in (ROOT / "patches/xberg/1.3.10").glob(prefix + "*"):
            shutil.copyfile(source, patches / source.name)
    for name in ["README.md", "source-manifest.json"]:
        shutil.copyfile(ROOT / "patches/xberg/1.3.10" / name, patches / name)
    shutil.copyfile(ROOT / "target/xberg-adapted/source-manifest.json", patches / "full-source-manifest.json")
    for directory in [ROOT / "tools/xberg-intake-helper", ROOT / "crates/brn-intake"]:
        for source in directory.rglob("*"):
            if source.is_file() and source.suffix in {".rs", ".toml", ".txt"}:
                relative = source.relative_to(ROOT)
                destination = output / "brn-source" / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)
    for source in ROOT.glob("LICENSE*"):
        if source.is_file():
            shutil.copyfile(source, output / source.name)
    (output / "README.txt").write_text(
        "BRN Threads source and third-party notices\n"
        "Each registry-source/*.crate is the exact checksum-verified source archive, including\n"
        "its original copyright, license and NOTICE files. packages.json lists declarations;\n"
        "licenses remain authoritative in those archives. Both normal/build target graphs and\n"
        "locks are retained (build-only packages may be over-inclusive). Xberg is MIT; stock\n"
        "is unchanged. Only the four supplied BRN adaptation patches modify its optional copy.\n"
        "No model files or provider credentials are included. No runtime source download is needed.\n")
    (output / "packages.json").write_text(json.dumps(records, indent=2) + "\n")
    files = {str(path.relative_to(output)): digest(path) for path in sorted(output.rglob("*")) if path.is_file()}
    (output / "index.json").write_text(json.dumps({"schema": 1, "files": files}, indent=2) + "\n")
    return output


def package(binary, stock_helper, brn_helper, manifest_path, source_sha, output_dir, notices):
    manifest = read_manifest(manifest_path)
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha):
        raise ValueError("Source commit must be an exact SHA")
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("Preview packaging requires macOS Apple Silicon")
    binaries = {"brn-desktop": inspect_binary(binary),
                "brn-xberg-helper": inspect_binary(stock_helper),
                "brn-xberg-helper-brn": inspect_binary(brn_helper)}
    notices = Path(notices)
    if not notices.is_dir() or not (notices / "index.json").is_file():
        raise ValueError("Dependency/source notices directory and index.json are required")
    notice_index = json.loads((notices / "index.json").read_text())
    actual_files = {str(path.relative_to(notices)) for path in notices.rglob("*") if path.is_file() and path.name != "index.json"}
    if not notice_index.get("files") or actual_files != set(notice_index["files"]):
        raise ValueError("Notice index must cover every retained file")
    if any(path.is_symlink() for path in notices.rglob("*")):
        raise ValueError("Symlinks are not permitted in notices")
    for relative, expected in notice_index["files"].items():
        path = notices / relative
        if (Path(relative).is_absolute() or ".." in Path(relative).parts
                or path.is_symlink() or digest(path) != expected):
            raise ValueError("Notice/source checksum or path differs")
    output = Path(output_dir)
    if not output.is_absolute() or output.exists() or output.is_symlink():
        raise ValueError("Output must be an absent absolute directory")
    output.parent.mkdir(parents=True, exist_ok=True)
    name = f"BRN-Threads-{manifest['version']}-macos-arm64"
    build = dict(manifest, source_sha=source_sha, signing="ad-hoc; not notarized",
                 binaries=binaries, data_directory="~/Library/Application Support/BRN Threads Preview/workspace",
                 qualification={"native_interaction": "deferred", "live_provider": "deferred",
                                "recovery_acceptance": "pending"},
                 external_pdf_tools=[],
                 notices_index_sha256=digest(notices / "index.json"),
                 bundled_embeddings=False, bundled_provider_credentials=False,
                 cargo_lock_sha256=digest(ROOT / "Cargo.lock"),
                 rust_toolchain=(ROOT / "rust-toolchain.toml").read_text())
    with tempfile.TemporaryDirectory(prefix="brn-preview-", dir=output.parent) as scratch:
        stage = Path(scratch) / "distribution"
        stage.mkdir()
        app = stage / "BRN Threads.app"
        run("bash", str(ROOT / "scripts/make-macos-app.sh"), "--output", str(app),
            "--binary", str(binary), "--xberg-helper", str(stock_helper),
            "--xberg-helper-brn", str(brn_helper),
            "--preview-version", manifest["version"], "--build-number", str(manifest["build_number"]))
        shutil.copytree(notices, app / "Contents/Resources/notices")
        for executable in binaries:
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
    if len(os.sys.argv) > 1 and os.sys.argv[1] == "--prepare-notices":
        parser = argparse.ArgumentParser(description="Collect verified cached source/legal custody")
        parser.add_argument("--prepare-notices", type=Path, required=True)
        parser.add_argument("--graph", type=Path, action="append", required=True)
        args = parser.parse_args()
        print(prepare_notices(args.prepare_notices, args.graph))
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--xberg-helper", type=Path, required=True)
    parser.add_argument("--xberg-helper-brn", type=Path, required=True)
    parser.add_argument("--notices", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=ROOT / "release/preview.json")
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        print(package(args.binary, args.xberg_helper, args.xberg_helper_brn, args.manifest, args.source_sha, args.output_dir, args.notices))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Preview packaging failed: {error}\n")


if __name__ == "__main__":
    main()
