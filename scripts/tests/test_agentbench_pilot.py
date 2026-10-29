"""Offline release/deadline checks; no Mac, provider, or credential access."""
import importlib.util
from pathlib import Path
import unittest


def load_tests(loader, tests, pattern):
    path = Path(__file__).resolve().parents[2] / 'benchmarks/brn_agent/pilots/terra_sol_v2/test_deadline.py'
    spec = importlib.util.spec_from_file_location('agentbench_pilot_tests', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return unittest.defaultTestLoader.loadTestsFromModule(module)
