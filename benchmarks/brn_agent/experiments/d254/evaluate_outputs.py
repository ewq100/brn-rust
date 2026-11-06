"""Bind frozen native outputs to the existing independent grader/reader importers.

No candidate reruns or custom scoring. Model dispatch goes only through supervise;
the fixed rubric, packets and reviewer instructions remain separately frozen.
"""
import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
sys.path.insert(0,str(REPO/'benchmarks/brn_agent'))
import semantic
from evidence import verify_manifest, anonymize_local_paths
BENCH = REPO/'benchmarks/brn_agent/bench.py'
SUPERVISE = Path(__file__).with_name('supervise_outputs.py')
BATCH_SIZE = 2  # Within the predeclared maximum8; all arms use identical grouping.

def load(path):
    return json.loads(Path(path).read_text())

def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, ensure_ascii=False)+'\n')

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def bench(*args):
    subprocess.run([sys.executable,str(BENCH),*map(str,args)],check=True,capture_output=True,text=True)

def dispatch(args, operation):
    path = args.root/'evaluation-operations'/(operation['id']+'.json')
    path.parent.mkdir(exist_ok=True)
    save(path,operation)
    subprocess.run([sys.executable,str(SUPERVISE),'--operation',str(path),
        '--account-file',str(args.account_file),'--cycle',str(args.task_root/'cycle-start.json'),
        '--roles',str(args.task_root)],check=True,capture_output=True,text=True)

def grade(args, packets, tag):
    packets = list(packets)
    random.Random(25420261106).shuffle(packets)
    batch_size=1 if tag=='reader' else BATCH_SIZE
    if args.grading_pass:
        tag+='-'+args.grading_pass
    folder = args.root/('judgments-'+tag)
    if folder.exists():
        # One explicit correction of the known outer-deadline fault. None of
        # that interrupted batch produced a judgment; valid replies are never
        # regraded by this path and the original directory remains untouched.
        prior=folder/'0-run'
        disposition=args.task_root/'grading-dispositions'/(args.root.name+'-'+tag+'-0.json')
        if not disposition.exists() or not (prior/'receipt.json').exists() or (prior/'receipt.json').stat().st_size:
            raise RuntimeError('Existing grading artifacts require independent disposition')
        folder=args.root/('judgments-'+tag+'-deadline-correction')
        tag+='-deadline-correction'
    folder.mkdir(exist_ok=False)
    def call(batch,label):
        used_seconds=0
        for p in args.task_root.glob('phase*/judgments-*/*-run/receipt.json'):
            if p.stat().st_size:
                used_seconds+=load(p).get('elapsed_ms',0)/1000
            else:
                rows=[json.loads(x) for x in Path(str(p.parent)+'.deadline.jsonl').read_text().splitlines()]
                used_seconds+=next(x['elapsed_seconds'] for x in reversed(rows) if x.get('event')=='terminal')
        if used_seconds+45>1200:
            raise RuntimeError('Original20min grading ceiling exhausted')
        path=folder/(label+'-packets.json')
        save(path,batch)
        identity=args.root.name+'-'+tag+'-'+label
        out=folder/(label+'-run')
        dispatch(args,{'id':identity,'kind':'grader','role':'grader','phase':args.phase,
            'root':str(out),'seconds':90,'minimum_seconds':45,'binary':str(args.judge),
            'packet':str(path),'instructions':str(args.task_root/'grader-instructions.txt')})
        receipt=load(out/'receipt.json')
        try:
            decoded=json.loads(receipt['result']['text'])
            if not isinstance(decoded,list):
                raise ValueError('Expected array')
            return decoded
        except (KeyError,ValueError):
            return []
    replies=[]
    repairs=[]
    for offset in range(0,len(packets),batch_size):
        batch=packets[offset:offset+batch_size]
        decoded=call(batch,str(offset//batch_size))
        for number,packet in enumerate(batch):
            bound=[r for r in decoded if isinstance(r,dict) and r.get('id')==packet['id']
                and r.get('packet_sha256')==packet['packet_sha256']]
            if len(bound)!=1:
                # Only a malformed/missing binding gets one unchanged-packet
                # continuation. Valid low scores are never replaced.
                repairs.append({'id':packet['id'],'reason':'missing/duplicate/mismatched packet binding'})
                corrected=call([packet],f'protocol-{offset+number}')
                bound=[r for r in corrected if isinstance(r,dict) and r.get('id')==packet['id']
                    and r.get('packet_sha256')==packet['packet_sha256']]
            replies.append(bound[0] if len(bound)==1 else {'id':packet['id'],
                'packet_sha256':packet['packet_sha256'],'error':'No valid hash-bound judgment after one protocol continuation'})
    save(folder/'protocol-continuations.json',repairs)
    save(folder/'replies.json',replies)
    return folder/'replies.json'

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['root','suite','truth','task-root','binary','judge','account-file']:
        p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--phase',choices=['phase1','phase2'],required=True)
    p.add_argument('--role',choices=['phase1','phase2-development','phase2-broad','phase2-fresh'],required=True)
    p.add_argument('--no-readers',action='store_true')
    p.add_argument('--readers-only',action='store_true',help='Preserve missing semantic grades; never infer them from reader answers')
    p.add_argument('--grading-pass',choices=['two-packet'])
    p.add_argument('--reader-pass',choices=['setup-correction'])
    args=p.parse_args()
    evidence=args.root/'agentbench-evidence.json'
    if evidence.exists():
        verify_manifest(load(evidence),args.root)
    else:
        bench('collect','--root',args.root,'--suite',args.suite)
    prepared=args.root/'semantic-packets'
    if prepared.exists():
        expected=semantic.prepare(args.root,args.suite,evidence,truth_path=args.truth)
        if load(prepared/'private-result.json') != expected:
            raise ValueError('Previously prepared evidence changed')
    else:
        bench('semantic-prepare','--root',args.root,'--suite',args.suite,'--truth',args.truth,
              '--evidence',evidence,'--output',prepared)
    private=load(prepared/'private-result.json')
    suite=load(args.suite); cases={c['id']:c for c in suite['scenarios']}
    attempts={a['id']:a for a in load(evidence)['attempts']}
    packets=load(prepared/'packets.json')
    for packet in packets:
        key=private['private_key'][packet['id']]
        packet['checkpoint_number']=key['checkpoint']
        attempt=attempts[key['attempt']]
        packet['criterion_ids']=cases[attempt['scenario']]['criteria']
        raw=load(args.root/attempt['result'])
        # Read-only answer/recovery cases need their measured answer, not an
        # inference from unchanged Notes. Preserve the existing packet/hash and
        # bind this anonymous auxiliary evidence separately to its native bytes.
        auxiliary=anonymize_local_paths({'native_result_sha256':attempt['result_sha256'],
            'events':[{k:event.get(k) for k in ['index','terminal','answer','operations','receipts']}
                for event in raw['steps'] if event['index']<=key['checkpoint']]},args.root)
        packet['event_observation']=auxiliary
        packet['event_observation_sha256']=hashlib.sha256(json.dumps(auxiliary,
            sort_keys=True,ensure_ascii=False).encode()).hexdigest()
    if args.readers_only:
        replies=args.root/'semantic-unmeasured-replies.json'
        save(replies,[])
    else:
        replies=grade(args,packets,'semantic')
    reviewed=args.root/'semantic-reviewed'
    config=REPO/'benchmarks/brn_agent/wiki_challenge/reviewer.json'
    if reviewed.exists():
        expected=semantic.import_reviewed(args.root,args.suite,evidence,private,load(replies),
            'wiki-reviewer-v1',config,truth_path=args.truth)
        if load(reviewed/'private-result.json')!=expected:
            raise ValueError('Existing semantic import differs from retained replies')
    else:
        bench('semantic-reviewed','--root',args.root,'--suite',args.suite,'--truth',args.truth,
              '--evidence',evidence,'--prepared',prepared/'private-result.json','--replies',replies,
              '--revision','wiki-reviewer-v1','--reviewer-config',config,'--output',reviewed)
    reader_artifact=None
    if suite.get('score_family')=='wiki-consolidation-v1' and not args.no_readers:
        truth=load(args.truth); index=[]
        for attempt in attempts.values():
            native=load(args.root/attempt['result'])
            final=max(cases[attempt['scenario']]['grade_checkpoints'])
            checkpoint=(args.root/attempt['result']).parent/'checkpoints'/str(final)
            records=load(checkpoint/'records.json')
            articles=[{'id':r['id'],'version':r['version']} for r in records
                if not r.get('archived') and 'Note' in r['data']
                and not r['data']['Note'].get('protected') and r['data']['Note'].get('chat') is None]
            if not articles:
                continue  # Missing maintained state stays unmeasured; never use protected raw sources.
            for q in truth['reader_questions'][attempt['scenario']]:
                label=hashlib.sha256((attempt['id']+q['id']).encode()).hexdigest()[:16]
                if args.reader_pass:
                    label+='-'+args.reader_pass
                out=args.root/'readers'/label
                dispatch(args,{'id':args.root.name+'-reader-'+label,'kind':'reader','role':args.role,
                    'root':str(out),'seconds':120,'minimum_seconds':90,'checkpoint':str(checkpoint),
                    'question':q['question'],'records':articles,'binary':str(args.binary)})
                receipt=out/'reader-result.json'
                index.append({'attempt':attempt['id'],'question_id':q['id'],
                    'measurement':{'path':str(receipt.relative_to(args.root)),'sha256':sha(receipt)}})
        save(args.root/'reader-index.json',index)
        rp=args.root/'reader-packets'
        bench('reader-review-prepare','--root',args.root,'--suite',args.suite,'--truth',args.truth,
            '--evidence',evidence,'--index',args.root/'reader-index.json','--output',rp)
        rr=grade(args,load(rp/'packets.json'),'reader')
        ri=args.root/'readers-reviewed'
        bench('reader-review-import','--root',args.root,'--suite',args.suite,'--truth',args.truth,
            '--evidence',evidence,'--index',args.root/'reader-index.json',
            '--prepared',rp/'private-result.json','--replies',rr,'--revision','wiki-reviewer-v1','--output',ri)
        reader_artifact=ri/'private-result.json'
    grades=load(reviewed/'grades.json')
    reports=[]
    for arm in sorted({a['configuration'] for a in attempts.values()}):
        arm_grades=[g for g in grades if attempts[g['attempt']]['configuration']==arm]
        path=args.root/('grades-'+arm+'.json');save(path,arm_grades)
        out=args.root/('report-'+arm)
        command=['report','--suite',args.suite,'--truth',args.truth,
            '--evidence',evidence,'--grades',path,'--configuration',arm,'--output',out]
        if reader_artifact:
            command+=['--reader-grades',reader_artifact]
        bench(*command);reports.append(str(out))
    save(args.root/'evaluation-summary.json',{'reports':reports,'semantic_packets':len(packets),
        'reader_measurements':len(index) if reader_artifact else 0,
        'qualification':'independent frozen judgments; semantic competence and same-family bias remain limitations'})
    print(json.dumps({'status':'complete','reports':reports,'semantic_packets':len(packets),
        'reader_measurements':len(index) if reader_artifact else 0}))

if __name__=='__main__':
    main()
