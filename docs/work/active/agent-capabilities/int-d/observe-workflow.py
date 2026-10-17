#!/usr/bin/env python3
"""One predeclared normal-CLI case, only after explicit principal activation.

No protocol/provider driver: starts the exact prepared CLI argv, observes its
canonical candidate through normal CLI, signals/reaps only its own process.
Never auto-applies a model candidate. --help and importing perform no I/O calls.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, value):
    # Single operator/observer; no scheduler or concurrent controller supported.
    stage = path.with_suffix(".stage")
    with stage.open("x") as f:
        json.dump(value, f, indent=2)
        f.write("\n")
        f.flush()
        os.fsync(f.fileno())
    stage.replace(path)


def reap(child):
    if child.poll() is None:
        child.send_signal(signal.SIGINT)
    try:
        return child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        child.terminate()
    try:
        return child.wait(timeout=3)
    except subprocess.TimeoutExpired:
        child.kill()
        return child.wait(timeout=3)


def exposed_metrics(stdout):
    try:
        result = json.loads(stdout)["result"]
        trace = result.get("trace")
    except (ValueError, KeyError, TypeError):
        return {"trace_available": False, "qualification": "no complete normal envelope; counters unknown"}
    if not isinstance(trace, dict):
        return {"trace_available": False, "terminal": result.get("terminal"), "counters": "unknown"}
    def usage(value):
        if not isinstance(value, dict):
            return None
        observed = dict(value)
        observed["cached_share"] = value["cached_input"] / value["input"] if value.get("input") and "cached_input" in value else None
        return observed
    return {"trace_available": True, "terminal": result.get("terminal"),
            "traced_model_turns": trace.get("model_turns"), "rounds": trace.get("rounds"),
            "max_rounds": trace.get("max_rounds"), "stopped": trace.get("stopped"),
            "runtime_elapsed_ms": trace.get("elapsed_ms"), "parent_usage": usage(trace.get("usage")),
            "turn_usage": [usage(v) for v in trace.get("turn_usage", [])],
            "summary": trace.get("summary"), "helpers": trace.get("helpers", []),
            "recorded_provider_resends": len(trace.get("provider_retries", [])),
            "auth_requests": "not exposed", "account_identity": "not exposed",
            "qualification": "observed trace only; no inferred HTTP totals/summary latency/free retries; missing usage unknown"}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--preflight", type=Path, required=True)
    p.add_argument("--activation", type=Path, required=True,
                   help="principal activation receipt binding this exact preflight/pin")
    p.add_argument("--ordinal", type=int, required=True)
    a = p.parse_args()
    plan = json.loads(a.preflight.read_text())
    approval = json.loads(a.activation.read_text())
    if approval.get("state") != "ACTIVATED" or approval.get("source_pin") != plan["source_pin"] \
            or approval.get("preflight_sha256") != sha(a.preflight):
        p.error("explicit exact-preflight activation is absent")
    if plan["mcp"] != "off in every workflow; deterministic real SDK on-path is separate evidence":
        p.error("this observer supports only the prepared MCP-off method")
    for row in plan["compiled_artifacts"].values():
        if sha(row["path"]) != row["sha256"] or sha(row["dep_info"]) != row["dep_info_sha256"]:
            p.error("executable/dep-info changed since pinned preflight")
    if sha(__file__) != plan["observer_sha256"]:
        p.error("observer changed since pinned preflight")
    if a.ordinal < 1 or a.ordinal > len(plan["workflows"]):
        p.error("unknown case ordinal")
    case = plan["workflows"][a.ordinal - 1]
    if sha(case["prompt"]) != case["prompt_sha256"]:
        p.error("fixed prompt changed")
    out = a.preflight.parent / "observations"
    out.mkdir(exist_ok=True, mode=0o700)
    ledger_file = out / "ledger.json"
    ledger = json.loads(ledger_file.read_text()) if ledger_file.exists() else {"starts": [], "finished_cases": []}
    if ledger["finished_cases"] != list(range(1, a.ordinal)):
        p.error("case order/repeat is fixed; interrupted cases cannot be silently retried")
    if any(r["ordinal"] == a.ordinal for r in ledger["starts"]):
        p.error("this case already started; no replacement attempt")
    home = out / "synthetic-home"
    home.mkdir(exist_ok=True, mode=0o700)
    env = {k: v for k, v in os.environ.items() if not k.startswith("BRN_")
           and not k.startswith("AGENT_CONTRACT_")}
    env.update(HOME=str(home), CODEX_HOME=str(home / ".codex"),
               BRN_NATIVE_MODEL_DIR="", BRN_EMBEDDING_MODEL_DIR="")
    def local(label, cmd, timeout=10):
        r = subprocess.run(cmd, env=env, capture_output=True, timeout=timeout)
        (out / f"{a.ordinal}-{label}.stdout").write_bytes(r.stdout)
        (out / f"{a.ordinal}-{label}.stderr").write_bytes(r.stderr)
        save(out / f"{a.ordinal}-{label}.json", {"command": cmd, "exit": r.returncode})
        return r
    config = local("configure", case["configure_command"])
    if config.returncode:
        raise SystemExit("configuration refused; no ask started, no automatic retry")
    binary = case["ask_command"][0]
    data = case["facts"]["data_dir"]
    common = [binary, "--json", "--data-dir", data, "--timeout", "2"]
    def workflow(label, cmd, observe):
        if len(ledger["starts"]) >= 12:
            raise SystemExit("12-start ceiling reached")
        row = {"ordinal": a.ordinal, "phase": label, "command": cmd,
               "reserved_before_spawn": True, "wall_started": time.time()}
        ledger["starts"].append(row)
        save(ledger_file, ledger)  # failures/interruptions consume a start too
        began = time.monotonic()
        child = None
        observed = None
        try:
            with (out / f"{a.ordinal}-{label}.stdout").open("xb") as stdout, \
                    (out / f"{a.ordinal}-{label}.stderr").open("xb") as stderr:
                child = subprocess.Popen(cmd, env=env, stdout=stdout, stderr=stderr)
                while child.poll() is None and time.monotonic() - began < 120:
                    if observe:
                        poll = subprocess.run(common + ["candidates", "--limit", "100"], env=env,
                                              capture_output=True, timeout=3)
                        if poll.returncode == 0:
                            body = json.loads(poll.stdout)
                            candidates = body.get("result", {}).get("candidates", [])
                            for candidate in candidates:
                                if any(w.get("id") == case["facts"]["note"] for w in candidate.get("writes", [])):
                                    observed = candidate
                                    break
                        if observed is not None:
                            if child.poll() is None:
                                child.send_signal(signal.SIGINT)
                                row["stop_signal_sent"] = True
                            break
                    time.sleep(min(1, max(0, 120 - (time.monotonic() - began))))
                row["exit"] = reap(child)
        finally:
            if child is not None and child.poll() is None:
                row["cleanup_exit"] = reap(child)
            row["elapsed_seconds"] = time.monotonic() - began
            row["observed_candidate"] = observed
            row["stop_stage"] = "actual candidate observed" if observed else "not observed; gap if Stop case"
            stdout_path = out / f"{a.ordinal}-{label}.stdout"
            row["exposed_metrics"] = exposed_metrics(stdout_path.read_bytes()) if stdout_path.exists() else {"trace_available": False}
            save(ledger_file, ledger)
        for name, cmd in [("runs", common + ["runs", "--thread", case["facts"]["chat"]]),
                          ("note", common + ["read", "--id", case["facts"]["note"]])]:
            local(label + "-" + name, cmd)
        if observed:
            local(label + "-operation", common + ["operation", "--id", observed["operation"]])
        # CLI operation summaries deliberately omit candidate bodies. Use ONLY
        # the approved test-only exporter on this explicit synthetic pair.
        candidates_run = local(label + "-candidates", common + ["candidates", "--limit", "100"])
        candidates = json.loads(candidates_run.stdout).get("result", {}).get("candidates", []) if candidates_run.returncode == 0 else []
        target_candidates = [c for c in candidates if any(w.get("id") == case["facts"]["note"] for w in c.get("writes", []))]
        if len(target_candidates) == 1:
            candidate = target_candidates[0]
            export_output = Path("/private/tmp") / f"brn-int-d-candidate-export-20261017-{a.preflight.parent.name}-{a.ordinal}-{label}"
            export_env = dict(env, BRN_INT_D_INSPECT_ROOT=str(Path(data).parent),
                              BRN_INT_D_INSPECT_ARM=case["arm"],
                              BRN_INT_D_INSPECT_OPERATION=candidate["operation"],
                              BRN_INT_D_INSPECT_OUTPUT=str(export_output),
                              BRN_INT_D_INSPECT_IDENTITY=json.dumps({"source_pin": plan["source_pin"],
                                  "preflight_sha256": sha(a.preflight), "corpus_sha256": plan["corpus_sha256"],
                                  "compiled_artifacts": plan["compiled_artifacts"]}))
            export_run = subprocess.run([plan["compiled_artifacts"]["generator"]["path"], "--exact",
                "candidate_export::exact_synthetic_candidate_export_refuses_wrong_fixture_operation_and_preserves_content",
                "--test-threads=1", "--nocapture"], env=export_env, capture_output=True, timeout=15)
            (out / f"{a.ordinal}-{label}-candidate-export.log").write_bytes(export_run.stdout + export_run.stderr)
            row["candidate_inspection"] = {"exit": export_run.returncode, "output": str(export_output),
                                           "operation": candidate["operation"], "request_hash": candidate["request_hash"],
                                           "automatic_apply": False}
            save(ledger_file, ledger)
        else:
            row["candidate_inspection_gap"] = "no unique exact target candidate; no body substituted or automatic Apply"
            save(ledger_file, ledger)
        if row.get("exit") not in (0, 130):
            raise SystemExit("actual unsuccessful workflow retained; stop window, no retry/fallback")
    workflow("ask", case["ask_command"], case["stop_observer"] is not None)
    if case["continue_command"] is not None:
        workflow("continue", case["continue_command"], False)
    ledger["finished_cases"].append(a.ordinal)
    save(ledger_file, ledger)
    print("Case observed; semantic assessment and exact owner review remain required.")


if __name__ == "__main__":
    main()
