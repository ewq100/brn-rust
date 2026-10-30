"""Authored witnesses only: no upstream corpus, shell checks or providers."""
import copy
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))
from external import (digest, memoryagentbench_native, clawarena_native,
                      adapted_checkpoint_gold)
import evaluate_wiki_v2 as native


def manifest(cases, adapter, **kwargs):
    return dict(origin='synthetic-authored', synthetic_only=True, allowed_use='synthetic-witness',
                adapter=adapter, revision='authored-schema-witness-1', authored_by='BRN tests',
                provenance='New fictional test records, no external corpus bytes',
                schema_reviewed_at='2026-10-29', raw_sha256=digest(cases), **kwargs)


def memory_cases():
    return [{'context': '1. The fictional Kestrel shelf holds blue cups.\n2. The Kestrel shelf now holds red cups.',
             'questions': ['What color are the cups?', 'Where are the cups?'],
             'answers': ['SECRET-GOLD-COLOR', ['SECRET-GOLD-PLACE']],
             'metadata': {'source': 'synthetic_factconsolidation',
                          'qa_pair_ids': ['private-id-1', 'private-id-2'],
                          'question_dates': ['2026-01-03', '2026-01-04'],
                          'has_answer': 'PRIVATE-LABEL'}}]


def claw_cases():
    return [{'scenario_id': 'private-claw-id', 'source_family': 'authored-larch',
             'initial_documents': [{'id': 'source-1', 'date': '2026-01-01', 'text': 'The desk is blue.'}],
             'updates': {'update-1': [{'id': 'source-2', 'date': '2026-01-02', 'text': 'The desk is now green.'}]},
             'rounds': [
                 {'id': 'private-round-1', 'type': 'multi_choice', 'question': 'Which color is current?',
                  'eval': {'options': {'A': 'Blue', 'B': 'Green'}, 'answer': ['A']},
                  'feedback': {'correct': 'PRIVATE-GOLD-FEEDBACK'}, 'update_ids': []},
                 {'id': 'private-round-2', 'type': 'multi_choice', 'question': 'Which color is current?',
                  'eval': {'options': {'A': 'Blue', 'B': 'Green'}, 'answer': ['B']},
                  'update_ids': ['update-1']}]}]


class ExternalSchemaTests(unittest.TestCase):
    def test_memory_full_context_all_queries_and_private_labels(self):
        cases = memory_cases()
        fixture, gold, locks = memoryagentbench_native(cases, manifest(cases, 'memoryagentbench',
            component='FactConsolidation', source='synthetic_factconsolidation'))
        self.assertEqual(native.validate_fixture(fixture), [])
        steps = fixture['cases'][0]['steps']
        self.assertEqual(len(steps), 3)
        self.assertIn(cases[0]['context'], steps[0]['text'])
        self.assertIn('larger serial number', steps[1]['prompt'])
        self.assertIn('2026-01-03', steps[1]['prompt'])
        encoded = json.dumps(fixture)
        for secret in ('SECRET-GOLD', 'private-id', 'PRIVATE-LABEL'):
            self.assertNotIn(secret, encoded)
        self.assertEqual(gold[0]['checkpoints'][0]['answer'], 'SECRET-GOLD-COLOR')
        self.assertEqual(locks[0]['raw_sha256'], digest(cases[0]))
        self.assertEqual(locks[0]['transformed_sha256'], digest(fixture['cases'][0]))
        self.assertFalse(locks[0]['official_score'])
        one = adapted_checkpoint_gold(gold, fixture['cases'][0]['id'], 2)
        self.assertEqual(len(one['questions']), 1)
        self.assertNotIn('SECRET-GOLD-PLACE', json.dumps(one))

    def test_single_question_alternatives_are_not_truncated(self):
        cases = memory_cases()
        cases[0]['questions'] = ['Where are the cups?']
        cases[0]['answers'] = ['on the shelf', 'the Kestrel shelf']
        cases[0]['metadata']['qa_pair_ids'] = ['private-id-1']
        cases[0]['metadata']['question_dates'] = ['2026-01-04']
        fixture, gold, _ = memoryagentbench_native(cases, manifest(cases, 'memoryagentbench',
            component='FactConsolidation', source='synthetic_factconsolidation'))
        self.assertEqual(gold[0]['checkpoints'][0]['answer'], cases[0]['answers'])
        with self.assertRaises(ValueError):
            adapted_checkpoint_gold(gold, fixture['cases'][0]['id'], 3)

    def test_memory_eventqa_rule_and_shared_context_split(self):
        cases = memory_cases()
        second = copy.deepcopy(cases[0])
        second['metadata']['qa_pair_ids'] = ['private-3', 'private-4']
        cases.append(second)
        fixture, _, locks = memoryagentbench_native(cases, manifest(cases, 'memoryagentbench',
            component='EventQA', source='synthetic_factconsolidation'))
        self.assertIn('next event', fixture['cases'][0]['steps'][1]['prompt'])
        self.assertEqual(locks[0]['split_group'], locks[1]['split_group'])
        self.assertNotEqual(fixture['cases'][0]['id'], fixture['cases'][1]['id'])

    def test_memory_refuses_misalignment_duplicates_unknown_components_and_real_admission(self):
        for kind in ('short-answers', 'short-dates', 'duplicate-id', 'wrong-source'):
            cases = memory_cases()
            if kind == 'short-answers': cases[0]['answers'].pop()
            if kind == 'short-dates': cases[0]['metadata']['question_dates'].pop()
            if kind == 'duplicate-id': cases[0]['metadata']['qa_pair_ids'][1] = 'private-id-1'
            if kind == 'wrong-source': cases[0]['metadata']['source'] = 'another'
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                memoryagentbench_native(cases, manifest(cases, 'memoryagentbench',
                    component='FactConsolidation', source='synthetic_factconsolidation'))
        cases = memory_cases()
        for change in ({'component': 'LoCoMo'}, {'allowed_use': 'approved'}, {'synthetic_only': False}, {'raw_sha256': 'wrong'}):
            m = manifest(cases, 'memoryagentbench', component='FactConsolidation', source='synthetic_factconsolidation')
            m.update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                memoryagentbench_native(cases, m)

    def test_claw_updates_happen_before_the_right_round_and_gold_is_private(self):
        cases = claw_cases()
        fixture, gold, locks = clawarena_native(cases, manifest(cases, 'clawarena', asset_contract='brn-normalized-text-v1'))
        self.assertEqual(native.validate_fixture(fixture), [])
        steps = fixture['cases'][0]['steps']
        self.assertEqual([s['kind'] for s in steps], ['intake', 'ask', 'intake', 'ask'])
        self.assertIn('now green', steps[2]['text'])
        self.assertNotIn('now green', json.dumps(steps[:2]))
        self.assertNotIn('PRIVATE-GOLD', json.dumps(fixture))
        self.assertNotIn('private-claw-id', json.dumps(fixture))
        self.assertNotIn('private-round', json.dumps(fixture))
        checkpoint = adapted_checkpoint_gold(gold, fixture['cases'][0]['id'], 2)
        self.assertEqual(checkpoint['questions'][0]['answer'], ['A'])
        self.assertEqual(checkpoint['questions'][0]['available_updates'], [])
        self.assertEqual(gold[0]['checkpoints'][1]['answer'], ['B'])
        self.assertEqual(locks[0]['native_fixture_sha256'], digest(fixture))

    def test_claw_refuses_shell_checks_missing_unused_and_replayed_updates(self):
        for kind in ('shell', 'pref', 'missing', 'unused', 'replay', 'bad-answer', 'duplicate-round'):
            cases = claw_cases()
            rounds = cases[0]['rounds']
            if kind == 'shell':
                rounds[0]['type'] = 'exec_check'
                rounds[0]['eval'] = {'command': 'touch /tmp/SHOULD-NEVER-RUN'}
            elif kind == 'pref': rounds[0]['pref'] = {'command': 'do not execute'}
            elif kind == 'missing': rounds[1]['update_ids'] = ['unknown']
            elif kind == 'unused': rounds[1]['update_ids'] = []
            elif kind == 'replay': rounds[1]['update_ids'] = ['update-1', 'update-1']
            elif kind == 'bad-answer': rounds[0]['eval']['answer'] = ['Z']
            else: rounds[1]['id'] = rounds[0]['id']
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                clawarena_native(cases, manifest(cases, 'clawarena', asset_contract='brn-normalized-text-v1'))

    def test_claw_shared_document_connects_distinct_families(self):
        cases = claw_cases()
        other = copy.deepcopy(cases[0])
        other.update(scenario_id='other-private', source_family='different-family')
        cases.append(other)
        _, _, locks = clawarena_native(cases, manifest(cases, 'clawarena', asset_contract='brn-normalized-text-v1'))
        self.assertEqual(locks[0]['split_group'], locks[1]['split_group'])
