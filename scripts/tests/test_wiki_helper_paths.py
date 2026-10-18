"""Current/historical selection uses only fake paths; no executable/provider starts."""
import importlib.util
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

v1 = load('wiki_helpers_v1', 'scripts/evaluate-wiki-guidance.py')
v2 = load('wiki_helpers_v2', 'scripts/evaluate_wiki_v2.py')

class HelperPathsTests(unittest.TestCase):
    def test_sibling_defaults_and_explicit_historical_selection(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); (root / 'EVALUATION-OWNED').touch()
            with patch.dict(os.environ, {'BRN_INTAKE_HELPER': '/ambient-old', 'BRN_XBERG_HELPER': '/ambient-stock'}):
                cli = v1.Cli(Path('/arm/brn'), root / 'data', root)
                self.assertNotIn('BRN_INTAKE_HELPER', cli.environment)
                self.assertEqual(cli.environment['BRN_XBERG_HELPER'], '/arm/brn-xberg-helper')
                self.assertEqual(cli.environment['BRN_XBERG_HELPER_BRN'], '/arm/brn-xberg-helper-brn')
                explicit = v1.Cli(Path('/old/brn'), root / 'data', root, Path('/old/helper'),
                                  stock_helper=Path('/chosen/stock'), brn_helper=Path('/chosen/adapted'))
                self.assertEqual(explicit.environment['BRN_INTAKE_HELPER'], '/old/helper')
                self.assertEqual(explicit.environment['BRN_XBERG_HELPER'], '/chosen/stock')
                self.assertEqual(explicit.environment['BRN_XBERG_HELPER_BRN'], '/chosen/adapted')
                self.assertEqual(cli.commands, [])
                for paths in [{'stock_helper': Path('relative')}, {'brn_helper': Path('relative')}]:
                    with self.assertRaisesRegex(ValueError, 'absolute'):
                        v1.Cli(Path('/arm/brn'), root / 'data', root, **paths)

    def test_v2_uses_each_arm_and_keeps_active_research_helper_independent(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); (root / 'EVALUATION-OWNED').touch()
            args = SimpleNamespace(helper=None, timeout=1, stock_helper=None, brn_helper=None)
            arm = {'model_dir': None, 'helper': {'model': 'gpt-6-luna', 'effort': 'high'}}
            for folder in ['one', 'two']:
                cli = v2.make_cli(arm, Path('/') / folder / 'brn', root / folder, root, args)
                self.assertEqual(cli.environment['BRN_XBERG_HELPER'], f'/{folder}/brn-xberg-helper')
                self.assertEqual(cli.environment['BRN_XBERG_HELPER_BRN'], f'/{folder}/brn-xberg-helper-brn')
                self.assertNotIn('BRN_INTAKE_HELPER', cli.environment)
                self.assertEqual(cli.commands, [])
            self.assertEqual(arm['helper']['model'], 'gpt-6-luna')
            parsed = v2.build_parser().parse_args(['--root', '/synthetic', '--stock-helper', '/s', '--brn-helper', '/b'])
            self.assertEqual((parsed.stock_helper, parsed.brn_helper, parsed.helper), (Path('/s'), Path('/b'), None))
