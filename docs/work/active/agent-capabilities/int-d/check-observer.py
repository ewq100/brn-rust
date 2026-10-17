#!/usr/bin/env python3
"""Bounded local PROCESS harness qualification. No BRN/provider/auth process.

The fake CLI emits the documented envelope shape and blocks on a real signal.
These results qualify observation/counting only, never BRN runtime semantics.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


FAKE = r'''#!/usr/bin/env python3
import json,os,pathlib,signal,sys
root=pathlib.Path(os.environ['INT_D_HARNESS_ROOT']);args=sys.argv[1:]
def emit(result): print(json.dumps({'status':'ok','result':result}),flush=True)
if '--exact' in args:
    out=pathlib.Path(os.environ['BRN_INT_D_INSPECT_OUTPUT']);out.mkdir()
    (out/'candidate.json').write_text(json.dumps({'fixture_only':True,'operation':'synthetic-op','request_hash':'synthetic-hash','automatic_apply':False}))
    (root/'exported').write_text('test-only fake export called')
    raise SystemExit(0)
if 'configure' in args: emit({'configured':'synthetic; auth not opened'})
elif 'candidates' in args:
    emit({'candidates':[{'operation':'synthetic-op','request_hash':'synthetic-hash','writes':[{'id':'synthetic-note','version':1}]}] if (root/'ready').exists() else []})
elif 'operation' in args: emit({'operation':'synthetic-op','state':'Prepared','request_hash':'synthetic-hash'})
elif 'runs' in args: emit({'runs':[]})
elif 'read' in args: emit({'id':'synthetic-note','version':1})
elif 'ask' in args:
    with (root/'actual-starts').open('a') as f:f.write('start\n')
    if os.environ['INT_D_HARNESS_MODE']=='failure':
        emit({'terminal':'Failed:fixture','failure':'synthetic command failure; no auth/provider'})
        raise SystemExit(7)
    prompt=args[args.index('--prompt-file')+1]
    if 'continue' in prompt: emit({'terminal':'Completed','trace':{'model_turns':1,'rounds':0,'usage':{'input':100,'output':10,'cached_input':25},'turn_usage':[],'helpers':[]}})
    else:
        def stopped(*_):
            (root/'signalled').write_text('actual SIGINT received by owned fake ask')
            emit({'terminal':'Interrupted','trace':{'model_turns':2,'rounds':1,'usage':{'input':100,'output':10,'cached_input':50},'turn_usage':[],'helpers':[]}})
            raise SystemExit(130)
        signal.signal(signal.SIGINT,stopped)
        (root/'ready').write_text('candidate exists only after handler is installed')
        signal.pause()
else: raise SystemExit('unexpected synthetic command')
'''


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    observer = Path(__file__).with_name("observe-workflow.py").resolve()
    pin = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    rows = []
    with tempfile.TemporaryDirectory(prefix="brn-int-d-observer-process-", dir="/private/tmp") as tmp:
        root = Path(tmp)
        for mode in ("stop", "failure", "spawn-failure", "cap"):
            case_root = root / (mode + "-" + root.name)
            case_root.mkdir()
            fake = case_root / "fake-cli"
            fake.write_text(FAKE)
            fake.chmod(0o700)
            dep = case_root / "fake.d"
            dep.write_text("Synthetic Python fake CLI fixture; NOT a Cargo/BRN artifact.\n")
            prompt = case_root / "ask.txt"
            prompt.write_text("Public synthetic harness input")
            cont = case_root / "continue.txt"
            cont.write_text("Public synthetic fresh Continue")
            row = {"path": str(fake), "sha256": sha(fake), "dep_info": str(dep), "dep_info_sha256": sha(dep)}
            common = [str(fake), "--json", "--data-dir", str(case_root / "candidate"), "--timeout", "120"]
            ask = common + ["ask", "--thread", "synthetic-chat", "--prompt-file", str(prompt), "--budget", "8", "--trace", "summary"]
            if mode == "spawn-failure":
                ask[0] = str(case_root / "absent-executable")
            plan = {"source_pin": pin, "harness_only": True,
                    "mcp": "off in every workflow; deterministic real SDK on-path is separate evidence",
                    "observer_sha256": sha(observer), "compiled_artifacts": {"binary": row, "generator": row},
                    "corpus_sha256": "synthetic-process-fixture-not-real-corpus",
                    "workflows": [{"arm": "candidate", "facts": {"data_dir": str(case_root / "candidate"), "note": "synthetic-note", "chat": "synthetic-chat"},
                        "prompt": str(prompt), "prompt_sha256": sha(prompt), "configure_command": common + ["configure"],
                        "ask_command": ask, "stop_observer": {}, "continue_command": common + ["ask", "--thread", "synthetic-chat", "--prompt-file", str(cont), "--budget", "8", "--trace", "summary"]}]}
            preflight = case_root / "preflight.json"
            preflight.write_text(json.dumps(plan))
            activation = case_root / "fake-activation.json"
            activation.write_text(json.dumps({"state": "ACTIVATED", "source_pin": pin, "preflight_sha256": sha(preflight),
                                               "qualification": "LOCAL FAKE CLI ONLY; no principal live activation"}))
            if mode == "cap":
                (case_root / "observations").mkdir()
                (case_root / "observations/ledger.json").write_text(json.dumps({"starts": [{"ordinal": 0} for _ in range(12)], "finished_cases": []}))
            env = dict(os.environ, INT_D_HARNESS_ROOT=str(case_root), INT_D_HARNESS_MODE=mode)
            cmd = [sys.executable, str(observer), "--preflight", str(preflight), "--activation", str(activation), "--ordinal", "1"]
            run = subprocess.run(cmd, env=env, capture_output=True, timeout=25)
            ledger = json.loads((case_root / "observations/ledger.json").read_text())
            actual = (case_root / "actual-starts").read_text().splitlines() if (case_root / "actual-starts").exists() else []
            if mode == "stop":
                assert run.returncode == 0, run.stderr.decode()
                assert len(actual) == 2 and len(ledger["starts"]) == 2
                assert (case_root / "signalled").exists() and (case_root / "exported").exists()
                assert ledger["starts"][0]["stop_signal_sent"] and ledger["starts"][0]["exit"] == 130
                assert ledger["starts"][1]["exit"] == 0 and ledger["finished_cases"] == [1]
                assert ledger["starts"][0]["exposed_metrics"]["parent_usage"]["cached_share"] == .5
            elif mode == "failure":
                assert run.returncode != 0 and len(actual) == 1 and len(ledger["starts"]) == 1
                assert ledger["starts"][0]["exit"] == 7 and not ledger["finished_cases"]
            elif mode == "spawn-failure":
                assert run.returncode != 0 and not actual and len(ledger["starts"]) == 1
                assert ledger["starts"][0]["reserved_before_spawn"] and not ledger["finished_cases"]
            else:
                assert run.returncode != 0 and not actual and len(ledger["starts"]) == 12
                assert b"ceiling reached" in run.stderr
            repeat = subprocess.run(cmd, env=env, capture_output=True, timeout=5)
            assert repeat.returncode != 0
            assert json.loads((case_root / "observations/ledger.json").read_text()) == ledger
            rows.append({"case": mode, "exit": run.returncode, "repeat_exit": repeat.returncode,
                         "reserved_starts": len(ledger["starts"]), "fake_ask_processes": len(actual),
                         "ledger": ledger, "stderr": run.stderr.decode(), "stdout": run.stdout.decode()})
            for start in ledger["starts"]:
                inspection = start.get("candidate_inspection")
                if inspection and inspection["exit"] == 0:
                    path = Path(inspection["output"])
                    assert json.loads((path / "candidate.json").read_text())["fixture_only"]
                    (path / "candidate.json").unlink()
                    path.rmdir()
    output = {"qualification": "local fake CLI process harness only; no BRN/provider/auth calls; not runtime semantic evidence",
              "head_at_check": pin, "observer_sha256": sha(observer), "test_sha256": sha(__file__), "cases": rows,
              "live_workflow_starts": 0, "auth_starts": 0}
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
