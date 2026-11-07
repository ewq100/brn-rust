"""Grading dispatch must use the exact prepared payload and verified transport."""
import copy
import importlib.util
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).resolve().parents[1]/'experiments/d254'
sys.path.insert(0,str(HERE))
spec = importlib.util.spec_from_file_location('d254_output_bindings',HERE/'evaluate_outputs.py')
outputs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(outputs)


class OutputBindingTests(unittest.TestCase):
    def packet(self):
        content={'criteria':['retained_information'],'checkpoint':1,
                 'events':[{'answer':'SYNTHETIC original answer'}]}
        return {'id':'opaque-id','packet_sha256':outputs.semantic.packet_hash(content),'packet':content}

    def test_changed_export_with_original_identifiers_is_refused(self):
        original=self.packet()
        altered=copy.deepcopy(original)
        altered['packet']['events'][0]['answer']='ALTERED answer'
        with self.assertRaises(ValueError):
            outputs.semantic_packets({'packets':[original]},[altered])

    def test_metadata_is_derived_from_hash_bound_packet_without_mutating_preparation(self):
        original=self.packet()
        packets=outputs.semantic_packets({'packets':[original]},[original])
        outputs.validate_packet(packets[0])
        self.assertNotIn('criterion_ids',original)
        self.assertEqual(packets[0]['checkpoint_number'],1)
        for mutation in ('content','checkpoint_number','criterion_ids','extra-evidence'):
            with self.subTest(mutation=mutation):
                altered=copy.deepcopy(packets[0])
                if mutation=='content':
                    altered['packet']['events'][0]['answer']='ALTERED'
                else:
                    altered[mutation]='ALTERED'
                with self.assertRaises(ValueError):
                    outputs.validate_packet(altered)

    def test_reader_payload_uses_the_same_digest_check(self):
        packet=self.packet()
        packet['packet']={'kind':'reader-answer-review-v1','answer':'SYNTHETIC answer'}
        packet['packet_sha256']=outputs.semantic.packet_hash(packet['packet'])
        outputs.validate_packet(packet)
        packet['packet']['answer']='ALTERED'
        with self.assertRaises(ValueError):
            outputs.validate_packet(packet)

    def test_judgment_requires_request_digest_model_effort_and_single_observed_request(self):
        receipt={'status':'completed','model':'gpt-6.1-sol','effort':'high','tools':[],
            'request_sha256':'bound-request','result':{'returned_model':'gpt-6.1-sol'},
            'wire_requests':[{'model':'gpt-6.1-sol','effort':'high'}],
            'model_call_budget':{'limit':1,'started':1,'refused':False}}
        outputs.validate_judgment(receipt,'bound-request')
        for mutation in ('digest','returned-model','wire-model','wire-effort','missing-wire','extra-request','budget'):
            with self.subTest(mutation=mutation):
                altered=copy.deepcopy(receipt)
                if mutation=='digest': altered['request_sha256']='wrong'
                elif mutation=='returned-model': altered['result']['returned_model']=None
                elif mutation=='wire-model': altered['wire_requests'][0]['model']='wrong'
                elif mutation=='wire-effort': altered['wire_requests'][0]['effort']='low'
                elif mutation=='missing-wire': altered['wire_requests']=[]
                elif mutation=='extra-request': altered['wire_requests']*=2
                else: altered['model_call_budget']['started']=2
                with self.assertRaises(ValueError):
                    outputs.validate_judgment(altered,'bound-request')
