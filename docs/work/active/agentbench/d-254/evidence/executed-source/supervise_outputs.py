"""Original native supervisor plus explicit containment of a read-only grader fault.

The frozen native supervisor is unchanged. Only a separately recorded, reaped,
read-only grader failure may be closed; its reservation is never refunded and
its operation identity cannot be repeated. Candidate failures still stop work.
"""
import hashlib
import json
from pathlib import Path
import sys
import supervise as native_supervisor


def audit_operations(roles, current=None):
    for marker in roles.glob('operation-*.json'):
        operation=json.loads(marker.read_text())
        native_supervisor.audit(operation['root'])
        receipt=Path(operation['root']+'.deadline.jsonl')
        if not receipt.exists():
            if operation['root']!=current:
                disposition=roles/'grading-dispositions'/(marker.stem.removeprefix('operation-')+'.json')
                if disposition.exists():
                    closed=json.loads(disposition.read_text())
                    request=Path(closed.get('request_path',''))
                    if (closed.get('kind')=='reader-setup-never-dispatched'
                            and closed.get('root')==operation['root']
                            and not Path(operation['root']).exists()
                            and request.is_file()
                            and hashlib.sha256(request.read_bytes()).hexdigest()==closed.get('request_sha256')
                            and json.loads(request.read_text()).get('kind')=='reader'
                            and closed.get('actual_HTTP')==0):
                        continue
                raise native_supervisor.CycleStop('Earlier operation never recorded terminal outcome')
            continue
        rows=[json.loads(line) for line in receipt.read_text().splitlines()]
        terminals=[r for r in rows if r.get('event')=='terminal']
        if not terminals:
            if operation['root']!=current:
                raise native_supervisor.CycleStop('Earlier operation outcome uncertain')
            continue
        if terminals[-1].get('exit_code')==0:
            continue
        disposition=roles/'grading-dispositions'/(marker.stem.removeprefix('operation-')+'.json')
        if operation.get('role')!='grader' or not disposition.exists():
            raise native_supervisor.CycleStop('Earlier operation failed')
        closed=json.loads(disposition.read_text())
        if (closed.get('kind')!='uncertain-read-only-grader-contained'
                or closed.get('root')!=operation['root']
                or closed.get('deadline_receipt_sha256')!=hashlib.sha256(receipt.read_bytes()).hexdigest()
                or closed.get('reservation_retained') is not True
                or terminals[-1].get('cleanup_errors')!=[]):
            raise native_supervisor.CycleStop('Invalid grader containment receipt')


if __name__=='__main__':
    request=Path(sys.argv[sys.argv.index('--operation')+1])
    Path(json.loads(request.read_text())['root']).parent.mkdir(parents=True,exist_ok=True)
    native_supervisor.audit_operations=audit_operations
    sys.exit(native_supervisor.main())
