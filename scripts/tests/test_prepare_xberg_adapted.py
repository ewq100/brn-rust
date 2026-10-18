import importlib.util
import io
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
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


if __name__ == "__main__":
    unittest.main()
