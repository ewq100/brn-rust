"""Exercise real macOS verification/replacement, with only transport/faults substituted.

No production URL, signature or filesystem verification bypass is shipped. The test
copy substitutes absolute tools to serve local package bytes and inject crashes.
"""
import importlib.util
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import signal
import subprocess
import tempfile
import time
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("updater_packaging", ROOT / "scripts/package-macos-preview.py")
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


@unittest.skipUnless(platform.system() == "Darwin" and platform.machine() == "arm64", "real macOS updater witness")
class UpdaterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory(prefix="brn-update-packages-")
        cls.packages = Path(cls.scratch.name).resolve()
        source = cls.packages / "fixture.c"
        source.write_text('#include <stdlib.h>\n#include <unistd.h>\nint main(void) { if(getenv("BRN_SYNTHETIC_WAIT")) sleep(60); return 0; }\n')
        binary = cls.packages / "fixture"
        subprocess.run(["xcrun", "clang", "-arch", "arm64", "-mmacosx-version-min=15.0", str(source), "-o", str(binary)], check=True, capture_output=True)
        notices = cls.packages / "notices"
        notices.mkdir()
        (notices / "README").write_text("Synthetic fixture, not a product qualification")
        (notices / "index.json").write_text(json.dumps({"files": {"README": packaging.digest(notices / "README")}}))
        cls.deliveries = []
        for number in (1, 2):
            manifest = json.loads((ROOT / "release/preview.json").read_text())
            manifest.update(version=f"v0.2.0-alpha.{number}", build_number=100 + number)
            path = cls.packages / f"manifest{number}.json"
            path.write_text(json.dumps(manifest))
            cls.deliveries.append(packaging.package(binary, binary, binary, path, str(number) * 40, cls.packages / f"delivery{number}", notices))

    @classmethod
    def tearDownClass(cls):
        cls.scratch.cleanup()

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="brn-update-test-")
        self.root = Path(self.scratch.name).resolve()
        self.app = self.root / "Applications ü ' /BRN Threads.app"
        self.app.parent.mkdir()
        shutil.copytree(self.deliveries[0] / "BRN Threads.app", self.app)
        self.assets = self.root / "assets"
        shutil.copytree(self.deliveries[1], self.assets)
        # Sentinels model files outside the bundle. Real SQLite reopen is a separate gate.
        self.data = self.root / "synthetic home"
        self.data.mkdir()
        for name in ("notes.sqlite3", "settings.json", "draft", "copilot-credential-sentinel"):
            (self.data / name).write_bytes(b"synthetic only\x00" + name.encode())
        self.before = {p.name: p.read_bytes() for p in self.data.iterdir()}
        self.env = dict(os.environ, HOME=str(self.data), FIXTURE_ASSETS=str(self.assets), FIXTURE_ROOT=str(self.root))
        self.curl = self.root / "curl"
        self.curl.write_text('''#!/bin/bash
set -eu
printf '%s\\n' "$@" >> "$FIXTURE_ROOT/requests"
[[ ! -f "$FIXTURE_ROOT/offline" ]] || exit 7
if [[ -f "$FIXTURE_ROOT/pause-download" ]]; then touch "$FIXTURE_ROOT/stopped"; sleep 60; fi
while (($#)); do
 if [[ "$1" == --output ]]; then output="$2"; shift 2; else url="$1"; shift; fi
done
[[ "$url" == https://github.com/ewq100/brn-rust/releases/download/v0.2.0-alpha.2/* ]] || exit 9
/bin/cp "$FIXTURE_ASSETS/${url##*/}" "$output"
''')
        self.curl.chmod(0o700)
        self.updater = self.app / "Contents/Resources/bin/brn-update"
        self.updater.write_text(self.updater.read_text().replace("/usr/bin/curl", str(self.curl)))

    def tearDown(self):
        self.assertEqual({p.name: p.read_bytes() for p in self.data.iterdir()}, self.before)
        self.scratch.cleanup()

    def run_update(self):
        return subprocess.run(["/bin/bash", str(self.updater), "v0.2.0-alpha.2"], env=self.env, capture_output=True, text=True, timeout=30)

    def transaction(self):
        return next(self.app.parent.glob(".BRN Threads.app.update-??????"))

    def recover(self):
        return subprocess.run(["/bin/bash", str(self.transaction() / "brn-update"), "--recover"], env=self.env, capture_output=True, text=True, timeout=10)

    def build(self):
        return json.loads((self.app / "Contents/Resources/build-manifest.json").read_text())["build_number"]

    def receipt(self):
        return next(self.assets.glob("*.manifest.json"))

    def refresh_checksums(self):
        paths = [*self.assets.glob("*.zip"), self.receipt()]
        (self.assets / "SHA256SUMS").write_text("".join(f"{packaging.digest(p)}  {p.name}\n" for p in paths))

    def refresh_archive(self):
        archive = next(self.assets.glob("*.zip"))
        data = json.loads(self.receipt().read_text())
        data["archive"].update(sha256=packaging.digest(archive), bytes=archive.stat().st_size)
        self.receipt().write_text(json.dumps(data))
        self.refresh_checksums()

    def test_verified_replacement_and_backup(self):
        result = self.run_update()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.build(), 102)
        old = self.transaction() / "previous.app/Contents/Resources/build-manifest.json"
        self.assertEqual(json.loads(old.read_text())["build_number"], 101)
        requests = (self.root / "requests").read_text()
        self.assertIn("--proto\n=https\n--proto-redir\n=https", requests)
        self.assertIn("/releases/download/v0.2.0-alpha.2/", requests)
        self.assertFalse((self.app.parent / ".BRN Threads.app.update-lock").exists())

    def test_update_without_developer_tools(self):
        # Xcode tool shims refuse this setting. codesign/plutil/ditto still work.
        self.env["DEVELOPER_DIR"] = "/nonexistent-brn-developer-tools"
        result = self.run_update()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.build(), 102)

    def test_bad_downloads_keep_current_app(self):
        for defect in ("offline", "missing", "checksum", "wrong-version", "old-build", "platform"):
            with self.subTest(defect=defect):
                manifest = self.receipt()
                saved = manifest.read_bytes()
                if defect == "offline": (self.root / "offline").touch()
                elif defect == "missing": manifest.rename(manifest.with_suffix(".missing"))
                elif defect == "checksum": manifest.write_text("corrupt")
                else:
                    data = json.loads(saved)
                    data[{"wrong-version": "version", "old-build": "build_number", "platform": "platform"}[defect]] = {"wrong-version": "v9.9.9-alpha.2", "old-build": 101, "platform": "macos-x86_64"}[defect]
                    manifest.write_text(json.dumps(data))
                    self.refresh_checksums()
                result = self.run_update()
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertEqual(self.build(), 101)
                recovery = self.recover()
                self.assertEqual(recovery.returncode, 0, recovery.stderr)
                shutil.rmtree(self.transaction())
                (self.root / "offline").unlink(missing_ok=True)
                manifest.with_suffix(".missing").unlink(missing_ok=True)
                manifest.write_bytes(saved)
                self.refresh_checksums()

    def test_corrupt_zip_with_valid_checksum_refuses(self):
        archive = next(self.assets.glob("*.zip"))
        archive.write_bytes(b"not a zip")
        data = json.loads(self.receipt().read_text())
        data["archive"].update(sha256=packaging.digest(archive), bytes=archive.stat().st_size)
        self.receipt().write_text(json.dumps(data))
        self.refresh_checksums()
        self.assertNotEqual(self.run_update().returncode, 0)
        self.assertEqual(self.build(), 101)

    def test_traversal_and_symlink_archive_entries_refuse_before_extraction(self):
        archive = next(self.assets.glob("*.zip"))
        saved = archive.read_bytes()
        for name, mode in (("BRN Threads.app/../../escaped", 0o100644), ("BRN Threads.app/link", 0o120777)):
            with self.subTest(name=name):
                with zipfile.ZipFile(archive, "a") as zipped:
                    item = zipfile.ZipInfo(name)
                    item.create_system = 3
                    item.external_attr = mode << 16
                    zipped.writestr(item, "../../escaped")
                self.refresh_archive()
                result = self.run_update()
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertFalse((self.transaction() / "extracted").exists())
                self.assertFalse((self.root / "escaped").exists())
                self.assertEqual(self.build(), 101)
                self.assertEqual(self.recover().returncode, 0)
                shutil.rmtree(self.transaction())
                archive.write_bytes(saved)
                self.refresh_archive()

    def test_wrong_bundle_or_tampered_signed_app_refuses(self):
        archive = next(self.assets.glob("*.zip"))
        for defect in ("bundle", "signature", "binary"):
            with self.subTest(defect=defect):
                app = self.assets / "BRN Threads.app"
                shutil.rmtree(app)
                shutil.copytree(self.deliveries[1] / app.name, app)
                if defect == "bundle":
                    plist = app / "Contents/Info.plist"
                    value = plistlib.loads(plist.read_bytes())
                    value["CFBundleIdentifier"] = "dev.unrelated"
                    plist.write_bytes(plistlib.dumps(value))
                elif defect == "signature":
                    (app / "Contents/MacOS/BRN-Threads").write_text("#!/bin/bash\nexit 0\n")
                else:
                    (app / "Contents/Resources/bin/brn-desktop").write_bytes(b"invalid Mach-O")
                archive.unlink()
                subprocess.run(["/usr/bin/ditto", "-c", "-k", "--keepParent", str(app), str(archive)], check=True)
                self.refresh_archive()
                result = self.run_update()
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertEqual(self.build(), 101)
                self.assertEqual(self.recover().returncode, 0)
                shutil.rmtree(self.transaction())

    def test_external_library_inventory_refuses(self):
        data = json.loads(self.receipt().read_text())
        data["binaries"]["brn-desktop"]["libraries"] = ["/opt/homebrew/lib/unshipped.dylib"]
        self.receipt().write_text(json.dumps(data))
        self.refresh_checksums()
        result = self.run_update()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("External runtime dependency", result.stderr)
        self.assertEqual(self.build(), 101)

    def test_unsupported_os_and_architecture(self):
        saved = self.updater.read_text()
        for tool, output in (("sw_vers", "14.0"), ("uname", "x86_64")):
            with self.subTest(tool=tool):
                stub = self.root / tool
                stub.write_text(f'#!/bin/bash\nif [[ "${{1:-}}" == -s ]]; then echo Darwin; else echo {output}; fi\n')
                stub.chmod(0o700)
                self.updater.write_text(saved.replace(f"/usr/bin/{tool}", str(stub)))
                result = self.run_update()
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.build(), 101)
                if tool == "sw_vers":
                    self.assertEqual(self.recover().returncode, 0)
                    shutil.rmtree(self.transaction())
        self.updater.write_text(saved)

    def test_failed_replacement_restores_old_immediately(self):
        wrapper = self.root / "mv"
        wrapper.write_text('#!/bin/bash\n[[ "$1" != */extracted/* ]] || exit 1\nexec /bin/mv "$@"\n')
        wrapper.chmod(0o700)
        self.updater.write_text(self.updater.read_text().replace("/bin/mv", str(wrapper)))
        result = self.run_update()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.build(), 101)
        self.assertEqual(self.recover().returncode, 0)

    def test_launcher_refuses_during_update(self):
        lock = self.app.parent / ".BRN Threads.app.update-lock"
        lock.write_text("test lock")
        result = subprocess.run([str(self.app / "Contents/MacOS/BRN-Threads")],
                                env=dict(self.env, BRN_LAUNCH_TEST_NO_OPEN="1", BRN_LAUNCH_LOG_DIR=str(self.root / "logs")), capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("update or recovery", (self.root / "logs/startup.log").read_text())

    def stop_at(self, phase):
        wrapper = self.root / "mv"
        wrapper.write_text(f'''#!/bin/bash
set -eu
if [[ "$1" == "$FIXTURE_APP" ]]; then step=old; else step=new; fi
if [[ "$step" == {phase} ]]; then
 /bin/mv "$@"
 touch "$FIXTURE_ROOT/stopped"
 sleep 60
else /bin/mv "$@"; fi
''')
        wrapper.chmod(0o700)
        self.updater.write_text(self.updater.read_text().replace("/bin/mv", str(wrapper)))
        self.env["FIXTURE_APP"] = str(self.app)

    def start(self):
        return subprocess.Popen(["/bin/bash", str(self.updater), "v0.2.0-alpha.2"], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)

    def await_stop(self, process):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline and process.poll() is None:
            if (self.root / "stopped").exists(): return
            time.sleep(0.02)
        stdout, stderr = process.communicate(timeout=2)
        self.fail(stdout + stderr)

    def kill_and_recover(self, process):
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate(timeout=5)
        # Recovery must use real mv, not the deliberate crash injector.
        runner = self.transaction() / "brn-update"
        runner.write_text(runner.read_text().replace(str(self.root / "mv"), "/bin/mv"))
        result = self.recover()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_sigkill_between_renames_restores_old(self):
        self.stop_at("old")
        process = self.start()
        self.await_stop(process)
        self.assertFalse(self.app.exists())
        self.kill_and_recover(process)
        self.assertEqual(self.build(), 101)

    def test_sigkill_after_install_retains_new_and_backup(self):
        self.stop_at("new")
        process = self.start()
        self.await_stop(process)
        self.kill_and_recover(process)
        self.assertEqual(self.build(), 102)
        self.assertTrue((self.transaction() / "previous.app").is_dir())

    def test_interrupted_download_and_concurrent_attempt(self):
        (self.root / "pause-download").touch()
        process = self.start()
        self.await_stop(process)
        second = self.run_update()
        self.assertNotEqual(second.returncode, 0)
        self.assertIn("Another update", second.stderr)
        self.assertNotEqual(self.recover().returncode, 0)
        self.kill_and_recover(process)
        self.assertEqual(self.build(), 101)

    def interrupted_recovery(self, phase, pid_only=False):
        txn = self.app.parent / ".BRN Threads.app.update-fixture"
        txn.mkdir()
        self.app.rename(txn / "previous.app")
        runner = txn / "brn-update"
        original = self.updater.read_text() if self.updater.exists() else (txn / "previous.app/Contents/Resources/bin/brn-update").read_text()
        (txn / "app-path").write_text(str(self.app) + "\n")
        (txn / "pid").write_text("99999999\n")
        lock = self.app.parent / ".BRN Threads.app.update-lock"
        (txn / "owner").write_text(str(runner))
        os.link(txn / "owner", lock)
        self.env["FIXTURE_LOCK"] = str(lock)
        tool = {"claim": "kill", "restore": "mv", "unlock": "rm"}[phase]
        stub = self.root / f"recover-{tool}"
        before = 'touch "$FIXTURE_ROOT/stopped"; sleep 60' if phase != "restore" else ':'
        after = 'touch "$FIXTURE_ROOT/stopped"; sleep 60' if phase == "restore" else ':'
        stub.write_text(f'#!/bin/bash\n{before}\n/bin/{tool} "$@"\ncode=$?\n{after}\nexit "$code"\n')
        stub.chmod(0o700)
        runner.write_text(original.replace(f"/bin/{tool}", str(stub)))
        process = subprocess.Popen(["/bin/bash", str(runner), "--recover"], env=self.env,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        self.await_stop(process)
        second = subprocess.run(["/bin/bash", str(runner), "--recover"], env=self.env, capture_output=True, text=True, timeout=3)
        self.assertNotEqual(second.returncode, 0, "Concurrent recovery must refuse")
        if pid_only:
            os.kill(process.pid, signal.SIGKILL)
            process.wait(timeout=5)
            second = subprocess.run(["/bin/bash", str(runner), "--recover"], env=self.env, capture_output=True, text=True, timeout=3)
            self.assertNotEqual(second.returncode, 0, "Surviving filesystem worker must retain guard")
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate(timeout=5)
        runner.write_text(original)
        for _ in range(2):
            result = subprocess.run(["/bin/bash", str(runner), "--recover"], env=self.env, capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.build(), 101)
        self.assertFalse(lock.exists())

    def test_recovery_pid_only_sigkill_retains_worker_guard(self):
        self.interrupted_recovery("unlock", pid_only=True)

    def test_recovery_sigkill_after_claim_can_retry(self):
        self.interrupted_recovery("claim")

    def test_recovery_sigkill_after_restore_can_retry(self):
        self.interrupted_recovery("restore")

    def test_recovery_sigkill_before_unlock_can_retry(self):
        self.interrupted_recovery("unlock")

    def test_running_app_is_not_terminated(self):
        process = subprocess.Popen([str(self.app / "Contents/Resources/bin/brn-desktop")], env=dict(self.env, BRN_SYNTHETIC_WAIT="1"))
        try:
            time.sleep(0.1)
            result = self.run_update()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Quit BRN normally", result.stderr)
            self.assertIsNone(process.poll())
            self.assertEqual(self.build(), 101)
        finally:
            process.terminate()
            process.wait(timeout=5)

    def test_nonwritable_folder(self):
        self.app.parent.chmod(0o500)
        try:
            result = self.run_update()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("~/Applications", result.stderr)
            self.assertEqual(self.build(), 101)
        finally:
            self.app.parent.chmod(0o700)


def qualify_update(delivery_a, delivery_b, host_cli, evidence):
    """Real BRN A→B and store preservation; only HTTPS transport is substituted.

    Prepare local packages v0.2.0-alpha.1/build101 and alpha.2/build102 with the
    normal packager. These test tags must not be approved/published for this run.
    This is headless evidence, not clean-Mac/Gatekeeper/Copilot acceptance.
    """
    case = UpdaterTests("test_verified_replacement_and_backup")
    case.deliveries = [Path(delivery_a).resolve(), Path(delivery_b).resolve()]
    # Verify both unmodified source packages before A's transport-only test edit.
    for delivery in case.deliveries:
        subprocess.run(["/usr/bin/codesign", "--verify", "--strict", "--deep", str(delivery / "BRN Threads.app")], check=True)
    case.setUp()
    try:
        workspace = case.data / "Library/Application Support/BRN Threads Preview/workspace"
        cli = str(Path(host_cli).resolve())
        calls = []
        def call(*arguments, data=workspace):
            result = subprocess.run([cli, "--json", "--data-dir", str(data), *arguments], env=case.env, capture_output=True, text=True, timeout=30)
            assert result.returncode == 0, (arguments, result.stdout, result.stderr)
            payload = json.loads(result.stdout)
            assert payload["status"] == "ok", payload
            calls.append(list(arguments))
            return payload["result"]
        call("init", "--purpose", "test")
        note = call("note-create", "--title", "Synthetic update note", "--text", "Harbor water: 12 cm. Preserve UTF-8: õ ü.")
        project = call("project-create", "--title", "Synthetic update project")
        project_id = next(write["after"]["id"] for write in project["writes"] if "Project" in write.get("after", {}).get("data", {}))
        call("project-link", "--project", project_id, "--note", note["id"])
        chat = call("thread-start", "--title", "Synthetic update chat")
        call("queue", "--thread", chat["id"], "--text", "Unsent synthetic work; never dispatch")
        call("configure", "--provider", "Copilot", "--model", "synthetic-unavailable-model", "--effort", "low")
        call("delegation", "--choice", "review-first")
        token = case.root / "synthetic-edit-token"
        edit = call("begin-edit", "--note", note["id"], "--base", "1", "--token-out", str(token))
        call("update-buffer", "--session", edit["id"], "--generation", "1", "--text", "Unsaved synthetic draft", "--token-file", str(token))
        source = case.root / "synthetic-import.md"
        source.write_text("# Synthetic source\n\nThe quay closes at 18:00.\n")
        # Real helper discovery uses the supplied relocated package, not a build path.
        cli_dir = case.root / "test-cli-bin"
        cli_dir.mkdir()
        shutil.copy2(cli, cli_dir / "brn")
        for name in ("brn-xberg-helper", "brn-xberg-helper-brn"):
            shutil.copy2(case.app / "Contents/Resources/bin" / name, cli_dir / name)
        cli = str(cli_dir / "brn")
        call("intake", "--source", str(source), "--intent", "full", "--title", "Synthetic imported source", "--key", "update-import")
        call("intake", "--source", str(ROOT / "crates/brn-intake/tests/fixtures/harbor.docx"), "--intent", "full", "--title", "Synthetic figures", "--key", "update-figures")
        backup = case.root / "pre-update.sqlite3"
        call("backup", "--destination", str(backup))
        inspections = ("notes", "records", "history", "settings", "recovery", "projects", "threads")
        before = {name: call(name) for name in inspections}
        def files():
            return {str(p.relative_to(case.data)): packaging.digest(p) for p in case.data.rglob("*") if p.is_file()}
        hashes_before = files()
        assets = [record for record in before["records"] if "Asset" in record.get("data", {})]
        assert assets, "Real synthetic figure asset was not retained"

        result = case.run_update()
        assert result.returncode == 0, (result.stdout, result.stderr)
        assert case.build() == 102
        assert files() == hashes_before, "Updater changed an external data/settings/credential file"
        startup = subprocess.run([str(case.app / "Contents/Resources/bin/brn-desktop"), "--data-dir", str(workspace), "--headless-check", "startup"], env=case.env, capture_output=True, text=True, timeout=30)
        assert startup.returncode == 0, (startup.stdout, startup.stderr)
        after = {name: call(name) for name in inspections}
        assert after == before, "Real B startup changed expected saved state"
        restored = case.root / "restored-workspace"
        call("restore", "--source", str(backup), data=restored)
        assert call("notes", data=restored) == before["notes"]
        report = {
            "outcome": "passed",
            "scope": "local real BRN packages, headless A→B/store/backup/restore; HTTPS transport only substituted",
            "limitations": ["Not native acceptance", "Not clean-Mac installation or Gatekeeper acceptance",
                             "Synthetic Copilot selection/credential sentinel only; no live connection",
                             "Not publication or final integrated Agent A candidate"],
            "packages": [json.loads(next(p.glob("*.manifest.json")).read_text()) for p in case.deliveries],
            "checks": {"external_files_unchanged_before_B_reopen": True,
                       "logical_state_unchanged_after_B_headless_startup": True,
                       "previous_app_retained": (case.transaction() / "previous.app").is_dir(),
                       "pre_update_backup_restored_to_fresh_store": True},
            "synthetic": {"note_id": note["id"], "project_id": project_id, "thread_id": chat["id"],
                          "asset_count": len(assets), "inspection_commands": list(inspections),
                          "file_hashes_before_update": hashes_before},
            "commands": calls, "update_stdout": result.stdout, "update_stderr": result.stderr,
            "B_headless_stdout": startup.stdout,
        }
        Path(evidence).write_text(json.dumps(report, indent=2) + "\n")
        print("Real BRN local A→B, unchanged external files, B headless reopen and backup/restore passed")
    finally:
        # The case's byte sentinels are unchanged; the added real workspace is
        # intentionally retained only until this disposable qualification ends.
        case.scratch.cleanup()


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "--qualify-update":
        import argparse
        parser = argparse.ArgumentParser()
        parser.add_argument("--qualify-update", action="store_true")
        parser.add_argument("--delivery-a", required=True)
        parser.add_argument("--delivery-b", required=True)
        parser.add_argument("--host-cli", required=True)
        parser.add_argument("--evidence", required=True)
        args = parser.parse_args()
        qualify_update(args.delivery_a, args.delivery_b, args.host_cli, args.evidence)
    else:
        unittest.main()
