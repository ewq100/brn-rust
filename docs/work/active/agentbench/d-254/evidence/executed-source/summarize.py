"""Describe retained native observations; missing quality stays null."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics


def describe(root):
    runs = []
    for path in sorted((root/'runs').glob('*/*/result.json')):
        raw = json.loads(path.read_text())
        events = [event for event in raw['steps'] if event.get('kind') in ('ask','intake')]
        traces = [event.get('trace') for event in events]
        complete = all(trace is not None for trace in traces)
        traces = [trace for trace in traces if trace is not None]
        usage = [trace.get('usage') for trace in traces]
        usage_complete = complete and all(item is not None for item in usage)
        failed_checks = [item for item in raw['checks'] if item.get('passed') is False]
        runs.append({'case':raw['case'], 'arm':raw['arm'], 'repetition':raw['repetition'],
            'model':raw['model'], 'effort':raw['effort'], 'status':raw['status'],
            'mechanical_pass':raw['passed'], 'failed_checks':failed_checks,
            'terminals':[event.get('terminal') for event in events],
            'completed_events':sum(event.get('terminal')=='Completed' for event in events),
            'events':len(events), 'safety':raw['safety'],
            'reported_elapsed_seconds':sum(trace['elapsed_ms'] for trace in traces)/1000 if complete else None,
            'HTTP_started':sum(trace['model_call_budget']['started'] for trace in traces) if complete else None,
            'reported_usage':{key:sum(item.get(key,0) for item in usage)
                for key in ('input','output','cached_input')} if usage_complete else None,
            'result':str(path.relative_to(root)),
            'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    reviewed = root/'readers-reviewed/private-result.json'
    readers = None
    if reviewed.exists():
        raw = json.loads(reviewed.read_text())
        readers = {'scores':{key:value['criterion']['score'] for key,value in raw['grades'].items()},
            'failures':raw['failures'], 'missing_packets':raw['missing_packets'],
            'safety_findings':raw['safety_findings'], 'artifact':str(reviewed.relative_to(root)),
            'sha256':hashlib.sha256(reviewed.read_bytes()).hexdigest()}
    arms = {}
    for arm in sorted({run['arm'] for run in runs}):
        subset = [run for run in runs if run['arm']==arm]
        times = [run['reported_elapsed_seconds'] for run in subset]
        arms[arm] = {'runs':len(subset), 'completed_events':sum(run['completed_events'] for run in subset),
            'events':sum(run['events'] for run in subset),
            'median_reported_run_seconds':statistics.median(times) if all(x is not None for x in times) else None,
            'HTTP_started':sum(run['HTTP_started'] for run in subset) if all(run['HTTP_started'] is not None for run in subset) else None}
    return {'root':str(root), 'runs':runs, 'arms':arms, 'fixed_reader':readers,
            'semantic_quality':None, 'charges':None,
            'limitation':'Reported native times are per-history sums; no missing scores or subscription charges inferred.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(describe(args.root),indent=2)+'\n')


if __name__ == '__main__':
    main()
