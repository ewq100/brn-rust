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
    if sha(plan["corpus"]) != plan["corpus_sha256"]:
        p.error("fixed corpus changed")
    if sha(case["manifest"]) != case["manifest_sha256"]:
        p.error("fixture manifest changed")
    manifest = json.loads(Path(case["manifest"]).read_text())
    if case["facts"] not in manifest["cases"]:
        p.error("fixture identity is absent from the bound manifest")
    if sha(Path(case["facts"]["data_dir"]) / "threads.sqlite3") != case["initial_database_sha256"]:
        p.error("initial case database changed; no replacement/reuse")
    if sha(case["prompt"]) != case["prompt_sha256"]:
        p.error("fixed prompt changed")
    if sha(case["continue_prompt"]) != case["continue_prompt_sha256"]:
        p.error("fixed Continue prompt changed")
    if case["stop_observer"] is not None:
        expected_poll = [plan["compiled_artifacts"]["binary"]["path"], "--json", "--data-dir",
                         case["facts"]["data_dir"], "--timeout", "2", "candidates", "--limit", "100"]
        if case["stop_observer"]["poll_command"] != expected_poll or not 0 < case["stop_observer"]["python_timeout_seconds"] <= 3:
            p.error("poll must be the bound local inspection with CLI timeout2 and Python timeout<=3")
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
    def inspection_timeout(stage, cmd, timeout):
        event = {"stage": stage, "kind": "timeout", "command": cmd, "timeout_seconds": timeout,
                 "outcome": "harness_failure; stop window, no retry or Continue"}
        save(out / f"{a.ordinal}-harness-failure.json", event)
        return event
    def local(label, cmd, timeout=10):
        try:
            r = subprocess.run(cmd, env=env, capture_output=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            inspection_timeout(label, cmd, timeout)
            raise SystemExit("local inspection timed out; window stopped") from None
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
    def workflow(label, cmd, observe, follows_stop=None):
        if len(ledger["starts"]) >= 12:
            raise SystemExit("12-start ceiling reached")
        row = {"ordinal": a.ordinal, "phase": label, "command": cmd,
               "reserved_before_spawn": True, "wall_started": time.time()}
        if follows_stop is not None:
            row["continuation_measurement"] = "after observed Stop" if follows_stop else "non-Stop Continue; Stop stage missed"
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
                        poll_cmd = case["stop_observer"]["poll_command"]
                        poll_timeout = case["stop_observer"]["python_timeout_seconds"]
                        row["actual_poll_command"] = poll_cmd
                        row["poll_python_timeout_seconds"] = poll_timeout
                        try:
                            poll = subprocess.run(poll_cmd, env=env, capture_output=True, timeout=poll_timeout)
                        except subprocess.TimeoutExpired:
                            row["harness_failure"] = inspection_timeout("candidate_poll", poll_cmd, poll_timeout)
                            raise SystemExit("candidate inspection timed out; window stopped") from None
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
            export_cmd = [plan["compiled_artifacts"]["generator"]["path"], "--exact",
                "candidate_export::exact_synthetic_candidate_export_refuses_wrong_fixture_operation_and_preserves_content",
                "--test-threads=1", "--nocapture"]
            try:
                export_run = subprocess.run(export_cmd, env=export_env, capture_output=True, timeout=15)
            except subprocess.TimeoutExpired:
                inspection_timeout("candidate_export", export_cmd, 15)
                raise SystemExit("candidate export timed out; window stopped") from None
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
        workflow("continue", case["continue_command"], False, follows_stop=bool(ledger["starts"][-1].get("stop_signal_sent")))
    ledger["finished_cases"].append(a.ordinal)
    save(ledger_file, ledger)
    print("Case observed; semantic assessment and exact owner review remain required.")



# History-only has a distinct activation and ledger; no legacy configuration,
# polling Stop, Continue or automatic owner Apply can enter this path.
def history_main():
    import importlib.util
    spec = importlib.util.spec_from_file_location("int_d_preparation", Path(__file__).with_name("prepare-qualification.py"))
    preparation = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(preparation)
    p = argparse.ArgumentParser(description="One activated, fixed history-only case")
    p.add_argument("--preflight", type=Path, required=True)
    p.add_argument("--activation", type=Path, required=True)
    p.add_argument("--ordinal", type=int, required=True)
    a = p.parse_args()
    def refuse(message):
        p.error(message)
    try:
        plan = preparation.bounded_json(a.preflight)
        approval = preparation.bounded_json(a.activation, 8192)
        now = time.time()
        keys = {"format", "task", "state", "source_pin", "preflight_sha256", "max_starts", "issued_at", "expires_at"}
        if set(approval) != keys or approval["format"] != preparation.HISTORY_FORMAT or approval["task"] != preparation.HISTORY_TASK \
                or approval["state"] != "ACTIVATED" or approval["source_pin"] != plan["source_pin"] \
                or approval["preflight_sha256"] != sha(a.preflight) or type(approval["max_starts"]) is not int or approval["max_starts"] != 4 \
                or type(approval["issued_at"]) is not int or type(approval["expires_at"]) is not int \
                or not approval["issued_at"] <= now < approval["expires_at"] <= approval["issued_at"] + 900:
            refuse("missing/stale/old or inexact history activation")
        if plan["format"] != preparation.HISTORY_FORMAT or plan["phase"] != "history-only" or plan["task"] != preparation.HISTORY_TASK \
                or plan["limits"] != {"total_workflow_starts": 4, "max_tool_rounds_each": 8, "host_timeout_seconds_each": 120} \
                or plan["route"] != {"provider": "Codex", "model": "gpt-6.1-sol", "effort": "medium", "auth_reference": preparation.AUTH_REFERENCE}:
            refuse("inexact phase/route/limits")
        if a.preflight.parent.is_symlink() or a.preflight.parent.resolve() != a.preflight.parent \
                or a.preflight.parent.stat().st_mode & 0o077 or a.activation.parent != a.preflight.parent:
            refuse("real private preflight directory required")
        preparation.verify_source(plan["source_root"], plan["source_pin"], plan["source_inputs"])
        scripts = {str(Path(__file__).with_name(n).resolve()) for n in ("prepare-qualification.py", "observe-workflow.py", "check-observer.py")}
        if set(plan["scripts"]) != scripts:
            refuse("exact three scripts required")
        for path, digest in plan["scripts"].items():
            if sha(path) != digest:
                refuse("script changed")
            relative = str(Path(path).relative_to(plan["source_root"]))
            if hashlib.sha256(subprocess.check_output(["git", "show", plan["source_pin"] + ":" + relative], cwd=plan["source_root"])).hexdigest() != digest:
                refuse("script not pinned")
        if set(plan["compiled_artifacts"]) != {"binary", "generator"}:
            refuse("exact binary/exporter required")
        for row in plan["compiled_artifacts"].values():
            if sha(row["path"]) != row["sha256"] or sha(row["dep_info"]) != row["dep_info_sha256"]:
                refuse("artifact/dep-info changed")
        if set(plan["bound_method"]) != {str(a.preflight.parent / n) for n in ("method.md", "rubric.json")}:
            refuse("exact method/rubric required")
        for path, digest in plan["bound_method"].items():
            if sha(path) != digest:
                refuse("method/rubric changed")
        if sha(plan["corpus"]) != plan["corpus_sha256"]:
            refuse("corpus changed")
        corpus = preparation.bounded_json(plan["corpus"])
        if [(r["repeat"], r["arm"]) for r in plan["workflows"]] != preparation.HISTORY_ORDER or not 1 <= a.ordinal <= 4:
            refuse("fixed four-case order required")
        binary = plan["compiled_artifacts"]["binary"]["path"]
        for ordinal, r in enumerate(plan["workflows"], 1):
            expected = [binary, "--json", "--data-dir", r["facts"]["data_dir"], "--timeout", "120", "ask", "--thread", r["facts"]["chat"], "--prompt-file", r["prompt"], "--budget", "8", "--trace", "summary"]
            if r["ordinal"] != ordinal or any(r[k] is not None for k in ("stop_observer", "continue_command", "configure_command")) or r["ask_command"] != expected:
                refuse("no alternate commands, configuration, Stop or Continue")
        case = plan["workflows"][a.ordinal - 1]
        # Only a not-yet-mutated case is stamped: completed previous cases may
        # legitimately differ. Bound raw context/manifest files remain immutable.
        for r in plan["workflows"]:
            for key in ("prompt", "manifest"):
                if sha(r[key]) != r[key + "_sha256"]:
                    refuse("prompt/manifest changed")
            if Path(r["prompt"]).read_text() != preparation.history_prompt(r["facts"]["note"]):
                refuse("prompt semantics changed")
            for key in ("context", "records"):
                if sha(r[key + "_path"]) != r[key + "_sha256"]:
                    refuse("canonical metadata changed")
            manifest = preparation.bounded_json(r["manifest"])
            if r["facts"] not in manifest["cases"]:
                refuse("wrong manifest arm")
            context = preparation.history_context(preparation.bounded_json(r["records_path"]), r["facts"], corpus)
            if context != r["context"] or preparation.bounded_json(r["context_path"]) != context:
                refuse("canonical context/auth reference changed")
        if preparation.stamp_database(case["facts"]["data_dir"]) != case["database"]:
            refuse("main/WAL/SHM changed before first checked live open")
        out = Path(plan["output_namespace"])
        if out != a.preflight.parent / "observations" or out.is_symlink():
            refuse("inexact output namespace")
        if out.exists():
            if out.resolve() != out or not (out / "ledger.json").is_file():
                refuse("occupied output without a ledger")
            ledger = preparation.bounded_json(out / "ledger.json", 256 * 1024)
        else:
            ledger = {"starts": [], "finished_cases": [], "window_failed": False}
        if ledger.get("window_failed") or ledger["finished_cases"] != list(range(1, a.ordinal)) \
                or len(ledger["starts"]) != a.ordinal - 1 or len(ledger["starts"]) >= 4:
            refuse("failed window, duplicate, order or cap refusal")
        if out.exists() and any((out / f"{a.ordinal}-reservation.json").exists() for _ in [0]):
            refuse("start already consumed")
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        refuse("invalid bounded preflight or activation binding")
    out.mkdir(mode=0o700, exist_ok=True)
    home = out / "synthetic-home"
    home.mkdir(mode=0o700, exist_ok=True)
    env = {k: v for k, v in os.environ.items() if not k.startswith(("BRN_", "AGENT_CONTRACT_"))}
    env.update(HOME=str(home), CODEX_HOME=str(home / ".codex"), BRN_NATIVE_MODEL_DIR="", BRN_EMBEDDING_MODEL_DIR="")
    row = {"ordinal": a.ordinal, "phase": "history", "command": case["ask_command"], "reserved_before_spawn": True, "wall_started": time.time()}
    # create_new plus file+directory fsync precedes even process spawn failure.
    with (out / f"{a.ordinal}-reservation.json").open("x") as f:
        json.dump(row, f); f.flush(); os.fsync(f.fileno())
    began = time.monotonic()
    child = None
    local_children = []
    interrupted = False
    completed_case = False
    stage = "reservation_sync"
    ledger["starts"].append(row)
    def failure(error, at):
        nonlocal interrupted
        interrupted = interrupted or isinstance(error, KeyboardInterrupt)
        # Fixed class vocabulary, never exception Display/body/URL or a custom
        # exception's arbitrary class name. The first cause is never replaced.
        classes = {"AttributeError", "RuntimeError", "KeyboardInterrupt", "OSError",
                   "ValueError", "KeyError", "TypeError", "TimeoutError",
                   "FileNotFoundError", "FileExistsError", "PermissionError",
                   "InterruptedError", "BrokenPipeError", "SystemExit"}
        name = type(error).__name__
        event = {"stage": at, "class": name if name in classes else "Exception",
                 "outcome": "STOP ENTIRE WINDOW; no retry/Continue/replacement"}
        if "harness_failure" not in row:
            row["harness_failure"] = event
        else:
            row.setdefault("secondary_failures", []).append(event)
        ledger["window_failed"] = True
        ledger["finished_cases"] = [n for n in ledger["finished_cases"] if n != a.ordinal]
    def notice():
        try:
            event = row.get("secondary_failures", [row["harness_failure"]])[-1]
            print("History observer evidence persistence failed: " + event["class"] + "; window stopped.", file=__import__("sys").stderr)
        except BaseException:
            pass  # unavailable stderr is not a successful evidence write
    def group_reap(process):
        for sig, wait in ((signal.SIGINT, 3), (signal.SIGTERM, 2), (signal.SIGKILL, 2)):
            try:
                os.killpg(process.pid, sig)
            except ProcessLookupError:
                pass
            try:
                code = process.wait(timeout=wait)
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                return code
            except subprocess.TimeoutExpired:
                continue
        raise RuntimeError("owned process did not reap")
    def local(label, cmd, timeout=3, local_env=None):
        nonlocal stage
        stage = label
        process = subprocess.Popen(cmd, env=local_env or env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        local_children.append((label, process))
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            raise TimeoutError(label) from None  # finalization still owns child
        (out / f"{a.ordinal}-{label}.stdout").write_bytes(stdout)
        (out / f"{a.ordinal}-{label}.stderr").write_bytes(stderr)
        save(out / f"{a.ordinal}-{label}.json", {"command": cmd, "exit": process.returncode, "python_timeout_seconds": timeout})
        if process.returncode != 0:
            raise ValueError(label + " failed")
        return stdout
    def persist_failed():
        # Independent attempts: failure evidence must not suppress ledger write,
        # nor may cleanup/persistence failure leave a successful finished row.
        for label, path, value in (
                ("failure_persistence", out / f"{a.ordinal}-harness-failure.json", row),
                ("ledger_persistence", out / "ledger.json", ledger)):
            try:
                save(path, value)
            except BaseException as error:
                failure(error, label)
                notice()
    try:
        directory = os.open(out, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
        save(out / "ledger.json", ledger)
        with (out / f"{a.ordinal}-ask.stdout").open("xb") as stdout, (out / f"{a.ordinal}-ask.stderr").open("xb") as stderr:
            child = subprocess.Popen(case["ask_command"], env=env, stdout=stdout, stderr=stderr, start_new_session=True)
            row["pid"] = child.pid; row["pgid"] = child.pid
            try:
                stage = "workflow_wait"
                row["exit"] = child.wait(timeout=120)
            except subprocess.TimeoutExpired:
                raise TimeoutError("workflow deadline; cleanup is not checked Stop") from None
        row["elapsed_seconds"] = time.monotonic() - began
        stage = "workflow_metrics"
        row["exposed_metrics"] = exposed_metrics((out / f"{a.ordinal}-ask.stdout").read_bytes())
        if row["exit"] != 0:
            raise ValueError("unsuccessful workflow")
        stage = "workflow_envelope"
        envelope = preparation.bounded_json(out / f"{a.ordinal}-ask.stdout")
        if envelope.get("status") != "ok" or envelope.get("result", {}).get("terminal") != "Completed":
            raise ValueError("malformed or non-Completed workflow")
        common = [binary, "--json", "--data-dir", case["facts"]["data_dir"], "--timeout", "2"]
        snapshots = {}
        for name, argv in (("runs", ["runs", "--thread", case["facts"]["chat"]]), ("note", ["read", "--id", case["facts"]["note"]]), ("candidates", ["candidates", "--limit", "100"])):
            snapshots[name] = json.loads(local(name, common + argv))
            if snapshots[name].get("status") != "ok" or not isinstance(snapshots[name].get("result"), dict):
                raise ValueError("malformed inspection envelope")
        note = snapshots["note"]["result"]
        if note["id"] != case["facts"]["note"] or note["version"] != 2:
            raise ValueError("target identity/version changed; authority invariant")
        candidates = snapshots["candidates"]["result"]["candidates"]
        if not isinstance(candidates, list):
            raise ValueError("malformed candidates")
        target = [c for c in candidates if any(w.get("id") == case["facts"]["note"] for w in c["writes"])]
        if len(target) > 1:
            raise ValueError("ambiguous target candidates")
        if target:
            c = target[0]
            operation = json.loads(local("operation", common + ["operation", "--id", c["operation"]]))
            if operation.get("status") != "ok" or operation.get("result", {}).get("state") != "prepared" or operation["result"].get("receipt") is not None:
                raise ValueError("operation is not unapplied Prepared")
            export_output = Path("/private/tmp") / f"brn-int-d-candidate-export-20261017-history-{plan['source_pin'][:8]}-{a.ordinal}"
            export_env = dict(env, BRN_INT_D_INSPECT_ROOT=str(Path(case["facts"]["data_dir"]).parent), BRN_INT_D_INSPECT_ARM=case["arm"],
                              BRN_INT_D_INSPECT_OPERATION=c["operation"], BRN_INT_D_INSPECT_OUTPUT=str(export_output),
                              BRN_INT_D_INSPECT_IDENTITY=json.dumps({"source_pin": plan["source_pin"], "preflight_sha256": sha(a.preflight), "compiled_artifacts": plan["compiled_artifacts"], "corpus_sha256": plan["corpus_sha256"]}))
            cmd = [plan["compiled_artifacts"]["generator"]["path"], "--exact", "candidate_export::exact_synthetic_candidate_export_refuses_wrong_fixture_operation_and_preserves_content", "--test-threads=1", "--nocapture"]
            local("candidate-export", cmd, timeout=15, local_env=export_env)
            exported = preparation.bounded_json(export_output / "candidate.json", 1024 * 1024)
            if exported["operation"] != c["operation"] or exported["request_hash"] != c["request_hash"] or exported["target_current"]["version"] != 2 or exported["automatic_apply"] is not False:
                raise ValueError("exported candidate identity mismatch")
            row["candidate_inspection"] = {"output": str(export_output), "sha256": sha(export_output / "candidate.json"), "operation": c["operation"], "request_hash": c["request_hash"], "automatic_apply": False}
        else:
            row["objective_gap"] = "Completed without exact candidate; not objective success or retry authority"
        completed_case = True
    except BaseException as error:
        failure(error, stage)
    finally:
        owned = ([ ("workflow", child) ] if child is not None else []) + local_children
        cleanup = []
        for label, process in owned:
            status = {"stage": label, "pid": process.pid, "pgid": process.pid,
                      "status": "unresolved"}
            try:
                code = group_reap(process)
                status.update(status="reaped", exit=code)
                if label == "workflow":
                    row["cleanup_exit"] = code
                    row["owned_group_reaped"] = True
            except BaseException as error:
                if label == "workflow":
                    row["owned_group_reaped"] = False
                failure(error, "cleanup_" + label)
            finally:
                cleanup.append(status)
        row["cleanup"] = cleanup
        row["elapsed_seconds_including_cleanup"] = time.monotonic() - began
        if completed_case and not ledger["window_failed"]:
            ledger["finished_cases"].append(a.ordinal)
            try:
                save(out / "ledger.json", ledger)
            except BaseException as error:
                failure(error, "ledger_persistence")
        if ledger["window_failed"]:
            persist_failed()
    if interrupted:
        raise SystemExit(130)  # consumed start, saved STOP, never continue
    if ledger["window_failed"]:
        raise SystemExit("History window stopped; bounded failure preserved.")
    print("History case reaped; exact semantic/owner review remains separate.")


if __name__ == "__main__":
    import sys
    if "--preflight" in sys.argv and json.loads(Path(sys.argv[sys.argv.index("--preflight") + 1]).read_text()).get("phase") == "history-only":
        history_main()
    else:
        main()
