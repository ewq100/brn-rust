import copy
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace as NS
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))
import evaluate_wiki_v2 as v2
from evidence import collect, verified_proof_packet
from reader import run, packet_hash


@unittest.skipUnless(os.environ.get('BRN_AGENTBENCH_TEST_BINARY') and os.environ.get('BRN_AGENTBENCH_READER_BINARY'),
                     'explicit native CLI and controlled reader binaries required')
class ReaderProviderTests(unittest.TestCase):
    def test_scoped_native_queries_alternative_answers_and_provider_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)/'experiment'
            v2.create_root(root)
            binary = os.environ['BRN_AGENTBENCH_TEST_BINARY']
            arm = {'name': 'A', 'binary': binary, 'model_dir': None, 'provider': None,
                   'model': None, 'effort': None, 'binary_sha256': 'unit'}
            sample = v2.Sample(NS(helper=None, stock_helper=None, brn_helper=None, timeout=30, measure_bin=None),
                              root, {'id': 'UNIT', 'checks': []}, 0,
                              {'arm': 'A', 'repetition': 1, 'label': None}, arm, root, {}, {'trace': True})
            sample.folder.mkdir(parents=True)
            sample.cli = v2.make_cli(arm, binary, sample.folder/'data', root, sample.args)
            sample.cli.call('note-create', title='Uncompiled source', text='Raw private diagnostic: SOURCE_EXCLUDED_932')
            sample.cli.call('note-create', title='Maintained wiki', text='Target is May 1. Roof inspection is April 12.')
            initial = sample.cli.call('records')
            wiki = next(r for r in initial if r['data'].get('Note', {}).get('title') == 'Maintained wiki')
            reads = [{'id': wiki['id'], 'version': wiki['version']}]
            sample.checkpoint(0)
            (root/'manifest.json').write_text('{"fixture_sha256":"unit-fixture"}')
            suite = root/'suite.json'
            suite.write_text('{"fixture_sha256":"unit-fixture"}')
            (root/'agentbench-evidence.json').write_text(json.dumps(collect(root, suite)))
            checkpoint = sample.folder/'checkpoints/0'
            fixture = {'mode': 'wiki-only', 'question': 'Give the target and the unrelated inspection date.',
                       'records': reads, 'packet_sha256': packet_hash(verified_proof_packet(checkpoint, wiki_records=reads)),
                       'queries': ['target', 'inspection', 'SOURCE_EXCLUDED_932'], 'max_calls': 5,
                       'controlled_answer': 'Target: May 1. Inspection: April 12.'}
            for index, answer in enumerate(['Target: May 1. Inspection: April 12.',
                                            'The roof inspection is April 12; the target is May 1.']):
                fixture['controlled_answer'] = answer
                result = run(checkpoint, fixture, binary, os.environ['BRN_AGENTBENCH_READER_BINARY'], Path(temp)/f'reader{index}')
                observed = result['observed']
                self.assertEqual(observed['request_count'], 5)
                self.assertEqual(observed['answer'], answer)
                self.assertEqual(len(observed['queries']), 3)
                history = json.dumps(observed['model_visible_history'])
                self.assertIn('April 12', history)
                # The query text is visible, but the source content must not be returned.
                self.assertNotIn('Raw private diagnostic', history)
                self.assertEqual(sample.cli.call('records'), initial)
            failed = dict(fixture, provider_error='controlled transport outage')
            output = Path(temp)/'outage'
            with self.assertRaises(ValueError):
                run(checkpoint, failed, binary, os.environ['BRN_AGENTBENCH_READER_BINARY'], output)
            receipt = json.loads((output/'reader-result.json').read_text())
            self.assertEqual(receipt['status'], 'failed')
            self.assertIn('observed', receipt)
            self.assertNotEqual(receipt['observed']['terminal'], 'Completed')
            self.assertTrue((output/'reader.stdout').is_file())
            self.assertEqual(sample.cli.call('records'), initial)
            with self.assertRaises(ValueError):
                run(checkpoint, dict(fixture, max_calls=4), binary,
                    os.environ['BRN_AGENTBENCH_READER_BINARY'], Path(temp)/'budget')
            self.assertFalse((Path(temp)/'budget').exists())
