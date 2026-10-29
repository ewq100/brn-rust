import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from score import WEIGHTS, assess, sha256
from reporting import compare, export_diffs, provenance
from evidence import collect


class ReportingTests(unittest.TestCase):
    def suite(self, worlds=('one', 'two', 'three')):
        return {'version': 1, 'weights': WEIGHTS, 'fixture_sha256': 'fixture', 'scenarios': [
            {'id': world + '-' + category, 'world': world, 'category': category, 'weight': 1,
             'repetitions': 1, 'criteria': ['quality'], 'coverage': 'partial'}
            for world in worlds for category in WEIGHTS]}

    def reports(self, suite):
        reports = []
        for config in ('A', 'B'):
            attempts, grades = [], []
            for case in suite['scenarios']:
                identity = config + '-' + case['id']
                attempts.append({'id': identity, 'configuration': config, 'scenario': case['id'],
                                 'repetition': 1, 'status': 'complete', 'result_sha256': identity})
                score = 2 if config == 'A' else {'one': 1, 'two': 3, 'three': 4}[case['world']]
                grades.append({'attempt': identity, 'result_sha256': identity, 'grader_revision': 'controlled',
                               'completed': True, 'breaches': [], 'criteria': {'quality': {
                                   'score': score, 'evidence': ['synthetic witness'], 'explanation': 'Controlled only'}}})
            reports.append(assess(suite, config, attempts, grades))
        return reports

    def test_paired_world_bootstrap_deterministic_and_not_fact_pseudoreplication(self):
        suite = self.suite()
        reports = self.reports(suite)
        result = compare(suite, reports, bootstrap_samples=500, seed=7)
        self.assertEqual(result, compare(suite, reports, bootstrap_samples=500, seed=7))
        pair = result['comparisons'][0]
        self.assertAlmostEqual(pair['overall_delta'], 50/3)
        self.assertEqual(pair['clusters'], 3)
        self.assertEqual(pair['uncertainty']['status'], 'pilot_interval')
        self.assertLessEqual(pair['uncertainty']['interval_95'][0], -25)
        self.assertGreaterEqual(pair['uncertainty']['interval_95'][1], 50)
        self.assertFalse(pair['eligible_for_selection'])
        self.assertFalse(result['qualified'])
        # Eight categories per world still constitute only three independent clusters.
        self.assertEqual(len(pair['scenario_deltas']), 24)

    def test_missing_slots_and_sparse_worlds_never_create_polished_comparison(self):
        suite = self.suite(('one',))
        result = compare(suite, self.reports(suite))['comparisons'][0]
        self.assertEqual(result['uncertainty']['status'], 'insufficient_clusters')
        self.assertIsNone(result['uncertainty']['interval_95'])
        suite = self.suite()
        reports = self.reports(suite)
        reports[1]['scenarios']['one-creation'] = None
        reports[1]['overall_quality'] = None
        pair = compare(suite, reports)['comparisons'][0]
        self.assertIsNone(pair['overall_delta'])
        self.assertEqual(pair['missing_pairs'], ['one-creation'])
        self.assertEqual(pair['uncertainty']['status'], 'incomplete_pairs')
        changed_grader = copy.deepcopy(reports)
        changed_grader[1]['grade_revisions'] = ['different-grader']
        with self.assertRaises(ValueError):
            compare(suite, changed_grader)
        changed_score = copy.deepcopy(reports)
        changed_score[0]['overall_quality'] = 99
        with self.assertRaises(ValueError):
            compare(suite, changed_score)
        altered = copy.deepcopy(suite)
        altered['scenarios'][0]['weight'] = 2
        with self.assertRaises(ValueError):
            compare(altered, reports)
        with self.assertRaises(ValueError):
            compare(suite, [reports[0], reports[0]])
        reports[0]['scenarios']['one-creation'] = float('nan')
        with self.assertRaises(ValueError):
            compare(suite, reports)

    def test_mixed_grader_revisions_cannot_hide_swapped_scenario_assignments(self):
        suite = self.suite()
        reports = self.reports(suite)
        # The same revision set does not prove each scenario used the same judge:
        # G1/G2 could have swapped assignments between configurations. Until we
        # expose per-slot grader bindings, comparisons admit exactly one revision.
        for revisions in (['G1', 'G2'], [], [''], ['G1', 'G1']):
            mixed = copy.deepcopy(reports)
            for report in mixed:
                report['grade_revisions'] = revisions
            with self.assertRaisesRegex(ValueError, 'one identical frozen grader revision'):
                compare(suite, mixed)
        self.assertEqual(compare(suite, reports)['comparisons'][0]['uncertainty']['status'], 'pilot_interval')

    def test_cluster_category_loss_is_not_silently_filtered(self):
        suite = self.suite()
        suite['scenarios'] = [c for c in suite['scenarios'] if c['category'] != 'creation' or c['world'] == 'one']
        pair = compare(suite, self.reports(suite), bootstrap_samples=100)['comparisons'][0]
        self.assertEqual(pair['uncertainty']['status'], 'incomplete_cluster_category_coverage')
        self.assertGreater(pair['uncertainty']['rejected_draws'], 0)
        self.assertIsNone(pair['uncertainty']['interval_95'])

    def test_explicit_provenance_fails_on_missing_files(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp)/'source'
            source.write_text('source one')
            first = provenance({'guidance': source}, {'revision': 'synthetic'})
            self.assertNotIn(temp, json.dumps(first))
            source.write_text('source two')
            self.assertNotEqual(first['identity'], provenance({'guidance': source}, {'revision': 'synthetic'})['identity'])
            with self.assertRaises(ValueError):
                provenance({'missing': Path(temp)/'absent'})

    def test_diffs_link_exact_versions_and_reject_tampering(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)/'experiment'
            folder = root/'runs/case-0/A-r1'
            folder.mkdir(parents=True)
            captures = []
            for step, text in enumerate(('Old exception.\n', 'New exception.\n')):
                checkpoint = folder/'checkpoints'/str(step)
                checkpoint.mkdir(parents=True)
                (checkpoint/'workspace.sqlite3').write_bytes(b'synthetic backup')
                (checkpoint/'records.json').write_text(json.dumps([
                    {'id': '<script>unsafe</script>', 'version': step+1,
                     'data': {'Note': {'title': 'Note', 'markdown': text}}}]))
                captures.append({'step': step, 'records_sha256': sha256(checkpoint/'records.json'),
                                 'backup_sha256': sha256(checkpoint/'workspace.sqlite3')})
            (root/'manifest.json').write_text(json.dumps({'fixture_sha256': 'fixture'}))
            suite_path = root/'suite.json'
            suite_path.write_text(json.dumps(self.suite()))
            (folder/'result.json').write_text(json.dumps({'case': 'one-creation', 'arm': 'A',
                'status': 'complete', 'repetition': 1, 'attempt': 1, 'steps': [], 'checkpoints': captures}))
            manifest = collect(root, suite_path)
            output = Path(temp)/'diffs'
            index = export_diffs(root, manifest, output)
            change = index['entries'][-1]
            self.assertEqual((change['from_version'], change['to_version']), (1, 2))
            exact = json.loads((output/change['artifact']).read_text())
            self.assertEqual(exact['before']['data']['Note']['markdown'], 'Old exception.\n')
            self.assertEqual(exact['to_records_sha256'], captures[1]['records_sha256'])
            self.assertIn('-Old exception.', (output/change['diff']).read_text())
            self.assertNotIn('<script>', (output/'index.html').read_text())
            with self.assertRaises(FileExistsError):
                export_diffs(root, manifest, output)
            (folder/'checkpoints/1/records.json').write_text('[]')
            with self.assertRaises(ValueError):
                export_diffs(root, manifest, Path(temp)/'tampered')
            self.assertFalse((Path(temp)/'tampered').exists())

    def test_efficiency_keeps_failed_attempt_costs_and_unknowns(self):
        suite = self.suite(('one',))
        attempts, grades = [], []
        for case in suite['scenarios']:
            identity = case['id']
            attempts.append({'id': identity, 'configuration': 'A', 'scenario': identity,
                             'repetition': 1, 'status': 'complete', 'result_sha256': identity,
                             'usage': {'cost': 2, 'elapsed_ms': 20}})
            grades.append({'attempt': identity, 'result_sha256': identity, 'grader_revision': 'controlled',
                           'completed': True, 'breaches': [], 'criteria': {'quality': {
                               'score': 4, 'evidence': ['fixture'], 'explanation': 'Synthetic witness'}}})
        attempts[0]['status'] = 'provider_transport'
        attempts.append(dict(attempts[0], id='retry', replaces=attempts[0]['id'], status='complete'))
        grades.append(dict(grades[0], attempt='retry'))
        report = assess(suite, 'A', attempts, grades)
        self.assertEqual(report['efficiency']['measured_cost_per_completed'], 18/8)
        self.assertEqual(report['failure_status_counts'], {'provider_transport': 1})
        attempts[0]['usage']['cost'] = None
        self.assertIsNone(assess(suite, 'A', attempts, grades)['efficiency']['measured_cost_per_completed'])
