#!/usr/bin/env python3
"""Prepare public paired stores and commands ONLY. Never starts a provider.

Run after a local source pin. The tested CLI corpus generator is reused rather
than writing SQLite or duplicating the checked fixture construction here.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pin", required=True)
    p.add_argument("--binary", type=Path, required=True)
    p.add_argument("--generator", type=Path, required=True)
    p.add_argument("--receipt", type=Path, required=True)
    p.add_argument("--artifacts", type=Path, required=True)
    p.add_argument("--tag", required=True)
    a = p.parse_args()
    root = Path(__file__).resolve().parents[5]
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if head != a.pin:
        p.error("pin must equal current full HEAD; no implicit source substitution")
    if not a.tag or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in a.tag):
        p.error("tag must contain lowercase ASCII letters, digits or hyphens")
    receipt = json.loads(a.receipt.read_text())
    artifacts = json.loads(a.artifacts.read_text())
    if receipt["source_root"] != str(root) or not receipt["source_unchanged"]:
        p.error("source receipt must bind this unchanged literal source root")
    for path, digest in receipt["source_hashes"].items():
        if sha(root / path) != digest:
            p.error("compiled-input mismatch: " + path)
        pinned_bytes = subprocess.check_output(["git", "show", a.pin + ":" + path], cwd=root)
        if hashlib.sha256(pinned_bytes).hexdigest() != digest:
            p.error("compiled input is not committed at the supplied pin: " + path)
    for path in (Path(__file__), Path(__file__).with_name("observe-workflow.py")):
        pinned_bytes = subprocess.check_output(["git", "show", a.pin + ":" + str(path.relative_to(root))], cwd=root)
        if hashlib.sha256(pinned_bytes).hexdigest() != sha(path):
            p.error("preflight code is not the immutable pinned code")
    if artifacts["receipt_sha256"] != sha(a.receipt):
        p.error("artifact receipt does not bind supplied command receipt")
    pinned = {}
    for label, path in [("binary", a.binary), ("generator", a.generator)]:
        path = path.resolve(strict=True)
        matches = [r for r in artifacts["artifacts"] if r["path"] == str(path)
                   and r["not_modified_after_receipt"] and r["sha256"] == sha(path)]
        if not matches or not os.access(path, os.X_OK):
            p.error(label + " is not the source-associated executable in the receipt")
        row = matches[0]
        if not row["dep_info_sha256"] or sha(row["dep_info"]) != row["dep_info_sha256"]:
            p.error(label + " dep-info mismatch")
        pinned[label] = row
    corpus_path = root / "docs/work/active/agent-capabilities/int-d/corpus.json"
    corpus = json.loads(corpus_path.read_text())
    output = Path("/private/tmp") / ("brn-int-d-qualification-20261017-" + a.tag)
    output.mkdir(mode=0o700)  # no adoption, overwrite or cleanup of old data
    env = {k: v for k, v in os.environ.items() if not k.startswith("BRN_")
           and k not in ("AGENT_CONTRACT_BASELINE_BIN", "AGENT_CONTRACT_SCHEMA5_BIN")}
    env["TMPDIR"] = tempfile.mkdtemp(prefix="brn-int-d-preflight-", dir="/private/tmp")
    env["AGENT_CONTRACT_BASELINE_BIN"] = str(a.binary.resolve())
    workflows = []
    fixtures = []
    for group in ("history", "stop-continue"):
        for repeat in (1, 2):
            pair = Path("/private/tmp") / f"brn-int-d-qualification-20261017-{a.tag}-{group}-r{repeat}"
            if pair.exists():
                p.error("fixture output must be absent: " + str(pair))
            env["BRN_INT_D_FIXTURE_ROOT"] = str(pair)
            cmd = [str(a.generator.resolve()), "--exact",
                   "checked_source_labelled_pair_reopens_through_cli_with_explicit_scoped_selection",
                   "--test-threads=1", "--nocapture"]
            run = subprocess.run(cmd, env=env, cwd=root, capture_output=True, timeout=60)
            log = output / f"{group}-r{repeat}-generation.log"
            log.write_bytes(run.stdout + run.stderr)
            if run.returncode:
                p.error("fixture generator failed; preserved " + str(log))
            manifest = json.loads((pair / "manifest.json").read_text())
            if manifest["corpus_version"] != corpus["version"]:
                p.error("corpus version mismatch")
            fixtures.append({"manifest": str(pair / "manifest.json"),
                             "sha256": sha(pair / "manifest.json"),
                             "generation_log": str(log), "log_sha256": sha(log)})
            by_arm = {c["case"]: c for c in manifest["cases"]}
            for arm in (("control", "candidate") if repeat == 1 else ("candidate", "control")):
                case = by_arm[arm]
                prompt = output / f"{group}-r{repeat}-{arm}.txt"
                prompt.write_text(
                    f"Revise the synthetic Aurora delivery Note {case['note']} using the saved discussion. "
                    "Find the early accepted requirement and distinguish the later unaccepted proposal. "
                    "Read the exact saved Message and cite its ID/version and passage. "
                    "If an enabled review skill is available, read its reference before proposing. "
                    "Retain your objective and remaining work. Prepare the exact protected Note revision, "
                    "leave it for owner review, and do not claim it applied. Do not access external tools.\n"
                )
                cont = output / f"{group}-r{repeat}-{arm}-continue.txt"
                cont.write_text(corpus["continue_prompt"] + "\n")
                common = [str(a.binary.resolve()), "--json", "--data-dir", case["data_dir"], "--timeout", "120"]
                inspect = [str(a.binary.resolve()), "--json", "--data-dir", case["data_dir"], "--timeout", "2"]
                poll = inspect + ["candidates", "--limit", "100"]
                ask = common + ["ask", "--thread", case["chat"], "--prompt-file", str(prompt), "--budget", "8", "--trace", "summary"]
                workflows.append({"ordinal": len(workflows) + 1, "group": group, "repeat": repeat,
                                  "arm": arm, "facts": case, "prompt": str(prompt), "prompt_sha256": sha(prompt),
                                  "continue_prompt": str(cont), "continue_prompt_sha256": sha(cont),
                                  "manifest": str(pair / "manifest.json"), "manifest_sha256": sha(pair / "manifest.json"),
                                  "initial_database_sha256": sha(Path(case["data_dir"]) / "threads.sqlite3"),
                                  "configure_command": common + ["configure", "--provider", "Codex", "--model", "gpt-6.1-sol",
                                      "--effort", "medium", "--auth-file", "/Users/evokessler/.codex/auth.json"],
                                  "ask_command": ask,
                                  "stop_observer": None if group == "history" else {
                                      "poll_command": poll, "python_timeout_seconds": 3,
                                      "interval_seconds": 1, "maximum_polls": 120,
                                      "target": case["note"], "signal": "SIGINT to owned ask PID only",
                                      "stage_gap": "If process already ended or no exact candidate observed, record gap; no retry or fabricated Stop"},
                                  "continue_command": None if group == "history" else common + ["ask", "--thread", case["chat"],
                                      "--prompt-file", str(cont), "--budget", "8", "--trace", "summary"],
                                  "inspect_commands": [inspect + ["runs", "--thread", case["chat"]],
                                                       inspect + ["read", "--id", case["note"]]],
                                  "review": "Inspect exact operation/hash/source/current Note before owner Apply/replay/Undo. No blanket auto-approval."})
    report = {"state": "PREPARED_NOT_ACTIVATED", "live_workflow_starts": 0,
              "source_pin": a.pin, "source_root": str(root), "corpus": str(corpus_path), "corpus_sha256": sha(corpus_path),
              "preparer_sha256": sha(__file__),
              "observer_sha256": sha(Path(__file__).with_name("observe-workflow.py")),
              "compiled_artifacts": pinned,
              "check_receipt": str(a.receipt), "check_receipt_sha256": sha(a.receipt),
              "check_precommit_head": receipt["head"], "check_precommit_tracked_diff_sha256": receipt["diff_sha256"],
              "check_exit": receipt["exit"], "check_counts": receipt["test_summaries"],
              "route": {"provider": "Codex/ChatGPT", "model": "gpt-6.1-sol", "effort": "medium",
                        "credential_reference": "/Users/evokessler/.codex/auth.json",
                        "account_identity": "not exposed; not inspected", "auth_request_count": "not exposed"},
              "comparison": "Same current binary/history/summary runtime; installed-alone vs scoped selected configuration only",
              "mcp": "off in every workflow; deterministic real SDK on-path is separate evidence",
              "limits": {"total_workflow_starts": 12, "max_tool_rounds_each": 8, "host_timeout_seconds_each": 120,
                         "count_failed_stopped_continue": True, "automatic_retry_or_fallback": False},
              "fixtures": fixtures, "workflows": workflows}
    (output / "preflight.json").write_text(json.dumps(report, indent=2) + "\n")
    print(output / "preflight.json")



# Separate, prospective history-only method. Legacy preparation above is kept
# unchanged so old preflights retain their original meaning.
HISTORY_TASK = "int-d-history-phase-20261017"
AUTH_REFERENCE = "/Users/evokessler/.codex/auth.json"
HISTORY_FORMAT = "BRN-INT-D-HISTORY-1"
HISTORY_ORDER = [(1, "control"), (1, "candidate"), (2, "candidate"), (2, "control")]


def bounded_json(path, cap=4 * 1024 * 1024):
    path = Path(path)
    if path.is_symlink() or not path.is_file() or path.stat().st_size > cap:
        raise ValueError("missing, unsafe or oversized metadata file")
    return json.loads(path.read_bytes())


def stamp_database(data):
    result = {}
    for name in ("threads.sqlite3", "threads.sqlite3-wal", "threads.sqlite3-shm"):
        path = Path(data) / name
        if path.is_symlink():
            raise ValueError("database sidecar symlink")
        result[name] = {"sha256": sha(path), "bytes": path.stat().st_size} if path.exists() else None
    if result["threads.sqlite3"] is None:
        raise ValueError("missing main database")
    return result


def ordered_settings_digest(settings):
    # Rust serde_json preserve_order retains WorkspaceSettings declaration order.
    # Normal canonical `records` serializes that order; NEVER sort these keys.
    return hashlib.sha256(json.dumps(settings, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()


def history_context(envelope, facts, corpus):
    if envelope.get("status") != "ok" or envelope.get("authority", {}).get("workspace_purpose") != "test":
        raise ValueError("checked Test-purpose records required")
    records = envelope["result"]
    if not isinstance(records, list) or len(records) > 256:
        raise ValueError("bounded canonical records required")
    by_id = {r["id"]: r for r in records}
    if len(by_id) != len(records) or any("Run" in r["data"] for r in records):
        raise ValueError("duplicate records or existing Run")
    def record(identity, kind):
        r = by_id[identity]
        if r["archived"] or kind not in r["data"]:
            raise ValueError("canonical context unavailable")
        return r, r["data"][kind]
    project, _ = record(facts["project"], "Project")
    chat, thread = record(facts["chat"], "Thread")
    target, note = record(facts["note"], "Note")
    if thread["state"] != "Open" or not note["protected"] or target["version"] != 2 or facts["note_version"] != 2:
        raise ValueError("open context and protected Note v2 required")
    settings_rows = [r for r in records if not r["archived"] and "Settings" in r["data"]]
    if len(settings_rows) != 1:
        raise ValueError("unique canonical Settings required")
    settings_record = settings_rows[0]
    settings = settings_record["data"]["Settings"]
    if (settings["provider"], settings["model"], settings["effort"], settings["auth_file"]) != ("Codex", "gpt-6.1-sol", "medium", AUTH_REFERENCE) \
            or settings.get("credentials_dir") is not None or settings.get("helper") is not None or settings.get("mcp") is not None:
        raise ValueError("exact reference-only route, helpers/MCP off required")
    wanted = [{"id": facts["skill_id"], "digest": facts["skill_digest"], "project": facts["project"], "enabled": True, "selected": True}] if facts["case"] == "candidate" else []
    if facts["case"] not in ("control", "candidate") or facts["selected"] != (facts["case"] == "candidate") or settings.get("skills", []) != wanted:
        raise ValueError("exact scoped selected/installed-alone policy required")
    identities = facts["history_messages"]
    if len(identities) != 68 or len(set(identities)) != 68:
        raise ValueError("exact 68 canonical input identities required")
    messages = []
    for identity in identities:
        r, m = record(identity, "Message")
        if r["version"] != 1 or m["thread"] != facts["chat"]:
            raise ValueError("input identity/version/context changed")
        messages.append(r)
    if messages[0]["id"] != facts["early_message"] or messages[0]["data"]["Message"]["text"] != corpus["early_requirement"] \
            or messages[40]["id"] != facts["proposal_message"] or messages[40]["data"]["Message"]["text"] != corpus["later_proposal"]:
        raise ValueError("fixed early/later corpus differs")
    return {"purpose": "Test", "project": {"id": project["id"], "version": project["version"]},
            "chat": {"id": chat["id"], "version": chat["version"], "privacy": thread.get("private_context", False), "epoch": thread.get("context_epoch", 0)},
            "settings": {"id": settings_record["id"], "version": settings_record["version"], "policy_sha256": ordered_settings_digest(settings), "auth_reference": settings["auth_file"]},
            "target": {"id": target["id"], "version": target["version"], "protected": True},
            "inputs": sorted([{"id": r["id"], "version": r["version"]} for r in messages], key=lambda r: r["id"]),
            "package": {"id": facts["skill_id"], "digest": facts["skill_digest"], "selected": facts["selected"], "bindings": wanted}}


def history_prompt(note):
    return (f"Revise the synthetic Aurora delivery Note {note} using the saved discussion. "
            "Find the early accepted requirement and distinguish the later unaccepted proposal. "
            "Read the exact saved Message and cite its ID/version and passage. "
            "If an enabled review skill is available, read its reference before proposing. "
            "Retain your objective and remaining work. Prepare the exact protected Note revision, "
            "leave it for owner review, and do not claim it applied. Do not access external tools.\n")


def verify_source(root, pin, inputs):
    if len(inputs) != 334 or any(Path(p).is_absolute() or ".." in Path(p).parts for p in inputs):
        raise ValueError("exact bounded 334-input source map required")
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    names = list(inputs)
    blobs = subprocess.run(["git", "cat-file", "--batch"], cwd=root, env=env,
                           input="".join(pin + ":" + p + "\n" for p in names).encode(),
                           capture_output=True, timeout=10, check=True).stdout
    offset = 0
    for path in names:
        end = blobs.index(b"\n", offset)
        header = blobs[offset:end].split()
        if len(header) != 3 or header[1] != b"blob":
            raise ValueError("missing pinned source blob")
        size = int(header[2]); start = end + 1
        digest = hashlib.sha256(blobs[start:start + size]).hexdigest()
        if digest != inputs[path] or sha(Path(root) / path) != digest:
            raise ValueError("source/Git bytes changed")
        offset = start + size + 1


def history_main():
    p = argparse.ArgumentParser(description="OFFLINE history-only preflight, never generates/configures/asks")
    p.add_argument("--phase", choices=["history-only"], required=True)
    p.add_argument("--pin", required=True)
    p.add_argument("--receipt", type=Path, required=True)
    p.add_argument("--artifacts", type=Path, required=True)
    p.add_argument("--overlays", type=Path, nargs=2, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    root = Path(__file__).resolve().parents[5]
    receipt = bounded_json(a.receipt)
    if receipt["source_root"] != str(root) or not receipt["source_unchanged"] or receipt["exit"] != 0:
        p.error("passing exact-root source receipt required")
    verify_source(root, a.pin, receipt["source_hashes"])
    scripts = {}
    for name in ("prepare-qualification.py", "observe-workflow.py", "check-observer.py"):
        path = Path(__file__).with_name(name)
        relative = str(path.relative_to(root))
        pinned = subprocess.check_output(["git", "show", a.pin + ":" + relative], cwd=root)
        if hashlib.sha256(pinned).hexdigest() != sha(path):
            p.error("script not pinned")
        scripts[str(path)] = sha(path)
    artifacts = bounded_json(a.artifacts)
    if artifacts["receipt_sha256"] != sha(a.receipt):
        p.error("artifact/source receipt mismatch")
    pinned = artifacts["compiled_artifacts"]
    if set(pinned) != {"binary", "generator"}:
        p.error("exact CLI/generator/exporter association required")
    for row in pinned.values():
        if sha(row["path"]) != row["sha256"] or sha(row["dep_info"]) != row["dep_info_sha256"]:
            p.error("artifact changed")
    corpus_path = Path(__file__).with_name("corpus.json")
    corpus = bounded_json(corpus_path)
    out = a.output
    if out.parent != Path("/private/tmp") or not out.name.startswith("brn-int-d-history-preflight-20261017-"):
        p.error("fresh explicit owned preflight root required")
    # All fixture validation is read-only and precedes publishing the preflight.
    cases = {}
    overlays = []
    for repeat, path in enumerate(a.overlays, 1):
        overlay = bounded_json(path)
        pair = Path(overlay["pair"])
        if pair.parent != Path("/private/tmp") or pair.name != f"brn-int-d-qualification-20261017-history-{a.pin[:8]}-r{repeat}" or pair.resolve() != pair:
            p.error("wrong fresh pair/source/repeat identity")
        manifest_path = pair / "manifest.json"
        if sha(manifest_path) != overlay["manifest_sha256"]:
            p.error("initial manifest changed")
        manifest = bounded_json(manifest_path)
        if manifest["corpus_version"] != corpus["version"] or len(manifest["cases"]) != 2:
            p.error("wrong corpus manifest")
        for facts in manifest["cases"]:
            arm = facts["case"]
            record = overlay["cases"][arm]
            if facts["data_dir"] != str(pair / arm) or stamp_database(pair / arm) != record["database"]:
                p.error("final database/sidecar changed")
            for label in ("records", "context", "interop"):
                if sha(record[label]) != record[label + "_sha256"]:
                    p.error("postconfiguration/context/interop evidence changed")
            context = history_context(bounded_json(record["records"]), facts, corpus)
            if bounded_json(record["context"]) != context or bounded_json(record["interop"])["context"] != context:
                p.error("whole checked-copy context proof mismatch")
            cases[(repeat, arm)] = (facts, context, manifest_path, record)
        overlays.append({"path": str(path), "sha256": sha(path)})
    for label in ("method.md", "rubric.json"):
        source = Path(__file__).parent / "history-phase" / label
        relative = str(source.relative_to(root))
        if hashlib.sha256(subprocess.check_output(["git", "show", a.pin + ":" + relative], cwd=root)).hexdigest() != sha(source):
            p.error("method/rubric not pinned")
    out.mkdir(mode=0o700)
    bound = {}
    for label in ("method.md", "rubric.json"):
        source = Path(__file__).parent / "history-phase" / label
        (out / label).write_bytes(source.read_bytes())
        bound[str(out / label)] = sha(source)
    workflows = []
    for ordinal, (repeat, arm) in enumerate(HISTORY_ORDER, 1):
        facts, context, manifest, record = cases[(repeat, arm)]
        prompt = out / f"r{repeat}-{arm}.txt"
        prompt.write_text(history_prompt(facts["note"]))
        common = [pinned["binary"]["path"], "--json", "--data-dir", facts["data_dir"], "--timeout", "120"]
        workflows.append({"ordinal": ordinal, "repeat": repeat, "arm": arm, "facts": facts,
                          "context": context, "context_path": record["context"], "context_sha256": record["context_sha256"],
                          "records_path": record["records"], "records_sha256": record["records_sha256"],
                          "manifest": str(manifest), "manifest_sha256": sha(manifest), "database": record["database"],
                          "prompt": str(prompt), "prompt_sha256": sha(prompt), "stop_observer": None,
                          "continue_command": None, "configure_command": None,
                          "ask_command": common + ["ask", "--thread", facts["chat"], "--prompt-file", str(prompt), "--budget", "8", "--trace", "summary"]})
    report = {"format": HISTORY_FORMAT, "phase": "history-only", "task": HISTORY_TASK,
              "state": "PREPARED_NOT_ACTIVATED", "source_pin": a.pin, "source_root": str(root),
              "source_inputs": receipt["source_hashes"], "scripts": scripts, "bound_method": bound,
              "compiled_artifacts": pinned, "check_receipt": str(a.receipt), "check_receipt_sha256": sha(a.receipt),
              "corpus": str(corpus_path), "corpus_sha256": sha(corpus_path), "overlays": overlays,
              "route": {"provider": "Codex", "model": "gpt-6.1-sol", "effort": "medium", "auth_reference": AUTH_REFERENCE},
              "limits": {"total_workflow_starts": 4, "max_tool_rounds_each": 8, "host_timeout_seconds_each": 120},
              "output_namespace": str(out / "observations"), "workflows": workflows,
              "live_workflow_starts": 0, "auth_account_http": "UNKNOWN; not inspected", "activation": "NOT AUTHORIZED"}
    (out / "preflight.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print(out / "preflight.json")


if __name__ == "__main__":
    import sys
    if "--phase" in sys.argv:
        history_main()
    else:
        main()
