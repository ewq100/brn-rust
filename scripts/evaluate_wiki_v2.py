#!/usr/bin/env python3
"""Harness v2: compare BRN arms (bundles, retrieval bundles or models) on synthetic wiki cases.

Design: docs/work/active/knowledge-retrieval/evaluation-design.md (sections 1, 2 and 5).
Fixture: scripts/fixtures/wiki-eval-cases-v2.json. The v1 harness and fixture stay frozen.

Without --live nothing contacts a provider and no provider is configured: the run only validates the
fixture, seeds and copies templates, runs Knowledge check and checks that each arm binary opens a copy.
Everything lives under one exclusively created --root; nothing outside it is written. Auth paths are
passed to `brn configure` and never read here.

Arms: --arm NAME=BINARY[,model-dir=PATH][,provider=Codex|Copilot][,model=ID][,effort=low|medium|high]
A case runs the arms whose base name (text before the first '-') is listed in its `arms`, so a model
matrix may use B-sol, B-luna. Arm options override the global --provider/--model/--effort.

Check types (a check may carry `after_step` N, `arms` [base names], `report_only`):
  answer_contains_any{step,values}  answer_excludes{step|steps,values}  no_writes{step}
  read{step,keys,version?}  read_range{step,key,range[,section]}  private_exposures{steps?,max?}
  note_updated{key}  no_new_note_titled{pattern}  link_exists{from,to[,relation]}
  input_lineage{step,keys|source_of_step}  protected_preserved  undo{step?}
  marker_absent_general{values}  new_case_notes{min?,max?}  kc_delta{kinds,op}  kc_kinds_resolved
  isolated_none{scope}  topics_cover{scope}  sources_linked{steps,min}  relation_share
  note_contains{scope|key|keys,values[,mode]}  search_rank{queries,max_rank[,min_hits]}
  not_orphan{key}  no_silent_loss{values}  note_title{key,title}  topics_contains{values}
  topics_links{key,present}  pages_still_listed{keys}
Values written 'a|b' in note_contains, no_silent_loss, search_rank and topics_contains mean either.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
DEFAULT_FIXTURE = HERE / 'fixtures/wiki-eval-cases-v2.json'


def _load_v1():
    spec = importlib.util.spec_from_file_location('wiki_eval_v1', HERE / 'evaluate-wiki-guidance.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


V1 = _load_v1()
CallFailed = V1.CallFailed
public_snapshot = V1.public_snapshot
credential_options = V1.credential_options
create_root = V1.create_root

PROVIDERS = ('Codex', 'Copilot')
EFFORTS = ('low', 'medium', 'high')
ARM_OPTIONS = {'model-dir': 'model_dir', 'provider': 'provider', 'model': 'model', 'effort': 'effort'}
RECOMMENDED_RELATIONS = frozenset({'source', 'supersedes', 'contradicts', 'related'})
WRITE_KINDS = ('Note', 'Link', 'Action')
KEPT_KINDS = ('Note', 'Link', 'Thread', 'Source', 'Action')
SAFETY_CHECKS = frozenset({'private_exposures', 'marker_absent_general', 'protected_preserved', 'undo', 'no_writes'})
# Terminals the CLI reports with exit 0 plus answer and trace: the run's behaviour, evaluated like any other.
# Every other terminal (Network, RateLimited, ReconnectNeeded, CodeExpired, Interrupted, ...) is transport.
EVALUATED_TERMINALS = frozenset({'Completed', 'Failed:ToolLimitReached', 'Failed:InvalidToolUse'})
BUDGET_TERMINALS = frozenset({'Failed:ToolLimitReached', 'Failed:InvalidToolUse'})


def evaluated(terminal):
    """Completed, a model-behaviour failure, or a non-Applied intake outcome ('Outcome:Deferred', ...)."""
    return terminal in EVALUATED_TERMINALS or str(terminal).startswith('Outcome:')


FAILURE_DETAIL_FIELDS = ('origin', 'status', 'code', 'error_type', 'phase', 'retryable')


def failure_summary(failure):
    """The CLI's redacted provider-failure detail as one short line, e.g. 'origin=http_status status=503'."""
    detail = (failure or {}).get('detail') or {}
    parts = [f'{name}={detail[name]}' for name in FAILURE_DETAIL_FIELDS
             if detail.get(name) not in (None, False, '')]
    return ' '.join(parts)
SEARCH_TOOLS = ('search_notes',)
READ_TOOLS = ('search_notes', 'read_records', 'read_note', 'read_history', 'note_connections', 'read_source')
ID_READ_TOOLS = ('read_note', 'read_history', 'read_records')
MARKER = 'EVALUATION-OWNED'


# ---------------------------------------------------------------- arms

def arm_base(name):
    return name.split('-')[0]


def parse_arm(text):
    """NAME=BINARY[,key=value...] into a dict; validation of files happens later."""
    name, sep, rest = text.partition('=')
    if not sep or not re.fullmatch(r'[A-Za-z][A-Za-z0-9_-]*', name):
        raise ValueError(f'Arm must look like NAME=BINARY[,option=value]: {text!r}')
    parts = rest.split(',')
    if not parts[0]:
        raise ValueError(f'Arm {name} needs a binary')
    arm = {'name': name, 'binary': parts[0], 'model_dir': None, 'provider': None, 'model': None, 'effort': None}
    for item in parts[1:]:
        key, sep, value = item.partition('=')
        if not sep or key not in ARM_OPTIONS or not value:
            raise ValueError(f'Unknown or empty arm option {item!r} (allowed: {", ".join(ARM_OPTIONS)})')
        arm[ARM_OPTIONS[key]] = value
    if arm['provider'] and arm['provider'] not in PROVIDERS:
        raise ValueError('Arm provider must be Codex or Copilot')
    if arm['effort'] and arm['effort'] not in EFFORTS:
        raise ValueError('Arm effort must be low, medium or high')
    return arm


def resolve_arm(arm, provider=None, model=None, effort=None):
    """Arm-level provider, model and effort override the global ones."""
    return dict(arm, provider=arm['provider'] or provider, model=arm['model'] or model,
                effort=arm['effort'] or effort)


def parse_arms(texts):
    arms = [parse_arm(text) for text in texts]
    names = [arm['name'] for arm in arms]
    if len(set(names)) != len(names):
        raise ValueError('Arm names must be unique')
    return arms


def arm_order(arms, repetition):
    """Latin-square rotation: repetition 0 keeps the order, 1 rotates by one, and so on."""
    if not arms:
        return []
    shift = repetition % len(arms)
    return list(arms[shift:]) + list(arms[:shift])


def case_arm_names(case, names):
    allowed = case.get('arms')
    return [name for name in names if allowed is None or arm_base(name) in allowed]


# ---------------------------------------------------------------- fixture

CHECK_REQUIRED = {
    'answer_contains_any': ('step', 'values'), 'answer_excludes': ('values',), 'no_writes': ('step',),
    'read': ('step', 'keys'), 'read_range': ('step', 'key', 'range'), 'private_exposures': (),
    'note_updated': ('key',), 'no_new_note_titled': ('pattern',), 'link_exists': ('from', 'to'),
    'input_lineage': ('step',), 'protected_preserved': (), 'undo': (), 'marker_absent_general': ('values',),
    'new_case_notes': (), 'kc_delta': ('kinds', 'op'), 'kc_kinds_resolved': (), 'isolated_none': ('scope',),
    'topics_cover': ('scope',), 'sources_linked': ('steps', 'min'), 'relation_share': (),
    'note_contains': ('values',), 'search_rank': ('queries', 'max_rank'), 'not_orphan': ('key',),
    'no_silent_loss': ('values',), 'note_title': ('key', 'title'), 'topics_contains': ('values',),
    'topics_links': ('key', 'present'), 'pages_still_listed': ('keys',),
}
KEY_FIELDS = ('key', 'from', 'to')
STOPWORDS = frozenset('''the and for are was were with that this from have has had not but you your our
what when which where how who why does did can could would should will shall may might much many any
some into onto over under about after before than then them they their there here been being its out
too very just also only per via all each every most more less other another such own same'''.split())


def expand_seed_keys(case):
    seeds = case.get('seeds', {})
    notes = [note['key'] for note in seeds.get('notes', [])]
    distractors = [f"{item['key']}-{index}" for item in seeds.get('distractors', [])
                   for index in range(item['count'])]
    threads = ['case'] + [thread['key'] for thread in seeds.get('threads', [])]
    return notes, distractors, threads


def content_stems(text):
    stems = set()
    for word in re.findall(r"[^\W\d_]{3,}", text.casefold()):
        if word in STOPWORDS:
            continue
        for suffix in ('ing', 'ed', 'es', 's', 'ly'):
            if word.endswith(suffix) and len(word) - len(suffix) >= 4:
                word = word[:-len(suffix)]
                break
        stems.add(word)
    return stems


def shared_stems(first, second):
    """Content stems of two texts that match exactly or by a prefix of four or more letters."""
    one, two = content_stems(first), content_stems(second)
    return sorted({a for a in one for b in two
                   if a == b or (min(len(a), len(b)) >= 4 and (a.startswith(b) or b.startswith(a)))})


def validate_case(case):
    problems = []
    cid = case.get('id', '?')

    def bad(message):
        problems.append(f'{cid}: {message}')

    for field in ('id', 'steps', 'checks', 'seeds'):
        if field not in case:
            bad(f'missing {field}')
    if problems:
        return problems
    notes, distractors, threads = expand_seed_keys(case)
    seeds = case['seeds']
    keys = notes + distractors + threads
    duplicates = sorted({key for key in keys if keys.count(key) > 1})
    if duplicates:
        bad(f'duplicate seed keys {duplicates}')
    if 'case' in [thread['key'] for thread in seeds.get('threads', [])]:
        bad("thread key 'case' is reserved")
    known = set(keys) | {'topics'}
    for field in ('targets', 'useful', 'traps', 'private'):
        for key in case.get(field, []):
            if key not in known:
                bad(f'{field} references unknown key {key!r}')
    for note in seeds.get('notes', []):
        if note.get('chat') and note['chat'] not in threads:
            bad(f"note {note['key']} names unknown chat {note['chat']!r}")
    for link in seeds.get('links', []):
        for end in ('from', 'to'):
            if link[end] not in known:
                bad(f'link references unknown key {link[end]!r}')
    for key in (seeds.get('topics') or {}).get('pages', []):
        if key not in known:
            bad(f'topics page {key!r} is unknown')
    bound = set(threads)
    for index, step in enumerate(case['steps'], 1):
        kind = step.get('kind')
        if kind == 'knowledge-review':
            bound.add(step.get('as', 'review'))
            continue
        if kind not in ('ask', 'intake'):
            bad(f'step {index} has unknown kind {kind!r}')
            continue
        if step.get('thread') not in bound:
            bad(f"step {index} thread {step.get('thread')!r} is not seeded or bound")
        if not step.get('prompt' if kind == 'ask' else 'text'):
            bad(f'step {index} has no {"prompt" if kind == "ask" else "text"}')
        if kind == 'intake' and step.get('intent', 'useful') not in ('useful', 'full', 'chat'):
            bad(f'step {index} has unknown intent')
    scopes = case.get('scopes', {})
    for name, scope in scopes.items():
        for key in scope.get('keys', []):
            if key not in known:
                bad(f'scope {name} references unknown key {key!r}')
    last = len(case['steps'])
    for check in case['checks']:
        kind = check.get('type')
        if kind not in CHECK_REQUIRED:
            bad(f'unknown check type {kind!r}')
            continue
        for field in CHECK_REQUIRED[kind]:
            if field not in check:
                bad(f'{kind} check needs {field}')
        if kind == 'input_lineage' and not (check.get('keys') or check.get('source_of_step')):
            bad('input_lineage needs keys or source_of_step')
        if kind == 'note_contains' and not (check.get('scope') or check.get('key') or check.get('keys')):
            bad('note_contains needs scope, key or keys')
        if kind == 'new_case_notes' and 'min' not in check and 'max' not in check:
            bad('new_case_notes needs min or max')
        if kind == 'kc_delta' and check.get('op') not in ('<=', '>=', '=='):
            bad('kc_delta op must be <=, >= or ==')
        for field in ('step', 'after_step', 'source_of_step'):
            if field in check and not 1 <= check[field] <= last:
                bad(f'{kind} {field} {check[field]} is outside the steps')
        for field in ('steps',):
            for number in check.get(field, []):
                if not 1 <= number <= last:
                    bad(f'{kind} steps include {number} outside the steps')
        for field in KEY_FIELDS:
            if field in check and check[field] not in known:
                bad(f'{kind} references unknown key {check[field]!r}')
        for key in check.get('keys', []):
            if key not in known:
                bad(f'{kind} references unknown key {key!r}')
        if 'scope' in check and check['scope'] not in scopes:
            bad(f'{kind} names unknown scope {check["scope"]!r}')
        if kind == 'read_range':
            long_notes = {note['key'] for note in seeds.get('notes', []) if note.get('long')}
            if check.get('key') not in long_notes:
                bad('read_range key must be a seeded note with a `long` body')
    for diagnostic in case.get('diagnostics', []):
        if not 1 <= diagnostic.get('step', 0) <= last or diagnostic.get('budget', 0) < 1:
            bad('diagnostic needs a valid step and budget')
    if case.get('no_stem_overlap'):
        targets = [note for note in seeds.get('notes', []) if note['key'] in case.get('targets', [])]
        target_text = ' '.join(f"{note['title']} {note.get('text', '')}" for note in targets)
        for index, step in enumerate(case['steps'], 1):
            if step.get('kind') == 'ask':
                overlap = shared_stems(step['prompt'], target_text)
                if overlap:
                    bad(f'step {index} prompt shares content stems with the target: {overlap}')
    return problems


def validate_fixture(fixture):
    if fixture.get('version') != 2:
        return ['fixture version must be 2']
    problems = []
    ids = [case.get('id') for case in fixture.get('cases', [])]
    if len(set(ids)) != len(ids):
        problems.append('duplicate case ids')
    for case in fixture.get('cases', []):
        problems.extend(validate_case(case))
    return problems


def case_scales(case):
    return list(case.get('scales') or [case.get('filler', 0)])


def case_repetitions(case, scale, override=None):
    if override:
        return override
    value = case.get('repetitions', 1)
    return value.get(str(scale), 1) if isinstance(value, dict) else value


def plan_samples(cases, arm_names, scale=None, repetitions=None):
    """Ordered samples: repetition-major, cases interleaved, arm order rotated per repetition.
    Diagnostics follow the main plan as separately labelled samples."""
    units = [(case, s) for case in cases for s in case_scales(case) if scale in (None, s)]
    plan = []
    most = max([case_repetitions(case, s, repetitions) for case, s in units] or [0])
    for repetition in range(1, most + 1):
        for case, s in units:
            if repetition > case_repetitions(case, s, repetitions):
                continue
            for name in arm_order(case_arm_names(case, arm_names), repetition - 1):
                plan.append({'case': case['id'], 'scale': s, 'arm': name, 'repetition': repetition,
                             'planned': case_repetitions(case, s, repetitions),
                             'label': None, 'budget_step': None, 'budget': None})
    for case, s in units:
        for diagnostic in case.get('diagnostics', []):
            for name in case_arm_names(case, arm_names):
                if arm_base(name) != arm_base(diagnostic['arm']):
                    continue
                for repetition in range(1, diagnostic.get('repetitions', 1) + 1):
                    plan.append({'case': case['id'], 'scale': s, 'arm': name, 'repetition': repetition,
                                 'planned': diagnostic.get('repetitions', 1), 'label': 'diagnostic', 'budget_step': diagnostic['step'],
                                 'budget': diagnostic['budget']})
    return plan


def slot_name(sample):
    prefix = 'd' if sample['label'] == 'diagnostic' else 'r'
    return f"{sample['arm']}-{prefix}{sample['repetition']}"


def slot_folder(root, sample, attempt):
    return Path(root) / 'runs' / f"{sample['case']}-{sample['scale']}" / (slot_name(sample) + ('' if attempt == 1 else '-rerun'))


def next_attempt(attempts):
    """Attempt number for the next run of a slot; a crashed (`running`) attempt is discarded, not counted."""
    return 2 if any(a.get('status') != 'running' for a in attempts) else 1


def resume_action(attempts):
    """attempts: persisted results for one slot, original first. Returns what --resume does."""
    if not attempts:
        return 'run'
    last = attempts[-1]
    if last.get('status') == 'running':
        return 'discard-and-run' if len(attempts) == 1 else 'discard-and-rerun'
    if last.get('status') == 'complete' or len(attempts) >= 2:
        return 'skip'
    return 'rerun'


# ---------------------------------------------------------------- trace metrics (section 1)

def norm(text):
    text = str(text).casefold()
    for space in ('\u00a0', '\u202f', '\u2009', '\u2007'):
        text = text.replace(space, ' ')
    return ' '.join(text.split())


def contains_alt(text, value):
    """`text` must already be normalised; 'a|b' accepts either."""
    return any(norm(part) in text for part in value.split('|'))


def ok_calls(trace):
    return [call for call in (trace or {}).get('calls', []) if call.get('outcome') != 'error']


def call_ids_read(call):
    args = call.get('args') or {}
    if call['tool'] in ('read_note', 'read_history'):
        return [args['id']] if args.get('id') else []
    if call['tool'] == 'read_records':
        return list(args.get('ids') or [])
    return []


def call_listed_ids(call):
    if call['tool'] in ('search_notes', 'read_records'):
        return list((call.get('result') or {}).get('ids') or [])
    return []


def step_metrics(trace, targets=(), useful=(), traps=(), private=(), kind=None, terminal=None):
    """Section 1 metrics from one run's trace; targets etc. are sets of record IDs."""
    if not trace:
        return {'trace': False}
    calls = ok_calls(trace)
    targets, useful, traps, private = set(targets), set(useful) | set(targets), set(traps), set(private)
    first_hit = rank = None
    for call in calls:
        listed = call_listed_ids(call)
        positions = [index for index, identity in enumerate(listed) if identity in targets]
        if positions:
            first_hit, rank = call['round'], positions[0] + 1
            break
    reads = [(call, identity) for call in calls for identity in call_ids_read(call)]
    target_reads = [call['round'] for call, identity in reads if identity in targets]
    searches = [call for call in calls if call['tool'] == 'search_notes']
    horizon = first_hit if first_hit is not None else float('inf')
    hashes = {(call.get('args') or {}).get('query_hash') for call in searches if call['round'] <= horizon}
    scans = [call for call in calls if call['tool'] == 'read_records'
             and (call.get('args') or {}).get('query') is not None]
    dumps = [call for call in calls if call['tool'] == 'read_records' and (call.get('args') or {}).get('catalog')]
    denominator = len(searches) + len(scans) + len(dumps)
    usage = trace.get('usage')
    tool_counts = {}
    for call in calls:
        tool_counts[call['tool']] = tool_counts.get(call['tool'], 0) + 1
    return {
        'trace': True, 'kind': kind,
        'first_hit_round': first_hit, 'target_rank': rank,
        'first_target_read_round': min(target_reads) if target_reads else None,
        'searches': len(searches), 'reformulations': max(len(hashes) - 1, 0) if searches else 0,
        'catalog_dumps': len(dumps), 'literal_scans': len(scans),
        'bytes_read': sum(call.get('bytes', 0) for call in calls if call['tool'] in READ_TOOLS),
        'wasted_reads': sum(1 for _, identity in reads if identity not in useful),
        'trap_reads': sum(1 for _, identity in reads if identity in traps),
        'private_exposures': private_exposure_count(trace, private),
        'search_share': len(searches) / denominator if denominator else None,
        'budget_exhausted': trace.get('stopped') == 'budget' or terminal in BUDGET_TERMINALS,
        'terminal': terminal,
        'stopped': trace.get('stopped'), 'rounds': trace.get('rounds'), 'model_turns': trace.get('model_turns'),
        'elapsed_ms': trace.get('elapsed_ms'), 'dropped_calls': trace.get('dropped_calls', 0),
        'tokens': (usage['input'] + usage['output']) if usage else None,
        'tool_counts': tool_counts, 'calls': len(calls),
    }


def private_exposure_count(trace, private):
    """Private IDs in any result, or read by ID, in this trace."""
    private = set(private)
    count = 0
    for call in ok_calls(trace):
        result = call.get('result') or {}
        seen = set(result.get('ids') or []) | set(call_ids_read(call))
        if call['tool'] == 'read_source':
            seen |= set([(call.get('args') or {}).get('id')] if (call.get('args') or {}).get('id') else [])
        count += len(seen & private)
    return count


SUMMED = ('searches', 'reformulations', 'catalog_dumps', 'literal_scans', 'bytes_read', 'wasted_reads',
          'trap_reads', 'private_exposures', 'rounds', 'model_turns', 'elapsed_ms', 'calls')
EARLIEST = ('first_hit_round', 'target_rank', 'first_target_read_round')


def run_metrics(per_step):
    """Sums counts and bytes over steps; first-hit style metrics come from the earliest step that has them."""
    traced = [m for m in per_step if m.get('trace')]
    if not traced:
        return {'trace': False}
    out = {'trace': True, 'per_step': per_step}
    for name in SUMMED:
        out[name] = sum(m[name] or 0 for m in traced)
    for name in EARLIEST:
        out[name] = next((m[name] for m in traced if m[name] is not None), None)
    out['budget_exhausted'] = any(m['budget_exhausted'] for m in traced)
    tokens = [m['tokens'] for m in traced if m['tokens'] is not None]
    out['tokens'] = sum(tokens) if tokens else None
    out['dropped_calls'] = sum(m['dropped_calls'] for m in traced)
    totals = {}
    for kind in {m['kind'] for m in traced if m['kind']}:
        group = [m for m in traced if m['kind'] == kind]
        searched = sum(m['searches'] for m in group)
        other = sum(m['literal_scans'] + m['catalog_dumps'] for m in group)
        totals[kind] = searched / (searched + other) if searched + other else None
    out['search_share'] = totals
    return out


# ---------------------------------------------------------------- snapshots

def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()[:16]


def condense_records(records, baseline):
    """Keeps what the checks use. Unchanged filler notes lose their text (baseline: id -> digest)."""
    kept = []
    for record in records:
        data = record.get('data', {})
        kind = next(iter(data), None)
        if kind not in KEPT_KINDS:
            continue
        if kind == 'Note' and record['id'] in baseline:
            note = data['Note']
            if digest(note.get('markdown', '')) == baseline[record['id']]:
                record = dict(record, data={'Note': dict(note, markdown=None, markdown_sha=baseline[record['id']])})
        kept.append(record)
    return [record for record in kept
            if not ('Link' in record['data'] and record['data']['Link']['from'] in baseline
                    and record['data']['Link']['to'] in baseline)]


def filler_baseline(records, seed_ids):
    return {record['id']: digest(record['data']['Note'].get('markdown', '')) for record in records
            if 'Note' in record['data'] and record['id'] not in seed_ids}


def condense_receipt(receipt):
    writes = []
    for write in receipt.get('writes', []):
        after = write['after']
        before = write.get('before')
        kind = next(iter(after['data']), None)
        entry = {'kind': kind, 'id': after['id'], 'version': after['version'],
                 'before_version': before['version'] if before else None, 'archived': after['archived']}
        if kind == 'Link':
            entry['ends'] = [after['data']['Link']['from'], after['data']['Link']['to']]
        writes.append(entry)
    return {'operation': receipt['operation'], 'undo_of': receipt.get('undo_of'), 'writes': writes}


def current_notes(records):
    return {record['id']: record['data']['Note'] for record in records
            if 'Note' in record['data'] and not record['archived']
            and record['data']['Note'].get('chat') is None and record['data']['Note'].get('superseded_by') is None}


def note_records(records):
    return {record['id']: record for record in records if 'Note' in record['data']}


HOST_RELATION_PREFIX = 'full-import:'


def link_triples(records, host=False):
    """Current links; `full-import:` links are host-owned (full-note import to Source) and left out by default."""
    return [(link['from'], link['to'], link['relation']) for record in records if 'Link' in record['data']
            and not record['archived'] for link in [record['data']['Link']]
            if host or not link['relation'].startswith(HOST_RELATION_PREFIX)]


def text_of(records, identity):
    note = note_records(records).get(identity)
    return norm(note['data']['Note'].get('markdown') or '') if note else ''


# ---------------------------------------------------------------- context helpers for checks

def key_id(ctx, key):
    seed = ctx['seed']
    if key == 'topics':
        return seed.get('topics')
    return seed['notes'].get(key) or seed['threads'].get(key)


def key_ids(ctx, keys):
    return {key: key_id(ctx, key) for key in keys}


def last_state(ctx):
    return ctx['states'][max(ctx['states'])]


def state_for(ctx, check):
    number = check.get('after_step', check.get('step'))
    if number is None:
        return last_state(ctx)
    if number not in ctx['states']:
        raise LookupError(f'state after step {number} was not captured')
    return ctx['states'][number]


def step_of(ctx, number):
    steps = ctx['steps']
    return steps[number - 1] if 1 <= number <= len(steps) and evaluated(steps[number - 1].get('terminal')) else None


def scope_ids(ctx, name, state):
    scope = ctx['case'].get('scopes', {})[name]
    ids = {key_id(ctx, key) for key in scope.get('keys', [])}
    if scope.get('include_new'):
        ids |= new_note_ids(ctx, state)
    return {identity for identity in ids if identity}


def new_note_ids(ctx, state):
    before = set(current_notes(ctx['states'][0]['records']))
    return set(current_notes(state['records'])) - before


def connected_ids(records, identity):
    """Current notes sharing a current link with `identity`."""
    current = set(current_notes(records))
    out = set()
    for source, target, _ in link_triples(records):
        if source == identity and target in current and target != identity:
            out.add(target)
        if target == identity and source in current and source != identity:
            out.add(source)
    return out


def traced_calls(step):
    return ok_calls(step['trace']) if step and step.get('trace') else None


# ---------------------------------------------------------------- checks (pure; (passed, detail))

def check_answer_contains_any(c, ctx):
    step = step_of(ctx, c['step'])
    if not step or step.get('answer') is None:
        return False, 'step has no answer'
    answer = norm(step['answer'])
    hits = [value for value in c['values'] if norm(value) in answer]
    return bool(hits), f'matched {hits}' if hits else f'none of {c["values"]}'


def check_answer_excludes(c, ctx):
    numbers = c.get('steps') or ([c['step']] if 'step' in c else range(1, len(ctx['steps']) + 1))
    leaked = []
    for number in numbers:
        step = step_of(ctx, number)
        if step and step.get('answer') is not None:
            answer = norm(step['answer'])
            leaked += [(number, value) for value in c['values'] if norm(value) in answer]
    return not leaked, f'leaked {leaked}' if leaked else 'no excluded value in the answers'


def check_no_writes(c, ctx):
    step = ctx['steps'][c['step'] - 1] if c['step'] <= len(ctx['steps']) else None
    if not step or not evaluated(step.get('terminal')):
        return False, 'step did not run to an evaluated terminal'
    # A Link counts when it touches a note; a chat-to-Source link keeps the
    # source in its chat and writes no knowledge. Unknown ends count.
    kinds = {record['id']: next(iter(record['data']), None) for state in ctx['states'].values()
             for record in state['records']}

    def knowledge(write):
        ends = write.get('ends')
        return write['kind'] != 'Link' or not ends or any(kinds.get(end, 'Note') == 'Note' for end in ends)

    writes = [(write['kind'], write['id']) for receipt in step.get('receipts', []) for write in receipt['writes']
              if write['kind'] in WRITE_KINDS and knowledge(write)]
    return not writes, f'{len(writes)} write(s): {writes[:5]}' if writes else 'no Note, note Link or Action write'


def check_read(c, ctx):
    calls = traced_calls(ctx['steps'][c['step'] - 1])
    if calls is None:
        return None, 'no trace'
    state = last_state(ctx)
    missing = []
    for key in c['keys']:
        identity = key_id(ctx, key)
        current = note_records(state['records']).get(identity, {}).get('version')
        found = False
        for call in calls:
            if identity not in call_ids_read(call):
                continue
            if c.get('version') == 'current' and call['tool'] == 'read_history':
                continue
            if c.get('version') == 'current' and call['tool'] == 'read_note':
                requested = (call.get('args') or {}).get('version')
                if requested is not None and requested != current:
                    continue
            found = True
        if not found:
            missing.append(key)
    return not missing, f'not read: {missing}' if missing else f'read {c["keys"]}'


def check_read_range(c, ctx):
    calls = traced_calls(ctx['steps'][c['step'] - 1])
    if calls is None:
        return None, 'no trace'
    identity = key_id(ctx, c['key'])
    span = ctx['seed'].get('ranges', {}).get(c['range'])
    if not span:
        return False, f'seeder reported no range {c["range"]!r}'
    for call in calls:
        args = call.get('args') or {}
        if call['tool'] != 'read_note' or args.get('id') != identity:
            continue
        start = args.get('start') or 0
        returned = (call.get('result') or {}).get('returned_bytes') or 0
        if start < span['end'] and start + returned > span['start']:
            return True, f'read bytes {start}..{start + returned}'
        if c.get('section') and norm(args.get('section') or '') == norm(c['section']):
            return True, 'read the section by heading'
    return False, 'no read_note range overlaps the planted section'


def check_private_exposures(c, ctx):
    numbers = c.get('steps') or range(1, len(ctx['steps']) + 1)
    private = ctx['private_ids']
    total, untraced = 0, []
    for number in numbers:
        step = ctx['steps'][number - 1] if number <= len(ctx['steps']) else None
        if not step or step.get('thread') in ctx['private_threads']:
            continue
        if not step.get('trace'):
            untraced.append(number)
            continue
        total += private_exposure_count(step['trace'], private)
    if untraced and not total:
        return None, f'steps {untraced} have no trace'
    return total <= c.get('max', 0), f'{total} private ID(s) in results'


def check_note_updated(c, ctx):
    identity = key_id(ctx, c['key'])
    state = state_for(ctx, c)
    before = note_records(ctx['states'][0]['records']).get(identity)
    after = note_records(state['records']).get(identity)
    if not before or not after or after['archived']:
        return False, 'note missing or archived'
    updated = after['version'] > before['version'] and after['data']['Note'].get('markdown') != before['data']['Note'].get('markdown')
    return updated, f'version {before["version"]} -> {after["version"]}'


def check_no_new_note_titled(c, ctx):
    state = state_for(ctx, c)
    pattern = norm(c['pattern'])
    new = new_note_ids(ctx, state)
    clashes = [current_notes(state['records'])[i]['title'] for i in new
               if pattern in norm(current_notes(state['records'])[i]['title'])]
    return not clashes, f'new note(s) titled like the pattern: {clashes}' if clashes else 'no new note with that title'


def check_link_exists(c, ctx):
    state = state_for(ctx, c)
    source, target = key_id(ctx, c['from']), key_id(ctx, c['to'])
    relations = c.get('relation')
    allowed = None if relations is None else ({relations} if isinstance(relations, str) else set(relations))
    found = [rel for a, b, rel in link_triples(state['records']) if a == source and b == target
             and (allowed is None or rel in allowed)]
    return bool(found), f'relation(s) {found}' if found else 'no such link'


def operation_inputs(step, lineage):
    ops = set(step.get('operations', []))
    return [item for item in lineage if item['operation'] in ops and item['applied']]


def check_input_lineage(c, ctx):
    number = c['step']
    step = ctx['steps'][number - 1] if number <= len(ctx['steps']) else None
    if not step:
        return False, 'step did not run'
    # The version current when the step read it: at least the version before
    # the step. The step's own group may then advance the record further.
    before = ctx['states'].get(number - 1) or ctx['states'][0]
    floor = {record['id']: record['version'] for record in before['records']}
    if c.get('source_of_step'):
        owner = ctx['steps'][c['source_of_step'] - 1]
        wanted = set(step_sources(owner))
        if not wanted:
            return False, 'the step recorded no Source'
        mode = any
    else:
        wanted = [key_id(ctx, key) for key in c['keys']]
        mode = all
    inputs = [(item['record'], item['version']) for op in operation_inputs(step, ctx['lineage'])
              for item in op['inputs'] if item.get('expectation', 'Current') == 'Current']

    def used(identity):
        return any(record == identity and version >= floor.get(identity, 1) for record, version in inputs)

    missing = [identity for identity in wanted if not used(identity)]
    ok = mode(used(identity) for identity in wanted)
    return ok, f'inputs lack a current version of {missing}' if not ok else 'applied inputs include the version read'


def step_sources(step):
    created = {write['id'] for receipt in step.get('receipts', []) for write in receipt['writes']
               if write['kind'] == 'Source' and write['before_version'] is None}
    return created | set(step.get('sources', []))


def check_protected_preserved(c, ctx):
    ok = V1.protection_preserved(ctx['states'][0]['records'], last_state(ctx)['records'])
    return ok, 'protected notes unchanged' if ok else 'a protected note changed'


# Host-managed Thread fields that Undo legitimately advances: a restored chat
# gets a new context epoch, and private context can only be added.
HOST_THREAD_FIELDS = ('context_epoch', 'private_context')


def without_host_fields(record):
    if not record or 'Thread' not in record.get('data', {}):
        return record
    thread = {k: v for k, v in record['data']['Thread'].items() if k not in HOST_THREAD_FIELDS}
    return dict(record, data={'Thread': thread})


def verify_undo(receipt, records):
    writes = [dict(write, before=without_host_fields(write.get('before')), after=without_host_fields(write['after']))
              for write in receipt['writes']]
    return V1.verify_undo(dict(receipt, writes=writes), [without_host_fields(r) for r in records])


def check_undo(c, ctx):
    result = ctx.get('undo', {}).get(c.get('step'))
    if result is None or result['status'] == 'not_run':
        return None, (result or {}).get('reason', 'undo was not run')
    ok = result['status'] == 'Applied' and result.get('affected_restored') and result.get('unrelated_preserved')
    return bool(ok), json.dumps({k: v for k, v in result.items() if k != 'operation'})


def check_marker_absent_general(c, ctx):
    """No marker in a current general note's body or title, nor in a current link relation."""
    state = last_state(ctx)
    surfaces = []
    for identity, note in current_notes(state['records']).items():
        surfaces.append((f'note {identity} body', norm(note.get('markdown') or '')))
        surfaces.append((f'note {identity} title', norm(note.get('title') or '')))
    surfaces += [(f'link {a}->{b} relation', norm(relation)) for a, b, relation in link_triples(state['records'], host=True)]
    leaks = [(where, value) for where, text in surfaces for value in c['values'] if norm(value) in text]
    return not leaks, f'marker in general content: {leaks[:5]}' if leaks else 'no marker in a current general note, title or relation'


def check_new_case_notes(c, ctx):
    count = len(new_note_ids(ctx, state_for(ctx, c)))
    ok = c.get('min', 0) <= count <= c.get('max', float('inf'))
    return ok, f'{count} new note(s); allowed {c.get("min", 0)}..{c.get("max", "any")}'


def kc_counts(kc):
    return (kc or {}).get('counts', {})


def check_kc_delta(c, ctx):
    before, after = kc_counts(ctx['states'][0].get('kc')), kc_counts(state_for(ctx, c).get('kc'))
    if ctx['states'][0].get('kc') is None or state_for(ctx, c).get('kc') is None:
        return None, 'Knowledge check was not captured'
    compare = {'<=': lambda a, b: a <= b, '>=': lambda a, b: a >= b, '==': lambda a, b: a == b}[c['op']]
    deltas = {kind: after.get(kind, 0) - before.get(kind, 0) for kind in c['kinds']}
    failed = {kind: delta for kind, delta in deltas.items() if not compare(delta, c['kinds'][kind])}
    return not failed, f'deltas {deltas}; failing {failed}' if failed else f'deltas {deltas}'


def check_kc_kinds_resolved(c, ctx):
    before, after = kc_counts(ctx['states'][0].get('kc')), kc_counts(state_for(ctx, c).get('kc'))
    resolved = sorted(kind for kind, count in before.items() if count > 0 and after.get(kind, 0) < count)
    return True, f'{len(resolved)} kind(s) improved: {resolved}'


def check_isolated_none(c, ctx):
    state = state_for(ctx, c)
    isolated = sorted(i for i in scope_ids(ctx, c['scope'], state)
                      if i in current_notes(state['records']) and not connected_ids(state['records'], i))
    return not isolated, f'{len(isolated)} page(s) without a link to another note' if isolated else 'every page is linked'


def check_topics_cover(c, ctx):
    state = state_for(ctx, c)
    topics = ctx['seed'].get('topics')
    if not topics:
        return False, 'the seed has no Topics note'
    members = scope_ids(ctx, c['scope'], state)
    triples = link_triples(state['records'])
    current = current_notes(state['records'])
    for page in {b for a, b, _ in triples if a == topics and b in current}:
        linked = {b for a, b, _ in triples if a == page}
        if members - {page} <= linked:
            return True, 'a topic page linked from Topics links every scope page'
    return False, 'no page linked from Topics links every scope page'


def check_sources_linked(c, ctx):
    state = last_state(ctx)
    current = set(current_notes(state['records']))
    triples = link_triples(state['records'], host=True)
    applied_inputs = {item['record'] for op in ctx['lineage'] if op['applied'] for item in op['inputs']}
    linked = total = 0
    for number in c['steps']:
        step = ctx['steps'][number - 1] if number <= len(ctx['steps']) else None
        sources = step_sources(step) if step else set()
        total += 1
        if any((a in current and b == s) or (b in current and a == s) for s in sources for a, b, _ in triples) \
                or sources & applied_inputs:
            linked += 1
    return linked >= c['min'], f'{linked} of {total} step Sources are linked or recorded as inputs'


def check_relation_share(c, ctx):
    state = last_state(ctx)
    before = set(link_triples(ctx['states'][0]['records']))
    new = [triple for triple in link_triples(state['records']) if triple not in before]
    if not new:
        return True, 'no new links'
    good = sum(1 for _, _, relation in new if relation in RECOMMENDED_RELATIONS)
    return True, f'{good}/{len(new)} = {good / len(new):.2f} new links use recommended relations'


def check_note_contains(c, ctx):
    state = state_for(ctx, c)
    if c.get('scope'):
        ids = scope_ids(ctx, c['scope'], state)
    else:
        ids = {key_id(ctx, key) for key in ([c['key']] if c.get('key') else c.get('keys', []))}
    notes = current_notes(state['records'])
    texts = [norm(notes[i].get('markdown') or '') for i in ids if i in notes]
    found = {value: any(contains_alt(text, value) for text in texts) for value in c['values']}
    ok = any(found.values()) if c.get('mode') == 'any' else all(found.values())
    return ok, json.dumps(found, ensure_ascii=False)


def check_search_rank(c, ctx):
    state = state_for(ctx, c)
    ranks = state.get('ranks')
    if not ranks:
        return None, 'brn search was not run'
    records = state['records']
    hits = 0
    detail = {}
    for query, literal in c['queries'].items():
        rank = None
        for position, hit in enumerate(ranks.get(query) or [], 1):
            text = text_of(records, hit['id']) or norm(hit.get('title', ''))
            if contains_alt(text, literal) or contains_alt(norm(hit.get('title', '')), literal):
                rank = position
                break
        detail[query] = rank
        hits += rank is not None and rank <= c['max_rank']
    return hits >= c.get('min_hits', len(c['queries'])), f'ranks {json.dumps(detail)}; {hits} within {c["max_rank"]}'


def check_not_orphan(c, ctx):
    state = state_for(ctx, c)
    linked = connected_ids(state['records'], key_id(ctx, c['key']))
    return bool(linked), f'{len(linked)} linked note(s)'


def check_no_silent_loss(c, ctx):
    state = state_for(ctx, c)
    texts = [norm(n.get('markdown') or '') for n in current_notes(state['records']).values()]
    missing = [v for v in c['values'] if not any(contains_alt(text, v) for text in texts)]
    attention = lambda s: sum(len(r['data']['Thread'].get('attention') or []) for r in s['records'] if 'Thread' in r['data'])
    raised = attention(state) > attention(ctx['states'][0])
    return not missing or raised, f'missing {missing}; new attention item: {raised}'


def check_note_title(c, ctx):
    note = current_notes(state_for(ctx, c)['records']).get(key_id(ctx, c['key']))
    return bool(note) and norm(note['title']) == norm(c['title']), f'title {note["title"] if note else None!r}'


def topics_note(ctx, state):
    return current_notes(state['records']).get(ctx['seed'].get('topics'))


def check_topics_contains(c, ctx):
    note = topics_note(ctx, state_for(ctx, c))
    if not note:
        return False, 'Topics note missing'
    text = norm(note.get('markdown') or '')
    found = {v: contains_alt(text, v) for v in c['values']}
    return all(found.values()), json.dumps(found, ensure_ascii=False)


def check_topics_links(c, ctx):
    state = state_for(ctx, c)
    linked = any(a == ctx['seed'].get('topics') and b == key_id(ctx, c['key']) for a, b, _ in link_triples(state['records']))
    return linked == c['present'], f'Topics links the page: {linked}'


def check_pages_still_listed(c, ctx):
    state = state_for(ctx, c)
    triples = link_triples(state['records'])
    current = current_notes(state['records'])
    topic_pages = {b for a, b, _ in triples if a == ctx['seed'].get('topics') and b in current}
    unlisted = [key for key in c['keys']
                if not any(a in topic_pages and b == key_id(ctx, key) for a, b, _ in triples)]
    return not unlisted, f'not linked from a topic page: {unlisted}' if unlisted else 'all pages listed'


CHECKS = {name[6:]: function for name, function in globals().items() if name.startswith('check_')}
TRACE_CHECKS = {'read', 'read_range', 'private_exposures'}


def evaluate_checks(case, ctx):
    """Runs every check; one failing check never stops the others."""
    results = []
    for index, check in enumerate(case['checks']):
        kind = check['type']
        entry = {'index': index, 'type': kind, 'step': check.get('step', check.get('after_step')), 'passed': None,
                 'report_only': bool(check.get('report_only')), 'applicable': True, 'unverified': False,
                 'safety': kind in SAFETY_CHECKS or bool(check.get('safety'))}
        if check.get('arms') and ctx['arm_base'] not in check['arms']:
            entry['applicable'] = False
            entry['detail'] = f'not applicable to arm {ctx["arm_base"]}'
        else:
            try:
                passed, entry['detail'] = CHECKS[kind](check, ctx)
            except (LookupError, KeyError, TypeError, ValueError) as error:
                passed, entry['detail'] = False, f'check could not run: {error!r}'
            entry['passed'] = None if entry['report_only'] else passed
            entry['unverified'] = passed is None and not entry['report_only']
        results.append(entry)
    return results


def case_passed(results, excluded=()):
    """All counted checks passed. A check that could not be verified is never a pass: the caller lists it
    in `excluded` (unverifiable on some arm of the comparison) or the sample does not pass."""
    counted = [item for item in results if item['applicable'] and not item['report_only']
               and item['index'] not in excluded]
    return all(item['passed'] is True for item in counted)


def unverified_checks(results):
    return [item for item in results if item.get('unverified')]


def safety_blockers(case, results, metrics, completed=True):
    blockers = [{'kind': item['type'], 'detail': item['detail']} for item in results
                if item['safety'] and item['passed'] is False]
    if metrics.get('trace') and metrics.get('private_exposures') and not any(
            b['kind'] == 'private_exposures' for b in blockers):
        blockers.append({'kind': 'private_exposures', 'detail': f"{metrics['private_exposures']} private ID(s) in a trace"})
    if case.get('zero_tolerance') and completed:
        blockers += [{'kind': f"zero_tolerance:{item['type']}", 'detail': item['detail']} for item in results
                     if item['passed'] is False and not item['safety']]
    return blockers


# ---------------------------------------------------------------- reporting

def median_range(values):
    values = [v for v in values if v is not None]
    if not values:
        return None
    return {'median': statistics.median(values), 'min': min(values), 'max': max(values), 'n': len(values)}


REPORT_METRICS = ('first_hit_round', 'target_rank', 'first_target_read_round', 'searches', 'reformulations',
                  'catalog_dumps', 'literal_scans', 'bytes_read', 'wasted_reads', 'trap_reads', 'rounds', 'tokens',
                  'elapsed_ms')


def final_results(results):
    """One result per slot: the labelled rerun supersedes the original, which stays listed as a failure."""
    slots = {}
    for result in results:
        slots.setdefault((result['case'], result['scale'], result['arm'], result['slot']), []).append(result)
    return {key: sorted(items, key=lambda r: r['attempt'])[-1] for key, items in slots.items()}, slots


def build_report(results, cases):
    cases_by_id = {case['id']: case for case in cases}
    finals, slots = final_results(results)
    main = [r for r in finals.values() if r.get('label') != 'diagnostic']
    blockers = []
    for result in results:
        for blocker in result.get('safety', []):
            blockers.append(dict(blocker, case=result['case'], scale=result['scale'], arm=result['arm'],
                                 repetition=result['repetition'], attempt=result['attempt'], label=result.get('label')))
    failures = [{'case': r['case'], 'scale': r['scale'], 'arm': r['arm'], 'repetition': r['repetition'],
                 'attempt': r['attempt'], 'status': r['status'], 'failure': r.get('failure')}
                for r in results if r['status'] not in ('complete', 'running')]
    # A check that cannot be verified on one arm is dropped for every arm of that case comparison, so all
    # arms are judged on the same checks; None is never counted as a pass.
    excluded = {}
    for result in main:
        if result['status'] != 'complete':
            continue
        for item in unverified_checks(result['checks']):
            excluded.setdefault((result['case'], result['scale']), {}).setdefault(
                item['index'], {'type': item['type'], 'step': item['step'], 'arms': []})['arms'].append(result['arm'])
    effective = {}
    for result in main:
        if result['status'] == 'complete':
            skip = set(excluded.get((result['case'], result['scale']), {}))
            effective[(result['case'], result['scale'], result['arm'], result['slot'])] = case_passed(result['checks'], skip)
    passed_of = lambda r: effective.get((r['case'], r['scale'], r['arm'], r['slot']), False)
    unverified_safety = [{'case': r['case'], 'scale': r['scale'], 'arm': r['arm'], 'repetition': r['repetition'],
                          'kind': item['type'], 'detail': item['detail']}
                         for r in results if r['status'] == 'complete' for item in unverified_checks(r['checks'])
                         if item['safety']]
    table = {}
    for result in main:
        cell = table.setdefault((result['case'], result['scale'], result['arm']), [])
        cell.append(result)
    rows = []
    for (case_id, scale, arm), items in sorted(table.items()):
        recorded = [r['planned'] for r in items if r.get('planned')]
        planned = max([len(items)] + (recorded or [case_repetitions(cases_by_id.get(case_id, {}), scale)]))
        done = [r for r in items if r['status'] == 'complete']
        passes = sum(1 for r in done if passed_of(r))
        rows.append({'case': case_id, 'scale': scale, 'arm': arm, 'passes': passes, 'completed': len(done),
                     'planned': planned, 'case_passed': passes * 2 > planned,
                     'excluded': sorted(excluded.get((case_id, scale), {}).values(), key=lambda e: (e['step'] or 0, e['type'])),
                     'metrics': {name: median_range([(r.get('metrics') or {}).get(name) for r in done])
                                 for name in REPORT_METRICS}})
    matrix = {}
    for arm in sorted({row['arm'] for row in rows}):
        arm_rows = [row for row in rows if row['arm'] == arm]
        sample = next(r for r in main if r['arm'] == arm)
        done = [r for r in main if r['arm'] == arm and r['status'] == 'complete']
        matrix[arm] = {
            'provider': sample.get('provider'), 'model': sample.get('model'), 'effort': sample.get('effort'),
            'binary_sha256': sample.get('binary_sha256'), 'cases_passed': sum(row['case_passed'] for row in arm_rows),
            'cases': len(arm_rows), 'samples_passed': sum(1 for r in done if passed_of(r)), 'samples': len(done),
            'transport_failures': sum(1 for f in failures if f['arm'] == arm),
            'safety_blockers': sum(1 for b in blockers if b['arm'] == arm),
            'median': {name: (median_range([(r.get('metrics') or {}).get(name) for r in done]) or {}).get('median')
                       for name in ('bytes_read', 'first_target_read_round', 'rounds', 'searches', 'tokens')}}
    diagnostics = [{'case': r['case'], 'arm': r['arm'], 'repetition': r['repetition'], 'status': r['status'],
                    'passed': r.get('passed'), 'budget_exhausted': (r.get('metrics') or {}).get('budget_exhausted')}
                   for r in finals.values() if r.get('label') == 'diagnostic']
    return {'safety_blockers': blockers, 'unverified_safety': unverified_safety, 'transport_failures': failures, 'cases': rows, 'matrix': matrix,
            'diagnostics': diagnostics,
            'note': 'Small N: report passes out of repetitions and medians with ranges; no p-values are claimed.'}


def format_report(report):
    lines = []
    if report['safety_blockers']:
        lines.append('SAFETY BLOCKERS (any occurrence blocks, regardless of N):')
        for b in report['safety_blockers']:
            lines.append(f"  {b['case']}@{b['scale']} arm {b['arm']} rep {b['repetition']}: {b['kind']} - {b['detail']}")
    else:
        lines.append('Safety blockers: none')
    for u in report.get('unverified_safety', []):
        lines.append(f"  UNVERIFIED safety check {u['kind']} for {u['case']}@{u['scale']} arm {u['arm']} "
                     f"rep {u['repetition']}: {u['detail']}")
    lines.append('')
    lines.append('Per case (passes/planned repetitions; medians):')
    for row in report['cases']:
        metrics = row['metrics']
        median = lambda name: (metrics[name] or {}).get('median')
        lines.append(f"  {row['case']}@{row['scale']} {row['arm']}: {row['passes']}/{row['planned']}"
                     f"{' PASS' if row['case_passed'] else ' fail'} first_read={median('first_target_read_round')}"
                     f" searches={median('searches')} bytes={median('bytes_read')} rounds={median('rounds')}")
        for item in row.get('excluded', []):
            lines.append(f"    excluded: {item['type']}{' (step %s)' % item['step'] if item['step'] else ''} "
                         f"unverifiable on arm {', '.join(sorted(set(item['arms'])))}")
    lines.append('')
    lines.append('Model matrix (by arm):')
    header = ['arm', 'provider', 'model', 'effort', 'cases', 'samples', 'bytes', 'first_read', 'rounds', 'searches',
              'tokens', 'transport', 'safety']
    lines.append('  ' + ' | '.join(header))
    for arm, row in report['matrix'].items():
        m = row['median']
        lines.append('  ' + ' | '.join(str(x) for x in [
            arm, row['provider'], row['model'], row['effort'], f"{row['cases_passed']}/{row['cases']}",
            f"{row['samples_passed']}/{row['samples']}", m['bytes_read'], m['first_target_read_round'], m['rounds'],
            m['searches'], m['tokens'], row['transport_failures'], row['safety_blockers']]))
    if report['transport_failures']:
        lines.append('')
        lines.append('Transport/setup failures (never counted as passes):')
        for f in report['transport_failures']:
            lines.append(f"  {f['case']}@{f['scale']} {f['arm']} rep {f['repetition']} attempt {f['attempt']}: {f['status']} {f['failure']}")
    if report['diagnostics']:
        lines.append('')
        lines.append('Diagnostics (not comparison arms):')
        for d in report['diagnostics']:
            lines.append(f"  {d['case']} {d['arm']} #{d['repetition']}: {d['status']} passed={d['passed']} budget_exhausted={d['budget_exhausted']}")
    lines += ['', report['note']]
    return '\n'.join(lines)


# ---------------------------------------------------------------- filesystem and process plumbing

def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def copy_tree(source, destination):
    """Copy-on-write clone on APFS (`cp -c`), plain copy elsewhere. Destination must not exist."""
    destination = Path(destination)
    if destination.exists():
        raise FileExistsError(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if sys.platform == 'darwin':
        done = subprocess.run(['cp', '-c', '-R', str(source), str(destination)], capture_output=True)
        if done.returncode == 0:
            return 'clone'
        shutil.rmtree(destination, ignore_errors=True)
    shutil.copytree(source, destination, symlinks=True)
    return 'copy'


def capabilities(binary):
    """What the arm binary's own help text offers; the help prints without opening any workspace."""
    try:
        text = subprocess.run([str(binary), '--help'], capture_output=True, text=True, timeout=20).stdout
    except (OSError, subprocess.TimeoutExpired):
        text = ''
    return {'trace': '--trace' in text, 'knowledge-check': 'knowledge-check' in text,
            'search': '\n  search ' in text or 'search --query' in text}


def root_tmp(root):
    """Scratch directory under the evaluation root, so converter temp files never leave it."""
    path = Path(root) / 'tmp'
    path.mkdir(mode=0o700, exist_ok=True)
    return path


def arm_environment(arm, helper, tmp=None):
    environment = dict(os.environ, BRN_INTAKE_HELPER=str(helper))
    if tmp:
        environment.update(TMPDIR=str(tmp), TMP=str(tmp), TEMP=str(tmp))
    environment.pop('BRN_EMBEDDING_MODEL_DIR', None)  # Only an arm's own model-dir may set it.
    if arm.get('model_dir'):
        environment['BRN_EMBEDDING_MODEL_DIR'] = str(arm['model_dir'])
    return environment


def make_cli(arm, binary, data, root, args, timeout=None):
    helper = args.helper or Path(binary).with_name('brn-intake-helper')
    cli = V1.Cli(Path(binary), data, root, helper, timeout or args.timeout)
    cli.environment = arm_environment(arm, helper, root_tmp(root))
    return cli


def run_seeder(args, root, case, scale, destination):
    command = [str(args.seeder), '--data-dir', str(destination), '--fixture', str(args.fixture),
               '--case', case['id'], '--filler', str(scale), '--seed', str(args.seed)]
    done = subprocess.run(command, capture_output=True, text=True, timeout=args.seed_timeout)
    if done.returncode:
        raise CallFailed(f'seeder failed: {done.stderr.strip()[-400:]}')
    try:
        seed = json.loads(done.stdout)
    except json.JSONDecodeError:
        raise CallFailed('seeder printed invalid JSON') from None
    return seed


def validate_seed(case, seed):
    problems = []
    notes, distractors, threads = expand_seed_keys(case)
    problems += [f'seeder omitted note {key}' for key in notes + distractors if key not in seed.get('notes', {})]
    problems += [f'seeder omitted thread {key}' for key in threads if key not in seed.get('threads', {})]
    return problems


# ---------------------------------------------------------------- one sample

class Sample:
    """Runs one arm in one copied template and returns a JSON-serialisable result."""

    def __init__(self, args, root, case, scale, sample, arm, template, seed, caps):
        self.args, self.root, self.case, self.scale, self.sample = args, root, case, scale, sample
        self.arm, self.template, self.seed, self.caps = arm, template, seed, caps
        self.attempt = sample.get('attempt', 1)
        self.folder = slot_folder(root, dict(sample, case=case['id'], scale=scale), self.attempt)
        self.result = {
            'format': 2, 'case': case['id'], 'scale': scale, 'arm': arm['name'], 'repetition': sample['repetition'],
            'slot': slot_name(sample), 'attempt': self.attempt, 'label': sample['label'] or ('rerun' if self.attempt > 1 else None),
            'status': 'running', 'failure': None, 'binary_sha256': arm['binary_sha256'], 'provider': arm['provider'],
            'model': arm['model'], 'effort': arm['effort'], 'model_dir': arm.get('model_dir'),
            'trace_level': 'full' if caps['trace'] else 'off', 'steps': [], 'checks': [], 'metrics': {},
            'safety': [], 'passed': None, 'commands': [], 'planned': sample.get('planned')}

    def persist(self):
        self.result['commands'] = self.cli.commands[-400:]
        (self.folder / 'result.json').write_text(
            json.dumps(public_snapshot(self.result), ensure_ascii=False, indent=2) + '\n')

    def measure(self, command, **options):
        """Knowledge check is a measuring instrument: it uses --measure-bin on a clone whenever one is given, so
        every arm is measured alike. Search ranks measure the arm's own retrieval and use --measure-bin only
        when the arm lacks search."""
        if not self.args.measure_bin or (command != 'knowledge-check' and self.caps.get('search')):
            return self.cli.call(command, **options)
        clone = self.folder / 'measure'
        if clone.exists():
            shutil.rmtree(clone)
        copy_tree(self.cli.data, clone / 'data')
        helper = self.args.helper or Path(self.args.measure_bin).with_name('brn-intake-helper')
        cli = V1.Cli(Path(self.args.measure_bin), clone / 'data', self.root, helper, self.args.timeout)
        cli.environment = arm_environment(self.arm, helper, root_tmp(self.root))
        try:
            return cli.call(command, **options)
        finally:
            self.cli.commands.extend(cli.commands)

    def capture_kc(self):
        check = self.measure('knowledge-check')
        return {'notes': check.get('notes'), 'links': check.get('links'), 'counts': check.get('counts', {}),
                'findings': check.get('findings', [])}

    def capture_ranks(self, queries):
        ranks = {}
        for query in queries:
            try:
                page = self.measure('search', query=query, limit=20)
                ranks[query] = [{'id': hit['note'], 'title': hit.get('title', '')} for hit in page.get('hits', [])]
            except CallFailed:
                ranks[query] = []
        return ranks

    def go(self):
        if self.folder.exists():
            shutil.rmtree(self.folder)  # A crashed earlier attempt of this very slot, possibly without a result.
        self.folder.mkdir(parents=True)
        data = self.folder / 'data'
        self.cli = make_cli(self.arm, self.arm['binary'], data, self.root, self.args)
        self.persist()  # A `running` stub, so a crash is recognised on --resume.
        try:
            self.run()
        except CallFailed as error:
            self.result['status'] = 'transport_failure' if self.result.get('stage') == 'steps' else 'setup_failure'
            self.result['failure'] = str(error)
        self.persist()
        return self.result

    def run(self):
        result, args, case = self.result, self.args, self.case
        result['stage'] = 'setup'
        result['copy'] = copy_tree(self.template / 'data', self.cli.data)
        self.persist()
        if args.live:
            configuration = {'provider': self.arm['provider'], 'model': self.arm['model'], 'effort': self.arm['effort']}
            configuration.update(credential_options(self.arm['provider'], args.auth_file, args.credentials_dir))
            self.cli.call('configure', **configuration)
        if not self.cli.call('settings').get('delegation_chosen'):
            self.cli.call('delegation', choice='maintain')
        needed = needed_states(case)
        queries = {}
        for check in case['checks']:
            if check['type'] == 'search_rank':
                queries.setdefault(check.get('after_step', len(case['steps'])), []).extend(check['queries'])
        raw0 = self.cli.call('records')
        seed_ids = set(self.seed['notes'].values()) | {self.seed.get('topics')}
        baseline = filler_baseline(raw0, seed_ids)
        states = {0: {'records': condense_records(raw0, baseline), 'kc': self.capture_kc()}}
        del raw0
        result['stage'] = 'steps'
        threads = dict(self.seed['threads'])
        steps = []
        raw_final = None
        for number, step in enumerate(case['steps'], 1):
            record = {'index': number, 'kind': step['kind'], 'thread': step.get('thread'), 'terminal': None,
                      'answer': None, 'trace': None, 'operations': [], 'sources': [], 'loaded_skills': []}
            steps.append(record)
            result['steps'] = steps
            outcome = self.run_step(number, step, threads, record)
            self.persist()
            if not evaluated(record['terminal']):
                detail = failure_summary(record.get('failure'))
                raise CallFailed(f'step {number} ended {record["terminal"]}' + (f' ({detail})' if detail else ''))
            if number in needed:
                raw = self.cli.call('records')
                states[number] = {'records': condense_records(raw, baseline), 'kc': self.capture_kc()}
                if queries.get(number):
                    states[number]['ranks'] = self.capture_ranks(dict.fromkeys(queries[number]))
                if number == len(case['steps']):
                    raw_final = raw
        receipts, lineage = [], []
        if any(item['operations'] for item in steps):
            operations = {op for item in steps for op in item['operations']}
            receipts = [r for r in self.cli.call('history') if r['operation'] in operations]
            lineage = V1.read_operation_inputs(self.cli.data, operations)
        for item in steps:
            ops = set(item['operations'])
            item['receipts'] = [condense_receipt(r) for r in receipts if r['operation'] in ops]
        undo = self.run_undo(case, steps, receipts, raw_final)
        self.finish(states, steps, lineage, undo)

    def run_step(self, number, step, threads, record):
        args, cli = self.args, self.cli
        budget = step.get('budget', 8)
        if self.sample.get('budget_step') == number:
            budget = self.sample['budget']
        trace = {'trace': 'full'} if self.caps['trace'] else {}
        if step['kind'] == 'knowledge-review':
            out = cli.call('knowledge-check', seed_chat='yes')
            threads[step.get('as', 'review')] = out['review_chat']
            record['terminal'] = 'Completed'
            return out
        thread = threads[step['thread']]
        if not args.live:
            raise CallFailed('provider steps need --live')
        if step['kind'] == 'ask':
            out = cli.call('ask', thread=thread, prompt=step['prompt'], budget=budget, **trace)
        else:
            source = self.folder / f'step-{number}.md'
            source.write_text(step['text'])
            options = dict(source=source, intent=step.get('intent', 'useful'), thread=thread, **trace)
            if step.get('title'):
                options['title'] = step['title']
            out = cli.call('intake', **options)
            if 'outcome' in out:
                record['terminal'] = 'Completed' if out['outcome'] == 'Applied' else f"Outcome:{out['outcome']}"
                return out
        record['terminal'] = out.get('terminal')
        if out.get('failure'):
            record['failure'] = out['failure']
        record['answer'] = out.get('answer')
        record['trace'] = out.get('trace')
        record['run'] = out.get('run')
        run = cli.call('read', id=out['run']) if out.get('run') else None
        details = ((run or {}).get('data') or {}).get('Run') or {}
        record['operations'] = list(details.get('operations', []))
        record['sources'] = list(details.get('sources', []))
        record['loaded_skills'] = list(details.get('loaded_skills', []))
        return out

    def run_undo(self, case, steps, receipts, raw_final):
        results = {}
        for check in case['checks']:
            if check['type'] != 'undo' or check.get('step') in results:
                continue
            number = check.get('step')
            pool = [step for index, step in enumerate(steps, 1) if number in (None, index)]
            ops = {op for step in pool for op in step['operations']}
            current = V1.by_id(raw_final or self.cli.call('records'))
            eligible = [r for r in receipts if r['operation'] in ops
                        and any('Note' in w['after']['data'] for w in r['writes'])
                        and all(current.get(w['after']['id'], {}).get('version') == w['after']['version']
                                for w in r['writes'])]
            if not eligible:
                results[number] = {'status': 'not_run', 'reason': 'No eligible current applied note group'}
                continue
            receipt = sorted(eligible, key=lambda r: r['committed_at'])[-1]
            before = self.cli.call('records')
            outcome = self.cli.call('undo', operation=receipt['operation'])
            after = self.cli.call('records')
            affected = {w['after']['id'] for w in receipt['writes']}
            results[number] = {'status': outcome['outcome'], 'operation': receipt['operation'],
                               'affected_restored': outcome['outcome'] == 'Applied' and verify_undo(receipt, after),
                               'unrelated_preserved': V1.unrelated_preserved(before, after, affected)}
        return results

    def finish(self, states, steps, lineage, undo):
        case, result, seed = self.case, self.result, self.seed
        keys = lambda names: {seed['notes'].get(k) or seed['threads'].get(k) for k in names}
        private_keys = case.get('private', [])
        private_ids = keys(private_keys)
        chats = {note.get('chat') for note in case['seeds'].get('notes', []) if note['key'] in private_keys}
        ctx = {'case': case, 'seed': seed, 'arm': self.arm['name'], 'arm_base': arm_base(self.arm['name']),
               'steps': steps, 'states': states, 'lineage': lineage, 'undo': undo,
               'private_ids': private_ids, 'private_threads': chats - {None}}
        per_step = [step_metrics(step['trace'], keys(case.get('targets', [])), keys(case.get('useful', [])),
                                 keys(case.get('traps', [])), private_ids, step['kind'], step['terminal'])
                    for step in steps if step['kind'] != 'knowledge-review'
                    and step.get('thread') not in ctx['private_threads']]
        result['metrics'] = run_metrics(per_step)
        result['checks'] = evaluate_checks(case, ctx)
        result['undo'] = undo
        result['kc'] = {'before': states[0]['kc'], 'after': states[max(states)]['kc']}
        result['loaded_skills'] = sorted({s for step in steps for s in step['loaded_skills']})
        result['status'] = 'complete'
        result['unverified'] = [item['type'] for item in unverified_checks(result['checks'])]
        result['passed'] = case_passed(result['checks'])  # Provisional; the report re-derives it per comparison.
        result['safety'] = safety_blockers(case, result['checks'], result['metrics'])
        result['stage'] = 'complete'


def needed_states(case):
    last = len(case['steps'])
    needed = {last}
    for check in case['checks']:
        number = check.get('after_step', check.get('step'))
        if check['type'] in ('read', 'read_range', 'answer_contains_any', 'answer_excludes', 'no_writes',
                             'private_exposures', 'undo', 'sources_linked', 'relation_share'):
            number = check.get('after_step') if check['type'] != 'read' else None
        if number:
            needed.add(number)
        if check['type'] in ('read', 'read_range', 'protected_preserved', 'marker_absent_general'):
            needed.add(last)
    return needed


# ---------------------------------------------------------------- orchestration

def read_json(path):
    return json.loads(Path(path).read_text())


def slot_attempts(root, sample):
    base = root / 'runs' / f'{sample["case"]}-{sample["scale"]}' / slot_name(sample)
    out = []
    for folder in (base, base.with_name(base.name + '-rerun')):
        if (folder / 'result.json').is_file():
            out.append(read_json(folder / 'result.json'))
    return out


def collect_results(root):
    results = []
    for path in sorted((root / 'runs').glob('*/*/result.json')) if (root / 'runs').is_dir() else []:
        results.append(read_json(path))
    return results


def open_root(root):
    root = Path(root)
    marker = root / MARKER
    if not root.is_absolute() or root.is_symlink() or not root.is_dir() or not marker.is_file():
        raise ValueError('--resume needs an existing evaluation root created by this harness')
    if root.stat().st_uid != os.getuid():
        raise ValueError('Evaluation root is not owned by this user')
    return root


def manifest_for(args, arms, fixture_sha, cases):
    return {'format': 2, 'fixture_sha256': fixture_sha, 'seed': args.seed, 'live': args.live,
            'repetitions': args.repetitions, 'scale': args.scale,
            'cases': [case['id'] for case in cases],
            'arms': [{key: arm[key] for key in ('name', 'binary_sha256', 'provider', 'model', 'effort', 'model_dir')}
                     for arm in arms]}


def ensure_template(args, root, case, scale, log):
    folder = root / 'templates' / f'{case["id"]}-{scale}'
    seed_file = folder / 'seed.json'
    if seed_file.is_file() and (folder / 'seed' / 'data').is_dir():
        return folder, read_json(seed_file)
    if folder.exists():
        shutil.rmtree(folder)
    (folder / 'seed').mkdir(parents=True)
    started = time.monotonic()
    seed = run_seeder(args, root, case, scale, folder / 'seed' / 'data')
    problems = validate_seed(case, seed)
    if problems:
        raise CallFailed('; '.join(problems[:5]))
    seed_file.write_text(json.dumps(seed, indent=2) + '\n')
    log(f'seeded {case["id"]}@{scale} in {time.monotonic() - started:.1f}s')
    return folder, seed


def ensure_warm(args, root, folder, arm, caps, log):
    """A copy of the seeded template opened once by this arm's binary and model folder, so every
    sample starts with its index built (and, for model-dir arms, the dense lane complete)."""
    key = digest(f"{arm['binary_sha256']}|{arm.get('model_dir')}")
    warm = folder / f'warm-{key}'
    if (warm / 'ready').is_file():
        return warm
    if warm.exists():
        shutil.rmtree(warm)
    warm.mkdir()
    copy_tree(folder / 'seed' / 'data', warm / 'data')
    cli = make_cli(arm, arm['binary'], warm / 'data', root, args, timeout=max(args.timeout, 60))
    deadline = time.monotonic() + args.prewarm_timeout
    if not caps['search']:
        cli.call('records')
    else:
        if arm.get('model_dir'):
            # The CLI never waits for idle-time embedding; the seeder's native
            # warm mode embeds every passage in-process and persists vectors.
            done = subprocess.run([str(args.seeder), '--warm', str(warm / 'data')], capture_output=True,
                                  text=True, env=cli.environment,
                                  timeout=max(1, deadline - time.monotonic()))
            if done.returncode:
                raise CallFailed(f'semantic prewarm failed: {done.stderr.strip()[-400:]}')
        page = cli.call('search', query='warm up', limit=1)
        semantic = page.get('semantic', 'none')
        if arm.get('model_dir') and semantic != 'complete':
            raise CallFailed(f'semantic prewarm did not complete: {semantic}')
    (warm / 'ready').write_text(semantic if caps['search'] else 'records\n')
    log(f'warmed {folder.name} for arm {arm["name"]}')
    return warm


def offline_checks(args, root, case, scale, folder, seed, arms, caps_by_arm, log):
    """Per case and scale: template copy, Knowledge check, and each arm binary opening a copy."""
    report = {'case': case['id'], 'scale': scale, 'seed_keys': len(seed['notes']), 'arms': {}}
    for arm in arms:
        caps = caps_by_arm[arm['name']]
        entry = {'capabilities': caps}
        try:
            warm = ensure_warm(args, root, folder, arm, caps, log)
            check = root / 'offline' / f'{case["id"]}-{scale}' / arm['name']
            if check.exists():
                shutil.rmtree(check)
            copy_tree(warm / 'data', check / 'data')
            cli = make_cli(arm, arm['binary'], check / 'data', root, args)
            records = cli.call('records')
            entry['records'] = len(records)
            kc_cli = cli if caps['knowledge-check'] else (
                make_cli(arm, args.measure_bin, check / 'data', root, args) if args.measure_bin else None)
            if kc_cli:
                kc = kc_cli.call('knowledge-check')
                entry['knowledge_check'] = kc.get('counts', {})
            else:
                entry['knowledge_check'] = 'unavailable: arm binary lacks knowledge-check; pass --measure-bin'
            entry['ok'] = True
        except (CallFailed, OSError) as error:
            entry.update(ok=False, error=str(error))
        report['arms'][arm['name']] = entry
    return report


def prepare_arms(args, parser):
    arms = []
    for spec in parse_arms(args.arm):
        arm = resolve_arm(spec, args.provider, args.model, args.effort)
        binary = Path(arm['binary'])
        if not binary.is_absolute() or not binary.is_file() or not os.access(binary, os.X_OK):
            parser.error(f'Arm {arm["name"]}: binary must be an existing absolute executable file')
        if arm['model_dir'] and not Path(arm['model_dir']).is_absolute():
            parser.error(f'Arm {arm["name"]}: model-dir must be absolute')
        if args.live:
            if not (arm['provider'] and arm['model'] and arm['effort']):
                parser.error(f'Arm {arm["name"]}: --live needs provider, model and effort (arm option or global)')
            if arm['provider'] == 'Copilot' and not (args.credentials_dir and args.credentials_dir.is_absolute()) :
                parser.error(f'Arm {arm["name"]}: Copilot needs an absolute --credentials-dir')
            if arm['provider'] == 'Codex' and args.credentials_dir and not args.auth_file:
                parser.error(f'Arm {arm["name"]}: Codex uses --auth-file, not --credentials-dir')
        arm['binary_sha256'] = sha256_file(binary)
        arms.append(arm)
    return arms


def build_parser():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--arm', action='append', default=[], metavar='NAME=BINARY[,option=value]',
                        help='repeatable; options: model-dir, provider, model, effort')
    parser.add_argument('--seeder', type=Path, help='built wiki_eval_seed example binary')
    parser.add_argument('--fixture', type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument('--cases', help='comma-separated case IDs (default: all)')
    parser.add_argument('--scale', type=int, choices=[1000, 10000], help='only this filler scale')
    parser.add_argument('--repetitions', type=int, choices=[1, 2, 3], help='override the cases\' repetitions')
    parser.add_argument('--root', type=Path, required=True, help='absolute fresh directory (or existing with --resume)')
    parser.add_argument('--resume', action='store_true', help='reuse --root: skip complete samples, rerun a transport failure once')
    parser.add_argument('--live', action='store_true', help='run the owner-selected provider; otherwise offline validation only')
    parser.add_argument('--report-only', action='store_true', help='re-render the report from --root and exit')
    parser.add_argument('--provider', choices=PROVIDERS)
    parser.add_argument('--model')
    parser.add_argument('--effort', choices=EFFORTS)
    parser.add_argument('--auth-file', type=Path)
    parser.add_argument('--credentials-dir', type=Path)
    parser.add_argument('--timeout', type=int, default=180, help='seconds per brn command (1..600)')
    parser.add_argument('--seed', type=int, default=1, help='filler seed passed to the seeder')
    parser.add_argument('--helper', type=Path, help='brn-intake-helper (default: next to each binary)')
    parser.add_argument('--measure-bin', type=Path, help='binary for every knowledge-check, and for search when an arm binary lacks it (runs on a clone)')
    parser.add_argument('--prewarm-timeout', type=int, default=1800)
    parser.add_argument('--seed-timeout', type=int, default=1800)
    return parser


def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    if not args.root.is_absolute():
        parser.error('--root must be absolute')
    args.root = args.root.resolve()  # brn needs canonical paths (macOS /var -> /private/var)
    if args.auth_file and not args.auth_file.is_absolute():
        parser.error('Auth profile path must be absolute')
    if not 1 <= args.timeout <= 600:
        parser.error('Timeout must be 1..600 seconds')
    fixture = read_json(args.fixture)
    problems = validate_fixture(fixture)
    if problems:
        print('\n'.join(['Fixture problems:'] + problems), file=sys.stderr)
        return 2
    wanted = set(args.cases.split(',')) if args.cases else None
    cases = [case for case in fixture['cases'] if wanted is None or case['id'] in wanted]
    if wanted and {case['id'] for case in cases} != wanted:
        parser.error('Unknown case id in --cases')
    if args.report_only:
        root = open_root(args.root)
        report = build_report(collect_results(root), cases)
        print(format_report(report))
        return 0
    if not args.arm or not args.seeder:
        parser.error('--arm (at least one) and --seeder are required')
    if not (args.seeder.is_absolute() and args.seeder.is_file() and os.access(args.seeder, os.X_OK)):
        parser.error('--seeder must be an existing absolute executable file')
    arms = prepare_arms(args, parser)
    for case in cases:
        if not case_arm_names(case, [a['name'] for a in arms]):
            parser.error(f'No arm matches case {case["id"]} (arms {case.get("arms")})')
    caps_by_arm = {arm['name']: capabilities(arm['binary']) for arm in arms}
    live_missing = [name for name, caps in caps_by_arm.items()
                    if not (caps['knowledge-check'] or args.measure_bin) or not (caps['search'] or args.measure_bin)]
    if live_missing and args.live:
        parser.error(f'Arms {live_missing} lack knowledge-check/search; pass --measure-bin')
    fixture_sha = sha256_file(args.fixture)
    manifest = manifest_for(args, arms, fixture_sha, cases)
    if args.resume:
        root = open_root(args.root)
        previous = read_json(root / 'manifest.json')
        if {k: previous[k] for k in ('fixture_sha256', 'seed', 'arms', 'live')} != \
                {k: manifest[k] for k in ('fixture_sha256', 'seed', 'arms', 'live')}:
            parser.error('--resume: fixture, seed, mode or arms differ from the stored manifest')
    else:
        create_root(args.root)
        root = args.root
        root_tmp(root)
        (root / 'manifest.json').write_text(json.dumps(public_snapshot(manifest), indent=2) + '\n')
    log = lambda message: print(json.dumps({'event': message}), flush=True)
    if not args.live:
        return offline(args, root, cases, arms, caps_by_arm, log)
    return live(args, root, cases, arms, caps_by_arm, log)


def offline(args, root, cases, arms, caps_by_arm, log):
    reports, failed = [], False
    for case in cases:
        for scale in case_scales(case):
            if args.scale not in (None, scale):
                continue
            try:
                folder, seed = ensure_template(args, root, case, scale, log)
            except CallFailed as error:
                reports.append({'case': case['id'], 'scale': scale, 'error': str(error)})
                failed = True
                continue
            names = case_arm_names(case, [a['name'] for a in arms])
            report = offline_checks(args, root, case, scale, folder, seed,
                                    [a for a in arms if a['name'] in names], caps_by_arm, log)
            failed |= not all(entry['ok'] for entry in report['arms'].values())
            reports.append(report)
    (root / 'offline.json').write_text(json.dumps(public_snapshot(reports), indent=2, ensure_ascii=False) + '\n')
    print(json.dumps({'mode': 'offline', 'fixture': 'valid', 'checked': len(reports), 'ok': not failed,
                      'report': str(root / 'offline.json')}), flush=True)
    return int(failed)


def persist_outcome(root, sample, attempt, outcome):
    """Writes a result that never reached a Sample into its slot, so reports and --resume see it."""
    folder = slot_folder(root, sample, attempt)
    if folder.exists():
        shutil.rmtree(folder)
    folder.mkdir(parents=True)
    (folder / 'result.json').write_text(json.dumps(public_snapshot(outcome), ensure_ascii=False, indent=2) + '\n')


def live(args, root, cases, arms, caps_by_arm, log):
    by_name = {arm['name']: arm for arm in arms}
    cases_by_id = {case['id']: case for case in cases}
    plan = plan_samples(cases, [arm['name'] for arm in arms], args.scale, args.repetitions)
    templates = {}
    for sample in plan:
        case, arm = cases_by_id[sample['case']], by_name[sample['arm']]
        attempts = slot_attempts(root, sample)
        action = resume_action(attempts)
        if action == 'skip' or (action == 'rerun' and not args.resume):
            continue
        folder_key = (case['id'], sample['scale'])
        attempt = next_attempt(attempts)
        outcome = None
        try:
            if folder_key not in templates:
                templates[folder_key] = ensure_template(args, root, case, sample['scale'], log)
            folder, seed = templates[folder_key]
            warm = ensure_warm(args, root, folder, arm, caps_by_arm[arm['name']], log)
        except CallFailed as error:
            outcome = {'format': 2, 'case': case['id'], 'scale': sample['scale'], 'arm': arm['name'],
                       'repetition': sample['repetition'], 'slot': slot_name(sample), 'attempt': attempt,
                       'label': sample['label'] or ('rerun' if attempt > 1 else None), 'status': 'setup_failure',
                       'failure': str(error), 'planned': sample.get('planned'),
                       'binary_sha256': arm['binary_sha256'], 'provider': arm['provider'], 'model': arm['model'],
                       'effort': arm['effort'], 'safety': [], 'metrics': {}, 'checks': [], 'passed': None}
            persist_outcome(root, sample, attempt, outcome)
        if outcome is None:
            run_sample = dict(sample, attempt=attempt)
            outcome = Sample(args, root, case, sample['scale'], run_sample, arm, warm, seed,
                             caps_by_arm[arm['name']]).go()
        log(f"{outcome['case']}@{outcome['scale']} {outcome['arm']} {outcome['slot']} attempt {outcome['attempt']}: "
            f"{outcome['status']} passed={outcome.get('passed')}")
        report = build_report(collect_results(root), cases)
        (root / 'report.json').write_text(json.dumps(public_snapshot(report), indent=2, ensure_ascii=False) + '\n')
    report = build_report(collect_results(root), cases)
    (root / 'report.json').write_text(json.dumps(public_snapshot(report), indent=2, ensure_ascii=False) + '\n')
    print(format_report(report))
    return int(bool(report['safety_blockers']))


if __name__ == '__main__':
    raise SystemExit(main())
