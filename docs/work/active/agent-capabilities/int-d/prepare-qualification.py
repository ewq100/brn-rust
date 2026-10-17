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
                ask = common + ["ask", "--thread", case["chat"], "--prompt-file", str(prompt), "--budget", "8", "--trace", "summary"]
                workflows.append({"ordinal": len(workflows) + 1, "group": group, "repeat": repeat,
                                  "arm": arm, "facts": case, "prompt": str(prompt), "prompt_sha256": sha(prompt),
                                  "initial_database_sha256": sha(Path(case["data_dir"]) / "threads.sqlite3"),
                                  "configure_command": common + ["configure", "--provider", "Codex", "--model", "gpt-6.1-sol",
                                      "--effort", "medium", "--auth-file", "/Users/evokessler/.codex/auth.json"],
                                  "ask_command": ask,
                                  "stop_observer": None if group == "history" else {
                                      "poll_command": common + ["candidates", "--limit", "100"],
                                      "interval_seconds": 1, "maximum_polls": 120,
                                      "target": case["note"], "signal": "SIGINT to owned ask PID only",
                                      "stage_gap": "If process already ended or no exact candidate observed, record gap; no retry or fabricated Stop"},
                                  "continue_command": None if group == "history" else common + ["ask", "--thread", case["chat"],
                                      "--prompt-file", str(cont), "--budget", "8", "--trace", "summary"],
                                  "inspect_commands": [common + ["runs", "--thread", case["chat"]],
                                                       common + ["read", "--id", case["note"]]],
                                  "review": "Inspect exact operation/hash/source/current Note before owner Apply/replay/Undo. No blanket auto-approval."})
    report = {"state": "PREPARED_NOT_ACTIVATED", "live_workflow_starts": 0,
              "source_pin": a.pin, "source_root": str(root), "corpus_sha256": sha(corpus_path),
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


if __name__ == "__main__":
    main()
