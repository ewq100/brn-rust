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


def qualify_app(app, host_cli=None):
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
        # Fixtures are public/synthetic inputs, not engine/build lookup paths.
        import hashlib
        archive_sha = "f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d"
        material = archive_sha + "\nutf8-identity-v1\n"
        for patch in ["0001-docx-table-grid-spans.patch", "0003-docx-namespaces.patch",
                      "0004-pptx-order-hidden.patch", "0005-docx-comment-references.patch"]:
            material += f"{patch} {packaging.digest(ROOT / 'patches/xberg/1.3.10' / patch)}\n"
        identities = {"xberg": f"xberg/1.3.10/stock/{archive_sha}",
                      "xberg_brn": "xberg/1.3.10/brn/" + hashlib.sha256(material.encode()).hexdigest()}
        fixtures = [("pdf", ROOT / "crates/brn-threads-intake/tests/fixtures/harbor-study.pdf")]
        fixtures += [(kind, ROOT / f"crates/brn-intake/tests/fixtures/{name}") for kind, name in
                     [("docx", "harbor.docx"), ("pptx", "quay.pptx"), ("xlsx", "forecast.xlsx"),
                      ("eml", "single.eml"), ("html", "textual.html")]]
        cases = [(kind, path.read_bytes()) for kind, path in fixtures]
        cases += [("text", b"Portable source\r\n"), ("markdown", b"# Portable source\n")]
        captures = []
        for backend, executable in [("xberg", "brn-xberg-helper"), ("xberg_brn", "brn-xberg-helper-brn")]:
            for kind, data in cases:
                request = {"schema": 1, "backend": backend, "kind": kind, "limits": {}, "bytes": list(data)}
                converted = subprocess.run([str(binaries / executable)], input=json.dumps(request).encode(),
                                           env={}, capture_output=True, check=True, timeout=35)
                result = json.loads(converted.stdout)
                assert set(result) == {"schema", "backend", "engine_identity", "source_sha256", "markdown", "assets", "gaps"}
                assert result["schema"] == 1 and result["backend"] == backend
                assert result["engine_identity"] == identities[backend]
                assert result["source_sha256"] == list(hashlib.sha256(data).digest())
                assert result["markdown"] and "\0" not in result["markdown"]
                assert isinstance(result["assets"], list) and isinstance(result["gaps"], list)
                assert all(isinstance(gap, str) for gap in result["gaps"])
                captures.append({"kind": kind, "request": request, "result": result})
        if host_cli is not None:
            # A test host, not an additional shipped binary. Its only helper
            # discovery is current_exe siblings copied from the relocated app.
            host_bin = root / "recipient CLI bin"
            host_bin.mkdir()
            cli = host_bin / "brn"
            shutil.copy2(Path(host_cli).resolve(strict=True), cli)
            for name in ("brn-xberg-helper", "brn-xberg-helper-brn"):
                shutil.copy2(binaries / name, host_bin / name)
                assert packaging.digest(host_bin / name) == packaging.digest(binaries / name)
            home = root / "recipient home"
            home.mkdir()
            environment = {"HOME": str(home), "CODEX_HOME": str(home), "TMPDIR": str(root),
                           "PATH": "/usr/bin:/bin:/usr/sbin:/sbin"}
            workspace = root / "CLI workspace"
            calls = []
            def call(*arguments):
                completed = subprocess.run([str(cli), "--json", "--data-dir", str(workspace), *arguments],
                                           env=environment, capture_output=True, timeout=45)
                calls.append({"arguments": arguments, "exit": completed.returncode,
                              "stdout": completed.stdout.decode(), "stderr": completed.stderr.decode()})
                assert completed.returncode == 0, calls[-1]
                envelope = json.loads(completed.stdout)
                assert envelope["status"] == "ok", envelope
                return envelope
            call("init", "--purpose", "test")
            assert call("settings")["result"]["document_conversion"] == {"converter": "xberg", "ai": False}
            for backend, choice in [("xberg", "xberg"), ("xberg_brn", "xberg-brn")]:
                call("document-conversion", "--converter", choice, "--ai", "off")
                for kind, data in cases:
                    extension = {"text": "txt", "markdown": "md"}.get(kind, kind)
                    source = root / f"input.{extension}"
                    source.write_bytes(data)
                    imported = call("intake", "--source", str(source), "--intent", "full", "--title", kind,
                                    "--key", f"{backend}-{kind}")
                    assert imported["outcome"] == "Applied", imported
            records = call("records")["result"]
            notes = [record["data"]["Note"] for record in records if "Note" in record["data"]]
            sources = [record["data"]["Source"] for record in records if "Source" in record["data"]]
            assert len(notes) == 16 and len(sources) == 8
            for note in notes:
                conversion = note["import"]["report"]["conversion"]
                assert note["protected"] and note["markdown"].strip()
                assert conversion["settings"]["ai"] is False
                assert conversion["engine_identity"] == identities[conversion["settings"]["converter"]]
            captures.append({"relocated_cli": {"calls": calls, "records": records,
                                               "environment_names": sorted(environment),
                                               "shipping_binary_set_unchanged": sorted(p.name for p in binaries.iterdir())}})
        if os.environ.get("BRN_PACKAGE_CAPTURES"):
            Path(os.environ["BRN_PACKAGE_CAPTURES"]).write_text(json.dumps(captures, indent=2) + "\n")

    print("Packaged headless startup/reopen, foreign-data preservation and both helpers/all eight typed fixtures passed")


class LauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.binary = self.root / "builder binary ' ü"
        self.binary.write_text('#!/bin/bash\nprintf "%s\\n" "$@" > "$BRN_TEST_ARGS"\nexit "${BRN_TEST_EXIT:-0}"\n')
        self.binary.chmod(0o700)
        self.helper = self.root / "brn-xberg-helper"
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
            args += ["--xberg-helper", str(self.helper), "--xberg-helper-brn", str(self.helper)]
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

    def test_either_missing_helper_refuses_before_creating_output(self):
        for flag in ["--xberg-helper", "--xberg-helper-brn"]:
            result = self.assemble(extra=(flag, str(self.root / "missing")))
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((self.root / "Preview.app").exists())

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
        (self.root / "Preview.app/Contents/Resources/bin/brn-xberg-helper-brn").unlink()
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
            notices = root / "notices"
            notices.mkdir()
            (notices / "README.txt").write_text("Synthetic notices")
            (notices / "index.json").write_text(json.dumps({"files": {"README.txt": packaging.digest(notices / "README.txt")}}))
            output = packaging.package(binary, binary, binary, manifest, "a" * 40, root / "delivery", notices)
            receipt = json.loads(next(output.glob("*.manifest.json")).read_text())
            for name in ("brn-desktop", "brn-xberg-helper", "brn-xberg-helper-brn"):
                installed = output / "BRN Threads.app/Contents/Resources/bin" / name
                self.assertEqual(receipt["binaries"][name]["sha256"], packaging.digest(installed))
            subprocess.run(["codesign", "--verify", "--strict", "--deep", str(output / "BRN Threads.app")],
                           check=True, capture_output=True)
            archive = next(output.glob("*.zip"))
            self.assertEqual(receipt["archive"]["sha256"], packaging.digest(archive))
            with zipfile.ZipFile(archive) as zipped:
                self.assertIn("BRN Threads.app/Contents/Resources/build-manifest.json", zipped.namelist())
            self.assertFalse((output / ".incomplete").exists())
            # Never copy unindexed files or follow a notice-directory symlink.
            extra = notices / "unindexed"
            extra.write_text("not approved for packaging")
            with self.assertRaisesRegex(ValueError, "every retained file"):
                packaging.package(binary, binary, binary, manifest, "a" * 40, root / "extra", notices)
            extra.unlink()
            link = notices / "linked"
            link.symlink_to(notices / "README.txt")
            index = {"README.txt": packaging.digest(notices / "README.txt"), "linked": packaging.digest(link)}
            (notices / "index.json").write_text(json.dumps({"files": index}))
            with self.assertRaisesRegex(ValueError, "Symlinks"):
                packaging.package(binary, binary, binary, manifest, "a" * 40, root / "linked", notices)
            link.unlink()
            (notices / "index.json").write_text(json.dumps({"files": {"README.txt": packaging.digest(notices / "README.txt")}}))
            with self.assertRaises(ValueError):
                packaging.package(binary, binary, binary, manifest, "a" * 40, output, notices)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--qualify-app":
        import argparse
        parser = argparse.ArgumentParser()
        parser.add_argument("--qualify-app", required=True)
        parser.add_argument("--host-cli", type=Path)
        args = parser.parse_args()
        qualify_app(args.qualify_app, args.host_cli)
    else:
        unittest.main()
