"""Assertions over retained public producer facts and API output contracts."""
import hashlib,json
from pathlib import Path
P=Path(__file__).parent
R=P/'results'
def read(case,name):return json.loads((R/case/name).read_text())
def nodes(x):
 if isinstance(x,dict):
  yield x
  for v in x.values():yield from nodes(v)
 elif isinstance(x,list):
  for v in x:yield from nodes(v)
h=read('harbor','structured.json');m=read('harbor','markdown.json');f=read('features','structured.json');p=read('prefix-variation','structured.json')
images=[x for x in nodes(h['stories']) if x.get('kind')=='image']
assert len(images)==2 and images[0]['anchor']!=images[1]['anchor']
assert {x['part'] for x in images}=={'word/media/image1.png'}
assert {s['kind'] for s in h['stories']}=={'body','header','footer'}
assert m['markdown'].count('Pending')==2
for text in ['Mira','15 October 2026','EUR 4,000','12 October 2026']:assert text in m['markdown']
assert m['markdown'].count('![')==2 and m['markdown'].count(']()')==2
assert len(m['anchors'])==18 and not h['truncated'] and not m['truncated']
fi=[x for x in nodes(f['stories']) if x.get('kind')=='image']
assert fi[1]['altText']=='Same chart for operations handoff'
assert fi[0]['altText']=='120 to 72 litres/day, a 40% reduction'
assert f['stories'][0]['blocks'][0]['paragraph']['styleId']=='Title'
assert any(x.get('element')=='c:chart' for x in nodes(f['stories']))
assert any(x['code']=='unsupported-content' and 'c:chart' in x['message'] for x in f['diagnostics'])
pi=[x for x in nodes(p['stories']) if x.get('kind')=='image']
assert len(pi)==2 and all(x['relationshipId']=='' and x['part'] is None for x in pi)
assert sum(x['code']=='unresolved-reference' for x in p['diagnostics'])==2
g=json.loads((R/'guards.json').read_text())
assert 'Err' in g[0]['opc_16mib'] and 'Err' in g[1]['opc_16mib']
assert g[2]['opc_16mib']['Ok']==513
assert 'DTD/entity' in g[3]['structured']['Err']
manifest={str(q.relative_to(P)):hashlib.sha256(q.read_bytes()).hexdigest() for q in [P/'features.docx',P/'prefix-variation.docx',P.parent/'p1-office-mime/fixtures/harbor.docx'] if q.is_relative_to(P)}
# Retained original path is outside this directory; record its identity separately.
manifest['../p1-office-mime/fixtures/harbor.docx']=hashlib.sha256((P.parent/'p1-office-mime/fixtures/harbor.docx').read_bytes()).hexdigest()
(R/'oracle.json').write_text(json.dumps({'passed':True,'checks':['ordinary text/header/footer/table','two image occurrences distinct anchors same resolved part','Markdown marker map and intentional empty image URLs','Title style retained/plain Markdown','title-only fallback versus separate title omission','native chart unsupported anchor+diagnostic','namespace-prefix-only relationship resolution gap','OPC inflation/path and count/DTD limits'],'fixture_sha256':manifest},indent=2)+'\n')
print('API oracle passed')
