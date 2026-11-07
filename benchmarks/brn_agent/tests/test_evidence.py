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

    def test_verified_packet_refuses_wrong_scenario_and_modified_records(self):
        from evidence import verified_proof_packet
        fixture_hash = proof.build()[1]['fixture_sha256']
        self.suite.write_text(json.dumps({'fixture_sha256': fixture_hash}))
        (self.root / 'manifest.json').write_text(json.dumps({'fixture_sha256': fixture_hash}))
        manifest = collect(self.root, self.suite)
        (self.root / 'agentbench-evidence.json').write_text(json.dumps(manifest))
        checkpoint = self.folder / 'checkpoints/0'
        packet = verified_proof_packet(checkpoint, scenario='F01', step=0)
        self.assertEqual(packet['checkpoint_gold']['scenario'], 'F01')
        with self.assertRaises(ValueError):
            verified_proof_packet(checkpoint, scenario='F02', step=0)
        import shutil
        alternate = checkpoint.with_name('00')
        shutil.copytree(checkpoint, alternate)
        (alternate / 'records.json').write_text('[]')
        with self.assertRaises(ValueError):
            verified_proof_packet(alternate, wiki_records=[])
        (checkpoint / 'records.json').write_text('[]')
        with self.assertRaises(ValueError):
            verified_proof_packet(checkpoint, scenario='F01', step=0)

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

class PacketAndIdentityTests(unittest.TestCase):
    def test_wire_identity_cannot_be_inferred_from_configuration(self):
        from evidence import identity_matches
        self.assertFalse(identity_matches({'wire_requests': []}, 'm', 'low'))
        trace = {'wire_requests': [{'model': 'm', 'effort': 'low'}], 'returned_models': ['m']}
        self.assertTrue(identity_matches(trace, 'm', 'low'))
        self.assertFalse(identity_matches(trace, 'm', 'high'))
        trace['returned_models'] = ['substitute']
        self.assertFalse(identity_matches(trace, 'm', 'low'))

    def test_wiki_reader_does_not_get_raw_note_sources(self):
        from evidence import grading_packet
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)
            (path / 'records.json').write_text(json.dumps([
                {'id':'source-note','version':1,'data':{'Note':{'markdown':'raw evidence'}}},
                {'id':'wiki-note','version':2,'data':{'Note':{'markdown':'saved synthesis'}}}]))
            with self.assertRaises(ValueError):
                grading_packet(path, wiki_only=True)
            packet = grading_packet(path, wiki_only=True, wiki_records=[{'id':'wiki-note','version':2}])
            self.assertEqual([r['id'] for r in packet['records']], ['wiki-note'])
            with self.assertRaises(ValueError):
                grading_packet(path, wiki_only=True, wiki_records=[{'id':'wiki-note','version':1}])

    def test_checkpoint_packet_gold_does_not_include_later_truth(self):
        gold = proof.checkpoint_gold('J01', 1)
        self.assertNotIn('2027-03-03', str(gold))
        self.assertNotIn('March 10', str(gold))
        self.assertIn('2027-03-03', str(proof.checkpoint_gold('J01', 2)))
        with self.assertRaises(ValueError):
            proof.checkpoint_gold('J01', 13)

class CompleteTraceTests(unittest.TestCase):
    def test_failed_calls_retained_and_pending_helpers_unknown(self):
        from evidence import full_calls
        trace = {'level': 'full', 'dropped_calls': 0,
                 'calls': [{'tool': 'read_message', 'outcome': 'error'}]}
        self.assertEqual(full_calls(trace), trace['calls'])
        trace['helpers'] = [{'trace': dict(trace, calls=[{'tool': 'read_note', 'outcome': 'pending'}])}]
        self.assertIsNone(full_calls(trace))

    def test_usage_requires_every_http_and_both_counters(self):
        from evidence import complete_trace_usage
        usage = dict(input=100, output=10, cached_input=25, complete=True)
        trace = dict(wire_requests_dropped=0, wire_requests=[{}], turn_usage=[usage])
        self.assertEqual(complete_trace_usage(trace)['input'], 100)
        usage['complete'] = False
        self.assertIsNone(complete_trace_usage(trace))
        usage['complete'] = True
        trace['wire_requests'].append({})
        self.assertIsNone(complete_trace_usage(trace))
        trace['summary'] = {'usage': dict(usage, input=20, cached_input=0)}
        self.assertEqual(complete_trace_usage(trace)['input'], 120)

    def test_cached_or_undispatched_summary_adds_no_http_usage(self):
        from evidence import complete_trace_usage
        usage = dict(input=100, output=10, cached_input=25, complete=True)
        trace = dict(wire_requests_dropped=0, wire_requests=[{}], turn_usage=[usage])
        for outcome in ('cached', 'unavailable_account', 'gap'):
            trace['summary'] = dict(outcome=outcome, usage=None)
            self.assertEqual(complete_trace_usage(trace)['input'], 100)
        # A failed summary with a real wire attempt still requires its counters.
        trace['wire_requests'].append({})
        self.assertIsNone(complete_trace_usage(trace))

    def test_message_and_archived_evidence_selected_without_expanding_wiki(self):
        from evidence import grading_packet
        with tempfile.TemporaryDirectory() as folder:
            records = [{'id': 'm1', 'version': 1, 'data': {'Message': {'text': 'source'}}},
                       {'id': 'n1', 'version': 1, 'archived': True, 'data': {'Note': {'markdown': 'past'}}},
                       {'id': 'm2', 'version': 1, 'data': {'Message': {'text': 'unselected'}}}]
            Path(folder, 'records.json').write_text(json.dumps(records))
            packet = grading_packet(folder, observed_ids=['m1', 'n1', 'absent'])
            self.assertEqual([r['id'] for r in packet['additional_checkpoint_records']], ['m1', 'n1'])
            self.assertEqual(packet['unavailable_observed_ids'], ['absent'])
            self.assertNotIn('additional_checkpoint_records', grading_packet(folder, wiki_only=True, wiki_records=[]))
