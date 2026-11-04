import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from evidence import collect
from score import sha256
import proof
import semantic


class SemanticProtocolTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.folder = self.root/'runs/F01-0/CANDIDATE_SECRET-r1'
        checkpoint = self.folder/'checkpoints/1'
        checkpoint.mkdir(parents=True)
        (checkpoint/'workspace.sqlite3').write_bytes(b'unit backup')
        (checkpoint/'records.json').write_text(json.dumps([
            {'id':'note','version':1,'data':{'Note':{'markdown':'Synthetic saved wiki.', 'import':{'source':str(self.folder/'step-1.md')}}}},
            {'id':'source','version':1,'data':{'Source':{'locator':str(self.folder/'step-1.md'),'label':'Historical evidence'}}}]))
        self.suite = self.root/'suite.json'
        self.suite.write_text(json.dumps(proof.build()[1]))
        (self.root/'manifest.json').write_text(json.dumps({'fixture_sha256':proof.build()[1]['fixture_sha256']}))
        (self.folder/'result.json').write_text(json.dumps({'case':'F01','arm':'CANDIDATE_SECRET','status':'complete','repetition':1,'attempt':1,
            'steps':[{'index':1,'kind':'ask','terminal':'Completed','answer':'Saved.','operations':[]}],
            'checkpoints':[{'step':1,'records_sha256':sha256(checkpoint/'records.json'),'backup_sha256':sha256(checkpoint/'workspace.sqlite3')}]}))
        self.manifest = self.root/'custom-evidence.json'
        self.manifest.write_text(json.dumps(collect(self.root,self.suite)))

    def test_blind_protocol_binding_and_grader_failures(self):
        prepared = semantic.prepare(self.root,self.suite,self.manifest)
        self.assertNotIn('CANDIDATE_SECRET',json.dumps(prepared['packets']))
        packet = prepared['packets'][0]
        reply = {'id':packet['id'],'packet_sha256':packet['packet_sha256'],'completed':True,'breaches':[],
                 'criteria':{name:{'score':3,'explanation':'Synthetic reviewed witness',
                                   'evidence':[{'checkpoint':1,'record':'note','version':1}]}
                             for name in packet['packet']['criteria']}}
        result = semantic.import_controlled(self.root,self.suite,self.manifest,prepared,[reply],'unit-v1')
        self.assertEqual(result['status'],'complete')
        self.assertFalse(result['qualified'])
        self.assertEqual(result['candidate_calls'],0)
        for mutation in ('bad-hash','bad-ref','missing-criterion','bad-score'):
            bad = copy.deepcopy(reply)
            criterion = next(iter(bad['criteria']))
            if mutation=='bad-hash': bad['packet_sha256']='wrong'
            elif mutation=='bad-ref': bad['criteria'][criterion]['evidence'][0]['version']=2
            elif mutation=='missing-criterion': del bad['criteria'][criterion]
            else: bad['criteria'][criterion]['score']=4.5
            with self.assertRaises(ValueError):
                semantic.import_controlled(self.root,self.suite,self.manifest,prepared,[bad],'unit-v1')
        failed = {'id':packet['id'],'packet_sha256':packet['packet_sha256'],'error':'controlled grader outage'}
        result = semantic.import_controlled(self.root,self.suite,self.manifest,prepared,[failed],'unit-v2')
        self.assertEqual(result['status'],'incomplete')
        self.assertEqual(len(result['grader_failures']),1)
        self.assertEqual(result['grades'],[])
        prepared['packets'][0]['packet']['records']=[]
        with self.assertRaises(ValueError):
            semantic.import_controlled(self.root,self.suite,self.manifest,prepared,[reply],'unit-v1')

    def test_independent_import_preserves_origin_and_frozen_configuration(self):
        prepared = semantic.prepare(self.root,self.suite,self.manifest)
        packet=prepared['packets'][0]
        reply=dict(id=packet['id'],packet_sha256=packet['packet_sha256'],completed=True,breaches=[],
                   criteria={name:dict(score=4,explanation='Independent schema witness only',evidence=[{'step':1}])
                             for name in packet['packet']['criteria']})
        config=Path(__file__).resolve().parents[1]/'wiki_challenge/reviewer.json'
        result=semantic.import_reviewed(self.root,self.suite,self.manifest,prepared,[reply],'test',config)
        self.assertEqual(result['provider'],'independent-review')
        self.assertEqual(result['grades'][0]['review_kind'],'independent-review')
        self.assertTrue(result['grades'][0]['grader_revision'].startswith('independent:'+sha256(config)))
        self.assertFalse(result['qualified'])
        changed=self.root/'reviewer.json'
        changed.write_text('{"model":"different"}')
        with self.assertRaisesRegex(ValueError,'frozen reviewer'):
            semantic.import_reviewed(self.root,self.suite,self.manifest,prepared,[reply],'test',changed)
