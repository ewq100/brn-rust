#!/usr/bin/env python3
"""One source-bound synthetic production-CLI qualification; never retries ask."""
import argparse
import hashlib
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

BASELINE = "6c86583d709e13e7fc3c5070008d49c824856d07"
ORIGINAL = ("# Synthetic harbor — Õhtu 🌊\n\n"
            "Unrelated opening: café, Tallinn and 東京 stay exactly as written.\n\n"
            "The unique harbor marker is cobalt boats beside pier 73.\n\n"
            "Unrelated closing: naïve sailors keep the green ledger.\n")
EXPECTED = ORIGINAL.replace("cobalt boats", "teal boats")
SENTINEL = "# Separate sentinel 🧭\n\nNever alter this: hõbe, 雪, 314159.\n"


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


def redact(value):
    if isinstance(value, dict):
        return {key: ("EXISTING_AUTH_LOCATION" if key in {"auth_file", "credentials_dir"}
                      and item is not None else redact(item)) for key, item in value.items()}
    if isinstance(value, list):
        return [redact(item) for item in value]
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["prepare", "live", "inspect", "verify"])
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--auth-file", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    state_file = root / "session.json"
    if args.phase == "prepare":
        if root.exists():
            parser.error("root must be fresh")
        if not args.auth_file or not args.auth_file.is_absolute():
            parser.error("prepare needs the existing absolute auth path")
        root.mkdir(mode=0o700)
        (root / "commands").mkdir()
        state = {"baseline": BASELINE, "session": "mac-precise-20261023-01",
                 "model": "gpt-6.1-sol", "effort": "medium", "attempts": [],
                 "maximum_attempts": 2, "timeout_seconds": 180, "tool_rounds": 4,
                 "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                 "fixtures": {"original": ORIGINAL, "expected": EXPECTED,
                              "sentinel": SENTINEL},
                 "hashes": {"original": sha(ORIGINAL), "expected": sha(EXPECTED),
                            "sentinel": sha(SENTINEL)}, "gates": {}}
    else:
        state = json.loads(state_file.read_text())
        assert state["binary_sha256"] == hashlib.sha256(args.binary.read_bytes()).hexdigest()

    def save():
        state_file.write_text(json.dumps(state, indent=2, ensure_ascii=False) + "\n")

    def cli(label, *options, allow_failure=False):
        command = [str(args.binary), "--json", "--data-dir", str(root / "data"), *options]
        start = time.monotonic()
        proc = subprocess.run(command, capture_output=True, text=True, timeout=195)
        result = json.loads(proc.stdout)
        # configure output/argv may contain the private auth location: never retain it.
        if label == "configure":
            record = {"command": ["configure", "--provider", "Codex", "--model",
                                  state["model"], "--effort", "medium", "--helper", "off",
                                  "--auth-file", "EXISTING_AUTH_FILE"],
                      "exit": proc.returncode, "ok": result["ok"]}
        else:
            record = {"command": options, "exit": proc.returncode, "result": result,
                      "elapsed_seconds": round(time.monotonic() - start, 3)}
        (root / "commands" / (label + ".json")).write_text(
            json.dumps(redact(record), indent=2, ensure_ascii=False) + "\n")
        if not allow_failure:
            assert proc.returncode == 0, (label, result)
        return result

    def note(label, ident, expected, version=None):
        record = cli(label, "read", "--id", ident)["result"]
        assert record["data"]["Note"]["markdown"] == expected, label
        if version is not None:
            assert record["version"] == version, label
        return record

    if args.phase == "prepare":
        cli("init", "init", "--purpose", "test")
        cli("configure", "configure", "--provider", "Codex", "--model", state["model"],
            "--effort", "medium", "--helper", "off", "--auth-file", str(args.auth_file))
        cli("delegation", "delegation", "--choice", "review-first")
        state["target"] = cli("target-create", "note-create", "--title", "Synthetic harbor",
                              "--text", ORIGINAL)["result"]["id"]
        state["sentinel"] = cli("sentinel-create", "note-create", "--title", "Sentinel",
                                "--text", SENTINEL)["result"]["id"]
        state["thread"] = cli("thread-create", "thread-start", "--title",
                              "One precise synthetic edit")["result"]["id"]
        note("baseline-target", state["target"], ORIGINAL, 1)
        note("baseline-sentinel", state["sentinel"], SENTINEL, 1)
        state["prompt"] = (
            f"In saved Note {state['target']}, replace only the unique exact phrase "
            "'cobalt boats' with 'teal boats'. Read the current Note, locate that exact "
            "passage and prepare a precise edit candidate for review. Preserve all other "
            "bytes, Unicode, paragraphs and metadata. Do not apply it or change any other "
            "record. Report the prepared candidate.")
        assert len((ORIGINAL + SENTINEL + state["prompt"]).encode()) <= 8192
        state["gates"]["preparation"] = "PASS"
    elif args.phase == "live":
        assert not state["attempts"], "no automatic retry or session reopening"
        assert state["gates"]["preparation"] == "PASS"
        # Lead checks current allowance and confirms admission before this phase.
        attempt = {"number": 1, "reserved_at": now(), "status": "reserved_before_launch"}
        state["attempts"].append(attempt)
        save()
        result = cli("ask", "--timeout", "180", "ask", "--thread", state["thread"],
                     "--prompt", state["prompt"], "--budget", "4", "--trace", "full",
                     allow_failure=True)
        attempt.update({"finished_at": now(), "status": "returned",
                        "cli_exit": result["exit"], "outcome": result["outcome"]})
        state["gates"]["live_transport"] = result["outcome"]
        save()
        note("after-ask-target", state["target"], ORIGINAL, 1)
        note("after-ask-sentinel", state["sentinel"], SENTINEL, 1)
        candidates = cli("candidates", "candidates")["result"]
        state["candidate_operations"] = [c["operation"] for c in candidates]
        state["gates"]["candidate"] = "PENDING_REVIEW" if len(candidates) == 1 else "FAIL"
    elif args.phase == "inspect":
        assert len(state["attempts"]) == 1
        note("inspect-target", state["target"], ORIGINAL, 1)
        note("inspect-sentinel", state["sentinel"], SENTINEL, 1)
        candidates = cli("candidates-inspected", "candidates")["result"]
        assert len(candidates) == 1
        candidate = candidates[0]
        writes = candidate["request"]["writes"]
        assert len(writes) == 1
        write = writes[0]
        assert write["id"] == state["target"] and write["expected_version"] == 1
        before = json.loads((root / "commands/baseline-target.json").read_text())
        expected_data = before["result"]["result"]["data"]
        expected_data["Note"]["markdown"] = EXPECTED
        assert write["data"] == expected_data and not write["archived"]
        state["candidate_operations"] = [candidate["operation"]]
        state["candidate_review"] = "PASS"
        state["gates"]["candidate"] = "PENDING_REVIEW"
    else:
        assert state["gates"]["candidate"] == "PENDING_REVIEW"
        op = state["candidate_operations"][0]
        # Lead independently inspects candidate request before authorizing this phase.
        assert state.get("candidate_review") == "PASS"
        applied = cli("apply", "apply", "--operation", op)
        assert applied["outcome"] == "Applied"
        note("applied-restarted-target", state["target"], EXPECTED, 2)
        note("applied-restarted-sentinel", state["sentinel"], SENTINEL, 1)
        undo = cli("undo", "undo", "--operation", op, "--key", "precise-undo-01")
        assert undo["outcome"] == "Applied"
        restored = note("undo-restarted-target", state["target"], ORIGINAL, 3)
        sentinel = note("undo-restarted-sentinel", state["sentinel"], SENTINEL, 1)
        replay = cli("undo-keyed-replay", "undo", "--operation", op, "--key", "precise-undo-01")
        assert replay["operation"] == undo["operation"]
        assert note("replay-restarted-target", state["target"], ORIGINAL, 3) == restored
        assert note("replay-restarted-sentinel", state["sentinel"], SENTINEL, 1) == sentinel
        state["gates"].update({"candidate": "PASS", "apply": "PASS",
                               "restart_read": "PASS", "undo_restart": "PASS",
                               "keyed_undo_replay": "PASS", "sentinel": "PASS"})
    save()
    print(json.dumps({"phase": args.phase, "gates": state["gates"]}))


if __name__ == "__main__":
    main()
