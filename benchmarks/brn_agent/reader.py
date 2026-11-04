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


def measured(checkpoint, question, records, mode, binary, output, *, auth_file, model_dir=None,
             live=False):
    """One native reader invocation, using the existing Sample reservation/dispatch.

    No supplied answers, custom provider clients or candidate scheduling. The
    caller must obtain fresh model allocation; this function never reuses pilots.
    Every mode/question gets an exclusively owned restored clone.
    """
    if not live:
        raise ValueError('Measured reader needs --live and a newly authorized allocation')
    if mode not in ('wiki-only', 'normal-product') or not isinstance(question, str) or not question.strip() or len(question) > 4096:
        raise ValueError('Reader needs a supported mode and a bounded question')
    if not Path(auth_file).is_absolute() or (model_dir is not None and not Path(model_dir).is_absolute()):
        raise ValueError('Use explicit credential and retrieval-model paths')
    config_path = Path(__file__).parent/'wiki_challenge/reader.json'
    config = json.loads(config_path.read_text())
    if records is None or not isinstance(records, list) or not records:
        raise ValueError('Reader needs an independently selected article ID/version allowlist')
    packet = verified_proof_packet(checkpoint, wiki_records=records if mode == 'wiki-only' else None)
    allowed = {(r['id'], r['version']) for r in records}
    if len(allowed) != len(records) or len(records) > 32:
        raise ValueError('Invalid reader allowlist')
    for identity, version in allowed:
        note = next((r for r in packet['records'] if r['id'] == identity and r['version'] == version), None)
        if type(version) is not int or not note or 'Note' not in note['data'] or note['data']['Note'].get('chat') is not None:
            raise ValueError('Reader articles must be captured general notes')
    checkpoint, output = Path(checkpoint).resolve(), Path(output).resolve()
    backup = checkpoint/'workspace.sqlite3'
    frozen = {'backup': sha256(backup), 'records': sha256(checkpoint/'records.json')}
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'scripts'))
    import evaluate_wiki_v2 as v2
    v2.create_root(output)
    args = SimpleNamespace(helper=None, stock_helper=None, brn_helper=None, timeout=config['timeout_seconds'],
                           live=True, model_call_limit=config['model_call_limit'],
                           max_total_model_calls=config['model_call_limit'])
    arm = dict(name='fixed-reader', binary=str(Path(binary).resolve()), binary_sha256=sha256(binary),
               provider=config['provider'], model=config['model'], effort=config['effort'],
               helper=None, model_dir=str(model_dir) if model_dir is not None else None)
    sample = v2.Sample(args, output, {'id':'READER','checks':[]}, 0,
                       dict(arm=arm['name'], repetition=1, label=None), arm, output, {}, {'trace':True})
    sample.folder.mkdir(parents=True)
    sample.cli = v2.make_cli(arm, binary, sample.folder/'data', output, args)
    result = dict(format=1, kind='measured-reader', status='running', qualified=False,
                  measured_answer=False, answer_quality=None, mode=mode, question=question,
                  checkpoint_records_sha256=frozen['records'], checkpoint_backup_sha256=frozen['backup'],
                  reader_config_sha256=sha256(config_path), cli_binary_sha256=sha256(binary),
                  article_allowlist=records, model=config['model'], effort=config['effort'], cost=None,
                  retrieval='native search_notes and read_note on an isolated checkpoint clone',
                  retrieval_assets='unverified; bind model directory assets in experiment lock' if model_dir else 'none requested; native build defaults, record features in input lock')
    def persist():
        (output/'reader-result.json').write_text(json.dumps(result, indent=2)+'\n')
    persist()
    try:
        thread, initial, probe = prepare_native_reader(sample.cli, backup, mode, records, question, config['instructions'])
        result['retrieval_probe'] = probe
        result['reader_thread'] = thread
        sample.cli.call('configure', provider=config['provider'], model=config['model'], effort=config['effort'], auth_file=auth_file)
        event = dict(index=1, kind='ask', thread=thread, operations=[], sources=[], loaded_skills=[])
        sample.result['steps'] = [event]
        sample.checkpoint(0)
        out = sample.run_step(1, dict(kind='ask', thread='reader', budget=config['budget'],
                             prompt=config['instructions']+'\n\nQuestion: '+question), {'reader':thread}, event)
        sample.checkpoint(1)
        trace = out.get('trace') or {}
        from evidence import identity_matches
        result['identity_verified'] = identity_matches(trace, config['model'], config['effort'])
        result['native_retrieval_observed'] = bool(trace.get('search')) and any(
            c.get('tool') == 'search_notes' and c.get('outcome') == 'success' for c in trace.get('calls', []))
        final = sample.cli.call('records')
        stable = [r for r in initial if set(r.get('data', {})) & {'Note','Link','Source','Action'}]
        if any(r not in final for r in stable):
            raise ValueError('Reader changed pre-existing evidence')
        additions = [r for r in final if r not in initial and set(r.get('data', {})) & {'Note','Action'}]
        if event['operations'] or additions:
            raise ValueError('Reader proposed or performed a write; answer is not measured')
        result.update(answer=out.get('answer'), terminal=out.get('terminal'), trace=trace,
                      measured_answer=out.get('terminal') == 'Completed' and result['identity_verified'] and result['native_retrieval_observed'])
        result['status'] = 'complete' if result['measured_answer'] else 'unverified'
        sample.result['status'] = 'complete' if out.get('terminal') == 'Completed' else 'model_failure'
    except (OSError, ValueError, v2.CallFailed) as error:
        result.update(status='failed', error=str(error))
        raise
    finally:
        sample.persist()
        result['native_result'] = str((sample.folder/'result.json').relative_to(output))
        result['native_result_sha256'] = sha256(sample.folder/'result.json')
        if frozen != {'backup':sha256(backup), 'records':sha256(checkpoint/'records.json')}:
            result.update(status='failed', measured_answer=False, error='Original checkpoint changed')
            persist()
            raise ValueError(result['error'])
        persist()
    return result


def verify_measurement(reference, root, checkpoint_records_sha256, expected_records=None):
    """Verify a retained real-reader receipt, never a supplied controlled answer."""
    if not isinstance(reference, dict) or set(reference) != {'path', 'sha256'}:
        raise ValueError('Reader evidence needs a relative receipt path and exact hash')
    root = Path(root).resolve()
    path = (root/reference['path']).resolve()
    if not path.is_relative_to(root) or sha256(path) != reference['sha256']:
        raise ValueError('Reader receipt changed or is outside the experiment')
    receipt = json.loads(path.read_text())
    if (receipt.get('kind') != 'measured-reader' or receipt.get('status') != 'complete'
            or receipt.get('measured_answer') is not True or receipt.get('mode') != 'wiki-only'
            or receipt.get('checkpoint_records_sha256') != checkpoint_records_sha256
            or receipt.get('reader_config_sha256') != sha256(Path(__file__).parent/'wiki_challenge/reader.json')):
        raise ValueError('Wiki usefulness requires a bound real wiki-only reader, not a controlled or normal-product answer')
    native_path = (path.parent/receipt['native_result']).resolve()
    if not native_path.is_relative_to(path.parent) or sha256(native_path) != receipt['native_result_sha256']:
        raise ValueError('Reader native result identity mismatch')
    native = json.loads(native_path.read_text())
    trace = native['steps'][0].get('trace') or {}
    from evidence import identity_matches
    config = json.loads((Path(__file__).parent/'wiki_challenge/reader.json').read_text())
    if (native['status'] != 'complete' or native['steps'][0].get('operations')
            or native['steps'][0].get('terminal') != 'Completed'
            or native['steps'][0].get('answer') != receipt.get('answer')
            or trace != receipt.get('trace') or trace.get('helpers')
            or not identity_matches(trace, config['model'], config['effort'])
            or not trace.get('search') or not any(c.get('tool') == 'search_notes' and c.get('outcome') == 'success'
                                               for c in trace.get('calls', []))):
        raise ValueError('Reader lacks verified real retrieval/identity or performed a write')
    captures = {}
    for c in native.get('checkpoints', []):
        folder = native_path.parent/'checkpoints'/str(c['step'])
        if sha256(folder/'records.json') != c['records_sha256'] or sha256(folder/'workspace.sqlite3') != c['backup_sha256']:
            raise ValueError('Reader checkpoint changed')
        captures[c['step']] = json.loads((folder/'records.json').read_text())
    if set(captures) != {0, 1}:
        raise ValueError('Reader before/after evidence missing')
    if expected_records is None or any(r not in captures[0] for r in expected_records):
        raise ValueError('Reader clone does not contain the bound candidate checkpoint records')
    articles = receipt.get('article_allowlist')
    if (not isinstance(articles,list) or not 1 <= len(articles) <= 32 or any(
            not isinstance(r,dict) or set(r) != {'id','version'} or not isinstance(r['id'],str)
            or not r['id'] or type(r['version']) is not int or r['version'] < 1 for r in articles)
            or len({r['id'] for r in articles}) != len(articles)):
        raise ValueError('Reader needs a nonempty unique exact article-version allowlist')
    for ref in articles:
        matches = [r for r in expected_records if r['id'] == ref['id'] and r['version'] == ref['version']]
        if (len(matches) != 1 or matches[0].get('archived') or 'Note' not in matches[0]['data']
                or matches[0]['data']['Note'].get('chat') is not None or matches[0] not in captures[0]):
            raise ValueError('Reader article version is not a current captured general note')
    thread = native['steps'][0].get('thread')
    run_id = native['steps'][0].get('run')
    run = next((r['data']['Run'] for r in captures[1] if r['id'] == run_id and 'Run' in r['data']), None)
    message = next((r['data']['Message'] for r in captures[1]
                    if run and r['id'] == run.get('instruction') and 'Message' in r['data']), None)
    prompt = config['instructions'] + '\n\nQuestion: ' + receipt['question']
    if (not run or run.get('thread') != thread or run.get('state') != 'Completed'
            or not message or message.get('role') != 'owner' or message.get('thread') != thread
            or message.get('text') != prompt or run.get('budget') != config['budget']
            or native['steps'][0].get('budget', {}).get('sent') != config['budget']
            or trace.get('max_rounds') != config['budget']
            or len(trace.get('wire_requests', [])) > config['model_call_limit']
            or native.get('binary_sha256') != receipt.get('cli_binary_sha256')):
        raise ValueError('Reader question/configuration is not bound to the actual native Run and owner Message')
    projects = [r['data']['Project'] for r in captures[0] if 'Project' in r['data']
                and thread in r['data']['Project'].get('chats', []) and not r.get('archived')]
    if (thread != receipt.get('reader_thread') or len(projects) != 1 or projects[0].get('use_brn_knowledge') is not False
            or projects[0].get('instructions') != config['instructions']
            or set(projects[0].get('documents', [])) != {r['id'] for r in receipt['article_allowlist']}):
        raise ValueError('Reader wiki-only scope is not bound to the fixed article allowlist')
    kinds = {'Note', 'Link', 'Source', 'Action'}
    before = [r for r in captures[0] if set(r['data']) & kinds]
    after = [r for r in captures[1] if set(r['data']) & kinds]
    if {r['id']:r for r in before} != {r['id']:r for r in after}:
        raise ValueError('Reader changed evidence')
    return dict(receipt, invocation_identity=run_id)


def prepare_native_reader(cli, backup, mode, records, question, instructions):
    """Restore and scope through checked services; perform real native retrieval, no model."""
    cli.call('restore', source=backup)
    initial = cli.call('records')
    settings = cli.call('settings')
    if settings.get('mcp') or settings.get('helper'):
        raise ValueError('Fixed reader refuses connector/helper configuration')
    cli.call('delegation', choice='review-first')
    project = None
    if mode == 'wiki-only':
        receipt = cli.call('project-create', title='Wiki-only fixed reader')
        project_record = next(w['after'] for w in receipt['writes'] if 'Project' in w['after']['data'])
        project = project_record['id']
        cli.call('project-settings', project=project, base=project_record['version'],
                        title='Wiki-only fixed reader', instructions=instructions, knowledge='no')
        for ref in records:
            cli.call('project-link', project=project, note=ref['id'])
        receipt = cli.call('project-chat', project=project, title='Fixed reader question')
        thread = next(w['after']['id'] for w in receipt['writes'] if 'Thread' in w['after']['data'])
    else:
        thread = cli.call('thread-start', title='Fixed reader question')['id']
    # Preflight actual native retrieval; do not substitute lexical Python filtering.
    options = {'project': project} if project else {}
    probe = cli.call('search', query=question, limit=10, **options)
    return thread, initial, probe
