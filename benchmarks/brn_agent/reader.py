"""Fixed controlled-reader execution on native restored checkpoint clones."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace
from evidence import verified_proof_packet
from score import sha256


def packet_hash(packet):
    return hashlib.sha256(json.dumps(packet, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def run(checkpoint, fixture, binary, reader_binary, output):
    """One controlled provider fixture, no arbitrary command or live-provider path."""
    required = {'mode', 'question', 'records', 'controlled_answer', 'packet_sha256'}
    optional = {'queries', 'max_calls', 'provider_error'}
    if not required <= set(fixture) or set(fixture) - required - optional:
        raise ValueError('Controlled reader requires the exact fixed protocol')
    if fixture['mode'] not in ('wiki-only', 'normal-product'):
        raise ValueError('Unknown reader mode')
    for field in ('question', 'controlled_answer'):
        if not isinstance(fixture[field], str) or not fixture[field] or len(fixture[field]) > 65536:
            raise ValueError('Reader text must be bounded and nonempty')
    reads = fixture['records']
    if not isinstance(reads, list) or not 1 <= len(reads) <= 16 or any(
            set(r) != {'id', 'version'} or not isinstance(r['id'], str) or type(r['version']) is not int for r in reads):
        raise ValueError('Reader needs bounded exact record/version selections')
    if len({r['id'] for r in reads}) != len(reads):
        raise ValueError('Duplicate reader selection')
    queries = fixture.get('queries', [])
    max_calls = fixture.get('max_calls', 32)
    if not isinstance(queries, list) or len(queries) > 4 or any(not isinstance(q, str) or not q or len(q) > 256 for q in queries):
        raise ValueError('Reader queries must be at most four bounded strings')
    if type(max_calls) is not int or not 1 <= max_calls <= 32:
        raise ValueError('Reader max_calls must be 1–32')
    if len(queries) + len(reads) + 1 > max_calls:
        raise ValueError('Reader fixture exceeds the declared model-call budget')
    if 'provider_error' in fixture and (not isinstance(fixture['provider_error'], str) or not fixture['provider_error'] or len(fixture['provider_error']) > 256):
        raise ValueError('Controlled reader error must be bounded text')
    packet = verified_proof_packet(checkpoint, wiki_records=reads if fixture['mode'] == 'wiki-only' else None)
    if packet_hash(packet) != fixture['packet_sha256']:
        raise ValueError('Controlled response belongs to a different frozen packet')
    notes = {(r['id'], r['version']) for r in packet['records'] if 'Note' in r['data']}
    if any((r['id'], r['version']) not in notes for r in reads):
        raise ValueError('Reader selection is absent from frozen notes')
    sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
    import evaluate_wiki_v2 as v2
    output = Path(output).resolve()
    checkpoint = Path(checkpoint).resolve()
    backup = checkpoint / 'workspace.sqlite3'
    frozen = sha256(backup)
    frozen_records = sha256(checkpoint / 'records.json')
    v2.create_root(output)
    result = {'format':1, 'status':'running', 'qualified':False, 'mode':fixture['mode'],
              'provider':'controlled-fixture', 'packet_sha256':fixture['packet_sha256'],
              'fixture_sha256':packet_hash(fixture), 'reader_binary_sha256':sha256(reader_binary),
              'cli_binary_sha256':sha256(binary), 'backup_sha256':frozen, 'cost':None, 'max_calls':max_calls,
              'queries':queries, 'retrieval':'native read_records over the explicit scoped allowlist'}
    def persist():
        (output / 'reader-result.json').write_text(json.dumps(result, indent=2))
    persist()
    try:
        cli = v2.make_cli({'model_dir':None}, binary, output / 'data', output,
                          SimpleNamespace(helper=None, timeout=60, stock_helper=None, brn_helper=None))
        cli.call('restore', source=backup)
        (output / 'AGENTBENCH-READER-CLONE').write_text(frozen + '\n')
        probe = {key:fixture[key] for key in ('mode','question','records','controlled_answer')}
        probe.update(queries=queries, max_calls=max_calls, provider_error=fixture.get('provider_error'))
        probe['data_dir'] = str(output / 'data')
        request = output / 'controlled-probe.json'
        request.write_text(json.dumps(probe, indent=2))
        process = subprocess.run([str(reader_binary), str(request)], text=True, capture_output=True,
                                 timeout=60, env=cli.environment)
        (output / 'reader.stdout').write_text(process.stdout)
        (output / 'reader.stderr').write_text(process.stderr)
        if process.returncode:
            raise ValueError(f'Controlled reader failed with exit {process.returncode}; evidence retained')
        observed = json.loads(process.stdout)
        if observed.get('provider') != 'controlled-fixture' or observed.get('qualified') is not False:
            raise ValueError('Reader binary did not return controlled diagnostic evidence')
        result['observed'] = observed
        if type(observed.get('request_count')) is not int or observed['request_count'] > max_calls:
            raise ValueError('Controlled reader exceeded the fixed invocation budget')
        if observed.get('terminal') != 'Completed':
            raise ValueError('Controlled reader did not reach a completed terminal result')
        result.update(status='complete', observed=observed)
    except (OSError, ValueError, subprocess.TimeoutExpired, v2.CallFailed) as error:
        result.update(status='failed', error=str(error))
        raise
    finally:
        if sha256(backup) != frozen or sha256(checkpoint / 'records.json') != frozen_records:
            result.update(status='failed', error='Original checkpoint changed during reader execution')
            persist()
            raise ValueError(result['error'])
        persist()
    return result
