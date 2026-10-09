#!/usr/bin/env python3
"""Bounded, retained shipping-CLI witness for 5,000 synthetic Current notes.

Only --help is side-effect free. A campaign requires a new absolute owned case
path and an absolute runtime directory containing brn and build-manifest.json.
Failures retain all evidence; this script never resets, retries or removes a case.
"""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import stat
import subprocess
import sys
import time
import uuid

NOTE_COUNT = 5000
PAGE_SIZE = 200
PROCESS_SECONDS = 60
CAMPAIGN_SECONDS = 600
NAMESPACE = uuid.UUID("15a0df88-c9de-4c5e-b6cc-a897b5695180")
ANCHORS = (0, 2500, 4999)
INDEX_FAMILY = ("index.sqlite", "index.sqlite-wal", "index.sqlite-shm", "index.sqlite-journal")


class QualificationError(Exception):
    pass


def require(condition, message):
    if not condition:
        raise QualificationError(message)


def utc_now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def write_json(path, value):
    # Only new files, or known receipt files in this exclusively created case.
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC | getattr(os, "O_NOFOLLOW", 0)
    with os.fdopen(os.open(path, flags, 0o600), "w", encoding="utf-8") as output:
        json.dump(value, output, ensure_ascii=False, indent=2)
        output.write("\n")


def file_info(path):
    """Hash a regular file without following a final symlink; detect read races."""
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode), f"Not a regular file: {path}")
        digest = hashlib.sha256()
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
        after = os.fstat(source.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    require(all(getattr(before, key) == getattr(after, key) for key in fields),
            f"File changed while hashing: {path}")
    return {"device": after.st_dev, "inode": after.st_ino,
            "byte_len": after.st_size, "mtime_ns": after.st_mtime_ns,
            "sha256": digest.hexdigest()}


def checked_absolute(raw, existing):
    path = Path(raw)
    require(path.is_absolute(), f"Path must be absolute: {path}")
    require(".." not in path.parts, f"Parent traversal is refused: {path}")
    if existing:
        require(path.resolve(strict=True) == path, f"Use the canonical path: {path}")
    else:
        require(not os.path.lexists(path), f"Case already exists; never reuse/reset it: {path}")
        require(path.parent.resolve(strict=True) == path.parent,
                f"Case parent must be an existing canonical directory: {path.parent}")
    return path


class Campaign:
    def __init__(self, runtime, case, started):
        self.started = started
        self.deadline = started + CAMPAIGN_SECONDS
        self.runtime = runtime
        self.case = case
        self.binary = runtime / "brn"
        self.manifest_path = runtime / "build-manifest.json"
        self.active = None
        self.spawning = False
        self.pending_signal = None
        self.identities = {}
        self.commands = []
        self.notes = {}
        self.result = {"format": 1, "outcome": "running", "started_at_utc": utc_now(),
                       "case_root": str(case), "runtime": str(runtime),
                       "limits": {"notes": NOTE_COUNT, "page_size": PAGE_SIZE,
                                  "process_seconds": PROCESS_SECONDS,
                                  "campaign_seconds": CAMPAIGN_SECONDS},
                       "scope": "Synthetic Current Markdown; keyword CLI correctness and observed wall times only. No semantic/native usability, SLA, complex approval recovery or provider qualification."}

    def remaining(self):
        seconds = self.deadline - time.monotonic()
        require(seconds > 0, "600-second aggregate campaign deadline reached")
        return seconds

    def check_owned(self):
        for path, identity in self.identities.items():
            metadata = path.lstat()
            require(stat.S_ISDIR(metadata.st_mode) and not stat.S_ISLNK(metadata.st_mode)
                    and metadata.st_uid == os.getuid()
                    and (metadata.st_dev, metadata.st_ino) == identity,
                    f"Owned case directory changed: {path}")

    def directory(self, path):
        path.mkdir(mode=0o700)
        metadata = path.lstat()
        self.identities[path] = (metadata.st_dev, metadata.st_ino)
        return path

    def initialize(self):
        self.remaining()
        manifest_info = file_info(self.manifest_path)
        build = json.loads(self.manifest_path.read_text(encoding="utf-8"))
        entries = [entry for entry in build["binaries"] if entry.get("name") == "brn"]
        require(len(entries) == 1, "Runtime manifest needs exactly one brn binary")
        expected = entries[0]["sha256"]
        require(isinstance(expected, str) and len(expected) == 64
                and all(character in "0123456789abcdef" for character in expected),
                "Runtime CLI SHA-256 is malformed")
        require(build.get("cli_sha256") == expected, "Runtime CLI SHA fields disagree")
        require(Path(entries[0]["path"]) == self.binary, "Manifest CLI path differs from runtime/brn")
        self.binary_info = file_info(self.binary)
        require(self.binary_info["sha256"] == expected
                and self.binary_info["byte_len"] == entries[0]["bytes"], "Runtime CLI does not match manifest")
        require(os.access(self.binary, os.X_OK), "Runtime brn is not executable")
        self.runtime_manifest_info = manifest_info
        self.expected_sha = expected
        parent = self.case.parent.lstat()
        require(stat.S_ISDIR(parent.st_mode) and parent.st_uid == os.getuid(),
                "Case parent must be an owned directory")
        require(not any(os.path.lexists(ancestor / ".git") for ancestor in self.case.parents),
                "Synthetic case must be outside Git repositories")
        require(self.runtime != self.case and self.runtime not in self.case.parents
                and self.case not in self.runtime.parents, "Runtime and case must be separate")
        self.directory(self.case)  # Exclusive mkdir is the no-reuse ownership gate.
        self.data = self.directory(self.case / "data")
        self.vault = self.directory(self.case / "vault")
        self.credentials = self.directory(self.case / "credentials")
        self.models = self.directory(self.data / "models")  # Default model leaf stays absent.
        self.inputs = self.directory(self.case / "inputs")
        self.receipts = self.directory(self.case / "receipts")
        self.logs = self.directory(self.case / "commands")
        self.retained = self.directory(self.case / "retained-index")
        self.temporary = self.directory(self.case / "tmp")
        self.result.update(cli_sha256=expected, runtime_manifest_sha256=manifest_info["sha256"],
                           source_commit=build.get("source_commit"),
                           environment={"platform": platform.platform(), "machine": platform.machine(),
                                        "python": sys.version, "script_sha256": file_info(Path(__file__).resolve())["sha256"]})
        write_json(self.case / "ownership.json", {"format": 1, "owner": str(uuid.uuid4()),
                   "case_root": str(self.case), "device": self.identities[self.case][0],
                   "inode": self.identities[self.case][1], "cli_sha256": expected})
        write_json(self.receipts / "runtime-manifest.json", build)
        write_json(self.case / "result.json", self.result)
        self.env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "en_US.UTF-8",
                    "TMPDIR": str(self.temporary)}
        for key in ("DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"):
            if key in os.environ:
                self.env[key] = os.environ[key]
        self.result["environment"]["child_environment"] = self.env

    def interrupt(self, signum, _frame=None):
        # Defer Python-level cancellation until Popen's returned child is owned.
        # Do not block OS signals: the child must not inherit a blocked mask.
        if self.spawning:
            self.pending_signal = self.pending_signal or signum
            return
        self.stop_child()
        raise QualificationError("Aggregate campaign deadline reached" if signum == signal.SIGALRM
                                 else f"Campaign interrupted by signal {signum}")

    def stop_child(self, process=None):
        process = self.active if process is None else process
        if process is not None:
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            process.wait()
            if self.active is process:
                self.active = None

    def run(self, label, arguments, command):
        self.check_owned()
        remaining = self.remaining()
        current = self.binary.stat(follow_symlinks=False)
        require(stat.S_ISREG(current.st_mode) and
                (current.st_dev, current.st_ino, current.st_size, current.st_mtime_ns) ==
                tuple(self.binary_info[key] for key in ("device", "inode", "byte_len", "mtime_ns")),
                "Frozen runtime identity changed")
        number = len(self.commands) + 1
        prefix = self.logs / f"{number:03}-{label}"
        argv = [str(self.binary), *arguments, "--data-dir", str(self.data),
                "--credentials-dir", str(self.credentials), "--json"]
        metadata = {"sequence": number, "label": label, "argv": argv,
                    "started_at_utc": utc_now(), "timeout_seconds": min(PROCESS_SECONDS, remaining),
                    "cli_sha256": self.expected_sha, "outcome": "starting"}
        self.commands.append(metadata)
        write_json(prefix.with_suffix(".json"), metadata)
        started = time.monotonic()
        child_deadline = min(self.deadline, started + PROCESS_SECONDS)
        process = None
        try:
            with prefix.with_suffix(".stdout").open("xb") as stdout, prefix.with_suffix(".stderr").open("xb") as stderr:
                self.spawning = True
                try:
                    process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=stdout,
                                               stderr=stderr, env=self.env, start_new_session=True)
                    self.active = process
                    metadata["pid"] = process.pid
                finally:
                    self.spawning = False
                    if self.pending_signal is not None:
                        signum, self.pending_signal = self.pending_signal, None
                        self.interrupt(signum)
                try:
                    code = process.wait(timeout=max(0, child_deadline - time.monotonic()))
                except subprocess.TimeoutExpired as error:
                    metadata["outcome"] = "timeout"
                    self.stop_child()
                    raise QualificationError(f"Child deadline reached: {label}") from error
                metadata["process_wall_seconds"] = time.monotonic() - started
                metadata["returncode"] = code
                self.active = None
            require(code == 0, f"CLI failed: {label}, exit {code}")
            value = json.loads(prefix.with_suffix(".stdout").read_bytes())
            require(value.get("schema_version") == 1 and value.get("ok") is True
                    and value.get("command") == command and "data" in value,
                    f"Unexpected CLI envelope: {label}")
            self.remaining()
            metadata["outcome"] = "passed"
            return value["data"]
        except BaseException:
            if metadata["outcome"] != "timeout":
                metadata["outcome"] = "failed"
            self.stop_child()
            raise
        finally:
            # Also own any returned local child if registration failed unusually.
            if process is not None and process.returncode is None:
                self.stop_child(process)
            metadata["wall_seconds"] = time.monotonic() - started
            if process is not None:
                metadata["returncode"] = process.returncode
            self.check_owned()
            for suffix in ("stdout", "stderr"):
                path = prefix.with_suffix("." + suffix)
                if path.exists():
                    metadata[suffix] = file_info(path)
            write_json(prefix.with_suffix(".json"), metadata)

    def generate(self):
        for folder in range(50):
            self.directory(self.vault / f"current-{folder:02}")
        for index in range(NOTE_COUNT):
            self.remaining()
            path = f"current-{index // 100:02}/{index:04}.md"
            note_id = str(uuid.uuid5(NAMESPACE, path))
            newline = "\r\n" if index % 3 == 0 else "\n"
            lines = ["---", f"brn_id: {note_id}", "brn_kind: knowledge", "brn_state: current", "---",
                     f"# Synthetic note {index:04}", "", "Synthetic qualification only. Õun, 日本語 and λ are retained exactly."]
            if index in ANCHORS:
                lines += ["", f"Unique scale witness brnscaleanchor{index:04} is retained here."]
            lines += ["", *[f"Synthetic paragraph {item}: deterministic local Markdown for note {index:04}; no real owner information." for item in range(8)]]
            text = ("\ufeff" if index % 7 == 0 else "") + newline.join(lines) + newline
            target = self.vault / path
            with target.open("xb") as output:
                output.write(text.encode("utf-8"))
            self.notes[path] = {"path": path, "note_id": note_id, **file_info(target)}
        self.result["dataset"] = {"current_notes": NOTE_COUNT, "folders": 50,
                                  "bytes": sum(item["byte_len"] for item in self.notes.values()),
                                  "anchors": [f"current-{index // 100:02}/{index:04}.md" for index in ANCHORS]}
        write_json(self.receipts / "note-manifest.json", list(self.notes.values()))

    def status(self, phase, initial=False):
        arguments = ["status"] + (["--vault", str(self.vault)] if initial else [])
        value = self.run(f"{phase}-status", arguments, "status")
        require(value["vault_root"] == str(self.vault) and value["model_installed"] is False,
                "Status did not confirm this vault with no model")
        return value

    def seed_guards(self):
        self.proposal_id = str(uuid.uuid5(NAMESPACE, "unapproved-proposal"))
        request = {"id": self.proposal_id, "group_id": None, "session_id": None,
                   "title": "Synthetic retained unapproved scale guard", "changes": [
                       {"kind": "create", "path": "unapproved-scale-guard.md",
                        "text": "Unapproved synthetic work õ\r\n"}], "sources": []}
        path = self.inputs / "proposal.json"
        write_json(path, request)
        proposal = self.run("seed-proposal", ["proposals", "create", "--file", str(path)], "proposals.create")
        require(proposal["draft"]["id"] == self.proposal_id and proposal["state"] == "draft",
                "Unapproved proposal guard was not retained as Draft")
        self.inbox_id = str(uuid.uuid5(NAMESPACE, "retained-original"))
        self.original_text = "\ufeffSynthetic retained Text Inbox original õ\r\n日本語 λ\r\n"
        original = self.inputs / "original.txt"
        with original.open("xb") as output:
            output.write(self.original_text.encode("utf-8"))
        item = self.run("seed-inbox", ["inbox", "add", "--id", self.inbox_id,
                        "--kind", "text", "--title", "Synthetic scale original",
                        "--file", str(original)], "inbox.add")
        require(item["capture"]["id"] == self.inbox_id, "Wrong Inbox guard identity")
        self.guards = self.read_guards("before")
        require(self.guards["proposal"] == proposal and self.guards["inbox"]["item"] == item,
                "Guard reads differ from initial typed creation receipts")
        copy = item["capture"]["copy"]
        directory = Path(copy["directory"])
        require(directory.is_absolute() and directory.resolve(strict=True) == directory
                and self.data in directory.parents, "Inbox original directory is outside the owned data")
        self.original_path = directory / f"{self.inbox_id}.txt"
        self.original_info = file_info(self.original_path)
        require((directory.stat().st_dev, directory.stat().st_ino) ==
                (copy["directory_device"], copy["directory_inode"]), "Inbox directory proof differs")
        require((self.original_info["device"], self.original_info["inode"], self.original_info["byte_len"]) ==
                (copy["file_device"], copy["file_inode"], copy["byte_len"])
                and self.original_info["sha256"] == bytes(copy["sha256"]).hex(),
                "Inbox physical original proof differs")
        write_json(self.receipts / "guard-baseline.json", {"typed": self.guards, "original": self.original_info})

    def read_guards(self, phase):
        proposal = self.run(f"{phase}-proposal", ["proposals", "show", self.proposal_id], "proposals.show")
        inbox = self.run(f"{phase}-inbox", ["inbox", "show", self.inbox_id], "inbox.show")
        proposals = self.run(f"{phase}-proposals", ["proposals", "list"], "proposals.list")
        actions = self.run(f"{phase}-actions", ["actions", "list"], "actions.list")
        conversations = self.run(f"{phase}-conversations", ["conversations", "list", "--state", "all"], "conversations.list")
        require(proposal["state"] == "draft" and proposal["draft"]["id"] == self.proposal_id,
                "Retained draft changed identity/state")
        require(proposals == [proposal], "Unexpected proposal membership/effects")
        require(inbox["item"]["capture"]["id"] == self.inbox_id
                and inbox["original"] == {"state": "available", "text": self.original_text},
                "Original bytes/availability changed")
        require(actions == {"entries": [], "next_before": None}, "Unexpected real Action effect")
        require(conversations == {"conversations": [], "lifecycles": [], "state": "all"},
                "Unexpected conversation/inference effect")
        return {"proposal": proposal, "inbox": inbox, "proposals": proposals,
                "actions": actions, "conversations": conversations}

    def inventory(self, phase):
        rows, cursor = [], None
        for page in range(25):
            arguments = ["notes", "list", "--scope", "current"]
            if cursor is not None:
                arguments += ["--cursor", cursor]
            value = self.run(f"{phase}-page-{page + 1:02}", arguments, "notes.list")
            require(value["scope"] == "current" and len(value["notes"]) == PAGE_SIZE,
                    f"Incorrect page size/scope: {phase}/{page + 1}")
            for row in value["notes"]:
                expected = self.notes.get(row["path"])
                require(expected is not None, f"Unexpected note: {row['path']}")
                facts = row["facts"]
                require(facts["note_id"] == expected["note_id"]
                        and facts["sha256"] == list(bytes.fromhex(expected["sha256"]))
                        and facts["source"] is False and facts["history"] is False,
                        f"Full UUID/hash/Current facts differ: {row['path']}")
            paths = [row["path"] for row in value["notes"]]
            require(paths == sorted(paths) and (cursor is None or paths[0] > cursor),
                    "Nonexclusive or unordered page")
            rows.extend(value["notes"])
            cursor = value["next_cursor"]
            require(cursor == (paths[-1] if page < 24 else None), "Unexpected terminal/keyset cursor")
        require([row["path"] for row in rows] == sorted(self.notes)
                and len({row["facts"]["note_id"] for row in rows}) == NOTE_COUNT,
                "Inventory is not exactly 5,000 unique paths/UUIDs")
        write_json(self.receipts / f"{phase}-inventory.json", rows)
        return rows

    def searches(self, phase):
        values = []
        for index in ANCHORS:
            path = f"current-{index // 100:02}/{index:04}.md"
            token = f"brnscaleanchor{index:04}"
            search = self.run(f"{phase}-search-{index:04}", ["search", token, "--profile", "keyword",
                              "--scope", "current", "--limit", "5"], "search")
            require(search["keyword_only"] is True and search["scope"] == "current"
                    and search["query"] == token and len(search["hits"]) == 1,
                    f"Unique keyword witness failed: {phase}/{token}")
            hit = search["hits"][0]
            require(hit["path"] == path, "Unique search returned another path")
            shown = self.run(f"{phase}-show-{index:04}", ["notes", "show", path, "--scope", "current"], "notes.show")
            require(shown["path"] == path and shown["scope"] == "current", "Wrong full-note reply")
            data = shown["text"].encode("utf-8")
            require(hashlib.sha256(data).hexdigest() == self.notes[path]["sha256"], "Full shown hash differs")
            start, end = hit["start_byte"], hit["end_byte"]
            require(type(start) is int and type(end) is int and 0 <= start < end <= len(data),
                    "Search byte range is malformed")
            require(data[start:end].decode("utf-8") == hit["quote"] and token in hit["quote"],
                    "Search quote is not the exact saved UTF-8 range")
            values.append({"search": search, "shown": shown})
        write_json(self.receipts / f"{phase}-searches.json", values)
        return values

    def preserve_index(self):
        self.remaining()
        self.check_owned()
        require(self.active is None, "A child is still active before index retention")
        require((self.data / "index.sqlite").is_file(), "No derived index to rebuild")
        moved = []
        for name in INDEX_FAMILY:
            self.remaining()
            self.check_owned()
            source, target = self.data / name, self.retained / name
            if not os.path.lexists(source):
                continue
            require(not os.path.lexists(target), "Retained index destination is occupied")
            info = file_info(source)
            entry = {"name": name, "before": info, "state": "move-intent"}
            moved.append(entry)
            write_json(self.receipts / "index-retention.json", moved)
            source.rename(target)
            require(file_info(target) == info, "Retained disposable index file changed")
            entry["state"] = "retained"
            write_json(self.receipts / "index-retention.json", moved)
        require(moved and moved[0]["name"] == "index.sqlite", "Main index was not retained")

    def verify_files(self):
        self.check_owned()
        actual = set()
        for directory, folders, files in os.walk(self.vault, followlinks=False):
            for folder in folders:
                require(not (Path(directory) / folder).is_symlink(), "Unexpected vault directory symlink")
            for name in files:
                self.remaining()
                path = Path(directory) / name
                relative = path.relative_to(self.vault).as_posix()
                require(relative in self.notes, f"Unexpected vault file/effect: {relative}")
                expected = {key: value for key, value in self.notes[relative].items() if key not in ("path", "note_id")}
                require(file_info(path) == expected, f"Note bytes/physical identity changed: {relative}")
                actual.add(relative)
        require(actual == set(self.notes), "Vault note membership changed")
        require(file_info(self.original_path) == self.original_info, "Retained Inbox physical proof changed")
        require(not list(self.credentials.iterdir()) and not list(self.models.iterdir()),
                "Credentials/model directories are no longer empty")
        require(file_info(self.binary) == self.binary_info and file_info(self.manifest_path) == self.runtime_manifest_info,
                "Frozen runtime/manifest changed during campaign")

    def execute(self):
        self.generate()
        self.status("cold", initial=True)
        self.seed_guards()
        before = self.inventory("before")
        searches = self.searches("before")
        self.status("restart")
        require(self.searches("restart") == searches, "Fresh-process search/read results differ")
        self.preserve_index()
        self.status("rebuild")
        require(self.inventory("rebuilt") == before, "Rebuilt inventory differs")
        require(self.searches("rebuilt") == searches, "Rebuilt search/read results differ")
        require(self.read_guards("after") == self.guards, "Retained operational/original records changed")
        self.verify_files()
        self.remaining()
        self.result["outcome"] = "passed"

    def finish(self, error=None):
        self.stop_child()
        if not self.case.exists() or not self.identities:
            return
        self.check_owned()
        if error is not None:
            self.result.update(outcome="failed", error=str(error), error_type=type(error).__name__)
        observed = {item["label"]: item["wall_seconds"] for item in self.commands if "wall_seconds" in item}
        self.result.update(finished_at_utc=utc_now(), wall_seconds=time.monotonic() - self.started,
                           commands=self.commands, process_count=len(self.commands),
                           observed_seconds={"cold_startup": observed.get("cold-status"),
                               "restart_startup": observed.get("restart-status"),
                               "index_rebuild_startup": observed.get("rebuild-status"),
                               "pages": {key: value for key, value in observed.items() if "-page-" in key},
                               "search_and_full_read": {key: value for key, value in observed.items() if "-search-" in key or "-show-" in key}})
        write_json(self.case / "result.json", self.result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, help="Absolute canonical immutable runtime directory with brn and build-manifest.json")
    parser.add_argument("--case-root", required=True, help="Absolute new nonexistent owned case path; failures are retained and reuse is refused")
    args = parser.parse_args()
    started = time.monotonic()
    campaign = None

    def interrupted(signum, _frame):
        if campaign is not None:
            return campaign.interrupt(signum)
        raise QualificationError("Aggregate campaign deadline reached" if signum == signal.SIGALRM
                                 else f"Campaign interrupted by signal {signum}")

    try:
        require(os.name == "posix", "This campaign requires POSIX process-group deadlines")
        signal.signal(signal.SIGALRM, interrupted)
        signal.signal(signal.SIGTERM, interrupted)
        signal.signal(signal.SIGINT, interrupted)
        signal.setitimer(signal.ITIMER_REAL, CAMPAIGN_SECONDS)
        runtime = checked_absolute(args.runtime, existing=True)
        case = checked_absolute(args.case_root, existing=False)
        campaign = Campaign(runtime, case, started)
        campaign.initialize()
        campaign.execute()
        campaign.finish()
        print(json.dumps({"outcome": "passed", "result": str(case / "result.json"),
                          "wall_seconds": campaign.result["wall_seconds"],
                          "process_count": len(campaign.commands)}))
        return 0
    except (Exception, KeyboardInterrupt) as error:
        # After an abort, only terminate/reap and preserve diagnostic receipts.
        # No qualification work continues beyond the aggregate deadline.
        signal.setitimer(signal.ITIMER_REAL, 0)
        receipt_error = None
        if campaign is not None:
            try:
                campaign.finish(error)
            except Exception as failure:
                receipt_error = str(failure)
        print(json.dumps({"outcome": "failed", "error": str(error),
                          "receipt_error": receipt_error,
                          "retained_case": str(campaign.case) if campaign is not None and campaign.identities else None}), file=sys.stderr)
        return 1
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == "__main__":
    sys.exit(main())
