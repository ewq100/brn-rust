"""Automatic checks are evidence, separate from execution and reviewed quality."""
REVISION = 'automatic-checks-v2'


def summarize(checks):
    counts = {'passed': 0, 'failed': 0, 'unverified': 0, 'absent': 0}
    applicable = [c for c in checks if c.get('applicable', True) and not c.get('report_only')]
    for check in applicable:
        counts['passed' if check.get('passed') is True else
               'failed' if check.get('passed') is False else 'unverified'] += 1
    status = ('failed' if counts['failed'] else 'unverified' if counts['unverified'] else
              'passed' if applicable else 'absent')
    return dict(counts, applicable=len(applicable), status=status,
                display='not evaluated' if status == 'absent' else status)
