"""Frozen truth remains usable after the installed bank revision changes."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from evidence import collect, verified_proof_packet
from score import sha256


class FrozenTruthTests(unittest.TestCase):
    def test_explicit_versioned_truth_and_native_binding(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            checkpoint = root/'runs/OLD-0/A-r1/checkpoints/1'
            checkpoint.mkdir(parents=True)
            (checkpoint/'records.json').write_text('[]')
            (checkpoint/'workspace.sqlite3').write_bytes(b'controlled backup')
            result = {'case':'OLD','arm':'A','attempt':1,'status':'complete','repetition':1,'steps':[],
                      'checkpoints':[{'step':1,'records_sha256':sha256(checkpoint/'records.json'),
                                      'backup_sha256':sha256(checkpoint/'workspace.sqlite3')}]}
            (checkpoint.parent.parent/'result.json').write_text(json.dumps(result))
            truth = root/'gold.json'
            gold = {'version':1,'fixture_sha256':'previous-revision',
                    'checkpoints':{'OLD':{'1':{'scenario':'OLD','step':1,'facts':['known then']},
                                          '2':{'scenario':'OLD','step':2,'facts':['future secret']}}}}
            truth.write_text(json.dumps(gold))
            suite = root/'suite.json'
            suite.write_text(json.dumps({'fixture_sha256':'previous-revision','gold_sha256':sha256(truth)}))
            (root/'manifest.json').write_text(suite.read_text())
            manifest = collect(root,suite)
            (root/'agentbench-evidence.json').write_text(json.dumps(manifest))
            packet = verified_proof_packet(checkpoint,scenario='OLD',step=1,truth_path=truth)
            self.assertIn('known then',str(packet))
            self.assertNotIn('future secret',str(packet))
            with self.assertRaises(ValueError):
                verified_proof_packet(checkpoint,scenario='OLD',step=1)
            with self.assertRaises(ValueError):
                verified_proof_packet(checkpoint,truth_path=truth)
            truth.write_text(truth.read_text()+' ')
            with self.assertRaises(ValueError):
                verified_proof_packet(checkpoint,scenario='OLD',step=1,truth_path=truth)
            # Editing the collected index cannot replace native experiment identity.
            manifest['gold_sha256']=sha256(truth)
            (root/'agentbench-evidence.json').write_text(json.dumps(manifest))
            with self.assertRaises(ValueError):
                verified_proof_packet(checkpoint,scenario='OLD',step=1,truth_path=truth)

    def test_legacy_unpinned_truth_can_still_be_regraded(self):
        import legacy_proof
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            checkpoint = root/'runs/J01-0/A-r1/checkpoints/1'
            checkpoint.mkdir(parents=True)
            (checkpoint/'records.json').write_text('[]')
            (checkpoint/'workspace.sqlite3').write_bytes(b'legacy controlled backup')
            raw = {'case':'J01','arm':'A','attempt':1,'status':'complete','repetition':1,'steps':[],
                   'checkpoints':[{'step':1,'records_sha256':sha256(checkpoint/'records.json'),
                                   'backup_sha256':sha256(checkpoint/'workspace.sqlite3')}]}
            (checkpoint.parent.parent/'result.json').write_text(json.dumps(raw))
            suite = root/'suite.json'
            suite.write_text(json.dumps(legacy_proof.build()[1]))
            (root/'manifest.json').write_text(json.dumps({'fixture_sha256':legacy_proof.build()[1]['fixture_sha256']}))
            manifest = collect(root,suite)
            (root/'agentbench-evidence.json').write_text(json.dumps(manifest))
            packet = verified_proof_packet(checkpoint,scenario='J01',step=1)
            self.assertEqual(packet['checkpoint_gold'],legacy_proof.checkpoint_gold('J01',1))
            self.assertNotIn('2027-03-03',str(packet))

    def test_resume_refuses_changed_harness_guidance_and_truth(self):
        import copy
        sys.path.insert(0,str(Path(__file__).resolve().parents[3]/'scripts'))
        import evaluate_wiki_v2 as native
        original = {'fixture_sha256':'fixture','seed':1,'arms':[],'live':False,
                    'provenance':{'files':{'agent/BRN.md':'original'}},'gold_sha256':'gold'}
        native.verify_resume_identity(original, copy.deepcopy(original))
        for field, replacement in [('provenance',None),('provenance',{'files':{'agent/BRN.md':'changed'}}),
                                   ('gold_sha256','new'),('fixture_sha256','new'),('model_call_limit',8)]:
            changed = copy.deepcopy(original)
            changed[field] = replacement
            with self.assertRaises(ValueError):
                native.verify_resume_identity(original,changed)

    def test_unknown_and_provider_failures_are_not_model_failure_evidence(self):
        from evidence import classify_failure
        for terminal, expected in [('Failed:Network','provider_transport'),('Failed:UnsafeCredentials','setup_or_compatibility'),
                                   ('Failed:Storage','brn_runtime'),('Failed:InvalidToolUse','model_or_tool_use')]:
            self.assertEqual(classify_failure({'status':'transport_failure','steps':[{'terminal':terminal}]}),expected)
        self.assertEqual(classify_failure({'status':'transport_failure','steps':[]}), 'unclassified_infrastructure')
