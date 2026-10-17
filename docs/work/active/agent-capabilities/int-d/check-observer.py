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
if 'configure' in args:
    (root/'configured').write_text('fake configuration only')
    emit({'configured':'synthetic; auth not opened'})
elif 'candidates' in args:
    if os.environ['INT_D_HARNESS_MODE']=='poll-timeout':
        (root/'poll-entered').write_text('actual local inspection blocks')
        signal.pause()
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
    if 'continue' in prompt or os.environ['INT_D_HARNESS_MODE']=='non-stop': emit({'terminal':'Completed','trace':{'model_turns':1,'rounds':0,'usage':{'input':100,'output':10,'cached_input':25},'turn_usage':[],'helpers':[]}})
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
        for mode in ("stop", "non-stop", "failure", "spawn-failure", "cap", "poll-timeout", "changed-corpus", "changed-manifest", "changed-database", "changed-continue"):
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
            corpus = case_root / "corpus.json"
            corpus.write_text(json.dumps({"continue_prompt": "Public synthetic fresh Continue"}))
            data = case_root / "candidate"
            data.mkdir()
            database = data / "threads.sqlite3"
            database.write_bytes(b"fake harness database identity; no SQLite/BRN execution")
            facts = {"data_dir": str(data), "note": "synthetic-note", "chat": "synthetic-chat"}
            manifest = case_root / "manifest.json"
            manifest.write_text(json.dumps({"cases": [facts]}))
            row = {"path": str(fake), "sha256": sha(fake), "dep_info": str(dep), "dep_info_sha256": sha(dep)}
            common = [str(fake), "--json", "--data-dir", str(case_root / "candidate"), "--timeout", "120"]
            ask = common + ["ask", "--thread", "synthetic-chat", "--prompt-file", str(prompt), "--budget", "8", "--trace", "summary"]
            if mode == "spawn-failure":
                ask[0] = str(case_root / "absent-executable")
            plan = {"source_pin": pin, "harness_only": True,
                    "mcp": "off in every workflow; deterministic real SDK on-path is separate evidence",
                    "observer_sha256": sha(observer), "compiled_artifacts": {"binary": row, "generator": row},
                    "corpus": str(corpus), "corpus_sha256": sha(corpus),
                    "workflows": [{"arm": "candidate", "facts": facts,
                        "prompt": str(prompt), "prompt_sha256": sha(prompt), "configure_command": common + ["configure"],
                        "continue_prompt": str(cont), "continue_prompt_sha256": sha(cont),
                        "manifest": str(manifest), "manifest_sha256": sha(manifest), "initial_database_sha256": sha(database),
                        "ask_command": ask, "stop_observer": {"poll_command": [str(fake), "--json", "--data-dir", str(data), "--timeout", "2", "candidates", "--limit", "100"], "python_timeout_seconds": .25 if mode == "poll-timeout" else 3},
                        "continue_command": common + ["ask", "--thread", "synthetic-chat", "--prompt-file", str(cont), "--budget", "8", "--trace", "summary"]}]}
            preflight = case_root / "preflight.json"
            preflight.write_text(json.dumps(plan))
            activation = case_root / "fake-activation.json"
            activation.write_text(json.dumps({"state": "ACTIVATED", "source_pin": pin, "preflight_sha256": sha(preflight),
                                               "qualification": "LOCAL FAKE CLI ONLY; no principal live activation"}))
            changed = {"changed-corpus": corpus, "changed-manifest": manifest, "changed-database": database, "changed-continue": cont}
            if mode in changed:
                changed[mode].write_bytes(changed[mode].read_bytes() + b"changed")
            if mode == "cap":
                (case_root / "observations").mkdir()
                (case_root / "observations/ledger.json").write_text(json.dumps({"starts": [{"ordinal": 0} for _ in range(12)], "finished_cases": []}))
            env = dict(os.environ, INT_D_HARNESS_ROOT=str(case_root), INT_D_HARNESS_MODE=mode)
            cmd = [sys.executable, str(observer), "--preflight", str(preflight), "--activation", str(activation), "--ordinal", "1"]
            run = subprocess.run(cmd, env=env, capture_output=True, timeout=25)
            if mode in changed:
                assert run.returncode == 2 and not (case_root / "configured").exists()
                assert not (case_root / "actual-starts").exists() and not (case_root / "observations").exists()
                rows.append({"case": mode, "exit": run.returncode, "refused_before_configure": True,
                             "fake_ask_processes": 0, "stderr": run.stderr.decode()})
                continue
            ledger = json.loads((case_root / "observations/ledger.json").read_text())
            actual = (case_root / "actual-starts").read_text().splitlines() if (case_root / "actual-starts").exists() else []
            if mode == "stop":
                assert run.returncode == 0, run.stderr.decode()
                assert len(actual) == 2 and len(ledger["starts"]) == 2
                assert (case_root / "signalled").exists() and (case_root / "exported").exists()
                assert ledger["starts"][0]["stop_signal_sent"] and ledger["starts"][0]["exit"] == 130
                assert ledger["starts"][1]["exit"] == 0 and ledger["finished_cases"] == [1]
                assert ledger["starts"][0]["exposed_metrics"]["parent_usage"]["cached_share"] == .5
                assert ledger["starts"][1]["continuation_measurement"] == "after observed Stop"
            elif mode == "non-stop":
                assert run.returncode == 0 and len(actual) == 2 and len(ledger["starts"]) == 2
                assert not (case_root / "signalled").exists() and not ledger["starts"][0].get("stop_signal_sent")
                assert ledger["starts"][1]["continuation_measurement"] == "non-Stop Continue; Stop stage missed"
            elif mode == "failure":
                assert run.returncode != 0 and len(actual) == 1 and len(ledger["starts"]) == 1
                assert ledger["starts"][0]["exit"] == 7 and not ledger["finished_cases"]
            elif mode == "spawn-failure":
                assert run.returncode != 0 and not actual and len(ledger["starts"]) == 1
                assert ledger["starts"][0]["reserved_before_spawn"] and not ledger["finished_cases"]
            elif mode == "poll-timeout":
                assert run.returncode != 0 and len(actual) == 1 and len(ledger["starts"]) == 1
                assert (case_root / "poll-entered").exists() and (case_root / "signalled").exists()
                start = ledger["starts"][0]
                assert start["cleanup_exit"] == 130 and start["harness_failure"]["kind"] == "timeout"
                assert start["actual_poll_command"] == plan["workflows"][0]["stop_observer"]["poll_command"]
                assert start["poll_python_timeout_seconds"] == .25 and not ledger["finished_cases"]
                assert not (case_root / "exported").exists()
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



HISTORY_FAKE = r'''#!/usr/bin/env python3
import json,os,pathlib,signal,sys
root=pathlib.Path(os.environ['INT_D_HISTORY_FAKE_ROOT']);mode=os.environ['INT_D_HISTORY_FAKE_MODE'];args=sys.argv[1:]
def emit(result):print(json.dumps({'status':'ok','result':result}),flush=True)
if '--exact' in args:
    if mode=='export-timeout':signal.pause()
    if mode=='export-failure':raise SystemExit(9)
    out=pathlib.Path(os.environ['BRN_INT_D_INSPECT_OUTPUT']);out.mkdir()
    (out/'candidate.json').write_text(json.dumps({'operation':'synthetic-op','request_hash':'synthetic-hash','target_current':{'version':2},'automatic_apply':False}))
    raise SystemExit(0)
data=pathlib.Path(args[args.index('--data-dir')+1])
facts=json.loads((data/'facts.json').read_text())
if 'ask' in args:
    preflight=json.loads((root/'preflight.json').read_text());ordinal=next(r['ordinal'] for r in preflight['workflows'] if r['facts']['data_dir']==str(data))
    assert (root/'observations'/f'{ordinal}-reservation.json').exists()
    with (root/'actual-starts').open('a') as f:f.write(str(ordinal)+'\n')
    (root/'ask-pid').write_text(str(os.getpid()))
    if mode in ('failure','timeout-exit'):emit({'terminal':'ToolLimitReached'});raise SystemExit(7)
    if mode=='malformed-ask':print('not-json');raise SystemExit(0)
    emit({'terminal':'Completed','trace':{'model_turns':1,'rounds':0,'usage':{'input':100,'output':10,'cached_input':25},'turn_usage':[],'helpers':[],'summary':{'outcome':'cached'}}})
elif 'configure' in args or 'continue' in args:raise SystemExit('forbidden fake configure/Continue')
elif 'runs' in args:
    (root/'inspection-entered').write_text('local inspection')
    if mode=='inspection-timeout':signal.pause()
    if mode=='inspection-failure':raise SystemExit(9)
    if mode=='malformed-inspection':print('not-json')
    else:emit({'runs':[]})
elif 'read' in args:emit({'id':facts['note'],'version':3 if mode=='authority-failure' else 2})
elif 'candidates' in args:
    emit({'candidates':[{'operation':'synthetic-op','request_hash':'synthetic-hash','writes':[{'id':facts['note']}]}] if mode.startswith('export-') else []})
elif 'operation' in args:emit({'state':'prepared','receipt':None})
else:raise SystemExit('unrecognized fake command')
'''


def check_history():
    import importlib.util
    import time
    import signal
    import shutil
    source = Path(__file__).resolve().parent
    spec = importlib.util.spec_from_file_location("preparation", source / "prepare-qualification.py")
    prep = importlib.util.module_from_spec(spec); spec.loader.exec_module(prep)
    rows = []
    modes = ["four-order", "failure", "timeout-exit", "spawn-failure", "malformed-ask", "inspection-failure", "inspection-timeout", "malformed-inspection", "export-success", "export-failure", "export-timeout", "authority-failure", "occupied-output", "duplicate", "cap", "wrong-order", "missing-activation", "old-activation", "expired-activation", "malformed-activation", "wrong-type-activation"]
    modes += ["changed-" + n for n in ("source", "script", "artifact", "depinfo", "corpus", "prompt", "method", "rubric", "manifest", "main", "wal", "shm", "context", "authref")]
    with tempfile.TemporaryDirectory(prefix="brn-int-d-history-fakes-", dir="/private/tmp") as temp:
        base = Path(temp)
        repo = base / "git-source";repo.mkdir()
        scripts = repo / "docs/work/active/agent-capabilities/int-d";scripts.mkdir(parents=True)
        for name in ("prepare-qualification.py", "observe-workflow.py", "check-observer.py"):
            shutil.copyfile(source / name, scripts / name)
        inputs = {}
        for i in range(334):
            path = repo / f"input-{i}.txt";path.write_text(f"synthetic Git input {i}\n");inputs[path.name] = sha(path)
        for cmd in (["git", "init", "-q", str(repo)], ["git", "-C", str(repo), "add", "."], ["git", "-C", str(repo), "-c", "user.name=Synthetic", "-c", "user.email=synthetic@invalid", "commit", "-qm", "synthetic history observer sources"]):
            subprocess.run(cmd, check=True, capture_output=True)
        pin = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
        for mode in modes:
            root = base / mode;root.mkdir(mode=0o700)
            fake = root / "fake-cli";fake.write_text(HISTORY_FAKE);fake.chmod(0o700)
            dep = root / "fake.d";dep.write_text("fake subprocess only; not compiled BRN\n")
            corpus = root / "corpus.json";corpus.write_bytes((source / "corpus.json").read_bytes());c = json.loads(corpus.read_text())
            method = root / "method.md";method.write_text("synthetic method binding\n")
            rubric = root / "rubric.json";rubric.write_text('{}\n')
            workflows=[]
            for ordinal,(repeat,arm) in enumerate(prep.HISTORY_ORDER,1):
                data=root / f"r{repeat}-{arm}";data.mkdir()
                for suffix in ('','-wal','-shm'):(data/('threads.sqlite3'+suffix)).write_text('synthetic database identity'+suffix)
                facts={'case':arm,'data_dir':str(data),'project':f'project-{ordinal}','chat':f'chat-{ordinal}','note':f'note-{ordinal}','note_version':2,'early_message':f'm-{ordinal}-0','early_version':1,'proposal_message':f'm-{ordinal}-40','history_messages':[f'm-{ordinal}-{n}' for n in range(68)],'selected':arm=='candidate','skill_id':'installed:int-d-review','skill_digest':'a'*64}
                (data/'facts.json').write_text(json.dumps(facts))
                binding=[{'id':facts['skill_id'],'digest':facts['skill_digest'],'project':facts['project'],'enabled':True,'selected':True}] if arm=='candidate' else []
                settings=dict(provider='Codex',model='gpt-6.1-sol',effort='medium',auth_file=prep.AUTH_REFERENCE,credentials_dir=None,maintenance=False,review_first=True,delegation_chosen=True)
                if binding:settings={'skills':binding,**settings}
                records=[{'id':facts['project'],'version':1,'archived':False,'data':{'Project':{}}},{'id':facts['chat'],'version':1,'archived':False,'data':{'Thread':{'state':'Open'}}},{'id':facts['note'],'version':2,'archived':False,'data':{'Note':{'protected':True}}},{'id':f'settings-{ordinal}','version':4,'archived':False,'data':{'Settings':settings}}]
                records += [{'id':i,'version':1,'archived':False,'data':{'Message':{'thread':facts['chat'],'text':c['early_requirement'] if n==0 else c['later_proposal'] if n==40 else 'synthetic public filler'}}} for n,i in enumerate(facts['history_messages'])]
                envelope={'status':'ok','authority':{'workspace_purpose':'test'},'result':records}
                record_path=root/f'{ordinal}-records.json';record_path.write_text(json.dumps(envelope))
                context=prep.history_context(envelope,facts,c);context_path=root/f'{ordinal}-context.json';context_path.write_text(json.dumps(context))
                manifest=root/f'{ordinal}-manifest.json';manifest.write_text(json.dumps({'cases':[facts]}))
                prompt=root/f'{ordinal}.txt';prompt.write_text(prep.history_prompt(facts['note']))
                ask=[str(fake),'--json','--data-dir',str(data),'--timeout','120','ask','--thread',facts['chat'],'--prompt-file',str(prompt),'--budget','8','--trace','summary']
                workflows.append({'ordinal':ordinal,'repeat':repeat,'arm':arm,'facts':facts,'context':context,'context_path':str(context_path),'context_sha256':sha(context_path),'records_path':str(record_path),'records_sha256':sha(record_path),'manifest':str(manifest),'manifest_sha256':sha(manifest),'database':prep.stamp_database(data),'prompt':str(prompt),'prompt_sha256':sha(prompt),'stop_observer':None,'continue_command':None,'configure_command':None,'ask_command':ask})
            artifact={'path':str(fake),'sha256':sha(fake),'dep_info':str(dep),'dep_info_sha256':sha(dep)}
            plan={'format':prep.HISTORY_FORMAT,'phase':'history-only','task':prep.HISTORY_TASK,'source_pin':pin,'source_root':str(repo),'source_inputs':inputs.copy(),'scripts':{str(scripts/n):sha(scripts/n) for n in ('prepare-qualification.py','observe-workflow.py','check-observer.py')},'compiled_artifacts':{'binary':artifact,'generator':artifact},'bound_method':{str(method):sha(method),str(rubric):sha(rubric)},'corpus':str(corpus),'corpus_sha256':sha(corpus),'route':{'provider':'Codex','model':'gpt-6.1-sol','effort':'medium','auth_reference':prep.AUTH_REFERENCE},'limits':{'total_workflow_starts':4,'max_tool_rounds_each':8,'host_timeout_seconds_each':120},'output_namespace':str(root/'observations'),'workflows':workflows}
            if mode=='spawn-failure':
                fake.write_text('#!/absent/synthetic-interpreter\n');plan['compiled_artifacts']['binary']['sha256']=sha(fake)
            if mode=='changed-source':plan['source_inputs']['input-0.txt']='0'*64
            if mode=='wrong-order':plan['workflows'][0],plan['workflows'][1]=plan['workflows'][1],plan['workflows'][0]
            if mode=='changed-authref':plan['workflows'][0]['context']['settings']['auth_reference']='/unapproved/reference'
            preflight=root/'preflight.json';preflight.write_text(json.dumps(plan))
            activation=root/'fake-only-activation.json'
            act={'format':prep.HISTORY_FORMAT,'task':prep.HISTORY_TASK,'state':'ACTIVATED','source_pin':pin,'preflight_sha256':sha(preflight),'max_starts':4,'issued_at':int(time.time()),'expires_at':int(time.time())+120}
            if mode=='wrong-type-activation':act['max_starts']='4'
            if mode=='old-activation':act={'state':'ACTIVATED','source_pin':pin,'preflight_sha256':sha(preflight)}
            if mode=='expired-activation':act['issued_at']-=200;act['expires_at']-=200
            activation.write_text(json.dumps(act))
            if mode=='malformed-activation':activation.write_text('[')
            if mode=='missing-activation':activation.unlink()
            changed={'changed-script':scripts/'check-observer.py','changed-artifact':fake,'changed-depinfo':dep,'changed-corpus':corpus,'changed-prompt':Path(workflows[0]['prompt']),'changed-rubric':rubric,'changed-method':method,'changed-manifest':Path(workflows[0]['manifest']),'changed-main':Path(workflows[0]['facts']['data_dir'])/'threads.sqlite3','changed-wal':Path(workflows[0]['facts']['data_dir'])/'threads.sqlite3-wal','changed-shm':Path(workflows[0]['facts']['data_dir'])/'threads.sqlite3-shm','changed-context':Path(workflows[0]['context_path'])}
            if mode in changed:changed[mode].write_bytes(changed[mode].read_bytes()+b'changed')
            if mode in ('occupied-output','cap'):
                (root/'observations').mkdir()
                if mode=='cap':(root/'observations/ledger.json').write_text(json.dumps({'starts':[{'ordinal':n} for n in range(1,5)],'finished_cases':[],'window_failed':False}))
            env=dict(os.environ,INT_D_HISTORY_FAKE_ROOT=str(root),INT_D_HISTORY_FAKE_MODE=mode)
            command=[sys.executable,str(scripts/'observe-workflow.py'),'--preflight',str(preflight),'--activation',str(activation),'--ordinal']
            ordinals=range(1,5) if mode=='four-order' else [1]
            runs=[]
            for ordinal in ordinals:
                r=subprocess.run(command+[str(ordinal)],env=env,capture_output=True,timeout=25);runs.append({'ordinal':ordinal,'exit':r.returncode,'stderr':r.stderr.decode(),'stdout':r.stdout.decode()})
            ledger_path=root/'observations/ledger.json';ledger=json.loads(ledger_path.read_text()) if ledger_path.exists() else None
            starts=(root/'actual-starts').read_text().splitlines() if (root/'actual-starts').exists() else []
            refusal=mode.startswith('changed-') or mode in ('occupied-output','cap','wrong-order','missing-activation','old-activation','expired-activation','malformed-activation','wrong-type-activation')
            if refusal:
                assert runs[0]['exit']!=0 and not starts,(mode,runs)
            elif mode in ('four-order','duplicate','export-success'):
                assert all(r['exit']==0 for r in runs),(mode,runs)
                assert starts==(['1','2','3','4'] if mode=='four-order' else ['1'])
                assert ledger['finished_cases']==([1,2,3,4] if mode=='four-order' else [1])
                assert all(r.get('owned_group_reaped') for r in ledger['starts'])
            else:
                assert runs[0]['exit']!=0 and ledger['window_failed'] and not ledger['finished_cases'],(mode,runs,ledger)
                assert len(ledger['starts'])==1 and ledger['starts'][0]['reserved_before_spawn']
                assert starts==([] if mode=='spawn-failure' else ['1'])
                # Fail-fast also denies the NEXT predeclared case, not just retry.
                next_run=subprocess.run(command+['2'],env=env,capture_output=True,timeout=5)
                assert next_run.returncode!=0 and json.loads(ledger_path.read_text())==ledger
            if ledger and mode!='cap':
                retry=subprocess.run(command+['1'],env=env,capture_output=True,timeout=5)
                assert retry.returncode!=0 and json.loads(ledger_path.read_text())==ledger
                for start in ledger['starts']:
                    if 'pgid' in start:
                        try:os.killpg(start['pgid'],0)
                        except ProcessLookupError:pass
                        else:raise AssertionError('owned fake group survived')
            if mode=='changed-script':shutil.copyfile(source/'check-observer.py',scripts/'check-observer.py')
            if mode=='export-success':
                exported=Path(ledger['starts'][0]['candidate_inspection']['output']);(exported/'candidate.json').unlink();exported.rmdir()
            rows.append({'case':mode,'runs':runs,'actual_fake_starts':len(starts),'ledger':ledger,'normal_Auth_or_provider_starts':0})
    return {'qualification':'offline fake CLI subprocesses and synthetic Git only, not a BRN/provider/Auth/activation proof','cases':rows,'live_starts':0}


if __name__ == "__main__":
    if "--history-only" in sys.argv:
        print(json.dumps(check_history(), indent=2))
    else:
        main()
