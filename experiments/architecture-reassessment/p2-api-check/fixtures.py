"""Synthetic variation of retained P1 producer fixtures; no user documents."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from xml.etree import ElementTree as E
P=Path(__file__).parent
F=P.parent/'p1-office-mime'/'fixtures'
W='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
A='http://schemas.openxmlformats.org/drawingml/2006/main'
WP='http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing'
R='http://schemas.openxmlformats.org/officeDocument/2006/relationships'
C='http://schemas.openxmlformats.org/drawingml/2006/chart'
for prefix,uri in [('w',W),('a',A),('wp',WP),('r',R),('c',C),('pic','http://schemas.openxmlformats.org/drawingml/2006/picture'),('mc','http://schemas.openxmlformats.org/markup-compatibility/2006')]: E.register_namespace(prefix,uri)
with ZipFile(F/'harbor.docx') as z: parts={n:z.read(n) for n in z.namelist()}
root=E.fromstring(parts['word/document.xml'])
body=root.find(f'{{{W}}}body')
first=body.find(f'{{{W}}}p')
pr=first.find(f'{{{W}}}pPr')
if pr is None: pr=E.Element(f'{{{W}}}pPr'); first.insert(0,pr)
style=pr.find(f'{{{W}}}pStyle')
if style is None: style=E.SubElement(pr,f'{{{W}}}pStyle')
style.set(f'{{{W}}}val','Title')
# Second picture has only title, to distinguish fallback from separate title preservation.
pictures=root.findall(f'.//{{{WP}}}docPr')
pictures[1].attrib.pop('descr',None)
# Native chart reference reuses the producer chart from retained P1 PPTX.
p=E.Element(f'{{{W}}}p'); run=E.SubElement(p,f'{{{W}}}r'); drawing=E.SubElement(run,f'{{{W}}}drawing')
inline=E.SubElement(drawing,f'{{{WP}}}inline')
E.SubElement(inline,f'{{{WP}}}extent',cx='4000000',cy='2500000')
E.SubElement(inline,f'{{{WP}}}docPr',id='900',name='Native inspection chart',title='Chart title must stay separate')
graphic=E.SubElement(inline,f'{{{A}}}graphic'); data=E.SubElement(graphic,f'{{{A}}}graphicData',uri=C)
E.SubElement(data,f'{{{C}}}chart',{f'{{{R}}}id':'rIdP2Chart'})
body.insert(len(body)-1,p)
parts['word/document.xml']=E.tostring(root,encoding='utf-8',xml_declaration=True)
rels=E.fromstring(parts['word/_rels/document.xml.rels'])
E.SubElement(rels,'{http://schemas.openxmlformats.org/package/2006/relationships}Relationship',Id='rIdP2Chart',Type=R+'/chart',Target='charts/chart1.xml')
parts['word/_rels/document.xml.rels']=E.tostring(rels,encoding='utf-8',xml_declaration=True)
with ZipFile(F/'harbor.pptx') as z: parts['word/charts/chart1.xml']=z.read('ppt/charts/chart1.xml')
types=E.fromstring(parts['[Content_Types].xml'])
E.SubElement(types,'{http://schemas.openxmlformats.org/package/2006/content-types}Override',PartName='/word/charts/chart1.xml',ContentType='application/vnd.openxmlformats-officedocument.drawingml.chart+xml')
parts['[Content_Types].xml']=E.tostring(types,encoding='utf-8',xml_declaration=True)
with ZipFile(P/'features.docx','w',ZIP_DEFLATED) as z:
 for n,b in parts.items(): z.writestr(n,b)
# A second package changes only namespace prefixes on the retained body XML.
# Namespace-expanded element/attribute tree stays identical.
with ZipFile(F/'harbor.docx') as z: alternate={n:z.read(n) for n in z.namelist()}
original=alternate['word/document.xml']
changed=original.replace(b'w:',b'xw:').replace(b'xmlns:w=',b'xmlns:xw=').replace(b'pic:',b'xpic:').replace(b'xmlns:pic=',b'xmlns:xpic=')
def expanded(e): return (e.tag,sorted(e.attrib.items()),e.text,e.tail,[expanded(c) for c in e])
assert expanded(E.fromstring(original))==expanded(E.fromstring(changed))
alternate['word/document.xml']=changed
with ZipFile(P/'prefix-variation.docx','w',ZIP_DEFLATED) as z:
 for n,b in alternate.items(): z.writestr(n,b)
