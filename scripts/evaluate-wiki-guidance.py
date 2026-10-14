#!/usr/bin/env python3
"""Compare two compiled BRN guidance bundles in exclusively owned synthetic state."""
import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import time

CASES = Path(__file__).resolve().parent / 'fixtures/wiki-guidance-cases.json'
PRIVATE_KEYS = {'auth_file', 'credentials_dir', 'access_token', 'refresh_token'}
OMIT = object()


def create_root(root):
    if not root.is_absolute():
        raise ValueError('Use an absolute fresh root')
    root.mkdir(mode=0o700)  # Never adopt an occupied directory or symlink.
    (root / 'EVALUATION-OWNED').write_text('Synthetic BRN wiki comparison only\n')


def public_snapshot(value):
    if isinstance(value, dict):
        if 'Settings' in value.get('data', {}):
            return OMIT
        return {key: clean for key, item in value.items()
                if key not in PRIVATE_KEYS and key != 'Settings'
                and (clean := public_snapshot(item)) is not OMIT}
    if isinstance(value, list):
        return [clean for item in value if (clean := public_snapshot(item)) is not OMIT]
    return value


def completed(outcome):
    return outcome.get('terminal') == 'Completed'


def credential_options(provider, auth_file, credentials_dir):
    # CLI configure chooses exactly one route; a credentials-dir clears auth-file.
    if provider == 'Codex':
        return {'auth_file': auth_file} if auth_file else {}
    return {'credentials_dir': credentials_dir}


def by_id(records):
    return {record['id']: record for record in records}


def protection_preserved(before, after):
    current = by_id(after)
    for record in before:
        note = record.get('data', {}).get('Note')
        if note and note['protected']:
            updated = current.get(record['id'])
            if not updated or updated['archived'] != record['archived'] or updated['data'] != record['data']:
                return False
    return True


def verify_undo(receipt, records):
    current = by_id(records)
    for write in receipt['writes']:
        prior, accepted = write.get('before'), write['after']
        result = current.get(accepted['id'])
        if not result:
            return False
        if prior is None:
            if not result['archived']:
                return False
        elif result['data'] != prior['data'] or result['archived'] != prior['archived']:
            return False
    return True


def unrelated_preserved(before, after, affected):
    current = by_id(after)
    return all(current.get(record['id']) == record for record in before if record['id'] not in affected)


def read_operation_inputs(data_dir, operation_ids):
    # The caller supplies only data under this run's exclusively created root.
    # Inspect accepted/pending request lineage; never emit complete request bodies.
    path = (data_dir / 'threads.sqlite3').resolve()
    output = []
    with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as database:
        for identity in sorted(set(operation_ids)):
            row = database.execute('SELECT request FROM operations WHERE id=?', (identity,)).fetchone()
            if not row or not row[0]:
                continue
            request = json.loads(row[0])
            inputs = [{'record': item['record'], 'version': item['version'],
                       'expectation': item.get('expectation', 'Current')}
                      for item in request.get('inputs', [])]
            applied = database.execute('SELECT 1 FROM receipts WHERE operation=?', (identity,)).fetchone() is not None
            output.append({'operation': identity, 'applied': applied, 'inputs': inputs})
    return output


class CallFailed(Exception):
    pass


class Cli:
    def __init__(self, binary, data, owned_root, helper, timeout=180):
        self.binary, self.data, self.timeout = binary, data, timeout
        self.commands = []
        parent = owned_root.resolve()
        if not (parent / 'EVALUATION-OWNED').is_file() or not data.resolve().is_relative_to(parent):
            raise ValueError('CLI data must belong to this exclusively created evaluation root')
        self.environment = dict(os.environ, BRN_INTAKE_HELPER=str(helper))

    def call(self, command, **options):
        args = [str(self.binary), '--data-dir', str(self.data), command]
        for key, value in options.items():
            args.extend(['--' + key.replace('_', '-'), str(value)])
        started = time.monotonic()
        process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   text=True, env=self.environment, start_new_session=True)
        try:
            stdout, _ = process.communicate(timeout=self.timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGINT)
            try:
                process.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate()
            self.commands.append({'command': command, 'seconds': round(time.monotonic() - started, 3), 'status': 'timeout'})
            raise CallFailed(f'{command}: timeout') from None
        observation = {'command': command, 'seconds': round(time.monotonic() - started, 3), 'exit': process.returncode}
        self.commands.append(observation)
        if process.returncode:
            observation['status'] = 'failed'
            raise CallFailed(f'{command}: exit {process.returncode}')
        try:
            result = json.loads(stdout)
        except json.JSONDecodeError:
            observation['status'] = 'invalid-json'
            raise CallFailed(f'{command}: invalid JSON') from None
        observation['status'] = result.get('terminal', 'ok') if isinstance(result, dict) else 'ok'
        return result


def snapshot(cli):
    records = cli.call('records')
    operations = {operation for record in records
                  for operation in record.get('data', {}).get('Run', {}).get('operations', [])}
    receipts = [receipt for receipt in cli.call('history') if receipt['operation'] in operations]
    return public_snapshot({'records': records, 'receipts': receipts, 'candidates': cli.call('candidates'),
                            'lineage': read_operation_inputs(cli.data, operations)})


FILLER_SUBJECTS = [
    ('Harbor crane maintenance', 'The harbor crane inspection found worn cables; delivery of spare parts is due in November.'),
    ('Harbor café menu', 'The harbor café serves soup and bread; supplier delivery arrives every Tuesday morning.'),
    ('Berth cleaning rota', 'Berth cleaning runs weekly; booking the sweeper needs 24 hours notice at the harbor office.'),
    ('Tide gauge calibration', 'The tide gauge was calibrated to 0.1 metres; access to the gauge hut needs a key from the harbor master.'),
    ('Cargo insurance renewal', 'Cargo insurance covers loads up to 30 tonnes; the renewal estimate arrives in October.'),
    ('Pilot boat schedule', 'The pilot boat runs at 07:00 and 15:00; medical transfers take priority over other bookings.'),
    ('Dredging survey archive', 'An older dredging survey measured depth near the quay; it is not a current access rule.'),
    ('Warehouse rack capacity', 'Warehouse racks hold 2 tonnes per shelf; delivery trucks unload at gate 3.'),
]


def filler_note(index):
    """Synthetic distractors that share the cases' vocabulary but not their subjects."""
    title, text = FILLER_SUBJECTS[index % len(FILLER_SUBJECTS)]
    return (f'{title} {index:04d}',
            f'# {title} {index:04d}\n\nArchive entry {index}. {text} Recorded for weekly operations review number {index % 52}.\n')


def saved_notes(records):
    return [record for record in records if not record['archived'] and 'Note' in record['data']
            and record['data']['Note']['superseded_by'] is None]


def structural_checks(case, before, after, filler=frozenset()):
    # Distractor notes are workspace scale, not part of the case.
    previous = [note for note in saved_notes(before['records']) if note['id'] not in filler]
    current = [note for note in saved_notes(after['records']) if note['id'] not in filler]
    current_ids = {note['id'] for note in current}
    checks = {'protected_references_preserved': protection_preserved(before['records'], after['records'])}
    if case['no_new_notes']:
        checks['conversation_only_no_new_note'] = current_ids == {note['id'] for note in previous}
        checks['conversation_only_no_action'] = not any('Action' in record['data'] for record in after['records'])
        checks['conversation_only_no_attention'] = not any(record['data'].get('Thread', {}).get('attention') for record in after['records'])
    else:
        checks['knowledge_change_applied'] = any('Note' in write['after']['data'] for receipt in after['receipts'] for write in receipt['writes'])
    expected = case['expected_existing_note']
    if expected:
        subject = next(note for note in previous if note['data']['Note']['title'] == expected)
        updated = by_id(current).get(subject['id'])
        checks['existing_subject_updated'] = bool(updated and updated['data'] != subject['data'])
    if case['id'] == 'reusable-synthesis':
        required = {note['id'] for note in previous}
        required_versions = {(note['id'], note['version']) for note in previous}
        created = current_ids - {note['id'] for note in previous}
        links = [record['data']['Link'] for record in after['records'] if not record['archived'] and 'Link' in record['data']]
        lineage = {operation['operation']: operation for operation in after['lineage'] if operation['applied']}
        candidates = []
        for identity in sorted(created):
            target = by_id(current)[identity]
            writers = [receipt for receipt in after['receipts']
                       if any(write['after']['id'] == identity and write['after']['version'] == target['version']
                              and 'Note' in write['after']['data'] for write in receipt['writes'])]
            inputs_ok = any(required_versions <= {(item['record'], item['version'])
                           for item in lineage.get(receipt['operation'], {}).get('inputs', [])
                           if item.get('expectation', 'Current') == 'Current'} for receipt in writers)
            connected = {link['to'] for link in links if link['from'] == identity} | {link['from'] for link in links if link['to'] == identity}
            candidates.append({'note': identity, 'inputs': inputs_ok, 'links': required <= connected})
        # Both assertions refer to the same actual saved Note and its writing operation.
        selected = next((candidate for candidate in candidates if candidate['inputs'] and candidate['links']), None)
        if selected is None:
            selected = candidates[0] if candidates else {'inputs': False, 'links': False}
        checks['both_reference_notes_declared_as_inputs'] = selected['inputs']
        checks['both_reference_notes_linked_to_synthesis'] = selected['links']
    return checks


def undo_latest_note_change(cli, after):
    current = by_id(after['records'])
    eligible = [receipt for receipt in after['receipts']
                if any('Note' in write['after']['data'] for write in receipt['writes'])
                and all(current.get(write['after']['id'], {}).get('version') == write['after']['version']
                        for write in receipt['writes'])]
    if not eligible:
        return {'status': 'not_run', 'reason': 'No eligible current applied model note group'}
    receipt = sorted(eligible, key=lambda item: item['committed_at'])[-1]
    result = cli.call('undo', operation=receipt['operation'])
    records = public_snapshot(cli.call('records'))
    affected = {write['after']['id'] for write in receipt['writes']}
    return {'status': result['outcome'], 'operation': receipt['operation'],
            'affected_restored': result['outcome'] == 'Applied' and verify_undo(receipt, records),
            'unrelated_preserved': unrelated_preserved(after['records'], records, affected)}


def verification_problem(checks, undo):
    failed = [name for name, passed in checks.items() if passed is not True]
    if failed:
        return 'Deterministic checks failed: ' + ', '.join(failed)
    if undo['status'] == 'not_run':
        return None
    if undo['status'] != 'Applied' or not undo.get('affected_restored') or not undo.get('unrelated_preserved'):
        return 'Undo application/restoration/preservation failed'
    return None


def run_arm(binary, root, folder, case, selection, args):
    folder.mkdir(parents=True)
    data = folder / 'data'
    cli = Cli(binary, data, root, args.helper, args.timeout)
    result = {'case': case['id'], 'rubric': case['rubric'], 'before': None, 'after': None,
              'outcomes': [], 'commands': cli.commands, 'failure': None, 'verification_failure': None,
              'semantic_status': 'not_run', 'provider_completed': False, 'completed': False, 'structural_checks': {},
              'undo': {'status': 'not_run'}, 'stage': 'setup'}

    def persist():
        (folder / 'result.json').write_text(json.dumps(public_snapshot(result), ensure_ascii=False, indent=2) + '\n')

    try:
        cli.call('init')
        if args.live:
            configuration = dict(selection)
            configuration.update(credential_options(args.provider, args.auth_file, args.credentials_dir))
            cli.call('configure', **configuration)
        cli.call('delegation', choice='maintain')
        filler = set()
        for index in range(args.filler):
            title, text = filler_note(index)
            filler.add(cli.call('note-create', title=title, text=text)['id'])
        result['filler_notes'] = len(filler)
        for index, seed in enumerate(case['seeds']):
            if seed['protected']:
                source = folder / f'reference-{index}.md'
                source.write_text(seed['text'])
                imported = cli.call('intake', source=source, intent='full', title=seed['title'])
                if imported['outcome'] != 'Applied':
                    raise CallFailed('Protected fixture import was not applied')
            else:
                cli.call('note-create', title=seed['title'], text=seed['text'])
        thread = cli.call('thread-start', title=case['id'])['id']
        result['stage'] = 'snapshot_before'
        result['before'] = snapshot(cli)
        persist()
    except CallFailed as error:
        result.update(failure=str(error), semantic_status='failed')
        persist()
        return result

    result['stage'] = 'provider'
    for index, step in enumerate(case['steps']):
        if not args.live:
            break
        try:
            if step['kind'] == 'intake':
                source = folder / f'update-{index}.md'
                source.write_text(step['text'])
                outcome = cli.call('intake', source=source, intent='useful', title=step['title'], thread=thread)
            else:
                outcome = cli.call('ask', thread=thread, prompt=step['prompt'], budget=8)
            result['outcomes'].append(outcome)
            print(json.dumps({'case': case['id'], 'arm': folder.parent.name, 'step': index,
                              'terminal': outcome['terminal'], 'seconds': cli.commands[-1]['seconds']}), flush=True)
            if not completed(outcome):
                result['failure'] = outcome['terminal']
                persist()
                break
        except CallFailed as error:
            result['failure'] = str(error)
            persist()
            break
        persist()
    result['provider_completed'] = (args.live and not result['failure']
                           and len(result['outcomes']) == len(case['steps'])
                           and all(map(completed, result['outcomes'])))
    result['semantic_status'] = 'requires_review' if result['provider_completed'] else ('failed' if result['failure'] else 'not_run')
    result['stage'] = 'snapshot_after'
    try:
        result['after'] = snapshot(cli)
        if args.live:
            result['structural_checks'] = structural_checks(case, result['before'], result['after'], frozenset(filler))
        persist()
        result['stage'] = 'undo'
        if result['provider_completed']:
            result['undo'] = undo_latest_note_change(cli, result['after'])
            result['verification_failure'] = verification_problem(result['structural_checks'], result['undo'])
            if result['verification_failure']:
                result['semantic_status'] = 'verification_failed'
        result['stage'] = 'complete'
    except CallFailed as error:
        result['verification_failure'] = str(error)
        result['semantic_status'] = 'verification_failed'
    result['completed'] = result['provider_completed'] and not result['verification_failure']
    persist()
    return result


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-bin', type=Path, required=True)
    parser.add_argument('--wiki-bin', type=Path, required=True)
    parser.add_argument('--helper', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--baseline-commit', required=True)
    parser.add_argument('--candidate-commit', required=True)
    parser.add_argument('--live', action='store_true', help='explicitly run the owner-selected connected provider')
    parser.add_argument('--provider', choices=['Codex', 'Copilot'])
    parser.add_argument('--model')
    parser.add_argument('--effort', choices=['low', 'medium', 'high'])
    parser.add_argument('--auth-file', type=Path)
    parser.add_argument('--credentials-dir', type=Path)
    parser.add_argument('--repetitions', type=int, choices=[1, 2], default=2)
    parser.add_argument('--timeout', type=int, default=180)
    parser.add_argument('--filler', type=int, default=0,
                        help='synthetic distractor notes created before each case, so retrieval matters')
    args = parser.parse_args()
    for binary in [args.baseline_bin, args.wiki_bin, args.helper]:
        if not binary.is_absolute() or not binary.is_file() or not os.access(binary, os.X_OK):
            parser.error('Executables must be existing absolute executable files')
    if args.live and not all([args.provider, args.model, args.effort]):
        parser.error('Live mode requires the explicitly selected provider, exact model and effort')
    if args.auth_file and not args.auth_file.is_absolute():
        parser.error('Auth profile path must be absolute')
    if args.live and args.provider == 'Copilot' and (not args.credentials_dir or not args.credentials_dir.is_absolute() or args.auth_file):
        parser.error('Copilot requires one existing absolute credentials-dir and no auth-file')
    if args.live and args.provider == 'Codex' and args.credentials_dir:
        parser.error('Codex uses auth-file; select only one credential route')
    if args.timeout < 1 or args.timeout > 600:
        parser.error('Timeout must be 1..600 seconds')
    if not 0 <= args.filler <= 5000:
        parser.error('Filler must be 0..5000 notes')
    create_root(args.root)
    frozen = json.loads(CASES.read_text())
    selection = {'provider': args.provider, 'model': args.model, 'effort': args.effort}
    report = {'mode': 'live' if args.live else 'offline', 'selection': selection if args.live else None,
              'budget': 8, 'repetitions': args.repetitions, 'fixture_sha256': digest(CASES),
              'baseline': {'commit': args.baseline_commit, 'binary_sha256': digest(args.baseline_bin)},
              'wiki': {'commit': args.candidate_commit, 'binary_sha256': digest(args.wiki_bin)},
              'token_usage': 'not exposed by production CLI', 'filler_notes': args.filler, 'arms': []}
    report_path = args.root / 'comparison.json'
    for repetition in range(args.repetitions):
        arms = [('baseline', args.baseline_bin), ('wiki', args.wiki_bin)]
        if repetition % 2:
            arms.reverse()
        for case in frozen['cases']:
            for arm, binary in arms:
                folder = args.root / str(repetition + 1) / arm / case['id']
                try:
                    result = run_arm(binary, args.root, folder, case, selection, args)
                except CallFailed as error:
                    result = {'case': case['id'], 'failure': str(error), 'completed': False, 'semantic_status': 'failed'}
                result.update(arm=arm, repetition=repetition + 1)
                report['arms'].append(result)
                report_path.write_text(json.dumps(public_snapshot(report), ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'report': str(report_path), 'completed': sum(item['completed'] for item in report['arms']),
                      'arms': len(report['arms']), 'mode': report['mode']}), flush=True)
    return int(any(item.get('failure') or item.get('verification_failure') for item in report['arms']))


if __name__ == '__main__':
    raise SystemExit(main())
