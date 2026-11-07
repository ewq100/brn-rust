import copy
import unittest
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from frozen_judge import digest, validate


class FrozenJudgeTests(unittest.TestCase):
    def test_receipt_binds_exact_bytes_settings_and_transport(self):
        packet, instructions = b'{"answer": "synthetic"}\n', b'Frozen rubric'
        receipt = dict(status='completed', packet_file_sha256=digest(packet),
                       instructions_sha256=digest(instructions), timeout_seconds=180,
                       provider='Codex', model='gpt-6.1-sol', effort='high', tools=[],
                       model_call_budget=dict(limit=1, started=1, refused=False),
                       wire_requests=[dict(model='gpt-6.1-sol', effort='high')],
                       result=dict(returned_model='gpt-6.1-sol', text='{}', usage=dict(input=10, output=3)))
        validate(receipt, packet, instructions, 180)
        for field, value in [('packet_file_sha256', digest(packet.rstrip())),
                             ('instructions_sha256', digest(b'Changed rubric')), ('timeout_seconds', 45),
                             ('wire_requests', []), ('model_call_budget', dict(limit=2, started=1, refused=False)),
                             ('result', dict(returned_model='other', text='{}', usage=dict(input=10, output=3)))]:
            changed = copy.deepcopy(receipt)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate(changed, packet, instructions, 180)

    def test_runner_retains_timeout_and_refuses_reusing_attempt_path(self):
        import tempfile
        from frozen_judge import run
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            binary = root / 'judge'
            binary.write_text('#!/bin/sh\nprintf \'{"status":"timeout","model_call_budget":{"started":1}}\\n\'\n')
            binary.chmod(0o700)
            packet, instructions, output = root / 'packet', root / 'instructions', root / 'receipt'
            packet.write_text('{"answer":"synthetic"}')
            instructions.write_text('Fixed evaluator')
            result = run(binary, root / 'auth', packet, instructions, output,
                         binary_sha256=digest(binary.read_bytes()))
            self.assertFalse(result['qualified'])
            self.assertEqual(result['receipt']['model_call_budget']['started'], 1)
            with self.assertRaises(FileExistsError):
                run(binary, root / 'auth', packet, instructions, output,
                    binary_sha256=digest(binary.read_bytes()))
