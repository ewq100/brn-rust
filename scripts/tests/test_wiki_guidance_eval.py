import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'evaluate-wiki-guidance.py'
spec = importlib.util.spec_from_file_location('wiki_eval', SCRIPT)
wiki_eval = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wiki_eval)


def note(identity, text, protected=False, version=1, archived=False):
    return {'id': identity, 'version': version, 'archived': archived,
            'data': {'Note': {'title': identity, 'markdown': text, 'protected': protected,
                              'confirmed': False, 'superseded_by': None, 'import': None}}}


class WikiGuidanceEvaluationTests(unittest.TestCase):
    def test_occupied_root_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sentinel = root / 'owner-file'
            sentinel.write_text('preserve')
            with self.assertRaises(FileExistsError):
                wiki_eval.create_root(root)
            self.assertEqual(sentinel.read_text(), 'preserve')

    def test_relative_root_is_refused(self):
        with self.assertRaises(ValueError):
            wiki_eval.create_root(Path('relative-workspace'))

    def test_nested_configuration_is_excluded(self):
        settings = {'id': 'settings', 'data': {'Settings': {'auth_file': 'SECRET'}}}
        value = {'records': [settings, note('safe', 'source-supported')],
                 'receipt': {'writes': [{'before': settings, 'after': settings}]},
                 'nested': {'credentials_dir': 'SECRET', 'safe': 4}}
        clean = wiki_eval.public_snapshot(value)
        self.assertNotIn('SECRET', json.dumps(clean))
        self.assertEqual(clean['records'][0]['id'], 'safe')

    def test_provider_failure_is_not_completed(self):
        for terminal in ['Failed:ReconnectNeeded', 'Interrupted', 'Failed:ToolLimitReached']:
            self.assertFalse(wiki_eval.completed({'terminal': terminal, 'answer': 'plausible'}))
        self.assertTrue(wiki_eval.completed({'terminal': 'Completed', 'answer': 'saved'}))

    def test_protected_change_is_detected(self):
        before = [note('reference', '22 November', True)]
        for after in [note('reference', '15 November', True, 2),
                      note('reference', '22 November', False, 2),
                      note('reference', '22 November', True, 2, True)]:
            self.assertFalse(wiki_eval.protection_preserved(before, [after]))
        self.assertTrue(wiki_eval.protection_preserved(before, before))

    def test_applied_inputs_are_available_without_request_bodies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            db = sqlite3.connect(root / 'threads.sqlite3')
            db.executescript('CREATE TABLE operations(id TEXT, request TEXT); CREATE TABLE receipts(operation TEXT);')
            request = {'inputs': [{'record': 'source', 'version': 2},
                                  {'record': 'older', 'version': 1, 'expectation': 'Historical'}],
                       'writes': [{'id': 'private', 'data': {'Settings': {'auth_file': 'SECRET'}}}]}
            db.execute('INSERT INTO operations VALUES (?,?)', ('run:proposal', json.dumps(request)))
            db.execute('INSERT INTO receipts VALUES (?)', ('run:proposal',))
            db.commit()
            db.close()
            inspected = wiki_eval.read_operation_inputs(root, ['run:proposal'])
            self.assertEqual(inspected[0]['inputs'][0], {'record': 'source', 'version': 2, 'expectation': 'Current'})
            self.assertTrue(inspected[0]['applied'])
            self.assertEqual(inspected[0]['inputs'][1]['expectation'], 'Historical')
            self.assertNotIn('SECRET', json.dumps(inspected))
            self.assertNotIn('writes', inspected[0])

    def test_undo_requires_restoration_of_affected_content(self):
        original = note('changed', 'before')
        receipt = {'writes': [{'before': original, 'after': note('changed', 'after', version=2)}]}
        restored = [note('changed', 'before', version=3)]
        self.assertTrue(wiki_eval.verify_undo(receipt, restored))
        self.assertFalse(wiki_eval.verify_undo(receipt, [note('changed', 'after', version=3)]))

    def test_created_note_undo_requires_archival(self):
        created = note('created', 'new')
        receipt = {'writes': [{'before': None, 'after': created}]}
        self.assertFalse(wiki_eval.verify_undo(receipt, [created]))
        self.assertTrue(wiki_eval.verify_undo(receipt, [note('created', 'new', version=2, archived=True)]))

    def test_unrelated_records_must_survive_undo(self):
        before = [note('changed', 'after', version=2), note('unrelated', 'later human work', version=8)]
        restored = [note('changed', 'before', version=3), note('unrelated', 'lost', version=9)]
        self.assertFalse(wiki_eval.unrelated_preserved(before, restored, {'changed'}))
        restored[1] = before[1]
        self.assertTrue(wiki_eval.unrelated_preserved(before, restored, {'changed'}))


if __name__ == '__main__':
    unittest.main()
