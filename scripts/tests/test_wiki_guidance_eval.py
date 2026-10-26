import importlib.util
import json
from pathlib import Path
import ast
import copy
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
        class StubCli:
            def __init__(self):
                self.calls = []
            def envelope(self, command, **options):
                self.calls.append((command, options))
                identity = options['id']
                if identity == 'missing':
                    return {'status': 'rejected', 'result': None, 'error': {'code': 'not_found'}}
                if identity == 'allocated':
                    return {'status': 'ok', 'error': None,
                            'result': {'state': 'allocated', 'inputs': [], 'inputs_total': 0, 'writes': []}}
                return {'status': 'ok', 'error': None, 'result': {
                    'state': 'applied', 'inputs_total': 2,
                    'inputs': [{'record': 'source', 'version': 2, 'expectation': 'Current'},
                               {'record': 'older', 'version': 1, 'expectation': 'Historical'}],
                    'writes': [{'id': 'private', 'kind': 'Settings', 'auth_file': 'SECRET'}]}}
        cli = StubCli()
        inspected = wiki_eval.read_operation_inputs(cli, ['run:proposal', 'missing', 'allocated'])
        self.assertEqual([call[0] for call in cli.calls], ['operation'] * 3)
        self.assertEqual(len(inspected), 1)
        self.assertEqual(inspected[0]['inputs'][0], {'record': 'source', 'version': 2, 'expectation': 'Current'})
        self.assertTrue(inspected[0]['applied'])
        self.assertEqual(inspected[0]['inputs'][1]['expectation'], 'Historical')
        self.assertNotIn('SECRET', json.dumps(inspected))
        self.assertNotIn('writes', inspected[0])

    def test_truncated_operation_inputs_fail_loudly(self):
        class StubCli:
            def envelope(self, command, **options):
                return {'status': 'ok', 'error': None, 'result': {
                    'state': 'prepared', 'inputs_total': 65,
                    'inputs': [{'record': f'r{i}', 'version': 1, 'expectation': 'Current'} for i in range(64)]}}
        with self.assertRaises(SystemExit) as raised:
            wiki_eval.read_operation_inputs(StubCli(), ['op'])
        self.assertIn('inputs truncated (64/65)', str(raised.exception))

    def test_evaluator_does_not_import_sqlite3(self):
        tree = ast.parse(SCRIPT.read_text())
        imported = {alias.name for node in ast.walk(tree) if isinstance(node, (ast.Import, ast.ImportFrom))
                    for alias in node.names}
        modules = {node.module for node in ast.walk(tree) if isinstance(node, ast.ImportFrom)}
        self.assertNotIn('sqlite3', imported | modules)

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
            def __init__(self, *args, stock_helper=None, brn_helper=None):
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


    def test_live_codex_needs_an_absolute_auth_file(self):
        executable = Path(__import__('sys').executable)
        argv = ['evaluate-wiki-guidance.py', '--baseline-bin', str(executable), '--wiki-bin', str(executable),
                '--helper', str(executable), '--root', '/nonexistent/eval-root', '--baseline-commit', 'a',
                '--candidate-commit', 'b', '--live', '--provider', 'Codex', '--model', 'm', '--effort', 'low']
        stderr = __import__('io').StringIO()
        with patch('sys.argv', argv), patch('sys.stderr', stderr), self.assertRaises(SystemExit) as raised:
            wiki_eval.main()
        self.assertEqual(raised.exception.code, 2)
        self.assertIn('Codex needs an absolute --auth-file', stderr.getvalue())
class MeetingActionOracleTests(unittest.TestCase):
    def record(self, identity, **fields):
        return {'id': identity, 'archived': False, 'data': {'Action': fields}}

    def candidate(self, identity, **fields):
        return {'request': {'writes': [self.record(identity, **fields)]}}

    def with_sources(self, after):
        request = after['candidates'][0]['request']
        action_id = request['writes'][0]['id']
        request['inputs'] = []
        for step in [0, 1]:
            identity = f'source:{step}'
            after['records'].append({'id': identity, 'version': 1, 'archived': False,
                                     'data': {'Source': {'locator': f'/fixture/update-{step}.md'}}})
            request['inputs'].append({'record': identity, 'version': 1})
            request['writes'].append({'id': f'link:{step}', 'archived': False,
                                     'data': {'Link': {'from': action_id, 'to': identity, 'relation': 'source'}}})
        return after

    def test_followup_only_intake_accepts_pending_action_without_inventing_a_note_requirement(self):
        cases = json.loads(wiki_eval.CASES.read_text())['cases']
        case = next(case for case in cases if case['id'] == 'meeting-duplicate-notes-one-followup')
        action = dict(state='Open', owner='Mari', follow_up='2028-03-03', evidence=None)
        after = self.with_sources({'records': [], 'receipts': [], 'candidates': [self.candidate('one', **action)]})
        before = {'records': []}
        self.assertTrue(all(wiki_eval.structural_checks(case, before, after).values()))
        after['candidates'][0]['request']['writes'][0]['data']['Action']['owner'] = None
        self.assertFalse(wiki_eval.structural_checks(case, before, after)['action_facts_including_pending'])
        after['candidates'] = []
        self.assertFalse(wiki_eval.structural_checks(case, before, after)['action_count_including_pending'])
        after['records'] = [self.record('one', **action)]
        self.assertFalse(wiki_eval.structural_checks(case, before, after)['saved_action_facts'])

    def test_source_grounding_cannot_fill_completion_evidence_in_pending_action(self):
        cases = json.loads(wiki_eval.CASES.read_text())['cases']
        case = next(case for case in cases if case['id'] == 'meeting-duplicate-notes-one-followup')
        action = dict(state='Open', owner='Mari', follow_up='2028-03-03', evidence='source:2')
        candidate = self.candidate('one', **action)
        candidate['request']['inputs'] = [{'record': 'source:2', 'version': 1}]
        candidate['request']['writes'].append({
            'id': 'link:1', 'archived': False,
            'data': {'Link': {'from': 'one', 'to': 'source:2', 'relation': 'source'}}})
        after = {'records': [], 'receipts': [], 'candidates': [candidate]}
        checks = wiki_eval.structural_checks(case, {'records': []}, after)
        self.assertTrue(checks['action_count_including_pending'])
        self.assertFalse(checks['action_facts_including_pending'])
        # Complete provenance cannot substitute for absent completion evidence.
        self.with_sources(after)
        grounded = wiki_eval.structural_checks(case, {'records': []}, after)
        self.assertTrue(grounded['action_source_links'])
        self.assertTrue(grounded['action_source_inputs'])
        self.assertFalse(grounded['action_facts_including_pending'])
        candidate['request']['writes'][0]['data']['Action']['evidence'] = None
        self.assertTrue(all(wiki_eval.structural_checks(case, {'records': []}, after).values()))
        self.assertEqual(len(candidate['request']['inputs']), 2)

    def test_published_duplicate_receipt_requires_second_source_on_active_request(self):
        receipt = json.loads((SCRIPT.parents[1] / 'docs/work/active/daily-followups-inbox-restore/evidence/mac-completion-20261025/receipt.json').read_text())
        after = receipt['steps'][-1]['snapshot']
        checks = wiki_eval.action_source_checks([0, 1], after)
        self.assertTrue(checks['action_sources_resolved'])
        self.assertFalse(checks['action_source_links'])
        self.assertFalse(checks['action_source_inputs'])
        fixed = copy.deepcopy(after)
        request = fixed['candidates'][0]['request']
        action_id = next(w['id'] for w in request['writes'] if 'Action' in w['data'])
        missing = receipt['reviews'][-1]['missing_provenance'][0]
        request['inputs'].append({'record': missing['id'], 'version': missing['version']})
        request['writes'].append({'id': 'synthetic-second-link', 'archived': False,
                                 'data': {'Link': {'from': action_id, 'to': missing['id'], 'relation': 'source'}}})
        self.assertTrue(all(wiki_eval.action_source_checks([0, 1], fixed).values()))
        case = next(c for c in json.loads(wiki_eval.CASES.read_text())['cases']
                    if c['id'] == 'meeting-duplicate-notes-one-followup')
        self.assertTrue(all(wiki_eval.structural_checks(case, {'records': []}, fixed).values()))
        self.assertFalse(all(wiki_eval.structural_checks(case, {'records': []}, after).values()))
        # The published failure is immutable; this is a synthetic oracle witness.
        self.assertEqual(after, receipt['steps'][-1]['snapshot'])
        for mutation in ['stale-version', 'context-only', 'null-expectation', 'empty-expectation', 'wrong-relation', 'wrong-action', 'archived-link', 'missing-source', 'ambiguous-source', 'historical-only']:
            with self.subTest(mutation=mutation):
                bad = copy.deepcopy(fixed)
                pending = bad['candidates'][0]['request']
                if mutation == 'stale-version': pending['inputs'][-1]['version'] += 1
                elif mutation == 'context-only': pending['inputs'][-1]['expectation'] = 'ChatContext'
                elif mutation == 'null-expectation': pending['inputs'][-1]['expectation'] = None
                elif mutation == 'empty-expectation': pending['inputs'][-1]['expectation'] = ''
                elif mutation == 'wrong-relation': pending['writes'][-1]['data']['Link']['relation'] = 'related'
                elif mutation == 'wrong-action': pending['writes'][-1]['data']['Link']['from'] = 'other-action'
                elif mutation == 'archived-link': pending['writes'][-1]['archived'] = True
                elif mutation == 'missing-source': bad['records'] = [r for r in bad['records'] if r['id'] != missing['id']]
                elif mutation == 'ambiguous-source': bad['records'].append(copy.deepcopy(next(r for r in bad['records'] if r['id'] == missing['id'])))
                else:
                    bad['lineage'].append({'operation': 'retired', 'applied': False, 'inputs': pending['inputs']})
                    pending['inputs'] = pending['inputs'][:-1]
                self.assertFalse(all(wiki_eval.action_source_checks([0, 1], bad).values()))
        self.assertFalse(all(wiki_eval.action_source_checks([0, 0], fixed).values()))

    def test_unavailable_candidates_leave_provenance_unverified_not_passing(self):
        case = next(c for c in json.loads(wiki_eval.CASES.read_text())['cases']
                    if c['id'] == 'meeting-duplicate-notes-one-followup')
        for unavailable in [None, {}, 'unsupported']:
            with self.subTest(candidates=unavailable):
                after = {'records': [], 'receipts': [], 'candidates': unavailable}
                checks = wiki_eval.structural_checks(case, {'records': []}, after)
                self.assertIsNone(checks['action_evidence_available'])
                for name in ['action_sources_resolved', 'action_source_links', 'action_source_inputs']:
                    self.assertIsNone(checks[name])
                problem = wiki_eval.verification_problem(checks, {'status': 'not_run'})
                self.assertIn('action_source_inputs', problem)
        self.assertTrue(all(value is None for value in
                            wiki_eval.action_source_checks([0, 1], {'records': []}).values()))

    def test_pending_update_reuses_saved_links_but_honors_candidate_overrides(self):
        after = self.with_sources({'records': [], 'candidates': [self.candidate('one', state='Suggested')]})
        request = after['candidates'][0]['request']
        saved_links = copy.deepcopy(request['writes'][1:])
        after['records'].extend(saved_links)
        after['records'].append(dict(self.record('one', state='Suggested'), version=1))
        request['writes'] = request['writes'][:1]
        self.assertTrue(all(wiki_eval.action_source_checks([0, 1], after).values()))
        for mutation in ['archive', 'retarget', 'missing-current-input']:
            with self.subTest(mutation=mutation):
                bad = copy.deepcopy(after)
                pending = bad['candidates'][0]['request']
                if mutation == 'missing-current-input':
                    bad['lineage'] = [{'operation': 'old', 'applied': True, 'inputs': pending['inputs']}]
                    pending['inputs'] = []
                    self.assertFalse(wiki_eval.action_source_checks([0, 1], bad)['action_source_inputs'])
                else:
                    override = copy.deepcopy(saved_links[0])
                    if mutation == 'archive': override['archived'] = True
                    else: override['data']['Link']['to'] = 'unrelated-source'
                    pending['writes'].append(override)
                    self.assertFalse(wiki_eval.action_source_checks([0, 1], bad)['action_source_links'])

    def test_saved_action_provenance_requires_its_current_receipt(self):
        after = self.with_sources({'records': [], 'candidates': [self.candidate('one', state='Suggested')]})
        request = after['candidates'].pop()['request']
        action = request['writes'][0]
        action['version'] = 2
        after['records'].extend(request['writes'])
        after['receipts'] = [{'operation': 'accepted', 'writes': [{'after': action}]}]
        after['lineage'] = [{'operation': 'accepted', 'applied': True, 'inputs': request['inputs']}]
        self.assertTrue(all(wiki_eval.action_source_checks([0, 1], after).values()))
        after['receipts'][0]['writes'][0]['after'] = dict(action, version=1)
        self.assertFalse(wiki_eval.action_source_checks([0, 1], after)['action_source_inputs'])

    def test_saved_action_source_metadata_write_uses_same_receipt_before_version(self):
        after = self.with_sources({'records': [], 'candidates': [self.candidate('one', state='Suggested')]})
        request = after['candidates'].pop()['request']
        action = dict(request['writes'][0], version=1)
        after['records'].extend([action] + request['writes'][1:])
        source = next(r for r in after['records'] if r['id'] == 'source:0')
        before = copy.deepcopy(source)
        source['version'] = 2
        source['data']['Source']['outcome'] = 'Recorded'
        after['receipts'] = [{'operation': 'accepted', 'writes': [
            {'before': None, 'after': copy.deepcopy(action)},
            {'before': before, 'after': copy.deepcopy(source)}]}]
        after['lineage'] = [{'operation': 'accepted', 'applied': True, 'inputs': request['inputs']}]
        self.assertTrue(all(wiki_eval.action_source_checks([0, 1], after).values()))
        for mutation in ['new-version', 'wrong-before-version', 'wrong-before-id', 'wrong-before-type',
                         'archived-before', 'other-operation', 'later-version', 'later-body', 'ambiguous-receipt']:
            with self.subTest(mutation=mutation):
                bad = copy.deepcopy(after)
                write = bad['receipts'][0]['writes'][1]
                if mutation == 'new-version': bad['lineage'][0]['inputs'][0]['version'] = 2
                elif mutation == 'wrong-before-version': write['before']['version'] = 2
                elif mutation == 'wrong-before-id': write['before']['id'] = 'different-source'
                elif mutation == 'wrong-before-type': write['before']['data'] = {'Note': {}}
                elif mutation == 'archived-before': write['before']['archived'] = True
                elif mutation == 'other-operation':
                    bad['receipts'][0]['writes'].pop()
                    bad['receipts'].append({'operation': 'unrelated', 'writes': [write]})
                elif mutation == 'later-version': bad['records'][0]['version'] = 3
                elif mutation == 'later-body': bad['records'][0]['data']['Source']['outcome'] = 'Changed later'
                else: bad['receipts'].append(copy.deepcopy(bad['receipts'][0]))
                self.assertFalse(wiki_eval.action_source_checks([0, 1], bad)['action_source_inputs'])

    def test_explicit_meeting_account_request_still_requires_an_applied_note(self):
        cases = json.loads(wiki_eval.CASES.read_text())['cases']
        case = next(case for case in cases if case['id'] == 'meeting-clarification-unknown-owner')
        after = {'records': [], 'receipts': [], 'candidates': [self.candidate(
            'one', state='Suggested', owner=None, follow_up=None, due=None, evidence=None)]}
        checks = wiki_eval.structural_checks(case, {'records': []}, after)
        self.assertFalse(checks['knowledge_change_applied'])
        self.assertTrue(checks['action_count_including_pending'])

    def test_pending_duplicate_is_counted_and_bad_facts_cannot_hide_in_review(self):
        expected = {'min': 1, 'max': 1, 'fields': {'owner': None, 'follow_up': None}}
        after = {'records': [self.record('one')], 'candidates': [self.candidate('two', owner='Invented', follow_up='2028-03-03')]}
        checks = wiki_eval.action_checks(expected, after)
        self.assertFalse(checks['action_count_including_pending'])
        self.assertFalse(checks['action_facts_including_pending'])

    def test_pending_update_does_not_double_count_same_identity_but_checks_after_fields(self):
        expected = {'min': 1, 'max': 1, 'fields': {'owner': None}}
        after = {'records': [self.record('one')], 'candidates': [self.candidate('one', owner='Invented')]}
        checks = wiki_eval.action_checks(expected, after)
        self.assertTrue(checks['action_count_including_pending'])
        self.assertFalse(checks['action_facts_including_pending'])
        after['candidates'] = [self.candidate('one', owner=None)]
        self.assertTrue(all(wiki_eval.action_checks(expected, after).values()))

    def test_missing_action_or_missing_pending_evidence_cannot_pass_required_followup(self):
        expected = {'min': 1, 'max': 1, 'fields': {'owner': None}}
        checks = wiki_eval.action_checks(expected, {'records': [], 'candidates': []})
        self.assertFalse(checks['action_count_including_pending'])
        self.assertIsNone(wiki_eval.action_checks(expected, {'records': []})['action_evidence_available'])

    def test_pending_open_proposal_is_not_misclassified_as_completed_work(self):
        expected = {'min': 1, 'max': 1, 'states': ['Suggested', 'Open', 'Waiting'],
                    'saved_fields': {'state': 'Suggested'}}
        after = {'records': [], 'candidates': [self.candidate('one', state='Open')]}
        self.assertTrue(all(wiki_eval.action_checks(expected, after).values()))
        accepted = {'records': [self.record('one', state='Open')], 'candidates': []}
        self.assertFalse(wiki_eval.action_checks(expected, accepted)['saved_action_facts'])
        after['candidates'][0]['request']['writes'][0]['data']['Action']['state'] = 'Done'
        self.assertFalse(wiki_eval.action_checks(expected, after)['action_states_including_pending'])

    def test_proposed_reassignment_is_distinct_from_silently_changed_saved_owner(self):
        expected = {'min': 1, 'max': 1, 'saved_fields': {'owner': 'Mari'}}
        after = {'records': [self.record('one', owner='Mari')], 'candidates': [self.candidate('one', owner='Leena')]}
        self.assertTrue(wiki_eval.action_checks(expected, after)['saved_action_facts'])
        after['records'][0]['data']['Action']['owner'] = 'Leena'
        self.assertFalse(wiki_eval.action_checks(expected, after)['saved_action_facts'])

    def test_authored_meeting_cases_have_explicit_independent_oracles(self):
        cases = json.loads(wiki_eval.CASES.read_text())['cases']
        meetings = [case for case in cases if case['id'].startswith('meeting-')]
        self.assertEqual(len(meetings), 4)
        self.assertTrue(all('action_expectation' in case and case['rubric'] for case in meetings))
        self.assertEqual(next(case for case in meetings if 'duplicate' in case['id'])['action_expectation']['max'], 1)


if __name__ == '__main__':
    unittest.main()
