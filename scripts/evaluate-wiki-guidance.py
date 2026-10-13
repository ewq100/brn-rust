#!/usr/bin/env python3
"""Compare two compiled BRN guidance bundles in exclusively owned synthetic state."""
import argparse
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


def create_root(root):
    if not root.is_absolute():
        raise ValueError('Use an absolute fresh root')
    root.mkdir(mode=0o700)  # Never adopt an occupied directory or symlink.
    (root / 'EVALUATION-OWNED').write_text('Synthetic BRN wiki comparison only\n')


def public_snapshot(value):
    if isinstance(value, dict):
        if 'Settings' in value.get('data', {}):
            return None
        return {key: clean for key, item in value.items()
                if key not in PRIVATE_KEYS and key != 'Settings'
                and (clean := public_snapshot(item)) is not None}
    if isinstance(value, list):
        return [clean for item in value if (clean := public_snapshot(item)) is not None]
    return value


def completed(outcome):
    return outcome.get('terminal') == 'Completed'


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
    with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as database:
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


def saved_notes(records):
    return [record for record in records if not record['archived'] and 'Note' in record['data']
            and record['data']['Note']['superseded_by'] is None]


def structural_checks(case, before, after):
    previous = saved_notes(before['records'])
    current = saved_notes(after['records'])
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
        inputs = {item['record'] for operation in after['lineage'] if operation['applied'] for item in operation['inputs']}
        checks['both_reference_notes_declared_as_inputs'] = required <= inputs
        created = current_ids - {note['id'] for note in previous}
        links = [record['data']['Link'] for record in after['records'] if not record['archived'] and 'Link' in record['data']]
        connected = {link['to'] for link in links if link['from'] in created} | {link['from'] for link in links if link['to'] in created}
        checks['both_reference_notes_linked_to_synthesis'] = required <= connected
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


def run_arm(binary, root, folder, case, selection, args):
    folder.mkdir(parents=True)
    data = folder / 'data'
    cli = Cli(binary, data, root, args.helper, args.timeout)
    cli.call('init')
    if args.live:
        configuration = dict(selection)
        # Use a task-owned cache, importing only the selected existing Codex auth profile.
        configuration['credentials_dir'] = folder / 'credentials'
        if args.auth_file:
            configuration['auth_file'] = args.auth_file
        cli.call('configure', **configuration)
    cli.call('delegation', choice='maintain')
    for index, seed in enumerate(case['seeds']):
        if seed['protected']:
            source = folder / f'reference-{index}.md'
            source.write_text(seed['text'])
            result = cli.call('intake', source=source, intent='full', title=seed['title'])
            if result['outcome'] != 'Applied':
                raise CallFailed('Protected fixture import was not applied')
        else:
            cli.call('note-create', title=seed['title'], text=seed['text'])
    thread = cli.call('thread-start', title=case['id'])['id']
    before = snapshot(cli)
    outcomes, failure = [], None
    for index, step in enumerate(case['steps']):
        if not args.live:
            break
        try:
            if step['kind'] == 'intake':
                source = folder / f'update-{index}.md'
                source.write_text(step['text'])
                result = cli.call('intake', source=source, intent='useful', title=step['title'], thread=thread)
            else:
                result = cli.call('ask', thread=thread, prompt=step['prompt'], budget=8)
            outcomes.append(result)
            print(json.dumps({'case': case['id'], 'arm': folder.parent.name, 'step': index,
                              'terminal': result['terminal'], 'seconds': cli.commands[-1]['seconds']}), flush=True)
            if not completed(result):
                failure = result['terminal']
                break
        except CallFailed as error:
            failure = str(error)
            break
    after = snapshot(cli)
    result = {'case': case['id'], 'rubric': case['rubric'], 'before': before, 'after': after,
              'outcomes': outcomes, 'commands': cli.commands, 'failure': failure,
              'semantic_status': 'requires_review' if args.live and not failure else ('failed' if failure else 'not_run'),
              'completed': args.live and not failure and len(outcomes) == len(case['steps']) and all(map(completed, outcomes)),
              'structural_checks': structural_checks(case, before, after) if args.live else {},
              'undo': undo_latest_note_change(cli, after) if args.live and not failure else {'status': 'not_run'}}
    (folder / 'result.json').write_text(json.dumps(public_snapshot(result), ensure_ascii=False, indent=2) + '\n')
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
    parser.add_argument('--repetitions', type=int, choices=[1, 2], default=2)
    parser.add_argument('--timeout', type=int, default=180)
    args = parser.parse_args()
    for binary in [args.baseline_bin, args.wiki_bin, args.helper]:
        if not binary.is_absolute() or not binary.is_file() or not os.access(binary, os.X_OK):
            parser.error('Executables must be existing absolute executable files')
    if args.live and not all([args.provider, args.model, args.effort]):
        parser.error('Live mode requires the explicitly selected provider, exact model and effort')
    if args.auth_file and not args.auth_file.is_absolute():
        parser.error('Auth profile path must be absolute')
    if args.timeout < 1 or args.timeout > 600:
        parser.error('Timeout must be 1..600 seconds')
    create_root(args.root)
    frozen = json.loads(CASES.read_text())
    selection = {'provider': args.provider, 'model': args.model, 'effort': args.effort}
    report = {'mode': 'live' if args.live else 'offline', 'selection': selection if args.live else None,
              'budget': 8, 'repetitions': args.repetitions, 'fixture_sha256': digest(CASES),
              'baseline': {'commit': args.baseline_commit, 'binary_sha256': digest(args.baseline_bin)},
              'wiki': {'commit': args.candidate_commit, 'binary_sha256': digest(args.wiki_bin)},
              'token_usage': 'not exposed by production CLI', 'arms': []}
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
    return int(any(item.get('failure') for item in report['arms']))


if __name__ == '__main__':
    raise SystemExit(main())
