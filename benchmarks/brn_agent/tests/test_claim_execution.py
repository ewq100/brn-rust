import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from evidence import collect, assess_checkpoint_claims
from score import sha256


class ClaimExecutionTests(unittest.TestCase):
    def test_native_baseline_receipts_and_exact_version_are_bound(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);folder=root/'runs/C-0/A-r1';checkpoints=folder/'checkpoints'
            source={'id':'s','version':1,'data':{'Note':{'markdown':'Budget is EUR 800.'}}}
            unrelated={'id':'u','version':1,'data':{'Note':{'markdown':'Keep unchanged.'}}}
            wiki={'id':'w','version':1,'data':{'Note':{'markdown':'Budget: EUR 800.'}}}
            link={'id':'l','version':1,'data':{'Link':{'from':'w','to':'s','relation':'source'}}}
            entries=[]
            for step,records in [(0,[source,unrelated]),(1,[source,unrelated,wiki,link])]:
                cp=checkpoints/str(step);cp.mkdir(parents=True)
                (cp/'records.json').write_text(json.dumps(records));(cp/'workspace.sqlite3').write_bytes(b'unit backup')
                entries.append({'step':step,'records_sha256':sha256(cp/'records.json'),'backup_sha256':sha256(cp/'workspace.sqlite3')})
            raw={'case':'C','arm':'A','attempt':1,'status':'complete','repetition':1,
                 'steps':[{'receipts':[{'operation':'saved-wiki'}]}],'checkpoints':entries}
            (folder/'result.json').write_text(json.dumps(raw))
            truth={'version':1,'fixture_sha256':'synthetic','checkpoints':{'C':{'1':{
                'scenario':'C','step':1,'facts':[{'id':'budget','value':800,'status':'current','source_keys':['plan']}],
                'required_fact_ids':['budget'],'task_contract':{'require_saved':True,'source_links_required':True,'preserve_unrelated':True}}}}}
            gold=root/'gold.json';gold.write_text(json.dumps(truth))
            suite=root/'suite.json';suite.write_text(json.dumps({'fixture_sha256':'synthetic','gold_sha256':sha256(gold)}))
            (root/'manifest.json').write_text(suite.read_text())
            (root/'agentbench-evidence.json').write_text(json.dumps(collect(root,suite)))
            annotation={'annotations':[{'fact_id':'budget','value':800,'status':'current','record':'w','version':1,'quote':'EUR 800'}],
                        'source_bindings':{'plan':'s','unrelated':'u'},'claimed_operations':['saved-wiki'],'adjudication_kind':'controlled-fixture'}
            result=assess_checkpoint_claims(checkpoints/'1','C',1,annotation,truth_path=gold)
            self.assertTrue(result['passed']);self.assertFalse(result['qualified'])
            bad=copy.deepcopy(annotation);bad['claimed_operations']=['unsent']
            self.assertFalse(assess_checkpoint_claims(checkpoints/'1','C',1,bad,truth_path=gold)['passed'])
            bad=copy.deepcopy(annotation);bad['annotations'][0]['version']=2
            with self.assertRaises(ValueError):assess_checkpoint_claims(checkpoints/'1','C',1,bad,truth_path=gold)
            (checkpoints/'0/records.json').write_text('[]')
            with self.assertRaises(ValueError):assess_checkpoint_claims(checkpoints/'1','C',1,annotation,truth_path=gold)
