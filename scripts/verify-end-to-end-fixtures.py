"""Synthetic simple-vault reads/search and separate legacy local fixture."""
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
    data, vault, legacy = (scratch / name for name in ("simple", "vault", "legacy"))
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
    run("brn-flow", data, "sources", code=1)

    source = scratch / "source.txt"
    source.write_text("The syntheticterm bluejaytheta identifies this private fixture.\n")
    first = run("brn-flow", legacy, "import", "--file", str(source), "--approve", "yes")
    again = run("brn-flow", legacy, "import", "--file", str(source), "--approve", "yes")
    check(first["source_id"] == again["source_id"] and first["version_id"] == again["version_id"] and again["changed"] is False, again)
    projection = run("brn-flow", legacy, "sources")
    check(len(projection["sources"]) == 1 and projection["source_states"][0]["current_state"] == "Current", projection)
    check(run("brn-flow", legacy, "sessions") == [], "unexpected history")
    run("brn-flow", legacy, "build")
    hits = run("brn-flow", legacy, "search", "--query", "syntheticterm", "--profile", "keyword")
    check(bool(hits["evidence"]), hits)
    run("brn-flow", legacy, "search", "--query", "syntheticterm", "--profile", "semantic", code=1)
    source.write_text("The syntheticterm copperbadger identifies the updated fixture.\n")
    run("brn-flow", legacy, "import", "--file", str(source), "--approve", "yes")
    stale = run("brn-flow", legacy, "search", "--query", "copperbadger", code=1)
    check("stale" in stale, stale)
    run("brn-flow", legacy, "build")
    updated = run("brn-flow", legacy, "search", "--query", "copperbadger")
    check(any("copperbadger" in hit["quote"] for hit in updated["evidence"]), updated)
    run("brn-flow", legacy, "ask", "--query", "question", code=1)
    check(not (legacy / "brn.sqlite").exists(), "mixed simple authority")
    print(f"End-to-end fixtures passed: {checks} assertions (simple vault + legacy local; no account/model/network).")
finally:
    # Only this exclusively created, UUID-owned fixture; never follow symlinks.
    for entry in sorted(scratch.rglob("*"), key=lambda p: len(p.parts), reverse=True):
        if entry.is_symlink() or not entry.is_dir():
            entry.unlink()
        else:
            entry.rmdir()
    scratch.rmdir()
