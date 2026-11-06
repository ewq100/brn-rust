#!/usr/bin/env python3
"""Prepare an approved, published A/B pair for a stock-updater HTTPS witness.

This developer-side preparation never executes an app/updater or publishes a
release. A missing command-enabled pair is a blocked gate, not a test pass.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import stat
import subprocess
import urllib.parse
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("preview_packaging", ROOT / "scripts/package-macos-preview.py")
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)
REPO = "ewq100/brn-rust"


def fetch(url, destination, maximum):
    request = urllib.request.Request(url, headers={"User-Agent": "BRN-published-update-qualification"})
    with urllib.request.urlopen(request, timeout=30) as response, destination.open("xb") as stream:
        if urllib.parse.urlsplit(response.url).scheme != "https":
            raise ValueError("Non-HTTPS redirect refused")
        total = 0
        while chunk := response.read(1024 * 1024):
            total += len(chunk)
            if total > maximum:
                raise ValueError("Published artifact exceeds preparation limit")
            stream.write(chunk)


def prepare_version(version, output, download=fetch):
    if not packaging.VERSION.fullmatch(version):
        raise ValueError("Explicit preview/alpha tag required")
    output.mkdir()
    metadata = output / "release.json"
    download(f"https://api.github.com/repos/{REPO}/releases/tags/{version}", metadata, 2_000_000)
    release = json.loads(metadata.read_bytes())
    if release.get("draft") is not False or release.get("tag_name") != version or not release.get("published_at"):
        raise ValueError("An explicit published release is required")
    name = f"BRN-Threads-{version}-macos-arm64"
    assets = release.get("assets", [])
    def asset(filename, maximum):
        matches = [a for a in assets if a.get("name") == filename]
        if len(matches) != 1:
            raise ValueError(f"Missing/ambiguous published asset: {filename}")
        item = matches[0]
        url = f"https://github.com/{REPO}/releases/download/{version}/{filename}"
        if item.get("browser_download_url") != url:
            raise ValueError("Asset belongs to an unexpected download route")
        path = output / filename
        download(url, path, maximum)
        if item.get("size") != path.stat().st_size or item.get("digest") != "sha256:" + packaging.digest(path):
            raise ValueError("GitHub asset digest/size differs")
        return path
    receipt = asset(name + ".manifest.json", 2_000_000)
    sums = asset("SHA256SUMS", 100_000)
    manifest = json.loads(receipt.read_bytes())
    lines = sums.read_text().splitlines()
    def checksum(path):
        expected = f"{packaging.digest(path)}  {path.name}"
        if [line for line in lines if line.endswith("  " + path.name)] != [expected]:
            raise ValueError("Missing/ambiguous/corrupt checksum receipt")
    checksum(receipt)
    source = manifest.get("source_sha", "")
    if len(source) != 40 or any(c not in "0123456789abcdef" for c in source) or release.get("target_commitish") != source:
        raise ValueError("Release/source identity differs")
    approved_file = output / "approved-preview.json"
    download(f"https://raw.githubusercontent.com/{REPO}/{source}/release/preview.json", approved_file, 100_000)
    approved = packaging.read_manifest(approved_file)
    if approved["version"] != version or any(manifest.get(k) != v for k, v in approved.items()):
        raise ValueError("Published package differs from its source-approved manifest")
    identity = {"version": version, "source_sha": source, "build_number": approved["build_number"],
                "published_at": release["published_at"], "manifest_sha256": packaging.digest(receipt)}
    updater = manifest.get("updater_sha256", "")
    if manifest.get("bundle_identifier") != "dev.brn.threads" or len(updater) != 64 or any(c not in "0123456789abcdef" for c in updater):
        return dict(identity, status="blocked", reason="Published artifact is not command-enabled; approved updater A/B is still required")
    archive = asset(name + ".zip", 4 * 1024**3)
    checksum(archive)
    if manifest.get("archive") != {"name": archive.name, "sha256": packaging.digest(archive), "bytes": archive.stat().st_size}:
        raise ValueError("Published archive identity differs")
    with zipfile.ZipFile(archive) as zipped:
        for entry in zipped.infolist():
            parts = entry.filename.split("/")
            if (parts[0] not in ("BRN Threads.app", "__MACOSX") or "\\" in entry.filename
                    or any(p in (".", "..") for p in parts) or stat.S_ISLNK(entry.external_attr >> 16)):
                raise ValueError("Unsafe published ZIP entry")
    subprocess.run(["/usr/bin/ditto", "-x", "-k", str(archive), str(output)], check=True)
    app = output / "BRN Threads.app"
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", "--deep", str(app)], check=True)
    script = app / "Contents/Resources/bin/brn-update"
    if not script.is_file() or script.is_symlink() or not script.stat().st_mode & 0o111 or packaging.digest(script) != updater:
        raise ValueError("Stock bundled updater identity differs")
    return dict(identity, status="ready", stock_updater_sha256=updater)


def prepare_pair(a, b, output, download=fetch):
    if not output.is_absolute() or output.exists() or output.is_symlink():
        raise ValueError("Output must be an absent absolute directory")
    output.mkdir(mode=0o700, parents=True)
    report = {"status": "blocked", "scope": "published-artifact preparation only; no updater/native/provider run"}
    try:
        report["A"] = prepare_version(a, output / "A", download)
        report["B"] = prepare_version(b, output / "B", download)
        if report["B"]["build_number"] <= report["A"]["build_number"]:
            raise ValueError("B must have an increasing approved build number")
        if report["A"]["status"] == report["B"]["status"] == "ready":
            report["status"] = "ready"
    except (ValueError, OSError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        report["error"] = str(error)
    (output / "preparation.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version-a", required=True)
    parser.add_argument("--version-b", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    report = prepare_pair(args.version_a, args.version_b, args.output)
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report["status"] == "ready" else 2)
