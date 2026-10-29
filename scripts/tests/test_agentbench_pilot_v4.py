"""Offline release/deadline/setup checks; no Mac, provider, or credential access."""
import importlib.util
from pathlib import Path
import unittest


def load_tests(loader, tests, pattern):
    root = Path(__file__).resolve().parents[2] / 'benchmarks/brn_agent/pilots/terra_sol_v4'
    suite = unittest.TestSuite()
    for name in ('test_deadline', 'test_setup'):
        spec = importlib.util.spec_from_file_location('agentbench_pilot_v4_' + name, root / (name + '.py'))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        suite.addTests(unittest.defaultTestLoader.loadTestsFromModule(module))
    return suite
