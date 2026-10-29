"""Include AgentBench's offline contract tests in the normal tooling test lane."""
from pathlib import Path
import unittest


def load_tests(loader, tests, pattern):
    directory = Path(__file__).resolve().parents[2] / 'benchmarks/brn_agent/tests'
    # Python 3.12 discovery can retain the nested top_level_dir on a shared
    # loader, making the next scripts/tests sibling appear outside the project.
    # Keep nested suite discovery independent of the caller's traversal state.
    return unittest.TestLoader().discover(str(directory), pattern='test_*.py', top_level_dir=str(directory))
