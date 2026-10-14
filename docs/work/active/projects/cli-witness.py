#!/usr/bin/env python3
"""Offline Projects journey through the built CLI; no direct database writes."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

binary = str(Path(sys.argv[1]).resolve(strict=True))
checks = 0
calls = 0


def check(condition):
    global checks
    assert condition
    checks += 1


with tempfile.TemporaryDirectory(prefix="brn-projects-cli-", dir="/private/tmp") as scratch:
    root = Path(scratch).resolve()
    data = root / "data"

    def run(command, *, directory=None, **options):
        global calls
        args = [binary, "--data-dir", str(directory or data), command]
        for name, value in options.items():
            args.extend(["--" + name.replace("_", "-"), str(value)])
        result = subprocess.run(args, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0, (command, result.stderr)
        calls += 1
        return json.loads(result.stdout)

    def created(receipt, kind):
        return next(w["after"]["id"] for w in receipt["writes"] if kind in w["after"]["data"])

    def hits(**options):
        return {hit["note"] for hit in run("search", query="mercury", **options)["hits"]}

    run("init")
    first = run("project-create", title="Japan", key="project-once")
    check(run("project-create", title="Japan", key="project-once") == first)
    a = created(first, "Project")
    b = created(run("project-create", title="Writing"), "Project")
    chat = created(run("project-chat", project=a, title="Japan planning"), "Thread")
    shared = created(run("project-document", project=a, title="Mercury itinerary", text="mercury shared"), "Note")
    exclusive = created(run("project-document", project=a, title="Mercury letter", text="mercury private"), "Note")
    general = run("note-create", title="Mercury public", text="mercury general")["id"]
    run("project-link", project=b, note=shared)
    project = run("read", id=a)
    run("project-settings", project=a, base=project["version"], title="Japan", instructions="Write for beginners", knowledge="no")
    check(hits(project=a) == {shared, exclusive})
    check(hits() == {general})
    check(shared in hits(project=b) and exclusive not in hits(project=b))
    run("chat-move", thread=chat, project=b)
    check(chat in run("read", id=b)["data"]["Project"]["chats"])
    run("chat-move", thread=chat, project="none")
    check(run("read", id=chat)["data"]["Thread"]["private_context"])
    run("chat-move", thread=chat, project=a)
    preview = run("project-delete-preview", project=a)
    check(any(r["id"] == exclusive for r in preview["removed"]))
    check(any(r["id"] == shared for r in preview["retained"]))
    preview_file = root / "preview.json"
    preview_file.write_text(json.dumps(preview))
    removed = run("project-delete", preview_file=preview_file, actions="{}", key="delete-once")
    check(run("project-delete", preview_file=preview_file, actions="{}", key="delete-once") == removed)
    check(run("read", id=exclusive)["archived"])
    check(shared in hits(project=b))
    note = run("read", id=shared)
    edit = run("begin-edit", note=shared, base=note["version"])
    update = run("update-buffer", session=edit["id"], generation=edit["generation"] + 1, text="mercury later shared edit")
    check(run("save", session=edit["id"], generation=update["session"]["generation"])["outcome"] == "Saved")
    deleted = run("read", id=a)
    run("project-restore", project=a, base=deleted["version"])
    check(not run("read", id=exclusive)["archived"])
    check(run("read", id=shared)["data"]["Note"]["markdown"] == "mercury later shared edit")
    restored_project = run("read", id=a)["data"]["Project"]
    check(restored_project["instructions"] == "Write for beginners" and not restored_project["use_brn_knowledge"])
    check(hits(project=a) == {shared, exclusive})
    backup = root / "backup"
    run("backup", destination=backup)
    restored = root / "restored"
    run("restore", source=backup, directory=restored)
    check(run("read", id=shared, directory=restored)["data"]["Note"]["markdown"] == "mercury later shared edit")
    check(run("projects", directory=restored) == run("projects"))
    check({h["note"] for h in run("search", query="mercury", project=a, directory=restored)["hits"]} == {shared, exclusive})
print(json.dumps({"result": "passed", "checks": checks, "cli_calls": calls, "fixtures": "fresh synthetic; removed", "providers": "none"}))
