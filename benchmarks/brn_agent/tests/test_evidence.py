import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from evidence import collect, verify_manifest, verify_grade_references
from score import sha256, assess
import proof


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.folder = self.root / 'runs/F01-0/A-r1'
        checkpoint = self.folder / 'checkpoints/0'
        checkpoint.mkdir(parents=True)
        (checkpoint / 'workspace.sqlite3').write_bytes(b'synthetic backup witness')
        (checkpoint / 'records.json').write_text(json.dumps([{'id': 'n1', 'version': 2, 'data': {'Note': {'markdown': 'known'}}}]))
        raw = {'case': 'F01', 'arm': 'A', 'status': 'transport_failure', 'repetition': 1, 'attempt': 1,
               'steps': [{'kind': 'ask', 'terminal': 'Network', 'trace': {'usage': None}}],
               'checkpoints': [{'step': 0, 'backup_sha256': sha256(checkpoint / 'workspace.sqlite3'),
                                'records_sha256': sha256(checkpoint / 'records.json')}]}
        (self.folder / 'result.json').write_text(json.dumps(raw))
        self.suite = self.root / 'suite.json'
        self.suite.write_text('{"fixture_sha256": "synthetic-test-fixture"}')
        (self.root / 'manifest.json').write_text('{"fixture_sha256": "synthetic-test-fixture"}')
        self.manifest = collect(self.root, self.suite)

    def test_metadata_cannot_relabel_failure_or_usage(self):
        verify_manifest(self.manifest, self.root)
        self.manifest['attempts'][0]['status'] = 'complete'
        with self.assertRaises(ValueError):
            verify_manifest(self.manifest, self.root)

    def test_wrong_fixture_refused_and_missing_helper_usage_unknown(self):
        self.suite.write_text('{"fixture_sha256": "different"}')
        with self.assertRaises(ValueError):
            collect(self.root, self.suite)
        self.suite.write_text('{"fixture_sha256": "synthetic-test-fixture"}')
        path = self.folder / 'result.json'
        raw = json.loads(path.read_text())
        raw['helper'] = {'model': 'synthetic', 'effort': 'low'}
        raw['steps'][0]['trace'] = None
        path.write_text(json.dumps(raw))
        manifest = collect(self.root, self.suite)
        self.assertIsNone(manifest['attempts'][0]['usage']['helper_input'])

    def test_checkpoint_tampering_detected_even_with_unchanged_result(self):
        (self.folder / 'checkpoints/0/records.json').write_text('[]')
        with self.assertRaises(ValueError):
            verify_manifest(self.manifest, self.root)

    def test_exact_record_version_and_result_step_refs(self):
        grade = {'attempt': self.manifest['attempts'][0]['id'], 'criteria': {'quality': {
            'evidence': [{'checkpoint': 0, 'record': 'n1', 'version': 2}, {'step': 1}]}}}
        verify_grade_references(self.manifest, [grade], self.root)
        grade['criteria']['quality']['evidence'][0]['version'] = 1
        with self.assertRaises(ValueError):
            verify_grade_references(self.manifest, [grade], self.root)
        grade['criteria']['quality']['evidence'] = ['invented']
        with self.assertRaises(ValueError):
            verify_grade_references(self.manifest, [grade], self.root)

    def test_orphan_then_successful_replacement_still_reports_incomplete(self):
        (self.folder / 'result.json').unlink()
        retry = self.folder.with_name('A-r1-rerun')
        retry.mkdir()
        (retry / 'result.json').write_text(json.dumps({'case': 'F01', 'arm': 'A', 'status': 'complete',
                                                       'repetition': 1, 'attempt': 2, 'steps': []}))
        manifest = collect(self.root, self.suite)
        verify_manifest(manifest, self.root)
        self.assertEqual(manifest['unindexed_attempts'], ['runs/F01-0/A-r1'])
        result = assess(proof.build()[1], 'A', manifest['attempts'], [])
        self.assertEqual(result['status'], 'incomplete')
        self.assertFalse(result['ranked'])
        self.assertEqual(result['operations']['replacements'], 1)

    def test_path_escape_refused(self):
        self.manifest['attempts'][0]['result'] = '../outside.json'
        with self.assertRaises(ValueError):
            verify_manifest(self.manifest, self.root)


if __name__ == '__main__':
    unittest.main()
