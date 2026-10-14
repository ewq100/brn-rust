#!/usr/bin/env python3
"""Synthetic daily-workflow CLI journey for the reliability evaluation.

Drives the real `brn` CLI against a fresh explicit data directory. The
configured Codex auth file does not exist, so every provider step fails
deterministically before any network request. Prints one JSON evidence
object; never reads real owner data.

Usage: cli-journey.py BRN_BINARY ROOT_DIRECTORY
"""
import json
import os
import shutil
import signal
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

BRN, ROOT = sys.argv[1], Path(sys.argv[2])
if ROOT.exists():
    shutil.rmtree(ROOT)
DATA = ROOT / "data"
FIX = ROOT / "fixtures"
FIX.mkdir(parents=True)
evidence = {"steps": [], "observations": {}}

(FIX / "harbor-plan.md").write_text(
    "# Harbor crane plan\n\nThe Nordcrane unit needs a dredged berth of at least 1.2 m.\n"
    "Assembly takes 5 working days after delivery.\n"
)
(FIX / "mari-email.txt").write_text(
    "From: Mari Example <mari@example.invalid>\nSubject: Crane delivery\n\n"
    "The berth survey finished early: measured depth is 1.4 m.\n"
    "Can we move crane delivery to 15 November?\n"
)
(FIX / "delivery-decision.md").write_text(
    "# Crane delivery decision\n\nDecision: crane delivery is 22 November.\n"
)


def brn(*args, ok=True):
    proc = subprocess.run([BRN, "--data-dir", str(DATA), *args], capture_output=True, text=True)
    out = proc.stdout.strip()
    try:
        value = json.loads(out) if out else None
    except json.JSONDecodeError:
        value = out
    step = {"args": [a if len(a) < 120 else a[:117] + "..." for a in args], "exit": proc.returncode}
    if proc.returncode:
        step["stderr"] = proc.stderr.strip()[-400:]
    evidence["steps"].append(step)
    if ok and proc.returncode:
        raise SystemExit(f"step failed: {args}: {proc.stderr}")
    return value


def db():
    # Read-only inspection of the canonical store.
    return sqlite3.connect(f"file:{DATA / 'threads.sqlite3'}?mode=ro", uri=True)


def kinds():
    counts = {}
    for (body,) in db().execute("SELECT body FROM records"):
        record = json.loads(body)
        kind = next(iter(record["data"]))
        key = kind + (" (archived)" if record.get("archived") else "")
        counts[key] = counts.get(key, 0) + 1
    return counts


def records(kind):
    rows = []
    for (rid, version, body) in db().execute("SELECT id,version,body FROM records"):
        record = json.loads(body)
        if kind in record["data"]:
            rows.append({"id": rid, "version": version, "archived": record.get("archived", False), **record["data"][kind]})
    return rows


obs = evidence["observations"]
brn("init")
obs["files"] = sorted(p.name for p in DATA.iterdir())
brn("configure", "--provider", "Codex", "--model", "gpt-6.1-sol", "--effort", "medium",
    "--auth-file", str(ROOT / "missing-auth.json"))
brn("delegation", "--choice", "maintain")

# 1. Project with instructions; a replayed create returns the same project.
created = brn("project-create", "--title", "Harbor crane", "--key", "journey-project")
again = brn("project-create", "--title", "Harbor crane", "--key", "journey-project")
project = brn("projects")[0]
obs["project_create_replay_same"] = created == again and len(brn("projects")) == 1
brn("project-settings", "--project", project["id"], "--base", str(project["version"]),
    "--title", "Harbor crane", "--knowledge", "yes",
    "--instructions", "Cite the source for every date. Never promise delivery dates; ask Eva first.")
chat = brn("project-chat", "--project", project["id"], "--title", "Crane delivery")
chat_id = next(w["after"]["id"] for w in chat["writes"] if "Thread" in w["after"]["data"])

# Existing general knowledge: an ordinary note and a protected decision.
note = brn("note-create", "--title", "Berth survey", "--text", "# Berth survey\n\nSurvey scheduled for October.\n")
decision = brn("intake", "--source", str(FIX / "delivery-decision.md"), "--intent", "full", "--title", "Crane delivery decision")

# 2. Document into the project (full, private), twice.
first_doc = brn("intake", "--source", str(FIX / "harbor-plan.md"), "--intent", "project", "--project", project["id"], "--title", "Harbor crane plan")
second_doc = brn("intake", "--source", str(FIX / "harbor-plan.md"), "--intent", "project", "--project", project["id"], "--title", "Harbor crane plan")
obs["after_document_twice"] = kinds()
obs["project_documents"] = len(brn("projects")[0]["documents"]) if "documents" in brn("projects")[0] else brn("projects")[0]
obs["second_document_receipt_reason"] = second_doc.get("receipt", {}).get("reason")
obs["general_search_excludes_project_doc"] = brn("search", "--query", "Nordcrane")["total"]
obs["project_search_finds_doc"] = brn("search", "--query", "Nordcrane", "--project", project["id"])["total"]

# Email as useful information "for the project" (no --thread): provider fails.
threads_before = len(brn("threads"))
brn("intake", "--source", str(FIX / "mari-email.txt"), "--intent", "useful", "--project", project["id"], ok=False)
brn("intake", "--source", str(FIX / "mari-email.txt"), "--intent", "useful", "--project", project["id"], ok=False)
threads_after = brn("threads")
obs["useful_intake_new_threads"] = len(threads_after) - threads_before
obs["useful_intake_thread_titles"] = sorted(t["data"]["Thread"]["title"] for t in threads_after)
obs["sources_after_useful_project_intake"] = [
    {"locator": Path(s["locator"]).name, "chat": s.get("chat"), "version": s["version"], "outcome": s["outcome"]}
    for s in records("Source")]
obs["runs_after_useful_intake"] = [{"thread": r["thread"] == chat_id, "state": r["state"]} for r in records("Run")]

# Email into the project chat: provider fails, nothing duplicated on retry.
brn("intake", "--source", str(FIX / "mari-email.txt"), "--intent", "useful", "--thread", chat_id, ok=False)
question = "When can the crane arrive, and is the berth deep enough? Use the plan, Mari's email and the survey note."
brn("ask", "--thread", chat_id, "--prompt", question, ok=False)
brn("ask", "--thread", chat_id, "--prompt", question, ok=False)
owner_messages = [m for m in records("Message") if m["thread"] == chat_id and m["role"] == "owner"]
obs["project_chat_owner_messages"] = [m["text"][:60] for m in owner_messages]
obs["project_chat_runs"] = [r["state"] for r in records("Run") if r["thread"] == chat_id]
obs["failed_run_progress"] = [r["progress"][:200] for r in records("Run") if r["thread"] == chat_id]
obs["needs_you"] = len(brn("needs-you"))

# Stale edit from another writer: session A (desktop) and B (CLI).
note_id = note["id"]
a = brn("begin-edit", "--note", note_id, "--base", "1")
second = brn("begin-edit", "--note", note_id, "--base", "1", ok=False)
obs["second_dirty_session_refused"] = evidence["steps"][-1]["exit"] != 0
obs["second_session_error"] = evidence["steps"][-1].get("stderr")
brn("update-buffer", "--session", a["id"], "--generation", "1", "--text", "# Berth survey\n\nDesktop typing: depth 1.4 m.\n")
obs["recovery_before_save"] = [(r["note"], r["generation"]) for r in brn("recovery")]
saved = brn("save", "--session", a["id"], "--generation", "1")
obs["save_outcome"] = saved["outcome"]
replay = brn("save", "--session", a["id"], "--generation", "1")
obs["save_replay_outcome"] = replay["outcome"]
old = brn("begin-edit", "--note", note_id, "--base", "1", ok=False)
obs["begin_edit_at_old_base"] = evidence["steps"][-1].get("stderr") or {"opened_base": old.get("base")}
if isinstance(old, dict):
    current = brn("begin-edit", "--note", note_id, "--base", "2", ok=False)
    obs["begin_edit_current_while_old_open"] = evidence["steps"][-1].get("stderr") or "opened"
    obs["recovery_with_old_base_session"] = [(r["note"], r["base_version"], r["generation"]) for r in brn("recovery")]
    brn("discard", "--session", old["id"], "--generation", str(old["generation"]), ok=False)
    obs["discard_untyped_old_session"] = evidence["steps"][-1].get("stderr") or "discarded"
b = brn("begin-edit", "--note", note_id, "--base", "2")
brn("update-buffer", "--session", b["id"], "--generation", "1", "--text", "# Berth survey\n\nCLI edit.\n")
# Another writer changes the note underneath the open session B.
undo = brn("undo", "--operation", saved["receipt"]["operation"], ok=False)
obs["undo_while_other_session_open"] = undo["outcome"] if isinstance(undo, dict) else evidence["steps"][-1].get("stderr")
stale = brn("save", "--session", b["id"], "--generation", "1")
obs["save_after_other_writer"] = stale["outcome"]
obs["recovery_after"] = [(r["note"], r["generation"], r["markdown"][-12:]) for r in brn("recovery")]

# Backup, interrupted backup attempts, restore.
backup = ROOT / "backup.sqlite"
brn("backup", "--destination", str(backup))
again = brn("backup", "--destination", str(backup), ok=False)
obs["backup_occupied_refused"] = evidence["steps"][-1]["exit"] != 0
killed = []
for delay in (0.02, 0.05, 0.1, 0.2):
    target = ROOT / f"killed-{delay}.sqlite"
    proc = subprocess.Popen([BRN, "--data-dir", str(DATA), "backup", "--destination", str(target)],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(delay)
    proc.send_signal(signal.SIGKILL)
    proc.wait()
    killed.append({"delay": delay, "final_exists": target.exists(),
                   "stages": len(list(ROOT.glob(".brn-backup-stage-*")))})
obs["killed_backups"] = killed
restored = ROOT / "restored"
proc = subprocess.run([BRN, "--data-dir", str(restored), "restore", "--source", str(backup)], capture_output=True, text=True)
obs["restore_exit"] = proc.returncode
obs["restored_threads_equal"] = proc.returncode == 0 and len(json.loads(subprocess.run(
    [BRN, "--data-dir", str(restored), "threads"], capture_output=True, text=True).stdout)) == len(brn("threads"))
proc = subprocess.run([BRN, "--data-dir", str(restored), "restore", "--source", str(backup)], capture_output=True, text=True)
obs["restore_into_existing_refused"] = proc.returncode != 0

obs["final_kinds"] = kinds()
obs["operations"] = db().execute("SELECT COUNT(*) FROM operations").fetchone()[0]
obs["receipts"] = db().execute("SELECT COUNT(*) FROM receipts").fetchone()[0]
print(json.dumps(evidence, indent=1))
