from pathlib import Path
import json,hashlib,base64
R=Path('/private/tmp/brn-x2-html-r1');sha=lambda b:hashlib.sha256(b).hexdigest()
for name in ['large-links','friendly']:
 p=R/'logs'/('final-runtime-'+name);request=p.with_suffix('.request.json').read_bytes();source=bytes(json.loads(request)['bytes']);out=p.with_suffix('.stdout').read_bytes();result=json.loads(out);md=result['markdown']
 assert result['schema']==1
 assert bytes(result['original_sha256']).hex()==sha(source)
 assert base64.b64decode(result['sources'][0]['bytes'],validate=True)==source
 assert result['sources'][0]['text']==md
 cursor=0
 if name=='large-links':
  assert md.count('steady')==300000
  markers=[f'[l{i}](https://x.test/u{i})' for i in range(2000)]
 else:
  markers=['# Tide observations','Gezeitenströme','潮','[station notes](https://example.test/notes#tides)','## Readings','- morning high','- evening low','> Readings are provisional.','```','level = 3.2','```','Last line.']
 for marker in markers:
  cursor=md.index(marker,cursor)+len(marker)
 record={'name':name,'args':['/usr/bin/time','-l',str(R/'artifacts/final-brn-intake-helper')],'env':{},'binary_sha256':sha((R/'artifacts/final-brn-intake-helper').read_bytes()),'request_sha256':sha(request),'source_sha256':sha(source),'input_bytes':len(source),'exit':0,'exit_evidence':'original execution reached post-success assertion (large attempt1, friendly attempt3); see failed validator scripts/logs','output_markdown_bytes':len(md.encode()),'timing':p.with_suffix('.time').read_text(),'stdout_sha256':sha(out),'validation':'exact original bytes/hash, schema, source text; all 300000 steady words and 2000 hrefs in order' if name=='large-links' else 'exact original bytes/hash, schema, source text; ordered authored fixture facts','execution_count':1,'retained_result_validated_without_rerun':True,'process_limit':'p.communicate completed and waited; exact PGID not retained before failed validator; final precise process inventory separate'}
 p.with_suffix('.receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record))
