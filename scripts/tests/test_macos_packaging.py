"""Portable launcher/package witnesses, using synthetic files and homes only."""
import importlib.util
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("macos_packaging", ROOT / "scripts/package-macos-preview.py")
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


def qualify_app(app):
    """Actual packaged binaries; never starts GPUI or accesses a real workspace."""
    app = Path(app).resolve()
    binaries = app / "Contents/Resources/bin"
    with tempfile.TemporaryDirectory(prefix="brn-packaged-fixtures-") as temp:
        root = Path(temp).resolve()
        desktop = binaries / "brn-desktop"
        for _ in range(2):
            subprocess.run([str(desktop), "--data-dir", str(root / "workspace"),
                            "--headless-check", "startup"], check=True, capture_output=True)
        assert (root / "workspace/threads.sqlite3").is_file()
        foreign = root / "foreign"
        foreign.mkdir()
        (foreign / "brn.sqlite").write_bytes(b"synthetic old BRN data")
        result = subprocess.run([str(desktop), "--data-dir", str(foreign),
                                 "--headless-check", "startup"], capture_output=True)
        assert result.returncode != 0
        assert list(foreign.iterdir()) == [foreign / "brn.sqlite"]
        assert (foreign / "brn.sqlite").read_bytes() == b"synthetic old BRN data"
        eml = b"From: test@example.invalid\r\nTo: test@example.invalid\r\nSubject: Synthetic\r\nContent-Type: text/plain\r\n\r\nPortable helper witness.\r\n"
        converted = subprocess.run([str(binaries / "brn-intake-helper")],
                                   input=json.dumps({"kind": "eml", "bytes": list(eml)}).encode(),
                                   env={}, capture_output=True, check=True)
        result = json.loads(converted.stdout)
        assert "Portable helper witness." in json.dumps(result)
    print("Packaged headless startup/reopen, foreign-data preservation and synthetic helper passed")


class LauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.binary = self.root / "builder binary ' ü"
        self.binary.write_text('#!/bin/bash\nprintf "%s\\n" "$@" > "$BRN_TEST_ARGS"\nexit "${BRN_TEST_EXIT:-0}"\n')
        self.binary.chmod(0o700)
        self.helper = self.root / "brn-intake-helper"
        self.helper.write_text("#!/bin/bash\nexit 0\n")
        self.helper.chmod(0o700)
        self.home = self.root / "recipient home ' ü"
        self.home.mkdir()
        self.env = dict(os.environ, HOME=str(self.home), BRN_TEST_ARGS=str(self.root / "args"),
                        BRN_LAUNCH_LOG_DIR=str(self.root / "logs"), BRN_LAUNCH_TEST_NO_OPEN="1")

    def tearDown(self):
        self.temp.cleanup()

    def assemble(self, name="Preview.app", version="v0.1.0-preview.1", helper=True, extra=()):
        args = ["bash", str(ROOT / "scripts/make-macos-app.sh"), "--output", str(self.root / name),
                "--binary", str(self.binary), "--preview-version", version]
        if helper:
            args += ["--intake-helper", str(self.helper)]
        return subprocess.run(args + list(extra), capture_output=True, text=True)

    def test_explicit_launcher_existing_contract(self):
        # The legacy script owns/canonicalizes its own mktemp fixture. Keep its
        # normal physical temp parent instead of adding an unnecessary nested one.
        result = subprocess.run(["bash", str(ROOT / "scripts/test-make-macos-app.sh")],
                                capture_output=True, text=True,
                                env=dict(os.environ, TMPDIR=str(Path(tempfile.gettempdir()).resolve())))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relocation_and_upgrade_keep_recipient_workspace(self):
        workspace = self.home / "Library/Application Support/BRN Threads Preview/workspace"
        for number in (1, 2):
            name = f"Preview{number}.app"
            self.assertEqual(self.assemble(name, f"v0.1.0-preview.{number}").returncode, 0)
            destination = self.root / f"relocated ü {number}" / name
            destination.parent.mkdir()
            shutil.move(str(self.root / name), destination)
            launcher = destination / "Contents/MacOS/BRN-Threads"
            self.assertNotIn(str(self.root), launcher.read_text())
            self.assertNotIn(str(self.binary), launcher.read_text())
            subprocess.run([str(launcher)], check=True, env=self.env, capture_output=True)
            self.assertEqual((self.root / "args").read_text().splitlines(), ["--data-dir", str(workspace)])
            # The launcher passes the directory to core; it creates no data itself.
            self.assertFalse(workspace.exists())
            with (destination / "Contents/Info.plist").open("rb") as stream:
                info = plistlib.load(stream)
            self.assertEqual(info["LSMinimumSystemVersion"], "15.0")
            self.assertEqual(info["CFBundleShortVersionString"], "0.1.0")

    def test_invalid_input_and_occupied_output_refuse(self):
        self.assertNotEqual(self.assemble(helper=False).returncode, 0)
        for version in ("v0.1.0", "v0.1.0-preview.0", "../evil", "v0.1.0-preview.1$(touch injected)"):
            self.assertNotEqual(self.assemble(version=version).returncode, 0)
        self.assertNotEqual(self.assemble(extra=("--data-dir", str(self.root))).returncode, 0)
        self.assertEqual(self.assemble().returncode, 0)
        before = (self.root / "Preview.app/Contents/Info.plist").read_bytes()
        self.assertNotEqual(self.assemble().returncode, 0)
        self.assertEqual((self.root / "Preview.app/Contents/Info.plist").read_bytes(), before)

    def test_missing_bundled_helper_and_failure_are_visible(self):
        self.assertEqual(self.assemble().returncode, 0)
        launcher = self.root / "Preview.app/Contents/MacOS/BRN-Threads"
        result = subprocess.run([str(launcher)], env=dict(self.env, BRN_TEST_EXIT="7"), capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("status 7", (self.root / "logs/startup.log").read_text())
        (self.root / "Preview.app/Contents/Resources/bin/brn-intake-helper").unlink()
        result = subprocess.run([str(launcher)], env=self.env, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("helper is missing", (self.root / "logs/startup.log").read_text())


@unittest.skipUnless(platform.system() == "Darwin" and platform.machine() == "arm64", "Apple Silicon signing witness")
class NativePackageTests(unittest.TestCase):
    def test_actual_macho_signing_extraction_manifest_and_collision(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            source = root / "synthetic.c"
            source.write_text("int main(void) { return 0; }\n")
            binary = root / "synthetic"
            subprocess.run(["xcrun", "clang", "-arch", "arm64", "-mmacosx-version-min=15.0",
                            str(source), "-o", str(binary)], check=True, capture_output=True)
            manifest = ROOT / "release/preview.json"
            output = packaging.package(binary, binary, manifest, "a" * 40, root / "delivery")
            receipt = json.loads(next(output.glob("*.manifest.json")).read_text())
            for name in ("brn-desktop", "brn-intake-helper"):
                installed = output / "BRN Threads.app/Contents/Resources/bin" / name
                self.assertEqual(receipt["binaries"][name]["sha256"], packaging.digest(installed))
            subprocess.run(["codesign", "--verify", "--strict", "--deep", str(output / "BRN Threads.app")],
                           check=True, capture_output=True)
            archive = next(output.glob("*.zip"))
            self.assertEqual(receipt["archive"]["sha256"], packaging.digest(archive))
            with zipfile.ZipFile(archive) as zipped:
                self.assertIn("BRN Threads.app/Contents/Resources/build-manifest.json", zipped.namelist())
            self.assertFalse((output / ".incomplete").exists())
            with self.assertRaises(ValueError):
                packaging.package(binary, binary, manifest, "a" * 40, output)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--qualify-app":
        qualify_app(sys.argv[2])
    else:
        unittest.main()
