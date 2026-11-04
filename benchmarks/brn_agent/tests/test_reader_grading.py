"""Synthetic protocol witnesses, never real-reader quality observations."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import evidence
import reader
import reader_grading
import semantic
import wiki_challenge
from score import assess, sha256


class ReaderGradingTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name).resolve()
        _,suite,truth=wiki_challenge.build()
        suite['scenarios']=[c for c in suite['scenarios'] if c['id']=='W06']
        self.suite=self.root/'suite.json';self.suite.write_text(json.dumps(suite))
        self.truth=self.root/'gold.json';self.truth.write_text(json.dumps(truth))
        suite['gold_sha256']=sha256(self.truth);self.suite.write_text(json.dumps(suite))
        records=[dict(id='wiki',version=1,data={'Note':{'title':'Maintained article','markdown':'30 public places. South lift during ramp closure through April 27 inclusive. Fee unspecified.'}})]
        self.folder=self.root/'runs/W06-0/A-r1';self.folder.mkdir(parents=True)
        captures=[]
        for n in range(4):
            captures.append(self.capture(self.folder,n,records))
        raw=dict(case='W06',arm='A',status='complete',repetition=1,attempt=1,
                 steps=[dict(index=n,kind='ask',terminal='Completed',answer='Synthetic saved wiki',operations=[]) for n in range(1,4)],checkpoints=captures)
        (self.folder/'result.json').write_text(json.dumps(raw))
        (self.root/'manifest.json').write_text(json.dumps(dict(fixture_sha256=suite['fixture_sha256'],gold_sha256=sha256(self.truth))))
        self.evidence=self.root/'agentbench-evidence.json';self.evidence.write_text(json.dumps(evidence.collect(self.root,self.suite)))
        self.attempt='runs/W06-0/A-r1/result.json'
        self.final_hash=captures[-1]['records_sha256'];self.records=records
        self.questions=truth['reader_questions']['W06']
        self.index=[dict(attempt=self.attempt,question_id=q['id'],measurement=self.measurement(i,q,records)) for i,q in enumerate(self.questions)]

    def capture(self,folder,n,records):
        p=folder/'checkpoints'/str(n);p.mkdir(parents=True)
        (p/'records.json').write_text(json.dumps(records));(p/'workspace.sqlite3').write_bytes(b'synthetic protocol backup, no provider')
        return dict(step=n,records_sha256=sha256(p/'records.json'),backup_sha256=sha256(p/'workspace.sqlite3'))

    def measurement(self,i,question,records):
        config=json.loads((Path(reader.__file__).parent/'wiki_challenge/reader.json').read_text())
        root=self.root/'readers'/str(i);folder=root/'runs/READER-0/fixed-reader-r1';folder.mkdir(parents=True)
        thread=f'reader-{i}';run=f'run-{i}';message=f'message-{i}'
        before=records+[dict(id=f'project-{i}',version=1,data={'Project':dict(chats=[thread],documents=['wiki'],use_brn_knowledge=False,instructions=config['instructions'])}),dict(id=thread,version=1,data={'Thread':{'title':'Reader'}})]
        after=before+[dict(id=message,version=1,data={'Message':dict(role='owner',thread=thread,text=config['instructions']+'\n\nQuestion: '+question['question'])}),dict(id=run,version=1,data={'Run':dict(thread=thread,instruction=message,state='Completed',budget=8)})]
        captures=[self.capture(folder,0,before),self.capture(folder,1,after)]
        trace=dict(search=True,max_rounds=8,wire_requests=[dict(model=config['model'],effort=config['effort'])],calls=[dict(tool='search_notes',outcome='success')])
        native=dict(status='complete',binary_sha256='synthetic-cli',steps=[dict(thread=thread,run=run,budget={'sent':8},terminal='Completed',answer=question['expected'],operations=[],trace=trace)],checkpoints=captures)
        native_path=folder/'result.json';native_path.write_text(json.dumps(native))
        receipt=dict(kind='measured-reader',status='complete',measured_answer=True,mode='wiki-only',
            question=question['question'],answer=question['expected'],checkpoint_records_sha256=self.final_hash,
            reader_config_sha256=sha256(Path(reader.__file__).parent/'wiki_challenge/reader.json'),native_result=str(native_path.relative_to(root)),
            native_result_sha256=sha256(native_path),trace=trace,reader_thread=thread,cli_binary_sha256='synthetic-cli',article_allowlist=[{'id':'wiki','version':1}])
        p=root/'reader-result.json';p.write_text(json.dumps(receipt))
        return dict(path=str(p.relative_to(self.root)),sha256=sha256(p))

    def prepare(self,index=None):
        return reader_grading.prepare(self.root,self.suite,self.evidence,self.truth,self.index if index is None else index)

    def replies(self,prepared):
        return [dict(id=p['id'],packet_sha256=p['packet_sha256'],score=4,explanation='Synthetic protocol judgment only',
            evidence=[{'reader_sha256':p['packet']['reader_sha256']}],breaches=[]) for p in prepared['packets']]

    def test_question_relabelling_rehash_cannot_fill_missing_coverage(self):
        ref=self.index[0]['measurement'];path=self.root/ref['path']
        value=json.loads(path.read_text());value['question']=self.questions[1]['question'];path.write_text(json.dumps(value))
        ref['sha256']=sha256(path)
        with self.assertRaisesRegex(ValueError,'actual native Run and owner Message'):
            self.prepare()

    def test_rehashed_wrong_empty_duplicate_private_or_archived_article_is_refused(self):
        ref=self.index[0]['measurement'];path=self.root/ref['path'];original=json.loads(path.read_text())
        for articles in ([{'id':'wiki','version':999}],[],[{'id':'wiki','version':1}]*2,[{'id':'wiki','version':True}]):
            value=copy.deepcopy(original);value['article_allowlist']=articles
            path.write_text(json.dumps(value));ref['sha256']=sha256(path)
            with self.assertRaisesRegex(ValueError,'article'):
                self.prepare()
        path.write_text(json.dumps(original));ref['sha256']=sha256(path)
        for key,value in [('archived',True),('private',True)]:
            records=copy.deepcopy(self.records)
            if key=='archived':records[0]['archived']=True
            else:records[0]['data']['Note']['chat']='other-chat'
            with self.assertRaises(ValueError):
                reader.verify_measurement(ref,self.root,self.final_hash,records)

    def test_missing_reader_and_controlled_answers_stay_unmeasured(self):
        prepared=self.prepare(self.index[:-1]);self.assertEqual(len(prepared['missing']),1)
        artifact=reader_grading.assess(self.root,self.suite,self.evidence,self.truth,self.index[:-1],prepared,self.replies(prepared),'unit')
        self.assertEqual(artifact['grades'],{})
        self.assertEqual(artifact['status'],'incomplete')
        ref=self.index[0]['measurement'];path=self.root/ref['path'];value=json.loads(path.read_text());value['kind']='controlled-fixture';path.write_text(json.dumps(value));ref['sha256']=sha256(path)
        with self.assertRaisesRegex(ValueError,'real wiki-only reader'):self.prepare()

    def test_incomplete_reader_review_keeps_known_safety_findings(self):
        prepared=self.prepare();replies=self.replies(prepared)[:-1]
        replies[0]['breaches']=['cross_scope_disclosure']
        artifact=reader_grading.assess(self.root,self.suite,self.evidence,self.truth,self.index,prepared,replies,'unit')
        self.assertEqual(artifact['status'],'incomplete')
        self.assertEqual(artifact['grades'],{})
        self.assertEqual(artifact['safety_findings'],[dict(attempt=self.attempt,kind='cross_scope_disclosure')])

    def test_bound_independent_reader_answers_join_candidate_grade(self):
        prepared=self.prepare()
        self.assertEqual(len(prepared['packets']),3)
        self.assertIn('expected',prepared['packets'][0]['packet'])
        self.assertNotIn('runs/W06-0/A-r1',json.dumps(prepared['packets']))
        replies=self.replies(prepared)
        artifact=reader_grading.assess(self.root,self.suite,self.evidence,self.truth,self.index,prepared,replies,'unit')
        self.assertFalse(artifact['qualified'])
        candidate=semantic.prepare(self.root,self.suite,self.evidence,truth_path=self.truth)
        candidate_replies=[dict(id=p['id'],packet_sha256=p['packet_sha256'],completed=True,breaches=[],criteria={c:dict(score=None if c=='downstream_usefulness' else 4,explanation='Synthetic protocol witness',evidence=[{'step':p['packet']['checkpoint']}]) for c in p['packet']['criteria']}) for p in candidate['packets']]
        imported=semantic.import_reviewed(self.root,self.suite,self.evidence,candidate,candidate_replies,'unit',reader_grading.CONFIG,truth_path=self.truth)
        merged=reader_grading.merge(self.root,self.suite,self.evidence,self.truth,artifact,imported['grades'])
        evidence.verify_grade_references(json.loads(self.evidence.read_text()),merged,self.root)
        self.assertEqual(merged[0]['criteria']['downstream_usefulness']['score'],4)
        self.assertTrue(merged[0]['reader_evidence_verified'])
        report=assess(json.loads(self.suite.read_text()),'A',json.loads(self.evidence.read_text())['attempts'],merged)
        self.assertEqual(report['challenge_quality'],100)
        self.assertFalse(report['ranked'])
        import bench
        candidate_file=self.root/'candidate-grades.json';candidate_file.write_text(json.dumps(imported['grades']))
        artifact_file=self.root/'reader-reviews.json';artifact_file.write_text(json.dumps(artifact))
        common=['report','--suite',str(self.suite),'--truth',str(self.truth),'--evidence',str(self.evidence),
                '--grades',str(candidate_file),'--configuration','A']
        self.assertEqual(bench.main(common+['--reader-grades',str(artifact_file),'--output',str(self.root/'joined-report')]),0)
        joined=json.loads((self.root/'joined-report/report.json').read_text())
        self.assertEqual(joined['challenge_quality'],100)
        self.assertFalse(joined['ranked'])
        self.assertEqual(bench.main(common+['--output',str(self.root/'unmeasured-report')]),0)
        self.assertIsNone(json.loads((self.root/'unmeasured-report/report.json').read_text())['challenge_quality'])
        changed=copy.deepcopy(artifact);changed['replies'][0]['packet_sha256']='different'
        with self.assertRaises(ValueError):reader_grading.merge(self.root,self.suite,self.evidence,self.truth,changed,imported['grades'])
        changed=copy.deepcopy(artifact);changed['prepared']['packets'][0]['packet']['answer']='changed after review'
        with self.assertRaises(ValueError):reader_grading.merge(self.root,self.suite,self.evidence,self.truth,changed,imported['grades'])


if __name__=='__main__':unittest.main()
