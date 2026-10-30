import json
import os
from pathlib import Path
import sys
import tempfile
import subprocess
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from helper_execution import run, assess
from score import sha256

class HelperInputTests(unittest.TestCase):
    def test_rejects_unknown_or_duplicate_cases_before_execution(self):
        with tempfile.TemporaryDirectory() as temp:
            for cases in ([], ['H01'], ['H07', 'H07']):
                with self.assertRaises(ValueError):
                    run('/missing', Path(temp) / 'no-output', cases)
            self.assertFalse((Path(temp) / 'no-output').exists())

@unittest.skipUnless(os.environ.get('BRN_AGENTBENCH_HELPER_BINARY'), 'explicit controlled helper binary required')
class NativeHelperAssessmentTests(unittest.TestCase):
    def test_native_input_size_bound_refuses_before_workspace_creation(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / 'oversized.json'
            source.write_bytes(b' ' * 262145)
            output = Path(temp) / 'must-not-exist'
            result = subprocess.run([os.environ['BRN_AGENTBENCH_HELPER_BINARY'], str(source), str(output)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('256 KiB', result.stderr)
            self.assertFalse(output.exists())

    def test_good_bad_and_alternative_assessment_with_retained_native_evidence(self):
        binary = os.environ['BRN_AGENTBENCH_HELPER_BINARY']
        with tempfile.TemporaryDirectory() as temp:
            for witness, expected in [('good', 7), ('alternative', 7), ('bad', 0)]:
                root = Path(temp) / witness
                result = run(binary, root, witness=witness)
                grade = result['assessment']
                self.assertEqual(grade['correct'], expected, grade)
                self.assertEqual(grade['total'], 7)
                self.assertFalse(grade['qualified'])
                if witness == 'bad':
                    h12 = next(r for r in grade['rows'] if r['id'] == 'H12')
                    self.assertGreater(h12['blocked_read_attempts'], 0)
                    self.assertTrue(h12['checks']['no_private_exposure'])
                result_path = root / 'native' / 'helper-result.json'
                original = result_path.read_text()
                changed = json.loads(original)
                changed['cases'][0]['answer'] = '{"decision":"fallback","facts":["weight_kg=999"]}'
                result_path.write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    assess(root / 'native')
                result_path.write_text(original)
                changed = json.loads(original)
                changed['cases'][0]['notes_unchanged'] = False
                result_path.write_text(json.dumps(changed))
                self.assertEqual(assess(root / 'native')['correct'], expected)
                result_path.write_text(original)
                input_path = root / 'native' / 'input.json'
                frozen_input = input_path.read_text()
                substituted = json.loads(frozen_input)
                substituted['cases'][0]['notes'][0]['text'] = 'The parcel weighs 999 kg.'
                input_path.write_text(json.dumps(substituted))
                changed = json.loads(original)
                changed['input_sha256'] = sha256(input_path)
                result_path.write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    assess(root / 'native')
                input_path.write_text(frozen_input)
                result_path.write_text(original)
                trace = root / 'native' / 'H07' / 'trace.json'
                trace.write_text('{}')
                with self.assertRaises(ValueError):
                    assess(root / 'native')
                with self.assertRaises(FileExistsError):
                    run(binary, root, witness=witness)
