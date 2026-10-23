#!/usr/bin/env python3
"""Offline, fail-closed materialization of the initial accepted optional bundle."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import tempfile

ARCHIVE = "f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d"
PATCHES = [
    ("0001-docx-table-grid-spans.patch", "92178fcfb5887328dec2f11b1c63d28687b628f4764e4cc41cae30a81598a6d7", "source-manifest.json", "56a1953f4e47f7ae7a53692b5cc417e6b0f5ada8fb6fd6362dad5f637dc3c81e"),
    ("0003-docx-namespaces.patch", "e64020a6df795f4cd1b32ad5eba124197e644fec08c88033cc904313442b851f", "0003-source-manifest.json", "c1a43690ae46797e4559162038139ec6b46bf228c2248d83173162f8d3a75bfb"),
    ("0004-pptx-order-hidden.patch", "aaf5c6f67c22ff492fae7b80aec446ce083b620837864d6eb80e782d64983a1d", "0004-source-manifest.json", "1663517db2adaa068c216da25eb9709a023ee23434c6b5daeb226ce53d39996b"),
    ("0005-docx-comment-references.patch", "b574b0374514860dccf0d6b503bd8408ce16a259f2ce68f3dd6cef8aa18a69b1", "0005-source-manifest.json", "9d85cd446f5368ed523a8de95536192b6762e4f960e1e68f92d552f2695e8a6d"),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def identity():
    material = ARCHIVE + "\nutf8-identity-v1\n" + "".join(f"{p} {h}\n" for p, h, _, _ in PATCHES)
    return "xberg/1.3.10/brn/" + sha(material.encode()) + "/figures-v1"


def read_archive(archive):
    if sha(archive.read_bytes()) != ARCHIVE:
        raise ValueError("stock archive SHA256 mismatch")
    with tarfile.open(archive) as tar:
        return safe_members(tar)


def safe_members(tar):
    """Validate before writing anything; never extract links or special entries."""
    result = {}
    for member in tar.getmembers():
        path = PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != "xberg-1.3.10":
            raise ValueError("unsafe archive path")
        if member.isdir():
            continue
        if not member.isfile() or len(path.parts) < 2:
            raise ValueError("unsafe archive member type")
        name = str(PurePosixPath(*path.parts[1:]))
        if name in result:
            raise ValueError("duplicate archive path")
        result[name] = tar.extractfile(member).read()
    return result


def file_hashes(root):
    result = {}
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError("symlink in generated source")
        if path.is_file():
            result[path.relative_to(root).as_posix()] = sha(path.read_bytes())
    return result


def no_symlinks(path):
    for parent in [path, *path.parents]:
        if parent.is_symlink():
            raise ValueError("symlink in output path")


def prepare(repo, archive, output):
    repo = repo.resolve(strict=True)
    output = output.absolute()
    no_symlinks(output)
    if output != repo / "target/xberg-adapted":
        raise ValueError("output must be this repository's ignored target/xberg-adapted")
    entries = read_archive(archive)
    if len(entries) != 764:
        raise ValueError("unexpected published package file count")
    patches = repo / "patches/xberg/1.3.10"
    specs = []
    for name, digest, manifest, manifest_hash in PATCHES:
        patch = patches / name
        if sha(patch.read_bytes()) != digest or sha((patches / manifest).read_bytes()) != manifest_hash:
            raise ValueError("accepted patch/source manifest mismatch")
        specs.append((patch, json.loads((patches / manifest).read_text())["selected_source_files"]))
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".xberg-adapted-", dir=output.parent) as temporary:
        stage = Path(temporary)
        source = stage / "source"
        source.mkdir()
        for name, data in entries.items():
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        expected = {name: sha(data) for name, data in entries.items()}
        git_env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        git_env["GIT_CEILING_DIRECTORIES"] = str(stage)
        for patch, rows in specs:
            subprocess.run(["git", "apply", "--check", str(patch)], cwd=source, env=git_env, check=True, capture_output=True)
            subprocess.run(["git", "apply", str(patch)], cwd=source, env=git_env, check=True, capture_output=True)
            for row in rows:
                expected[row["path"]] = row.get("compiled_final_sha256", row.get("compiled_candidate_sha256"))
            if file_hashes(source) != expected:
                raise ValueError("full adapted source mismatch after " + patch.name)
        if len(expected) != 765:
            raise ValueError("unexpected adapted package file count")
        workspace = stage / "workspace"
        workspace.mkdir()
        tool = repo / "tools/xberg-intake-helper"
        manifest = (tool / "Cargo.toml").read_text()
        manifest = manifest.replace('path = "../../crates/brn-intake"', f'path = "{repo}/crates/brn-intake"')
        manifest = manifest.replace('build = "build.rs"', f'build = "{tool}/build.rs"')
        for relative in ["src/main.rs", "tests/eight_kinds.rs", "tests/identity.rs"]:
            manifest = manifest.replace(f'path = "{relative}"', f'path = "{tool}/{relative}"')
        manifest += '\n[patch.crates-io]\nxberg = { path = "../source" }\n'
        (workspace / "Cargo.toml").write_text(manifest)
        (workspace / "build-identity.txt").write_text(f"backend=xberg_brn\nengine_identity={identity()}\n")
        lock = tool / "adapted.Cargo.lock"
        if lock.exists():
            shutil.copyfile(lock, workspace / "Cargo.lock")
        receipt = {"archive_sha256": ARCHIVE, "patches": [{"name": p, "sha256": h} for p, h, _, _ in PATCHES], "engine_identity": identity(), "files": expected}
        (stage / "source-manifest.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
        if output.exists():
            # Never replace an occupied/mutated generation, even with seemingly valid labels.
            for name in ["source-manifest.json", "workspace/Cargo.toml", "workspace/build-identity.txt"]:
                if not (output / name).is_file() or (output / name).read_bytes() != (stage / name).read_bytes():
                    raise ValueError("occupied/mismatched generated workspace")
            if file_hashes(output / "source") != expected:
                raise ValueError("occupied/mismatched generated source")
            if lock.exists() and (output / "workspace/Cargo.lock").read_bytes() != lock.read_bytes():
                raise ValueError("generated lock differs from committed adapted lock")
        else:
            stage.rename(output)
    return output / "workspace/Cargo.toml"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--archive", type=Path, required=True)
    args = parser.parse_args()
    print(prepare(args.repo, args.archive, args.repo / "target/xberg-adapted"))


if __name__ == "__main__":
    main()
