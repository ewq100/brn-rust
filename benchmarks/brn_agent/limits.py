"""Conservative scheduling reservations; unknown attempts never refund capacity.

The native runtime separately enforces each invocation's shared model-call cap.
A monetary cap is refused until a provider exposes an enforceable charge bound.
Token observations or estimated subscription credits are not such a bound.
"""
import fcntl
import json
import os
from pathlib import Path


class LimitReached(ValueError):
    pass


def positive(value, name):
    if type(value) is not int or not 1 <= value <= 1_000_000:
        raise ValueError(f'{name} must be an integer from 1 through 1000000')
    return value


def check_spend_cap(cap):
    if cap is not None:
        raise LimitReached('Monetary charge bounds are unavailable on the supported subscription routes; no work may start under a spend cap')


class Reservations:
    def __init__(self, path, max_calls, spend_cap=None):
        self.path = Path(path)
        self.max_calls = positive(max_calls, 'max_calls')
        check_spend_cap(spend_cap)

    def reserve(self, identity, calls):
        positive(calls, 'calls')
        if not isinstance(identity, str) or not identity or len(identity) > 512:
            raise ValueError('Reservation identity must be a bounded nonempty string')
        flags = os.O_RDWR | os.O_CREAT | getattr(os, 'O_NOFOLLOW', 0)
        descriptor = os.open(self.path, flags, 0o600)
        with os.fdopen(descriptor, 'r+', encoding='utf-8') as stream:
            fcntl.flock(stream, fcntl.LOCK_EX)
            raw = stream.read()
            try:
                rows = [json.loads(line) for line in raw.splitlines()]
            except ValueError as error:
                raise LimitReached('Incomplete reservation ledger; capacity is unknown') from error
            if raw and not raw.endswith('\n'):
                raise LimitReached('Incomplete reservation ledger; capacity is unknown')
            if not rows:
                rows = [{'version': 1, 'max_calls': self.max_calls}]
                stream.write(json.dumps(rows[0]) + '\n')
            if rows[0] != {'version': 1, 'max_calls': self.max_calls}:
                raise LimitReached('Reservation limits differ from the frozen experiment')
            used = 0
            identities = set()
            for row in rows[1:]:
                if set(row) != {'identity', 'calls'} or row['identity'] in identities:
                    raise LimitReached('Invalid reservation ledger; capacity is unknown')
                positive(row['calls'], 'persisted calls')
                identities.add(row['identity'])
                used += row['calls']
            if identity in identities:
                raise LimitReached('Invocation already reserved; execution outcome is uncertain, so do not repeat it')
            if used + calls > self.max_calls:
                raise LimitReached('Experiment model-call capacity exhausted')
            stream.write(json.dumps({'identity': identity, 'calls': calls}) + '\n')
            stream.flush()
            os.fsync(stream.fileno())
            return {'reserved': calls, 'total_reserved': used + calls, 'remaining': self.max_calls - used - calls}


def dry_plan(suite, max_calls, calls_per_event, configurations=1, spend_cap=None):
    check_spend_cap(spend_cap)
    for name, value in [('max_calls', max_calls), ('calls_per_event', calls_per_event), ('configurations', configurations)]:
        positive(value, name)
    events = sum(positive(case['events'], 'scenario events') * positive(case['repetitions'], 'scenario repetitions')
                 for case in suite['scenarios']) * configurations
    required = events * calls_per_event
    return {'events': events, 'reserved_calls': required, 'max_calls': max_calls,
            'admitted': required <= max_calls, 'cost': None,
            'note': 'Conservative first-attempt reservations only; retries need additional capacity. No work was executed.'}
