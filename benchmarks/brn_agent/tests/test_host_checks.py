from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
from unittest.mock import patch
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import host_checks


class HostCheckAccountingTests(unittest.TestCase):
    def test_zero_tests_and_unexpected_skips_do_not_qualify(self):
        cases = [('',0,0,'failed'),('unexpected',1,1,'failed'),
                 (host_checks.MANUAL_EXPORT,16,1,'passed')]
        with tempfile.TemporaryDirectory() as tmp:
            for index,(ignored,passed,count,status) in enumerate(cases):
                def run(command,cwd,stdout,stderr):
                    if ignored: stdout.write(f'test {ignored} ... ignored, manual export\n')
                    stdout.write(f'test result: ok. {passed} passed; 0 failed; {count} ignored; 0 measured\n')
                    return SimpleNamespace(returncode=0)
                def check_output(command,**kwargs):
                    return 'unit-commit\n' if 'rev-parse' in command else 'Cargo.lock\n'
                with patch.object(host_checks,'subprocess',SimpleNamespace(run=run,check_output=check_output,STDOUT=-2)):
                    result = host_checks.run(Path(tmp)/str(index),['process-kill'])
                self.assertEqual(result['status'],status)
                self.assertFalse(result['model_scores'])
                if status=='passed':
                    self.assertEqual(result['checks'][0]['excluded_manual_export'],host_checks.MANUAL_EXPORT)
