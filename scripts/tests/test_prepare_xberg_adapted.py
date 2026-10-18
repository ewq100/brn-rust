import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("prepare", ROOT / "scripts/prepare-xberg-adapted.py")
p = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(p)
ARCHIVE = Path.home() / ".cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/xberg-1.3.10.crate"


class PrepareTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name).resolve()
        for part in ["patches/xberg/1.3.10", "tools/xberg-intake-helper"]:
            shutil.copytree(ROOT / part, self.repo / part)
        subprocess.run(["git", "init", "--quiet", str(self.repo)], check=True)
        self.output = self.repo / "target/xberg-adapted"

    def tearDown(self):
        self.temp.cleanup()

    def test_complete_reproduction_and_stock_unchanged(self):
        before = ARCHIVE.read_bytes()
        manifest = p.prepare(self.repo, ARCHIVE, self.output)
        self.assertEqual(len(p.file_hashes(self.output / "source")), 765)
        self.assertIn('[patch.crates-io]', manifest.read_text())
        self.assertNotIn('[patch.crates-io]', (ROOT / "tools/xberg-intake-helper/Cargo.toml").read_text())
        self.assertEqual(ARCHIVE.read_bytes(), before)
        first = p.file_hashes(self.output)
        p.prepare(self.repo, ARCHIVE, self.output)
        self.assertEqual(first, p.file_hashes(self.output))

    def test_bad_archive(self):
        bad = self.repo / "bad.crate"
        bad.write_bytes(b"not the pinned archive")
        with self.assertRaisesRegex(ValueError, "archive SHA256"):
            p.prepare(self.repo, bad, self.output)

    def test_changed_patch(self):
        patch = self.repo / "patches/xberg/1.3.10" / p.PATCHES[0][0]
        patch.write_bytes(patch.read_bytes() + b"\n")
        with self.assertRaisesRegex(ValueError, "patch/source manifest"):
            p.prepare(self.repo, ARCHIVE, self.output)

    def test_occupied_and_mutated_source(self):
        self.output.mkdir(parents=True)
        with self.assertRaisesRegex(ValueError, "occupied"):
            p.prepare(self.repo, ARCHIVE, self.output)
        self.output.rmdir()
        p.prepare(self.repo, ARCHIVE, self.output)
        (self.output / "source/src/lib.rs").write_text("wrong source")
        with self.assertRaisesRegex(ValueError, "mismatched generated source"):
            p.prepare(self.repo, ARCHIVE, self.output)

    def test_archive_path_and_link_attacks(self):
        for name, kind in [("xberg-1.3.10/../escape",tarfile.REGTYPE),("/absolute",tarfile.REGTYPE),("xberg-1.3.10/link",tarfile.SYMTYPE)]:
            with self.subTest(name=name):
                data = io.BytesIO()
                with tarfile.open(fileobj=data,mode="w") as tar:
                    info = tarfile.TarInfo(name); info.type = kind
                    tar.addfile(info,io.BytesIO())
                data.seek(0)
                with tarfile.open(fileobj=data) as tar, self.assertRaises(ValueError):
                    p.safe_members(tar)

    def test_output_symlink_and_outside_root(self):
        other = self.repo / "elsewhere"
        other.mkdir()
        self.output.parent.mkdir()
        self.output.symlink_to(other, target_is_directory=True)
        with self.assertRaisesRegex(ValueError,"symlink"):
            p.prepare(self.repo, ARCHIVE,self.output)
        with self.assertRaises(ValueError):
            p.prepare(self.repo,ARCHIVE,other)

    def cleanup_from_setup_action(self):
        action = (ROOT / ".github/actions/setup-rust/action.yml").read_text()
        step = action.split("    - name: Discard restored generated Xberg workspace\n", 1)[1]
        step = step.split("    - name:", 1)[0]
        script = textwrap.dedent(step.split("      run: |\n", 1)[1])
        return subprocess.run(
            ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", script],
            cwd=self.repo, capture_output=True, text=True,
        )

    def assert_cleanup_reproduces_generation(self):
        foreign = self.repo / "target/document-helpers/artifacts/foreign-sentinel"
        foreign.parent.mkdir(parents=True, exist_ok=True)
        foreign.write_bytes(b"preserve cached helper artifacts")
        sibling = self.repo / "target/unrelated-source/sentinel"
        sibling.parent.mkdir()
        sibling.write_bytes(b"preserve foreign sibling")
        stale = p.file_hashes(self.output)
        with self.assertRaisesRegex(ValueError, "occupied/mismatched generated workspace"):
            p.prepare(self.repo, ARCHIVE, self.output)
        self.assertEqual(p.file_hashes(self.output), stale)
        cleaned = self.cleanup_from_setup_action()
        self.assertEqual(cleaned.returncode, 0, cleaned.stderr)
        self.assertFalse(self.output.exists())
        manifest = p.prepare(self.repo, ARCHIVE, self.output)
        receipt = json.loads((self.output / "source-manifest.json").read_text())
        self.assertEqual(len(receipt["files"]), 765)
        self.assertEqual(p.file_hashes(self.output / "source"), receipt["files"])
        self.assertEqual(receipt["archive_sha256"], p.ARCHIVE)
        self.assertEqual(receipt["engine_identity"], p.identity())
        self.assertEqual(
            (self.output / "workspace/build-identity.txt").read_text(),
            f"backend=xberg_brn\nengine_identity={p.identity()}\n",
        )
        self.assertIn(f'path = "{self.repo}/crates/brn-intake"', manifest.read_text())
        self.assertIn('[patch.crates-io]\nxberg = { path = "../source" }', manifest.read_text())
        self.assertEqual(
            (self.output / "workspace/Cargo.lock").read_bytes(),
            (self.repo / "tools/xberg-intake-helper/adapted.Cargo.lock").read_bytes(),
        )
        self.assertEqual(foreign.read_bytes(), b"preserve cached helper artifacts")
        self.assertEqual(sibling.read_bytes(), b"preserve foreign sibling")

    def test_setup_cleanup_rebuilds_partial_cached_generation(self):
        (self.output / "workspace").mkdir(parents=True)
        (self.output / "workspace/Cargo.toml").write_text("incomplete cached workspace\n")
        self.assert_cleanup_reproduces_generation()

    def test_setup_cleanup_rebuilds_checkout_specific_cached_generation(self):
        manifest = p.prepare(self.repo, ARCHIVE, self.output)
        stale = manifest.read_text().replace(str(self.repo), "/previous-ci-checkout")
        self.assertNotEqual(stale, manifest.read_text())
        manifest.write_text(stale)
        self.assert_cleanup_reproduces_generation()

    @unittest.skipIf(os.name == "nt", "Unix CI symlink boundary fixture")
    def test_setup_cleanup_unlinks_leaf_without_touching_foreign_tree(self):
        foreign = self.repo / "foreign"
        foreign.mkdir()
        sentinel = foreign / "sentinel"
        sentinel.write_bytes(b"outside derived workspace")
        self.output.parent.mkdir()
        self.output.symlink_to(foreign, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            p.prepare(self.repo, ARCHIVE, self.output)
        cleaned = self.cleanup_from_setup_action()
        self.assertEqual(cleaned.returncode, 0, cleaned.stderr)
        self.assertFalse(self.output.is_symlink())
        self.assertEqual(sentinel.read_bytes(), b"outside derived workspace")

    @unittest.skipIf(os.name == "nt", "Unix CI symlink boundary fixture")
    def test_setup_cleanup_refuses_symlinked_target_parent(self):
        foreign = self.repo / "foreign"
        (foreign / "xberg-adapted").mkdir(parents=True)
        sentinel = foreign / "xberg-adapted/sentinel"
        sentinel.write_bytes(b"outside checkout target")
        self.output.parent.symlink_to(foreign, target_is_directory=True)
        cleaned = self.cleanup_from_setup_action()
        self.assertNotEqual(cleaned.returncode, 0)
        self.assertEqual(sentinel.read_bytes(), b"outside checkout target")


if __name__ == "__main__":
    unittest.main()
