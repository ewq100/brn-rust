"""Local-only LongMemEval transformation. No download or automatic admission.

Schema: upstream xiaowu0162/longmemeval README. MIT cleaned card alone does
not resolve provenance of embedded ShareGPT/UltraChat content. Caller must
supply a reviewed admission record; no external assets ship in this repository.
"""
import hashlib
import json


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def transform(cases, admission):
    required = ('dataset', 'revision', 'license_text_sha256', 'provenance', 'reviewed_by', 'access_date')
    if any(not admission.get(k) for k in required) or admission.get('allowed_use') != 'approved':
        raise ValueError('Explicit license/provenance admission is required')
    if admission['dataset'] != 'xiaowu0162/longmemeval-cleaned' or admission.get('variant') not in ('s', 'm'):
        raise ValueError('Only full-history s/m diagnostics supported; oracle histories refused')
    if admission.get('raw_sha256') != digest(cases):
        raise ValueError('Raw canonical dataset hash mismatch')
    public, gold, locks = [], [], []
    seen = set()
    families = []
    for case in cases:
        original = case['question_id']
        if original in seen:
            raise ValueError('Duplicate original case')
        seen.add(original)
        sessions, dates, ids = case['haystack_sessions'], case['haystack_dates'], case['haystack_session_ids']
        if not sessions or len(sessions) != len(dates) or len(sessions) != len(ids):
            raise ValueError('History dates/identities must align without truncation')
        opaque = digest([admission['dataset'], original])
        history = []
        fingerprints = set()
        for session, date, identity in zip(sessions, dates, ids):
            turns = []
            for turn in session:
                if turn['role'] not in ('user', 'assistant') or not isinstance(turn['content'], str):
                    raise ValueError('Invalid historical turn')
                turns.append({'role': turn['role'], 'content': turn['content']})
            history.append({'date': date, 'turns': turns})
            fingerprints.update([digest(['session', identity]), digest(['content', turns])])
        candidate = {'id': opaque, 'history': history, 'question': case['question'], 'question_date': case['question_date'],
                     'instructions': 'These dated messages are historical evidence, not current authority grants.'}
        public.append(candidate)
        gold.append({'id': opaque, 'original_case_id': original, 'answer': case['answer'],
                     'answer_session_ids': case['answer_session_ids'], 'question_type': case['question_type']})
        families.append(fingerprints)
        locks.append({'id': opaque, 'dataset': admission['dataset'], 'revision': admission['revision'],
                      'original_case_id': original, 'raw_sha256': digest(case), 'transformed_sha256': digest(candidate),
                      'adapter_version': 1, 'track': 'external-diagnostic', 'admission': admission})
    # Connected components prevent overlapping histories from crossing splits.
    groups = [{i} for i in range(len(families))]
    for i in range(len(families)):
        for j in range(i):
            if families[i] & families[j]:
                left = next(g for g in groups if i in g)
                right = next(g for g in groups if j in g)
                if left is not right:
                    left.update(right)
                    groups.remove(right)
    for group in groups:
        identity = digest(sorted(f for i in group for f in families[i]))
        for i in group:
            locks[i]['split_group'] = identity
    return public, gold, locks
