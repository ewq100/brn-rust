"""Include AgentBench's offline contract tests in the normal tooling test lane."""
from pathlib import Path


def load_tests(loader, tests, pattern):
    directory = Path(__file__).resolve().parents[2] / 'benchmarks/brn_agent/tests'
    return loader.discover(str(directory), pattern='test_*.py', top_level_dir=str(directory))
