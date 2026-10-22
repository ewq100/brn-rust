#!/usr/bin/env python3
"""Prepare MAC-13 checked synthetic fixtures; inert unless --run is supplied."""
from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import stat
import subprocess
import sys
import threading
import time
import uuid

REPO = Path("/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust")
SOURCE_COMMIT = "3827d6a13953533c7322fd7157d975a32dee5cb2"
BUILD_RECEIPT = Path("/tmp/brn-mac13-preparation/build-receipt.json")
EXPECTED_ARTIFACT_DIR = Path("/Users/evokessler/.codex/artifacts/brn-mac13-3827d6a1")
FIXTURES = REPO / "docs/work/active/vacation-campaign/native-fixtures"
EXPECTED_FIXTURES = {
    "save-initial.txt": (78, "15c388ac54fea9240dedabf612bc6e6c7a4aca0c5fa5d3a785a32d7079f36c25"),
    "save-final.txt": (126, "210db7014db2a357be04eb2ad4072a77d1a0f717028768cd99fbcea3f156f48e"),
    "unsaved-recovery.txt": (152, "81e6224b4042ad42b4f68549095ec221cb8df312eb5cd39715b30f127a723529"),
}
MAX_OUTPUT = 1024 * 1024
COMMAND_TIMEOUT = 60.0
OVERALL_TIMEOUT = 600.0
GROUP_TERM_WAIT = 10.0
GROUP_KILL_WAIT = 10.0
CLEANUP_TIMEOUT = 22.0
CLEANUP_RESERVE = 30.0
START_DISK_FLOOR = 2 * 1024**3
LIVE_DISK_FLOOR = 1024**3


class PreparationError(RuntimeError):
    pass


class StopRequested(PreparationError):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> tuple[str, int]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
            size += len(chunk)
    return digest.hexdigest(), size


def atomic_json(path: Path, value: object) -> None:
    temp = path.with_name(path.name + f".tmp-{os.getpid()}-{uuid.uuid4().hex}")
    payload = json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True).encode("utf-8") + b"\n"
    try:
        with temp.open("xb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, path)
    finally:
        try:
            temp.unlink()
        except FileNotFoundError:
            pass
    try:
        fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    except OSError:
        pass


def group_present(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def wait_group_absent(pgid: int, seconds: float) -> bool:
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        if not group_present(pgid):
            return True
        time.sleep(0.1)
    return not group_present(pgid)


def stop_and_reap(proc: subprocess.Popen[bytes], signals_sent: list[str], errors: list[str]) -> bool:
    """Share one absolute cleanup deadline across every attempt for this child."""
    pgid = proc.pid
    deadline = getattr(proc, "_mac13_cleanup_deadline", None)
    if deadline is None:
        deadline = time.monotonic() + CLEANUP_TIMEOUT
        proc._mac13_cleanup_deadline = deadline

    def wait_absent(cap: float) -> bool:
        until = min(deadline, time.monotonic() + cap)
        while True:
            proc.poll()  # Reap the leader while checking any remaining descendants.
            if not group_present(pgid):
                return True
            remaining = until - time.monotonic()
            if remaining <= 0:
                return False
            time.sleep(min(0.1, remaining))

    def send(sig: int, label: str) -> None:
        try:
            os.killpg(pgid, sig)
            signals_sent.append(label)
        except ProcessLookupError:
            pass
        except BaseException as error:
            errors.append(f"{label} failed: {type(error).__name__}: {error}")

    try:
        proc.poll()
        present = group_present(pgid)
    except BaseException as error:
        errors.append(f"initial group probe failed: {type(error).__name__}: {error}")
        present = True
    if present:
        send(signal.SIGTERM, "SIGTERM")
        try:
            absent = wait_absent(GROUP_TERM_WAIT)
        except BaseException as error:
            errors.append(f"TERM wait failed: {type(error).__name__}: {error}")
            absent = False
        if not absent:
            send(signal.SIGKILL, "SIGKILL")
            try:
                wait_absent(GROUP_KILL_WAIT)
            except BaseException as error:
                errors.append(f"KILL wait failed: {type(error).__name__}: {error}")
    # Independently try reap and final group verification despite earlier errors.
    try:
        proc.wait(timeout=max(0.0, min(1.0, deadline - time.monotonic())))
    except BaseException as error:
        errors.append(f"leader reap failed: {type(error).__name__}: {error}")
    try:
        final_absent = not group_present(pgid)
    except BaseException as error:
        errors.append(f"final group probe failed: {type(error).__name__}: {error}")
        final_absent = False
    if not final_absent:
        send(signal.SIGKILL, "SIGKILL-final")
        try:
            final_absent = wait_absent(GROUP_KILL_WAIT)
        except BaseException as error:
            errors.append(f"final KILL wait failed: {type(error).__name__}: {error}")
            final_absent = False
        try:
            proc.wait(timeout=max(0.0, min(1.0, deadline - time.monotonic())))
        except BaseException as error:
            errors.append(f"final reap failed: {type(error).__name__}: {error}")
        try:
            final_absent = final_absent and not group_present(pgid)
        except BaseException as error:
            errors.append(f"post-KILL probe failed: {type(error).__name__}: {error}")
            final_absent = False
    if not final_absent:
        errors.append("owned process group remains or absence is unknown")
    return final_absent and proc.returncode is not None and not errors


class Runner:
    def __init__(self, run_dir: Path, cli: Path, build_receipt: dict, started_at: float):
        self.run_dir = run_dir
        self.cli = cli
        self.started = started_at
        self.deadline = self.started + OVERALL_TIMEOUT
        self.commands_dir = run_dir / "commands"
        self.commands_dir.mkdir(mode=0o700)
        self.secrets_dir = run_dir / "secrets"
        self.secrets_dir.mkdir(mode=0o700)
        self.workspaces_dir = run_dir / "workspaces"
        self.workspaces_dir.mkdir(mode=0o700)
        self.temp_dir = run_dir / "tmp"
        self.temp_dir.mkdir(mode=0o700)
        self.env = {
            "PATH": os.defpath,
            "TMPDIR": str(self.temp_dir),
            "LANG": "C",
            "LC_ALL": "C",
        }
        self.manifest: dict = {
            "schema": "brn.mac13-preparation.v1",
            "status": "running",
            "started_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            "source_commit": SOURCE_COMMIT,
            "checkout_head": build_receipt.get("head"),
            "build_receipt": str(BUILD_RECEIPT),
            "artifacts": build_receipt.get("artifacts"),
            "run_directory": str(run_dir),
            "commands": [],
            "steps": [],
            "native_observed": False,
            "owner_accepted": False,
            "notes": [
                "Fresh CLI-prepared synthetic roots only; native opening/navigation and MAC-13 remain pending.",
                "No provider, account, Run, direct SQL, or source mutation is part of this runner.",
            ],
            "process_environment_policy": {
                "passed_variables": sorted(self.env),
                "helper_overrides": "absent; frozen siblings selected",
                "HOME_and_CODEX_HOME": "not set or changed by runner",
                "credential_environment": "not inherited",
            },
        }
        self.manifest_path = run_dir / "preparation.json"
        self.command_index = 0
        self.write_manifest()

    def write_manifest(self) -> None:
        self.manifest["elapsed_seconds"] = round(time.monotonic() - self.started, 3)
        self.manifest["free_bytes"] = shutil.disk_usage("/private/tmp").free
        atomic_json(self.manifest_path, self.manifest)

    def checkpoint(self, name: str, data: dict) -> None:
        self.manifest["steps"].append({"name": name, **data})
        self.write_manifest()

    def ensure_budget(self) -> float:
        remaining = self.deadline - time.monotonic()
        # Reserve shared22-second cleanup, both possible reader-join passes and margin.
        if remaining <= CLEANUP_RESERVE:
            raise PreparationError("10-minute overall deadline leaves no safe command/cleanup budget")
        if shutil.disk_usage("/private/tmp").free < LIVE_DISK_FLOOR:
            raise PreparationError("/private/tmp crossed the 1 GiB free-space floor")
        return min(COMMAND_TIMEOUT, remaining - CLEANUP_RESERVE)

    def run_git(self, name: str, args: list[str], accepted_codes: tuple[int, ...] = (0,)) -> tuple[int, str, str]:
        """Run one bounded read-only Git query as an owned process group."""
        timeout = min(5.0, self.ensure_budget())
        self.command_index += 1
        prefix = f"{self.command_index:02d}-git-{name}"
        stdout_path = self.commands_dir / f"{prefix}.stdout"
        stderr_path = self.commands_dir / f"{prefix}.stderr"
        git = shutil.which("git", path=self.env["PATH"])
        if git is None:
            raise PreparationError("git is unavailable in the deterministic runner PATH")
        if args and args[0] == "diff":
            args = ["diff", "--no-ext-diff", "--no-textconv", *args[1:]]
        argv = [git, "--no-pager", "--no-optional-locks", "-C", str(REPO), *args]
        command = {"index": self.command_index, "name": f"git-{name}", "argv": argv,
                   "timeout_seconds": timeout, "stdout": str(stdout_path), "stderr": str(stderr_path),
                   "max_combined_output_bytes": MAX_OUTPUT,
                   "started_at": dt.datetime.now(dt.timezone.utc).isoformat()}
        self.manifest["commands"].append(command)
        self.write_manifest()
        env = dict(self.env, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
                   GIT_TERMINAL_PROMPT="0", GIT_PAGER="cat", PAGER="cat")
        proc: subprocess.Popen[bytes] | None = None
        stop_reason: str | None = None
        errors: list[str] = []
        signals_sent: list[str] = []
        try:
            with stdout_path.open("xb") as stdout_file, stderr_path.open("xb") as stderr_file:
                proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=stdout_file,
                                        stderr=stderr_file, cwd=self.run_dir, env=env,
                                        start_new_session=True, close_fds=True)
                command_deadline = min(time.monotonic() + timeout,
                                       self.deadline - CLEANUP_RESERVE)
                while proc.poll() is None:
                    if time.monotonic() >= command_deadline:
                        stop_reason = "Git command or overall deadline reached"
                        break
                    if shutil.disk_usage("/private/tmp").free < LIVE_DISK_FLOOR:
                        stop_reason = "free space fell below 1 GiB"
                        break
                    if stdout_path.stat().st_size + stderr_path.stat().st_size > MAX_OUTPUT:
                        stop_reason = "Git output exceeded 1 MiB"
                        break
                    time.sleep(0.05)
            clean = stop_and_reap(proc, signals_sent, errors)
            out_size = stdout_path.stat().st_size
            err_size = stderr_path.stat().st_size
            if out_size + err_size > MAX_OUTPUT:
                command.update({"stop_reason": "Git output exceeded 1 MiB", "output_overflow": True,
                                "combined_output_bytes": out_size + err_size,
                                "return_code": proc.returncode, "group_absent": clean,
                                "leader_reaped": proc.returncode is not None,
                                "signals_sent": signals_sent, "cleanup_errors": errors})
                self.write_manifest()
                raise PreparationError("Git output overflow; raw files retained without reading them")
            with stdout_path.open("rb") as stream:
                stdout = stream.read(MAX_OUTPUT + 1)
            with stderr_path.open("rb") as stream:
                stderr = stream.read(MAX_OUTPUT + 1)
            if len(stdout) + len(stderr) > MAX_OUTPUT:
                raise PreparationError("Git output grew beyond bounded read; refusing decode/hash")
            command.update({"return_code": proc.returncode, "signals_sent": signals_sent,
                            "group_absent": clean, "leader_reaped": proc.returncode is not None,
                            "cleanup_errors": errors, "combined_output_bytes": out_size + err_size,
                            "stdout_sha256": sha256(stdout), "stderr_sha256": sha256(stderr),
                            "finished_at": dt.datetime.now(dt.timezone.utc).isoformat()})
            self.write_manifest()
            if stop_reason or not clean or errors or proc.returncode not in accepted_codes:
                command["stop_reason"] = stop_reason or "Git command failed or cleanup was unresolved"
                raise PreparationError(f"bounded Git command {name} failed; see preparation.json")
            return proc.returncode, stdout.decode("utf-8", errors="strict"), stderr.decode("utf-8", errors="strict")
        except BaseException:
            if proc is not None:
                if not stop_and_reap(proc, signals_sent, errors):
                    command["cleanup_errors"] = errors
            try:
                command.setdefault("return_code", None if proc is None else proc.returncode)
                command.setdefault("signals_sent", signals_sent)
                command.setdefault("cleanup_errors", errors)
                command.setdefault("finished_at", dt.datetime.now(dt.timezone.utc).isoformat())
                self.write_manifest()
            except BaseException:
                pass
            raise

    def run_cli(self, name: str, data_dir: Path, args: list[str]) -> dict:
        timeout = self.ensure_budget()
        self.command_index += 1
        prefix = f"{self.command_index:02d}-{name}"
        stdout_path = self.commands_dir / f"{prefix}.stdout"
        stderr_path = self.commands_dir / f"{prefix}.stderr"
        argv = [str(self.cli), "--json", "--data-dir", str(data_dir), *args]
        command = {
            "index": self.command_index,
            "name": name,
            "argv": argv,
            "timeout_seconds": round(timeout, 3),
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
            "max_combined_output_bytes": MAX_OUTPUT,
            "started_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        }
        self.manifest["commands"].append(command)
        self.write_manifest()
        proc: subprocess.Popen[bytes] | None = None
        threads: list[threading.Thread] = []
        state = {"used": 0, "overflow": False}
        lock = threading.Lock()
        overflow = threading.Event()
        stop_reason: str | None = None
        cleanup_errors: list[str] = []
        signals_sent: list[str] = []
        readers: list[object] = []

        def drain(pipe, path: Path) -> None:
            try:
                with path.open("xb") as output:
                    while True:
                        chunk = pipe.read(65536)
                        if not chunk:
                            break
                        with lock:
                            remaining = MAX_OUTPUT - state["used"]
                            if remaining <= 0:
                                state["overflow"] = True
                                overflow.set()
                                break
                            kept = chunk[:remaining]
                            output.write(kept)
                            state["used"] += len(kept)
                            if len(kept) != len(chunk):
                                state["overflow"] = True
                                overflow.set()
                                break
                    output.flush()
                    os.fsync(output.fileno())
            except BaseException as error:
                with lock:
                    state.setdefault("reader_errors", []).append(f"{type(error).__name__}: {error}")
                overflow.set()
            finally:
                try:
                    pipe.close()
                except OSError:
                    pass

        try:
            proc = subprocess.Popen(
                argv,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                cwd=self.run_dir,
                env=self.env,
                start_new_session=True,
                close_fds=True,
            )
            assert proc.stdout is not None and proc.stderr is not None
            threads = [
                threading.Thread(target=drain, args=(proc.stdout, stdout_path), daemon=True),
                threading.Thread(target=drain, args=(proc.stderr, stderr_path), daemon=True),
            ]
            for thread in threads:
                thread.start()
            proc_deadline = time.monotonic() + timeout
            while proc.poll() is None:
                if overflow.is_set():
                    stop_reason = "combined command output exceeded 1 MiB or output capture failed"
                    break
                if time.monotonic() >= proc_deadline:
                    stop_reason = "60-second per-command deadline"
                    break
                if time.monotonic() >= self.deadline - CLEANUP_RESERVE:
                    stop_reason = "overall deadline cleanup reserve reached"
                    break
                if shutil.disk_usage("/private/tmp").free < LIVE_DISK_FLOOR:
                    stop_reason = "free space fell below 1 GiB"
                    break
                time.sleep(0.1)
            if proc.poll() is not None and stop_reason is None and group_present(proc.pid):
                # Reap the leader first, then stop/check any owned descendants that inherited the group.
                proc.wait(timeout=1.0)
                if not wait_group_absent(proc.pid, 0.25):
                    stop_reason = "owned child remained after command leader exited"
            if stop_reason is not None:
                command["stop_reason"] = stop_reason
            group_clean = stop_and_reap(proc, signals_sent, cleanup_errors)
            for thread in threads:
                thread.join(timeout=1.0)
            if any(thread.is_alive() for thread in threads):
                cleanup_errors.append("output reader did not finish after process-group cleanup")
            command.update({
                "return_code": proc.returncode,
                "signals_sent": signals_sent,
                "group_absent": group_clean,
                "leader_reaped": proc.returncode is not None,
                "cleanup_errors": cleanup_errors,
                "combined_output_bytes": state["used"],
                "output_overflow": state["overflow"],
                "reader_errors": state.get("reader_errors", []),
                "finished_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            })
            if stdout_path.exists() and stderr_path.exists() and not state["overflow"]:
                stdout = stdout_path.read_bytes()
                stderr = stderr_path.read_bytes()
                command["stdout_sha256"] = sha256(stdout)
                command["stderr_sha256"] = sha256(stderr)
                try:
                    response = json.loads(stdout.decode("utf-8"))
                    command["json_valid"] = isinstance(response, dict)
                    command["ok"] = response.get("ok") if isinstance(response, dict) else None
                    command["envelope_status"] = response.get("status") if isinstance(response, dict) else None
                except (UnicodeDecodeError, json.JSONDecodeError):
                    response = None
                    command["json_valid"] = False
                command["stderr_nonempty"] = bool(stderr)
            else:
                response = None
                command["json_valid"] = False
            self.write_manifest()
            if (stop_reason is not None or not group_clean or cleanup_errors or
                    state["overflow"] or proc.returncode != 0 or response is None or
                    response.get("ok") is not True):
                raise PreparationError(f"CLI command {name} failed; see bounded raw outputs and preparation.json")
            return response
        except BaseException:
            if proc is not None:
                stop_and_reap(proc, signals_sent, cleanup_errors)
            for thread in threads:
                thread.join(timeout=1.0)
            command.setdefault("return_code", None if proc is None else proc.returncode)
            command.setdefault("signals_sent", signals_sent)
            command.setdefault("group_absent", proc is None or not group_present(proc.pid))
            command.setdefault("leader_reaped", proc is None or proc.returncode is not None)
            command.setdefault("cleanup_errors", cleanup_errors)
            command.setdefault("combined_output_bytes", state["used"])
            command.setdefault("output_overflow", state["overflow"])
            command.setdefault("finished_at", dt.datetime.now(dt.timezone.utc).isoformat())
            try:
                self.write_manifest()
            except BaseException:
                pass
            raise


def verify_build(cli_receipt: dict, runner: Runner) -> tuple[Path, dict]:
    if not (cli_receipt.get("qualified_build") is True and cli_receipt.get("exit_code") == 0 and
            cli_receipt.get("source_unchanged") is True and cli_receipt.get("group_absent") is True and
            cli_receipt.get("leader_reaped") is True and not cli_receipt.get("cleanup_errors")):
        raise PreparationError("build receipt does not qualify a successful, cleaned build")
    if cli_receipt.get("production_source") != SOURCE_COMMIT:
        raise PreparationError("build receipt is not bound to selected main213 source")
    runtime = cli_receipt.get("runtime_paths")
    if runtime != ["crates", "agent", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]:
        raise PreparationError("build receipt runtime source scope differs from reviewed scope")
    head = cli_receipt.get("head")
    if not isinstance(head, str) or len(head) != 40:
        raise PreparationError("build receipt lacks full checkout HEAD")
    _, current_head, _ = runner.run_git("checkout-head", ["rev-parse", "HEAD"])
    current_head = current_head.strip()
    if not current_head:
        raise PreparationError("checkout HEAD query returned no identity")
    build_head_code, _, _ = runner.run_git("build-runtime-diff", ["diff", "--quiet", head, SOURCE_COMMIT, "--", *runtime], (0,))
    if build_head_code != 0:
        raise PreparationError("build receipt checkout does not match selected main213 runtime source")
    diff_code, _, _ = runner.run_git("runtime-diff", ["diff", "--quiet", SOURCE_COMMIT, "--", *runtime], (0,))
    staged_code, _, _ = runner.run_git("runtime-staged-diff", ["diff", "--cached", "--quiet", SOURCE_COMMIT, "--", *runtime], (0,))
    _, status_text, _ = runner.run_git("runtime-status", ["status", "--porcelain", "--", *runtime])
    if diff_code != 0 or staged_code != 0 or status_text:
        raise PreparationError("runtime source paths no longer match clean main213")

    artifacts = cli_receipt.get("artifacts")
    if not isinstance(artifacts, dict) or set(artifacts) != {"brn", "brn-xberg-helper", "brn-xberg-helper-brn"}:
        raise PreparationError("build receipt lacks CLI and both helper artifacts")
    cli_path = Path(artifacts["brn"].get("path", ""))
    if cli_path.parent != EXPECTED_ARTIFACT_DIR or not cli_path.is_absolute():
        raise PreparationError("CLI artifact is not at the selected frozen sibling-artifact path")
    helper_hashes: dict[str, str] = {}
    for name, artifact in artifacts.items():
        path = Path(artifact.get("path", ""))
        if path.parent != cli_path.parent or not path.is_absolute():
            raise PreparationError(f"{name} is not a CLI sibling")
        digest, size = sha256_file(path)
        if size != artifact.get("bytes") or digest != artifact.get("sha256"):
            raise PreparationError(f"{name} artifact hash/size does not match build receipt")
        if not os.access(path, os.X_OK):
            raise PreparationError(f"{name} artifact is not executable")
        helper_hashes[name] = digest

    # The copied helper binaries are independently tied to the earlier
    # qualified installation and preserved-hash receipts, not merely trusted
    # because the CLI build receipt listed them as siblings.
    reused = cli_receipt.get("reused_helpers")
    install_head = "3b3f9135c318a4a7959e591f2c0774b8177f9266"
    helper_sources = [
        "tools/xberg-intake-helper",
        "scripts/prepare-xberg-adapted.py",
        "scripts/build-document-helpers.sh",
    ]
    if (not isinstance(reused, dict) or reused.get("qualified_installation_head") != install_head or
            reused.get("source_paths") != helper_sources or
            reused.get("source_unchanged_to_product_baseline") is not True or
            reused.get("installation_receipt") != "docs/work/active/vacation-campaign/evidence/writing-review/ci-helper-installation.json.gz" or
            reused.get("hash_receipt") != "docs/work/active/vacation-campaign/evidence/helper-build-cache-reclamation.json"):
        raise PreparationError("build receipt lacks the approved reused-helper source binding")
    helper_diff_code, _, _ = runner.run_git("helper-source-diff", ["diff", "--quiet", install_head, SOURCE_COMMIT, "--", *helper_sources])
    _, helper_status_text, _ = runner.run_git("helper-source-status", ["status", "--porcelain", "--", *helper_sources])
    if helper_diff_code != 0 or helper_status_text:
        raise PreparationError("helper source changed between qualified installation and selected product source")
    install_receipt = REPO / reused.get("installation_receipt", "")
    hash_receipt = REPO / reused.get("hash_receipt", "")
    if not install_receipt.is_file() or not hash_receipt.is_file():
        raise PreparationError("reused-helper source/hash receipts are missing")
    import gzip
    install = json.loads(gzip.decompress(install_receipt.read_bytes()))
    preserved = json.loads(hash_receipt.read_text(encoding="utf-8")).get("preserved_qualified_helpers")
    if install.get("head") != install_head or not isinstance(install.get("helpers"), list) or not isinstance(preserved, dict):
        raise PreparationError("reused-helper receipts do not identify the approved installation")
    expected_helper_names = {"brn-xberg-helper", "brn-xberg-helper-brn"}
    installed = {Path(item.get("destination", "")).name: item for item in install["helpers"] if isinstance(item, dict)}
    if set(installed) != expected_helper_names:
        raise PreparationError("helper installation receipt does not contain both expected helpers")
    for name in expected_helper_names:
        item = installed[name]
        artifact = artifacts[name]
        path = Path(artifact["path"])
        source_path = Path(item.get("source", ""))
        if (item.get("source_sha256") != artifact.get("sha256") or
                item.get("destination_sha256") != artifact.get("sha256") or
                item.get("size") != artifact.get("bytes") or
                preserved.get(str(source_path)) != artifact.get("sha256") or
                helper_hashes[name] != artifact.get("sha256")):
            raise PreparationError(f"{name} does not match installed and preserved helper receipts")
    return cli_path, {"checkout_head_at_verification": current_head,
                      "reused_helpers": reused, "installation_head": install_head,
                      "helper_source_paths": helper_sources, "helper_sha256": {
                          name: helper_hashes[name] for name in sorted(expected_helper_names)}}


def verify_expected_fixtures() -> dict[str, bytes]:
    result = {}
    for name, (size, digest) in EXPECTED_FIXTURES.items():
        data = (FIXTURES / name).read_bytes()
        if len(data) != size or sha256(data) != digest:
            raise PreparationError(f"expected fixture changed: {name}")
        result[name] = data
    manifest = json.loads((FIXTURES / "manifest.json").read_text(encoding="utf-8"))
    listed = {entry["file"]: (entry["utf8_bytes"], entry["sha256"]) for entry in manifest["fixtures"]}
    for name, (size, digest) in EXPECTED_FIXTURES.items():
        if listed.get(name) != (size, digest):
            raise PreparationError(f"fixture manifest mismatch: {name}")
    return result


def require_ok(response: dict, name: str) -> dict:
    if response.get("ok") is not True:
        raise PreparationError(f"{name} was not accepted")
    result = response.get("result")
    if not isinstance(result, dict):
        raise PreparationError(f"{name} response has no result object")
    return result


def init_workspace(runner: Runner, data: Path) -> dict:
    if data.exists() or data.is_symlink():
        raise PreparationError(f"refusing non-fresh workspace path: {data}")
    init = runner.run_cli("init", data, ["init"])
    result = require_ok(init, "init")
    if result.get("ready") is not True or result.get("data") != str(data):
        raise PreparationError("init readiness or canonical data path differs from the intended fresh workspace")
    settings = result.get("settings")
    if not isinstance(settings, dict):
        raise PreparationError("init response has no explicit Settings object; intake stopped")
    if ("auth_file" not in settings or settings["auth_file"] is not None or
            "credentials_dir" not in settings or settings["credentials_dir"] is not None):
        raise PreparationError("fresh workspace exposes or omits an explicit credential source; intake stopped")
    document = settings.get("document_conversion")
    if (not isinstance(document, dict) or "converter" not in document or
            document.get("converter") != "xberg" or "ai" not in document or document.get("ai") is not False):
        raise PreparationError("fresh workspace is not explicitly Xberg with AI refinement disabled")
    runner.checkpoint("workspace-initialized", {"data_dir": str(data), "credential_source": None, "ai_refinement": False})
    return settings


def import_private(runner: Runner, data: Path, source: Path, title: str, key: str) -> tuple[str, dict, dict]:
    thread_result = require_ok(runner.run_cli("thread-start", data, ["thread-start", "--title", title + " chat", "--key", key + "-chat"]), "thread-start")
    chat = thread_result.get("id")
    if not isinstance(chat, str) or not chat:
        raise PreparationError("thread-start returned no chat ID")
    receipt_result = require_ok(runner.run_cli("chat-intake", data, ["intake", "--source", str(source), "--intent", "chat", "--thread", chat, "--title", title, "--key", key + "-note"]), "chat intake")
    if receipt_result.get("outcome") != "Applied" or not isinstance(receipt_result.get("receipt"), dict):
        raise PreparationError("chat intake did not return an Applied receipt")
    receipt = receipt_result["receipt"]
    notes = [w["after"] for w in receipt.get("writes", []) if isinstance(w, dict) and isinstance(w.get("after"), dict) and isinstance(w["after"].get("data"), dict) and "Note" in w["after"]["data"]]
    if len(notes) != 1:
        raise PreparationError("chat intake receipt did not contain exactly one Note")
    return chat, notes[0]["id"], receipt


def note_create(runner: Runner, data: Path, title: str, source: Path, key: str) -> tuple[str, dict]:
    result = require_ok(runner.run_cli("note-create", data, ["note-create", "--title", title, "--text-file", str(source), "--key", key]), "note-create")
    record_id = result.get("id")
    if not isinstance(record_id, str) or not isinstance(result.get("data", {}).get("Note"), dict):
        raise PreparationError("note-create returned no Note Record")
    return record_id, result


def read_record(runner: Runner, data: Path, record_id: str) -> dict:
    result = require_ok(runner.run_cli("read", data, ["read", "--id", record_id]), "read")
    if result.get("id") != record_id:
        raise PreparationError("read returned a different record")
    return result


def read_collection(runner: Runner, data: Path, command: str) -> list[dict]:
    response = runner.run_cli(command, data, [command])
    result = response.get("result")
    if response.get("ok") is not True or not isinstance(result, list) or not all(isinstance(x, dict) for x in result):
        raise PreparationError(f"{command} did not return a record list")
    return result


def record_data(record: dict, kind: str) -> dict:
    data = record.get("data")
    value = data.get(kind) if isinstance(data, dict) else None
    if not isinstance(value, dict):
        raise PreparationError(f"record {record.get('id')} is not {kind}")
    return value


def verify_fixture_root(runner: Runner, label: str, data: Path, expected_initial: bytes,
                        sentinel: bytes, private: bool) -> dict:
    records = read_collection(runner, data, "records")
    notes = read_collection(runner, data, "notes")
    history = read_collection(runner, data, "history")
    recovery = read_collection(runner, data, "recovery")
    note_records = [r for r in records if isinstance(r.get("data"), dict) and "Note" in r["data"]]
    candidates = [r for r in note_records if record_data(r, "Note").get("markdown", "").encode("utf-8") == expected_initial]
    sentinel_rows = [r for r in note_records if record_data(r, "Note").get("markdown", "").encode("utf-8") == sentinel]
    if len(candidates) != 1 or len(sentinel_rows) != 1:
        raise PreparationError(f"{label}: expected exactly one initial Note and one sentinel")
    note, sentinel_record = candidates[0], sentinel_rows[0]
    note_data = record_data(note, "Note")
    sentinel_data = record_data(sentinel_record, "Note")
    expected_general_ids = {sentinel_record["id"]} if private else {note["id"], sentinel_record["id"]}
    if {row.get("id") for row in notes} != expected_general_ids:
        raise PreparationError(f"{label}: general Notes projection is not exactly the expected fixture set")
    if sentinel_record["id"] == note["id"] or sentinel_data.get("chat") is not None:
        raise PreparationError(f"{label}: sentinel identity/scope invalid")
    if private:
        chat_ids = [r["id"] for r in records if isinstance(r.get("data"), dict) and "Thread" in r["data"]]
        if len(chat_ids) != 1 or note_data.get("chat") != chat_ids[0] or note_data.get("protected") is not True:
            raise PreparationError(f"{label}: private Note scope/protection invalid")
        imported = note_data.get("import")
        if not isinstance(imported, dict) or imported.get("full_note") is not True:
            raise PreparationError(f"{label}: private Note lacks full-import metadata")
        links = [record_data(r, "Link") for r in records if isinstance(r.get("data"), dict) and "Link" in r["data"]]
        if not any(link.get("from") == chat_ids[0] and link.get("to") == note["id"] and link.get("relation") == "attachment" for link in links):
            raise PreparationError(f"{label}: attachment Link absent")
        if any(r["id"] == note["id"] for r in notes):
            raise PreparationError(f"{label}: private Note appeared in general projection")
        private_chat = chat_ids[0]
    else:
        if note_data.get("chat") is not None or note_data.get("import") is not None:
            raise PreparationError(f"{label}: general Note unexpectedly scoped/imported")
        if not any(r["id"] == note["id"] for r in notes):
            raise PreparationError(f"{label}: general Note absent from general projection")
        private_chat = None
    if not any(r["id"] == sentinel_record["id"] for r in notes):
        raise PreparationError(f"{label}: general sentinel absent from general projection")
    if recovery:
        raise PreparationError(f"{label}: unexpected recovery session before native editing")
    read_note = read_record(runner, data, note["id"])
    read_sentinel = read_record(runner, data, sentinel_record["id"])
    if read_note != note or read_sentinel != sentinel_record:
        raise PreparationError(f"{label}: full Note read differs from records snapshot")
    return {
        "data_dir": str(data),
        "scope": "private-chat" if private else "general",
        "chat_id": private_chat,
        "note_id": note["id"],
        "note_version": note["version"],
        "note_markdown_sha256": sha256(expected_initial),
        "note_protected": note_data.get("protected"),
        "note_import_full": bool(note_data.get("import", {}).get("full_note")) if private else False,
        "sentinel_id": sentinel_record["id"],
        "sentinel_version": sentinel_record["version"],
        "sentinel_markdown_sha256": sha256(sentinel),
        "record_count": len(records),
        "general_note_ids": sorted(r["id"] for r in notes),
        "history_count": len(history),
        "recovery_count": len(recovery),
        "records_sha256": sha256(json.dumps(records, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")),
    }


def prepare_root(runner: Runner, label: str, data: Path, initial_path: Path, initial: bytes,
                 sentinel_path: Path, sentinel: bytes, private: bool) -> dict:
    init_workspace(runner, data)
    if private:
        chat, note_id, intake_receipt = import_private(runner, data, initial_path, f"MAC13 {label}", f"mac13-{label}")
        runner.checkpoint("private-import", {"workspace": label, "chat_id": chat, "note_id": note_id, "operation": intake_receipt.get("operation")})
    else:
        note_id, _ = note_create(runner, data, f"MAC13 {label} note", initial_path, f"mac13-{label}-note")
        chat = None
    sentinel_id, _ = note_create(runner, data, f"MAC13 {label} sentinel", sentinel_path, f"mac13-{label}-sentinel")
    if private:
        # This fixture is about privacy/recovery, not converter byte fidelity.
        # Preserve imported revision1 and its Source, then explicitly establish
        # the independently fixed editor bytes through ordinary checked Save.
        setup_state = {"note_id": note_id, "chat_id": chat, "sentinel_id": sentinel_id}
        checked_fixture_save(runner, label, setup_state, data, initial_path, initial,
                             sentinel, True, purpose="private-initialization")
    state = verify_fixture_root(runner, label, data, initial, sentinel, private)
    if state["note_id"] != note_id or state["sentinel_id"] != sentinel_id or state["chat_id"] != chat:
        raise PreparationError(f"{label}: created identities differ from verified fixture")
    runner.checkpoint("initial-state-verified", {"workspace": label, "state": state})
    return state


def checked_fixture_save(runner: Runner, label: str, initial_state: dict, data: Path,
                      advance_path: Path, advance_bytes: bytes, sentinel: bytes,
                      private: bool, *, purpose: str = "rehearsal") -> dict:
    before_records = read_collection(runner, data, "records")
    before_history = read_collection(runner, data, "history")
    before_recovery = read_collection(runner, data, "recovery")
    if before_recovery:
        raise PreparationError(f"{label}: rehearsal is not clean before checked version advance")
    before_note = read_record(runner, data, initial_state["note_id"])
    base = before_note["version"]
    token = runner.secrets_dir / f"{label}.token"
    if token.exists() or token.is_symlink():
        raise PreparationError(f"{label}: token path is not fresh")
    begin = require_ok(runner.run_cli("begin-edit", data, ["begin-edit", "--note", initial_state["note_id"], "--base", str(base), "--token-out", str(token)]), "begin-edit")
    if begin.get("token") is not None or begin.get("token_file") != str(token) or begin.get("base_version") != base or begin.get("note") != initial_state["note_id"]:
        raise PreparationError(f"{label}: begin-edit result not bound to expected clean base/token path")
    if not token.exists() or stat.S_IMODE(token.stat().st_mode) != 0o600:
        raise PreparationError(f"{label}: token file is absent or not mode 0600")
    session = begin.get("id")
    if not isinstance(session, str) or begin.get("generation") != 0:
        raise PreparationError(f"{label}: unexpected new edit-session identity/generation")
    try:
        update = require_ok(runner.run_cli("update-buffer", data, ["update-buffer", "--session", session, "--generation", "1", "--text-file", str(advance_path), "--token-file", str(token)]), "update-buffer")
        if update.get("outcome") != "Updated" or update.get("session", {}).get("generation") != 1:
            raise PreparationError(f"{label}: canonical advance buffer was not Updated")
        saved = require_ok(runner.run_cli("save", data, ["save", "--session", session, "--generation", "1", "--token-file", str(token)]), "save")
        if saved.get("outcome") != "Saved" or not isinstance(saved.get("receipt"), dict):
            raise PreparationError(f"{label}: canonical advance did not Save")
    finally:
        try:
            token.unlink()
        except FileNotFoundError:
            pass
    after_records = read_collection(runner, data, "records")
    after_history = read_collection(runner, data, "history")
    after_recovery = read_collection(runner, data, "recovery")
    after_note = read_record(runner, data, initial_state["note_id"])
    after_sentinel = read_record(runner, data, initial_state["sentinel_id"])
    if after_note["version"] != base + 1 or record_data(after_note, "Note").get("markdown", "").encode("utf-8") != advance_bytes:
        raise PreparationError(f"{label}: advanced Note version/content mismatch")
    expected_note = copy.deepcopy(before_note)
    expected_note["version"] = base + 1
    expected_data = expected_note["data"]["Note"]
    expected_data["markdown"] = advance_bytes.decode("utf-8")
    if expected_data.get("markdown") != record_data(after_note, "Note").get("markdown"):
        raise PreparationError(f"{label}: exact UTF-8 source text changed during checked Save")
    # Owner Save clears confirmation when the Markdown changes; all other Note
    # fields must remain byte-for-byte represented as in the pre-advance row.
    expected_data["confirmed"] = False
    if after_note != expected_note or after_sentinel["id"] != initial_state["sentinel_id"]:
        raise PreparationError(f"{label}: Note metadata or sentinel identity changed unexpectedly")
    before_map = {r["id"]: r for r in before_records}
    after_map = {r["id"]: r for r in after_records}
    if set(before_map) != set(after_map):
        raise PreparationError(f"{label}: record identity set changed during Save")
    for rid, old in before_map.items():
        if rid == initial_state["note_id"]:
            if after_map[rid] != expected_note:
                raise PreparationError(f"{label}: saved record differs beyond allowed Note update")
        elif old != after_map[rid]:
            raise PreparationError(f"{label}: unrelated record changed during Save")
    retained_history = list(after_history)
    try:
        for old_receipt in before_history:
            retained_history.remove(old_receipt)
    except ValueError as error:
        raise PreparationError(f"{label}: prior history receipt changed or disappeared") from error
    if retained_history != [saved["receipt"]]:
        raise PreparationError(f"{label}: history gained anything beyond the one checked Save receipt")
    if after_recovery:
        raise PreparationError(f"{label}: saved caller session remains open")
    projection = read_collection(runner, data, "notes")
    projection_ids = {r["id"] for r in projection}
    if (initial_state["note_id"] in projection_ids) != (not private) or initial_state["sentinel_id"] not in projection_ids:
        raise PreparationError(f"{label}: general projection changed incorrectly")
    result = {
        "workspace": label,
        "purpose": purpose,
        "data_dir": str(data),
        "note_id": initial_state["note_id"],
        "chat_id": initial_state["chat_id"],
        "base_version": base,
        "base_markdown_sha256": sha256(record_data(before_note, "Note")["markdown"].encode("utf-8")),
        "saved_version": after_note["version"],
        "saved_markdown_sha256": sha256(advance_bytes),
        "note_metadata_preserved": True,
        "sentinel_id": initial_state["sentinel_id"],
        "sentinel_unchanged": after_sentinel == before_map[initial_state["sentinel_id"]],
        "other_records_unchanged": True,
        "history_count_before": len(before_history),
        "history_count_after": len(after_history),
        "recovery_count_after": len(after_recovery),
        "session_id": session,
        "token_path": str(token),
        "token_bytes_retained": False,
    }
    runner.checkpoint(f"{purpose}-save-verified", result)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", action="store_true", help="create fresh synthetic workspaces; omitted means inert validation")
    args = parser.parse_args()
    if not args.run:
        print("Inert: pass --run only after independent review and lead selection.")
        return 0
    if sys.platform != "darwin":
        print("Refusing: this prepared route is qualified only for the selected Mac host.", file=sys.stderr)
        return 2
    if Path("/private/tmp").is_symlink() or Path("/private/tmp").resolve() != Path("/private/tmp"):
        print("Refusing: output parent must be canonical /private/tmp.", file=sys.stderr)
        return 2
    if shutil.disk_usage("/private/tmp").free < START_DISK_FLOOR:
        print("Refusing: less than 2 GiB free at start.", file=sys.stderr)
        return 2
    execution_started = time.monotonic()
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:10]
    run_dir = Path("/private/tmp") / f"mac13-preparation-{run_id}"
    try:
        run_dir.mkdir(mode=0o700, parents=False, exist_ok=False)
    except OSError as error:
        print(f"Refusing: cannot create unique private run directory: {type(error).__name__}", file=sys.stderr)
        return 2
    os.umask(0o077)
    initial_manifest = {
        "schema": "brn.mac13-preparation.v1",
        "status": "starting",
        "started_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "source_commit": SOURCE_COMMIT,
        "run_directory": str(run_dir),
        "commands": [],
        "steps": [],
        "native_observed": False,
        "owner_accepted": False,
    }
    manifest_path = run_dir / "preparation.json"
    try:
        atomic_json(manifest_path, initial_manifest)
    except BaseException as error:
        print(f"Could not create initial failure receipt: {type(error).__name__}", file=sys.stderr)
        return 1
    runner: Runner | None = None
    status = "failed"
    failure: str | None = None
    signal_name: str | None = None
    final_persisted = False

    def interrupt(signum, _frame):
        raise StopRequested(f"received {signal.Signals(signum).name}")

    for signum in (signal.SIGINT, signal.SIGTERM):
        signal.signal(signum, interrupt)

    try:
        receipt = json.loads(BUILD_RECEIPT.read_text(encoding="utf-8"))
        receipt_artifacts = receipt.get("artifacts") if isinstance(receipt, dict) else None
        cli_artifact = receipt_artifacts.get("brn") if isinstance(receipt_artifacts, dict) else None
        cli_candidate = Path(cli_artifact.get("path", "")) if isinstance(cli_artifact, dict) else Path()
        if not cli_candidate.is_absolute():
            raise PreparationError("build receipt has no absolute CLI artifact path")
        runner = Runner(run_dir, cli_candidate, receipt, execution_started)
        cli, helper_binding = verify_build(receipt, runner)
        fixtures = verify_expected_fixtures()
        initial = fixtures["save-initial.txt"]
        advance = fixtures["save-final.txt"]
        draft = fixtures["unsaved-recovery.txt"]
        fixture_dir = run_dir / "fixture-inputs"
        fixture_dir.mkdir(mode=0o700)
        source_path = fixture_dir / "private-initial.md"
        initial_path = fixture_dir / "general-initial.md"
        sentinel_path = fixture_dir / "sentinel.md"
        advance_path = fixture_dir / "save-final.txt"
        draft_path = fixture_dir / "unsaved-recovery.txt"
        for path, data in ((source_path, initial), (initial_path, initial),
                           (advance_path, advance), (draft_path, draft)):
            path.write_bytes(data)
        sentinel = f"MAC13-SENTINEL-{run_id}-GENERAL-ONLY\n".encode("utf-8")
        sentinel_path.write_bytes(sentinel)
        runner.cli = cli
        runner.manifest["checkout_head_at_runner_start"] = helper_binding["checkout_head_at_verification"]
        runner.manifest["fixture_bytes"] = {name: {"bytes": len(data), "sha256": sha256(data)} for name, data in fixtures.items()}
        runner.manifest["synthetic_sentinel"] = {"bytes": len(sentinel), "sha256": sha256(sentinel)}
        runner.manifest["fixture_input_paths"] = {"private_markdown": str(source_path), "general_markdown": str(initial_path), "save_final": str(advance_path), "unsaved_recovery": str(draft_path)}
        runner.manifest["helper_source_binding"] = helper_binding
        runner.manifest["artifact_hashes"] = receipt.get("artifacts")
        runner.write_manifest()
        workspaces = {name: runner.workspaces_dir / name / "data" for name in (
            "private-primary", "general-primary", "private-rehearsal", "general-rehearsal")}
        for parent in (runner.workspaces_dir / name for name in workspaces):
            parent.mkdir(mode=0o700)
        primary_private = prepare_root(runner, "private-primary", workspaces["private-primary"], source_path, initial, sentinel_path, sentinel, True)
        primary_general = prepare_root(runner, "general-primary", workspaces["general-primary"], initial_path, initial, sentinel_path, sentinel, False)
        rehearsal_private = prepare_root(runner, "private-rehearsal", workspaces["private-rehearsal"], source_path, initial, sentinel_path, sentinel, True)
        rehearsal_general = prepare_root(runner, "general-rehearsal", workspaces["general-rehearsal"], initial_path, initial, sentinel_path, sentinel, False)
        private_after = checked_fixture_save(runner, "private-rehearsal", rehearsal_private, workspaces["private-rehearsal"], advance_path, advance, sentinel, True)
        general_after = checked_fixture_save(runner, "general-rehearsal", rehearsal_general, workspaces["general-rehearsal"], advance_path, advance, sentinel, False)
        # Verify primary roots remain at the exact pre-advance initial state.
        primary_private_final = verify_fixture_root(runner, "private-primary", workspaces["private-primary"], initial, sentinel, True)
        primary_general_final = verify_fixture_root(runner, "general-primary", workspaces["general-primary"], initial, sentinel, False)
        if primary_private_final != primary_private or primary_general_final != primary_general:
            raise PreparationError("primary initial roots changed during rehearsal work")
        runner.manifest["primary_roots"] = [primary_private, primary_general]
        runner.manifest["rehearsals"] = [private_after, general_after]
        runner.manifest["native_fixture_manifest_updated"] = False
        runner.manifest["native_observed"] = False
        status = "prepared"
    except BaseException as error:
        failure = f"{type(error).__name__}: {error}"
        status = "failed"
        if isinstance(error, StopRequested):
            signal_name = str(error)
        if runner is not None:
            runner.manifest["status"] = "failed"
            runner.manifest["failure"] = failure
            runner.manifest["stop_signal"] = signal_name
            try:
                runner.write_manifest()
            except BaseException:
                pass
    finally:
        if runner is not None:
            # Secret files must not survive fixture preparation; token bytes were never captured.
            try:
                for token_file in runner.secrets_dir.iterdir():
                    if token_file.is_file() and not token_file.is_symlink():
                        token_file.unlink()
            except BaseException as error:
                status = "failed"
                failure = f"secret cleanup failed: {type(error).__name__}: {error}"
            runner.manifest["status"] = status
            runner.manifest["finished_at"] = dt.datetime.now(dt.timezone.utc).isoformat()
            runner.manifest["failure"] = failure
            runner.manifest["stop_signal"] = signal_name
            try:
                runner.write_manifest()
                final_persisted = True
            except BaseException as error:
                status = "failed"
                failure = f"final receipt persistence failed: {type(error).__name__}: {error}"
                runner.manifest["status"] = "failed"
                runner.manifest["failure"] = failure
                runner.manifest["finished_at"] = dt.datetime.now(dt.timezone.utc).isoformat()
                try:
                    atomic_json(runner.manifest_path, runner.manifest)
                    final_persisted = True
                except BaseException as fallback_error:
                    final_persisted = False
                    print(f"Final receipt could not be persisted: {type(fallback_error).__name__}", file=sys.stderr)
        else:
            status = "failed"
            initial_manifest["status"] = "failed"
            initial_manifest["finished_at"] = dt.datetime.now(dt.timezone.utc).isoformat()
            initial_manifest["failure"] = failure or "runner initialization failed"
            try:
                atomic_json(manifest_path, initial_manifest)
                final_persisted = True
            except BaseException as error:
                final_persisted = False
                print(f"Failure receipt could not be persisted: {type(error).__name__}", file=sys.stderr)
        result = {"status": status, "run_directory": str(run_dir),
                  "manifest": str(run_dir / "preparation.json"), "failure": failure,
                  "final_receipt_persisted": final_persisted}
        print(json.dumps(result, ensure_ascii=False), file=sys.stderr if status != "prepared" or not final_persisted else sys.stdout)

    return 0 if status == "prepared" and final_persisted and failure is None else 1


if __name__ == "__main__":
    raise SystemExit(main())
