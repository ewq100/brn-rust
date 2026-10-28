import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import importlib.util
import os
import tempfile
import unittest


@unittest.skipUnless(importlib.util.find_spec('inspect_ai'), 'optional pinned Inspect environment required')
class InspectCompatibilityTests(unittest.TestCase):
    def test_frozen_solver_never_generates(self):
        with tempfile.TemporaryDirectory() as directory:
            # Keep all Inspect state inside the explicit disposable workspace.
            from unittest.mock import patch
            with patch.dict(os.environ, {'XDG_DATA_HOME': directory, 'XDG_CACHE_HOME': directory}):
                from inspect_ai import eval
                from inspect_task import frozen_task
                rows = [{'id': 'unit', 'score': 3, 'output': 'Frozen synthetic note', 'evidence': 'unit checkpoint'}]
                logs = eval(frozen_task(rows), model='mockllm/model', log_dir=directory, display='none')
                self.assertEqual(logs[0].status, 'success')
                self.assertEqual(len(logs[0].samples), 1)
                self.assertFalse(logs[0].stats.model_usage)
                self.assertEqual(logs[0].samples[0].output.completion, rows[0]['output'])
