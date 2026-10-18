"""Synthetic negative controls; no Cargo, provider or network."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('retirement', ROOT / 'scripts/check-intake-retirement.py')
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)


class RetirementTests(unittest.TestCase):
    def test_current_tree(self):
        self.assertEqual(r.check(ROOT), [])

    def test_each_legacy_path_is_detected(self):
        for path in r.LEGACY_PATHS:
            with self.subTest(path=path), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text('')
                self.assertTrue(any('Retired path' in error for error in r.check(root)))

    def test_module_dependency_feature_and_build_negative_controls(self):
        cases = [('crates/brn-intake/src/lib.rs', 'pub mod prepared;'),
                 ('crates/brn-ai/src/conversion.rs', 'pub async fn convert() {}'),
                 ('crates/brn-intake/Cargo.toml', '[dependencies]\nxberg = "1.3.10"'),
                 ('crates/brn-intake/Cargo.toml', '[features]\nhelper = []'),
                 ('scripts/verify-storage.sh', 'cargo build --bin brn-intake-helper'),
                 ('Cargo.lock', '[[package]]\nname="mail-parser"\nversion="0.11.8"')]
        for path, text in cases:
            with self.subTest(path=path), tempfile.TemporaryDirectory() as temp:
                root = Path(temp); target = root / path
                target.parent.mkdir(parents=True, exist_ok=True); target.write_text(text)
                self.assertTrue(r.check(root))

    def test_history_and_explicit_historical_eval_support_are_not_live_readers(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for path in ['docs/archive/old.md', 'patches/xberg/old.patch', 'scripts/evaluate-wiki-guidance.py']:
                target = root / path; target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text('brn-intake-helper HelperRequest PreparedDocument')
            self.assertEqual(r.check(root), [])

    def test_workflow_argument_and_prerequisite_order_negative_controls(self):
        stock = 'cargo fetch --locked --manifest-path tools/xberg-intake-helper/Cargo.toml\n'
        tests = 'python3 -m unittest discover -s scripts/tests -v\n'
        generate = 'python3 scripts/prepare-xberg-adapted.py --archive "$archive"\n'
        adapted = 'cargo fetch --locked --manifest-path target/xberg-adapted/workspace/Cargo.toml\n'
        valid = stock + tests + generate + adapted
        self.assertEqual(r.workflow_failures('synthetic', valid), [])
        self.assertTrue(r.workflow_failures('synthetic', valid.replace(' --archive "$archive"', '')))
        self.assertTrue(r.workflow_failures('synthetic', tests + stock + generate + adapted))
        self.assertTrue(r.workflow_failures('synthetic', generate + stock + tests + adapted))
        for name in ['ci.yml', 'preview-release.yml']:
            text = (ROOT / '.github/workflows' / name).read_text()
            self.assertEqual(r.workflow_failures(name, text), [])
