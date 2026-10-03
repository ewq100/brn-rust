"""Synthetic current-vault reads/search, Save/recovery and marker refusal."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid

binary_dir = Path(sys.argv[1])
parent = Path(os.environ["TMPDIR"]).resolve(strict=True)
assert parent.is_dir() and parent.stat().st_uid == os.getuid()
scratch = parent / f"brn-e2e-{uuid.uuid4()}"
scratch.mkdir(mode=0o700)
checks = 0


def check(condition, detail):
    global checks
    assert condition, detail
    checks += 1


def run(binary, data, *args, code=0, envelope=False):
    output = subprocess.run(
        [str(binary_dir / binary), *args, "--data-dir", str(data)],
        capture_output=True, text=True, check=False,
    )
    check(output.returncode == code, (args, output.returncode, output.stderr))
    if not output.stdout:
        return output.stderr
    value = json.loads(output.stdout)
    if envelope:
        check(value["ok"] == (code == 0), value)
        return value["data"] if code == 0 else value["error"]
    return value


try:
    data, vault, legacy = (scratch / name for name in ("current", "vault", "old-marker"))
    for directory in (data, vault, legacy):
        directory.mkdir(mode=0o700)
    note = vault / "plan.md"
    original = b"\xef\xbb\xbf# Synthetic\r\nThe syntheticterm bluejaytheta launches Tuesday. \xce\xbb\r\n"
    note.write_bytes(original)
    (vault / "archive").mkdir()
    (vault / "archive" / "excluded.md").write_text("syntheticterm hidden")
    (vault / ".hidden.md").write_text("syntheticterm hidden")
    page = run("brn", data, "notes", "list", "--vault", str(vault), "--json", envelope=True)
    check([row["path"] for row in page["notes"]] == ["plan.md"], page)
    shown = run("brn", data, "notes", "show", "plan.md", "--json", envelope=True)
    check(shown["text"].encode() == original, shown)
    for profile in ("keyword", "semantic", "hybrid"):
        hits = run("brn", data, "search", "syntheticterm", "--profile", profile, "--json", envelope=True)
        check(hits["keyword_only"] is True and bool(hits["hits"]), hits)
        check(all(hit["path"] == "plan.md" for hit in hits["hits"]), hits)
    failed = run("brn", data, "ask", "question", "--json", code=1, envelope=True)
    check(failed["code"] == "AI_SELECTION_REQUIRED", failed)
    check(hashlib.sha256(note.read_bytes()).digest() == hashlib.sha256(original).digest(), "vault changed")
    note.write_bytes(original.replace(b"bluejaytheta", b"copperbadger"))
    refreshed = run("brn", data, "search", "copperbadger", "--json", envelope=True)
    check(bool(refreshed["hits"]), refreshed)
    check(not (data / "brn.sqlite3").exists(), "mixed legacy authority")
    check((data / "brn.sqlite").is_file(), "simple authority missing")
    # Each command is a fresh process, so rolling edits exercise restart recovery.
    opened = run("brn", data, "edit", "open", "plan.md", "--json", envelope=True)
    record = opened["record"]
    stamp = record["stamp"]
    input_file = scratch / "edit.txt"
    updated = original + "Unfinished synthetic edit: λ\r\n".encode()
    input_file.write_bytes(updated)
    common = ("plan.md", "--baseline", stamp["baseline"], "--expected-generation", "0", "--generation", "1", "--file", str(input_file))
    recovered = run("brn", data, "edit", "recover", *common, "--json", envelope=True)
    check(recovered["text"].encode() == updated and note.read_bytes() != updated, recovered)
    restarted = run("brn", data, "edit", "open", "plan.md", "--json", envelope=True)
    check(restarted["record"]["text"].encode() == updated, restarted)
    operation = str(uuid.uuid4())
    save_common = ("plan.md", "--baseline", stamp["baseline"], "--expected-generation", "1", "--generation", "1", "--file", str(input_file))
    saved = run("brn", data, "edit", "save", *save_common, "--operation", operation, "--json", envelope=True)
    check(saved["outcome"] == "Applied" and note.read_bytes() == updated, saved)
    replay = run("brn", data, "edit", "save", *save_common, "--operation", operation, "--json", envelope=True)
    check(replay == saved, replay)
    input_file.write_bytes(b"Copy synthetic exact bytes\r\n")
    stamp = saved["stamp"]
    copied = run("brn", data, "edit", "save", "plan.md", "--baseline", stamp["baseline"], "--expected-generation", "1", "--generation", "2", "--file", str(input_file), "--operation", str(uuid.uuid4()), "--copy", "copy.md", "--json", envelope=True)
    check(copied["outcome"] == "Applied" and (vault / "copy.md").read_bytes() == input_file.read_bytes(), copied)
    check(note.read_bytes() == updated, "copy changed original")
    occupied = run("brn", data, "edit", "save", "plan.md", "--baseline", stamp["baseline"], "--expected-generation", "2", "--generation", "2", "--file", str(input_file), "--operation", str(uuid.uuid4()), "--copy", "copy.md", "--json", code=1, envelope=True)
    check(occupied["code"] == "CONTEXT_STALE", occupied)
    (legacy / "brn.sqlite3-journal").write_bytes(b"synthetic old marker")
    failed = run("brn", legacy, "status", "--json", code=1, envelope=True)
    check(failed["code"] == "WORKSPACE_MODE_CONFLICT" and not (legacy / "brn.sqlite").exists(), failed)
    check((legacy / "brn.sqlite3-journal").read_bytes() == b"synthetic old marker", "marker changed")
    print(f"End-to-end fixtures passed: {checks} assertions (current vault, Save/recovery and marker refusal; no account/model/network).")
finally:
    # Only this exclusively created, UUID-owned fixture; never follow symlinks.
    for entry in sorted(scratch.rglob("*"), key=lambda p: len(p.parts), reverse=True):
        if entry.is_symlink() or not entry.is_dir():
            entry.unlink()
        else:
            entry.rmdir()
    scratch.rmdir()
