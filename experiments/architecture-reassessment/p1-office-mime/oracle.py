"""Independent stdlib producer/MIME oracle plus output checks for the bounded corpus."""
from pathlib import Path
import json,hashlib,zipfile,xml.etree.ElementTree as ET
from email import policy
from email.parser import BytesParser
R=Path(__file__).resolve().parent;E=R/'evidence';F=R/'fixtures'
hash=lambda b:hashlib.sha256(b).hexdigest()
load=lambda name:json.loads((E/name/'result.json').read_text())
image=(F/'water-use.png').read_bytes(); digest=hash(image)
assert image[:8]==b'\x89PNG\r\n\x1a\n'
# Independently inventory drawing containers and relationships in original producer archives.
ns={'wp':'http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing','a':'http://schemas.openxmlformats.org/drawingml/2006/main','p':'http://schemas.openxmlformats.org/presentationml/2006/main','r':'http://schemas.openxmlformats.org/officeDocument/2006/relationships'}
for kind,part,relpart,selector,prop in [('docx','word/document.xml','word/_rels/document.xml.rels','.//wp:inline','wp:docPr'),('pptx','ppt/slides/slide1.xml','ppt/slides/_rels/slide1.xml.rels','.//p:pic','p:nvPicPr/p:cNvPr')]:
 out=load('harbor-'+kind)
 with zipfile.ZipFile(F/('harbor.'+kind)) as z:
  tree=ET.fromstring(z.read(part));rels={x.attrib['Id']:x.attrib['Target'] for x in ET.fromstring(z.read(relpart))}
  pics=tree.findall(selector,ns); assert len(pics)==2
  for pic,occ in zip(pics,out['occurrences']):
   pr=pic.find(prop,ns);rid=pic.find('.//a:blip',ns).attrib['{'+ns['r']+'}embed']
   assert occ['picture_id']==pr.attrib['id'] and occ['title']==pr.attrib['title']
   assert occ['relationship']==rid and occ['asset_sha256']==digest
   assert (E/('harbor-'+kind)/(digest+'.png')).read_bytes()==image
   assert occ['source']==hash((F/('harbor.'+kind)).read_bytes())
  assert len({o['picture_id'] for o in out['occurrences']})==2
  assert len({o['asset_sha256'] for o in out['occurrences']})==1
  if kind=='pptx':
   assert tree.find('.//p:sp/p:nvSpPr/p:cNvPr[@title="Not a picture: dependency gate"]',ns) is not None
   assert all('Not a picture' not in o['title'] for o in out['occurrences'])
   assert out['charts_without_visual_preview']==['ppt/charts/chart1.xml']
   assert any('Presenter note: ask Mira' in s for s in out['text'])
   assert any('only after valve inspection' in s for s in out['text'])
  else:
   assert out['text'].count('Pending')==2
   pending=[p for p in out['text_evidence'] if p['text']=='Pending'];assert len(pending)==2 and pending[0]['wire_pointer']!=pending[1]['wire_pointer']
   assert any('budget capped at EUR 4,000' in s for s in out['text'])
   assert any('review date 12 October' in s for s in out['text'])
parents=[];docnodes=[]
for case,count in [('single',2),('plural',3)]:
 raw=(F/(case+'.eml')).read_bytes();m=BytesParser(policy=policy.default).parsebytes(raw);result=load(case+'-eml');parents.append(result['source']);assert result['source']==hash(raw)
 assert result['subject']=='Harbor pilot – review' and '+0300' in result['date_raw']
 assert result['message_id']==f'harbor-{case}@example.test'
 leaves=[p for p in m.walk() if not p.is_multipart() and p.get_filename()]; assert len(leaves)==len(result['children'])==count
 for p,child in zip(leaves,result['children']):
  b=p.get_payload(decode=True);assert child['sha256']==hash(b);assert child['parent']==result['source'];assert child['node']==result['source']+'-attachment-'+str(child['mime_attachment_index'])
  assert (E/(case+'-eml')/(child['sha256']+'.original')).read_bytes()==b
  if child['filename']=='harbor.docx':
   docnodes.append(child['node']);assert b==(F/'harbor.docx').read_bytes();assert child['extraction']['status']=='partial'
   assert len(child['extraction']['occurrences'])==2
   assert all(o['source']==child['node'] for o in child['extraction']['occurrences'])
  elif child['filename']=='forecast.xlsx':assert child['extraction']['status']=='unprocessed' and b==(F/'forecast.xlsx').read_bytes()
  else:assert child['cid']=='water-evidence' and b==image and child['extraction']['status']=='retained-inline' and 'cid:water-evidence' in result['html']
assert parents[0]!=parents[1] and docnodes[0]!=docnodes[1]
for kind,reason in [('path','unsafe member path'),('dtd','DTD forbidden'),('inflate','inflated budget 16MiB'),('members','member budget 512')]:
 r=load('hostile-'+kind+'-docx');assert r=={'status':'rejected','reason':reason}
measure=json.loads((E/'measurements.json').read_text());assert all(r['exit']==0 and not r['wall_timeout'] for r in measure['runs']);assert all(measure['restrictions']['assertions'].values());assert measure['cancel']['assertion']
summary={'result':'PASS','checks':['producer ZIP picture title/relationship/exact PNG','repeated PNG preserves two picture occurrences','titled nonpicture excluded','DOCX ordered duplicate cells and story content','PPTX notes/shape text; chart visual visibly omitted','stdlib MIME exact CID/attachment bytes and independent parent nodes','unsupported XLSX retained/unprocessed','four hostile outer-admission rejections','actual negative-access and process-tree cancellation witnesses']}
(E/'oracle.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
