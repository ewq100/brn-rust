"""Reuse BRN's deterministic native contracts; never treat these as model scores."""
import json
import hashlib
from pathlib import Path
import re
import subprocess
from score import sha256

# Fixed allowlist: no arbitrary fixture-supplied shell commands or test filters.
CHECKS = {
    'protected-edit': ('brn-ai', None, 'threads::note_edit_tests::protected_local_edit_needs_exact_review_or_scoped_owner_instruction_and_undo_is_checked', True),
    'historical-authority': ('brn-ai', None, 'threads::budget_tests::historical_instruction_is_tool_evidence_and_cannot_grant_protected_apply_or_survive_stop', True),
    'skill-scope': ('brn-ai', None, 'threads::skills_tests::project_instruction_and_references_cannot_read_foreign_data_or_grant_protected_apply', True),
    'skill-disable': ('brn-ai', None, 'threads::skills_tests::disable_fences_prepared_scope_and_changed_bytes_cannot_load_or_satisfy_selection', True),
    'stop-before-worker': ('brn-ai', None, 'threads::steering_tests::stop_before_worker_skips_credentials_stale_tools_and_completion', True),
    'precancel': ('brn-ai', None, 'threads::steering_tests::prepared_run_id_exists_before_worker_and_precancel_skips_credentials', True),
    'continuation': ('brn-ai', None, 'threads::continuation_tests::', False),
    'process-kill': ('brn-threads-app', 'killed_run_candidate', '', False),
    'orphan-recovery': ('brn-threads-app', 'run_recovery', '', False),
    'restore-skill-candidate': ('brn-threads-app', 'int_d', '', False),
    'daily-recovery': ('brn-threads-app', 'daily_workflow_eval', '', False),
}

# This ignored entry exports an owner's manual UI fixture; it is not a test of
# process-kill recovery. Keep the exclusion visible and reject any other skip.
MANUAL_EXPORT = 'native_check_store_with_a_killed_intake_and_its_candidate'


def run(output, selected=None):
    root = Path(__file__).resolve().parents[2]
    selected = list(CHECKS) if selected is None else selected
    if not selected or len(set(selected)) != len(selected) or any(k not in CHECKS for k in selected):
        raise ValueError('Choose unique known host checks')
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    result = {'format':1, 'kind':'offline-host-contracts', 'model_scores':False, 'status':'running',
              'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),
              'lock_sha256':sha256(root/'Cargo.lock'), 'checks':[]}
    source_files = subprocess.check_output(['git','ls-files','*.rs','Cargo.toml','Cargo.lock'],cwd=root,text=True).splitlines()
    result['source_sha256'] = hashlib.sha256(json.dumps(
        {name:sha256(root/name) for name in source_files},sort_keys=True).encode()).hexdigest()
    def persist():
        (output/'host-checks.json').write_text(json.dumps(result,indent=2))
    persist()
    for identity in selected:
        package, target, test_filter, exact = CHECKS[identity]
        command = ['cargo','test','--locked','--offline','-p',package,'--jobs','2']
        command += ['--test',target] if target else ['--features','scripted-runs','--lib']
        if test_filter:
            command.append(test_filter)
        if exact:
            command += ['--','--exact']
        entry = {'id':identity,'command':command,'status':'running'}
        result['checks'].append(entry)
        persist()
        with (output/f'{identity}.log').open('x') as stream:
            process = subprocess.run(command,cwd=root,stdout=stream,stderr=subprocess.STDOUT)
        log = (output/f'{identity}.log').read_text()
        matches = re.findall(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;',log)
        passed = sum(int(n) for n, _ in matches)
        ignored = sum(int(n) for _, n in matches)
        ignored_names = re.findall(r'^test (\S+) \.\.\. ignored',log,re.MULTILINE)
        allowed = identity=='process-kill' and ignored==1 and ignored_names==[MANUAL_EXPORT]
        entry.update(status='passed' if process.returncode==0 and passed>0 and (ignored==0 or allowed) else 'failed',
                     exit=process.returncode,passed=passed,ignored=ignored,log_sha256=sha256(output/f'{identity}.log'))
        if allowed:
            entry['excluded_manual_export'] = MANUAL_EXPORT
        persist()
    result['status'] = 'passed' if all(c['status']=='passed' for c in result['checks']) else 'failed'
    persist()
    return result
