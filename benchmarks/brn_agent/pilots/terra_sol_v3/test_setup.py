"""Complete setup control path with synthetic build tools, never a native build claim."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent

class SetupIsolationTests(unittest.TestCase):
    def test_complete_setup_survives_fault_tests_and_writes_readiness(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            tools=root/'tools'; tools.mkdir()
            ledger=root/'ledger'; ledger.mkdir()
            repo=root/'repository'; repo.mkdir()
            # Explicit synthetic executable fixtures cover control flow only.
            executable='#!'+sys.executable+'\n'
            helper_script='#!/bin/bash\nset -eu\nlast="${@: -1}"\nmkdir -p "$last/artifacts"\nprintf "synthetic helper\\n" > "$last/artifacts/brn-xberg-helper"\ncp "$last/artifacts/brn-xberg-helper" "$last/artifacts/brn-xberg-helper-brn"\nchmod +x "$last/artifacts/"*\n'
            fixtures={
                'rustup':"import sys;from pathlib import Path;print(Path(__file__).with_name(sys.argv[-1]))\n",
                'rustc':"print('rustc 1.98.1 (synthetic)')\n",
                'cargo':"import sys,os;from pathlib import Path\nif '--version' in sys.argv: print('cargo 1.98.1 (synthetic)')\nelse:\n p=Path(os.environ['CARGO_TARGET_DIR'])/'debug'/('examples/wiki_eval_seed' if '--example' in sys.argv else 'brn');p.parent.mkdir(parents=True,exist_ok=True);p.write_text('synthetic executable');p.chmod(0o700)\n",
                'git':"import sys;from pathlib import Path\nif 'worktree' in sys.argv:\n p=Path(sys.argv[-2]);(p/'scripts').mkdir(parents=True);(p/'scripts/build-document-helpers.sh').write_text("+repr(helper_script)+")\nelse: print('12251dfc78bc6ff9aa0eaf6809448886b0aa794c')\n"
            }
            for name,body in fixtures.items():
                path=tools/name;path.write_text(executable+body);path.chmod(0o700)
            env=dict(os.environ)
            # Never detach this fixture from an active supervising job.
            for name in ('AGENTBENCH_PILOT_SESSIONS','AGENTBENCH_PILOT_SCOPE'):
                self.assertFalse(env.get(name),'Run this independent setup fixture outside an active job')
            driver = "import sys;sys.path.insert(0,"+repr(str(HERE))+");import prepare_mac\n"
            driver += "from datetime import datetime, timezone\nclass FixtureClock(datetime):\n @classmethod\n def now(cls,tz=None): return datetime(2026,10,29,5,tzinfo=timezone.utc)\n"
            # Include the nested deadline suite plus synthetic build steps and
            # process-inventory/fsync contention within a bounded setup budget.
            driver += "OriginalGuard=prepare_mac.Guard\nclass FixtureGuard(OriginalGuard):\n def __init__(self,receipt,seconds,identity): super().__init__(receipt,min(seconds,120),identity)\n"
            driver += "prepare_mac.datetime=FixtureClock;prepare_mac.Guard=FixtureGuard;prepare_mac.sys.platform='darwin';sys.exit(prepare_mac.main())"

            manifest=hashlib.sha256((HERE/'package-manifest.json').read_bytes()).hexdigest()
            args=[sys.executable,'-B','-c',driver,'--repository',str(repo),'--source-dir',str(root/'source'),
                  '--claim-directory',str(ledger),'--cargo-bin',str(tools),'--auth-file',str(root/'unused-auth-selector'),
                  '--output',str(root/'experiment'),'--manifest-sha256',manifest,'--authorization-reference','synthetic-offline-test']
            result=subprocess.run(args,env=env,capture_output=True,text=True,timeout=150)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            self.assertIn('Ran 12 tests',result.stderr)
            prefix=ledger/'brn-terra-sol-pilot-002'
            receipt=Path(str(prefix)+'.setup-supervisor.jsonl')
            rows=[json.loads(line) for line in receipt.read_text().splitlines()]
            self.assertEqual(rows[-1]['status'],'completed')
            self.assertEqual(rows[-1]['exit_code'],0)
            self.assertEqual(rows[-1]['cleanup_errors'],[])
            registry=Path(str(receipt)+'.sessions.jsonl').read_text()
            self.assertNotIn('{broken',registry)
            active=set()
            for line in registry.splitlines():
                row=json.loads(line)
                if row['event']=='opened': active.add(row['sid'])
                elif row['event']=='closed': active.discard(row['sid'])
            self.assertEqual(active,set())
            # Closed accounting must agree with the OS, including helper groups.
            sys.path.insert(0,str(HERE))
            from deadline_guard import session_members
            for sid in {json.loads(line)['sid'] for line in registry.splitlines()
                        if json.loads(line)['event']=='opened'}:
                self.assertEqual(session_members(sid,runnable_only=True),set())
            readiness=json.loads(Path(str(prefix)+'.readiness.json').read_text())
            self.assertTrue(readiness['offline_build_and_supervisor_passed'])
            self.assertEqual(readiness['model_calls'],0)
            self.assertTrue(Path(str(prefix)+'.bindings.json').is_file())
            self.assertFalse((root/'experiment').exists())

if __name__=='__main__': unittest.main()
