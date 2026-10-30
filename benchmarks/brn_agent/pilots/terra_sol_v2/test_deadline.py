import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

HERE = Path(__file__).resolve().parent

class DeadlineTests(unittest.TestCase):
    def drive(self, folder, body, timeout=10):
        script = 'import sys\nsys.path.insert(0,'+repr(str(HERE))+')\nfrom deadline_guard import Guard\n'
        script += 'import subprocess,os,time,json\n'+body
        return subprocess.run([sys.executable, '-B', '-c', script], cwd=folder,
                              capture_output=True, text=True, timeout=timeout)

    def assert_dead(self, pid):
        for _ in range(40):
            out = subprocess.run(['/bin/ps','-o','stat=','-p',str(pid)], capture_output=True,text=True)
            if not out.stdout.strip() or out.stdout.strip().startswith('Z'):
                return
            time.sleep(.05)
        self.fail('Owned descendant survived deadline')

    def test_deadline_stops_separate_helper_group(self):
        with tempfile.TemporaryDirectory() as d:
            child = "import os,time;os.setpgid(0,0);open('helper.pid','w').write(str(os.getpid()));time.sleep(30)"
            parent = 'import subprocess,sys,time;subprocess.Popen([sys.executable,"-c",'+repr(child)+']);time.sleep(30)'
            result = self.drive(d, "g=Guard('claim',1,{})\ng.install()\nsubprocess.run([sys.executable,'-c',"+repr(parent)+"])\n")
            self.assertEqual(result.returncode,124,result.stderr)
            self.assert_dead(int((Path(d)/'helper.pid').read_text()))
            rows=[json.loads(x) for x in (Path(d)/'claim').read_text().splitlines()]
            self.assertEqual(rows[-1]['status'],'deadline')
            self.assertEqual(rows[0]['event'],'claimed')
            self.assertEqual(rows[1]['event'],'launch_intent')

    def test_completed_leader_cannot_leave_helper(self):
        with tempfile.TemporaryDirectory() as d:
            child = "import os,time;os.setpgid(0,0);open('helper.pid','w').write(str(os.getpid()));time.sleep(30)"
            parent = 'import subprocess,sys,time;subprocess.Popen([sys.executable,"-c",'+repr(child)+']);time.sleep(.3)'
            result=self.drive(d,"g=Guard('claim',4,{})\ng.install()\np=subprocess.Popen([sys.executable,'-c',"+repr(parent)+"])\np.wait()\ng.finish(0)\n")
            self.assertEqual(result.returncode,0,result.stderr)
            self.assert_dead(int((Path(d)/'helper.pid').read_text()))

    def test_consumed_claim_refuses_replay(self):
        with tempfile.TemporaryDirectory() as d:
            result=self.drive(d,"g=Guard('claim',2,{})\ng.install()\ng.finish(0)\ng=Guard('claim',2,{})\n")
            self.assertNotEqual(result.returncode,0)
            self.assertIn('FileExistsError',result.stderr)
            self.assertEqual(len((Path(d)/'claim').read_text().splitlines()),2)

    def test_receipt_failure_does_not_cancel_deadline(self):
        with tempfile.TemporaryDirectory() as d:
            result=self.drive(d,"g=Guard('claim',.3,{})\ng.install()\ng.record=lambda *a,**k: (_ for _ in ()).throw(OSError('fixture'))\ntime.sleep(30)\n")
            self.assertEqual(result.returncode,124,result.stderr)

    def test_hard_deadline_survives_final_receipt_failure(self):
        with tempfile.TemporaryDirectory() as d:
            result=self.drive(d,"g=Guard('claim',.5,{})\ng.install()\ng.record=lambda *a,**k: time.sleep(30)\ng.finish(0)\n")
            self.assertEqual(result.returncode,124,result.stderr)

    def test_cleanup_failure_is_not_success(self):
        with tempfile.TemporaryDirectory() as d:
            result=self.drive(d,"g=Guard('claim',2,{})\ng.install()\ng.stop_children=lambda: ['fixture']\nsys.exit(g.finish(0))\n")
            self.assertEqual(result.returncode,1,result.stderr)
            self.assertEqual(json.loads((Path(d)/'claim').read_text().splitlines()[-1])['status'],'failed')
            self.assertEqual(json.loads((Path(d)/'claim').read_text().splitlines()[-1])['exit_code'],1)

    def test_final_identity_failure_stops(self):
        sys.path.insert(0,str(HERE))
        from launch import check_completed_attempts, PilotStop
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'runs'/'F01-0'/'terra-r1';p.mkdir(parents=True)
            raw={'status':'complete','model':'gpt-6.1-terra','steps':[{'kind':'ask','terminal':'Completed','trace':{'wire_requests':[{'model':'wrong','effort':'medium'}]}}]}
            (p/'result.json').write_text(json.dumps(raw))
            with self.assertRaises(PilotStop):
                check_completed_attempts(d)
            raw['steps'][0]['trace']['wire_requests'][0]['model']='gpt-6.1-terra'
            (p/'result.json').write_text(json.dumps(raw))
            check_completed_attempts(d)
            raw['safety']=[{'kind':'privacy'}]
            (p/'result.json').write_text(json.dumps(raw))
            with self.assertRaises(PilotStop):
                check_completed_attempts(d)

    def test_setup_consumes_same_deadline(self):
        sys.path.insert(0,str(HERE))
        from launch import remaining_budget
        from datetime import datetime, timezone, timedelta
        start=datetime(2026,10,29,5,tzinfo=timezone.utc)
        release={'plan_sha256':'plan','package_manifest_sha256':'package'}
        setup={'started_at':start.isoformat(),'deadline_at':(start+timedelta(seconds=3600)).isoformat(),
               'job':'brn-terra-sol-pilot-001','runner':'brn-mac-relay-01',**release,
               'source_commit':'12251dfc78bc6ff9aa0eaf6809448886b0aa794c'}
        self.assertEqual(remaining_budget(setup,release,start+timedelta(seconds=1800)),1800)
        with self.assertRaises(ValueError):
            remaining_budget(setup,release,start+timedelta(seconds=3550))
        setup['deadline_at']=(start+timedelta(seconds=7200)).isoformat()
        with self.assertRaises(ValueError):
            remaining_budget(setup,release,start+timedelta(seconds=1800))

    def test_outer_stop_cleans_nested_supervisor_sessions(self):
        with tempfile.TemporaryDirectory() as d:
            child="import os,time;open('nested.pid','w').write(str(os.getpid()));time.sleep(30)"
            inner="import sys;sys.path.insert(0,"+repr(str(HERE))+");from deadline_guard import Guard;import subprocess,time;g=Guard('inner',20,{});g.install();subprocess.run([sys.executable,'-c',"+repr(child)+"])"
            result=self.drive(d,"g=Guard('outer',1.5,{})\ng.install()\nsubprocess.run([sys.executable,'-c',"+repr(inner)+"])\n")
            self.assertEqual(result.returncode,124,result.stderr)
            self.assert_dead(int((Path(d)/'nested.pid').read_text()))

    def test_partial_session_ledger_still_stops_active_child(self):
        with tempfile.TemporaryDirectory() as d:
            child="import os,time;open('active.pid','w').write(str(os.getpid()));time.sleep(30)"
            result=self.drive(d,"g=Guard('claim',1.5,{})\ng.install()\np=subprocess.Popen([sys.executable,'-c',"+repr(child)+"])\ntime.sleep(.2)\nopen(g.registry.path,'a').write('{broken')\ntime.sleep(30)\n")
            self.assertEqual(result.returncode,124,result.stderr)
            self.assert_dead(int((Path(d)/'active.pid').read_text()))
            terminal=json.loads((Path(d)/'claim').read_text().splitlines()[-1])
            self.assertTrue(terminal['cleanup_errors'])

    def test_no_release_no_launch(self):
        result=subprocess.run([sys.executable,'-B',str(HERE/'launch.py')],capture_output=True,text=True)
        self.assertEqual(result.returncode,2)
        self.assertIn('--release',result.stderr)

if __name__ == '__main__':
    unittest.main()
