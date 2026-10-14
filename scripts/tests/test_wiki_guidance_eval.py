import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

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

    def test_redaction_preserves_schema_nulls_for_failed_run_inspection(self):
        original = note('working', 'before')
        clean = wiki_eval.public_snapshot(original)
        self.assertIsNone(clean['data']['Note']['superseded_by'])
        self.assertEqual(wiki_eval.saved_notes([clean]), [clean])
        before = {'records': [clean]}
        after = {'records': [clean], 'receipts': []}
        case = {'id': 'sequential-project', 'no_new_notes': False, 'expected_existing_note': 'working'}
        checks = wiki_eval.structural_checks(case, before, after)
        self.assertFalse(checks['knowledge_change_applied'])
        self.assertTrue(checks['protected_references_preserved'])

    def test_provider_failure_is_not_completed(self):
        for terminal in ['Failed:ReconnectNeeded', 'Interrupted', 'Failed:ToolLimitReached']:
            self.assertFalse(wiki_eval.completed({'terminal': terminal, 'answer': 'plausible'}))
        self.assertTrue(wiki_eval.completed({'terminal': 'Completed', 'answer': 'saved'}))

    def test_codex_auth_file_is_not_cleared_by_cache_selection(self):
        options = wiki_eval.credential_options('Codex', Path('/selected/auth.json'), Path('/unused/cache'))
        self.assertEqual(set(options), {'auth_file'})
        self.assertEqual(wiki_eval.credential_options('Codex', None, None), {})
        self.assertEqual(wiki_eval.credential_options('Copilot', None, Path('/selected/cache')),
                         {'credentials_dir': Path('/selected/cache')})

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

    def test_failed_assertions_or_undo_cannot_count_as_verified(self):
        applied = {'status': 'Applied', 'affected_restored': True, 'unrelated_preserved': True}
        self.assertIsNone(wiki_eval.verification_problem({'protected': True}, applied))
        self.assertIsNone(wiki_eval.verification_problem({}, {'status': 'not_run'}))
        self.assertIsNotNone(wiki_eval.verification_problem({'protected': False}, applied))
        self.assertIsNotNone(wiki_eval.verification_problem({}, dict(applied, affected_restored=False)))
        self.assertIsNotNone(wiki_eval.verification_problem({}, dict(applied, unrelated_preserved=False)))
        self.assertIsNotNone(wiki_eval.verification_problem({}, {'status': 'NeedsReview'}))

    def test_unrelated_records_must_survive_undo(self):
        before = [note('changed', 'after', version=2), note('unrelated', 'later human work', version=8)]
        restored = [note('changed', 'before', version=3), note('unrelated', 'lost', version=9)]
        self.assertFalse(wiki_eval.unrelated_preserved(before, restored, {'changed'}))
        restored[1] = before[1]
        self.assertTrue(wiki_eval.unrelated_preserved(before, restored, {'changed'}))

    def test_disconnected_summaries_do_not_pass_synthesis_lineage(self):
        first, second = note('first', 'process A', True), note('second', 'process B', True)
        notes = [first, second, note('summary-a', 'A only'), note('summary-b', 'B only')]
        links = [{'id': 'link-a', 'archived': False, 'data': {'Link': {'from': 'summary-a', 'to': 'first'}}},
                 {'id': 'link-b', 'archived': False, 'data': {'Link': {'from': 'summary-b', 'to': 'second'}}}]
        after = {'records': notes + links,
                 'receipts': [{'operation': 'a', 'writes': [{'after': notes[2]}]},
                              {'operation': 'b', 'writes': [{'after': notes[3]}]}],
                 'lineage': [{'operation': 'a', 'applied': True, 'inputs': [{'record': 'first', 'version': 1}]},
                             {'operation': 'b', 'applied': True, 'inputs': [{'record': 'second', 'version': 1}]},
                             {'operation': 'unrelated-link-only', 'applied': True,
                              'inputs': [{'record': 'first', 'version': 1}, {'record': 'second', 'version': 1}]}]}
        case = {'id': 'reusable-synthesis', 'no_new_notes': False, 'expected_existing_note': None}
        checks = wiki_eval.structural_checks(case, {'records': [first, second]}, after)
        self.assertFalse(checks['both_reference_notes_declared_as_inputs'])
        self.assertFalse(checks['both_reference_notes_linked_to_synthesis'])

    def test_late_undo_failure_retains_the_completed_provider_and_snapshot(self):
        class FakeCli:
            def __init__(self, *args):
                self.commands = []
            def call(self, command, **options):
                self.commands.append({'command': command, 'seconds': 0})
                return {'id': 'thread', 'terminal': 'Completed', 'answer': '102'}
        case = {'id': 'conversation-only', 'seeds': [], 'steps': [{'kind': 'ask', 'prompt': '17 * 6'}],
                'rubric': ['102'], 'no_new_notes': True, 'expected_existing_note': None}
        saved = {'records': [], 'receipts': [], 'candidates': [], 'lineage': []}
        args = SimpleNamespace(live=True, helper=Path('/helper'), timeout=1, provider='Codex',
                               auth_file=None, credentials_dir=None, filler=0)
        with tempfile.TemporaryDirectory() as directory, patch.object(wiki_eval, 'Cli', FakeCli), \
                patch.object(wiki_eval, 'snapshot', return_value=saved), \
                patch.object(wiki_eval, 'undo_latest_note_change', side_effect=wiki_eval.CallFailed('undo failed')):
            result = wiki_eval.run_arm(Path('/binary'), Path(directory), Path(directory)/'arm', case,
                                      {'provider': 'Codex'}, args)
        self.assertEqual(result['outcomes'][0]['answer'], '102')
        self.assertEqual(result['after'], saved)
        self.assertTrue(result['provider_completed'])
        self.assertFalse(result['completed'])
        self.assertEqual(result['verification_failure'], 'undo failed')


if __name__ == '__main__':
    unittest.main()
