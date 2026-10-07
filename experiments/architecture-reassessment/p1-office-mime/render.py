"""Small evidence viewer over upstream wire objects; not an Office renderer."""
from pathlib import Path
import json, html
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, Image, PageBreak
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.lib import colors
import subprocess, os
from pypdf import PdfReader
Path('/private/tmp/brn-p1-fontcache').mkdir(exist_ok=True)
Path('/private/tmp/brn-p1-fonts.conf').write_text('<?xml version="1.0"?><fontconfig><dir>/System/Library/Fonts</dir><dir>/Library/Fonts</dir><cachedir>/private/tmp/brn-p1-fontcache</cachedir></fontconfig>')
R=Path(__file__).resolve().parent/'evidence'; S=getSampleStyleSheet(); S['BodyText'].fontSize=10; S['BodyText'].leading=14

def para(s,style='BodyText'):return Paragraph(html.escape(str(s)),S[style])
def leaves(v,key='text'):
 if isinstance(v,dict):
  for k,x in v.items():
   if k==key and isinstance(x,str):yield x
   else:yield from leaves(x,key)
 elif isinstance(v,list):
  for x in v:yield from leaves(x,key)
def images(v):
 if isinstance(v,dict):
  if 'image' in v:yield v['image']
  for x in v.values():yield from images(x)
 elif isinstance(v,list):
  for x in v:yield from images(x)
def picture(occ,directory,width=440):
 return [para(f"Occurrence {occ['occurrence']} - {occ['title']}"),Image(str(directory/(occ['asset_sha256']+'.png')),width=width,height=width*500/900),para(f"{occ['part']} picture {occ['picture_id']} / {occ['relationship']} / asset {occ['asset_sha256'][:16]}",'Italic'),Spacer(1,12)]
def output(kind):
 d=R/f'harbor-{kind}';result=json.loads((d/'result.json').read_text());wire=json.loads(next(d.glob('*.wire.json')).read_text()); story=[para(f'Adapted {kind.upper()} evidence - PARTIAL','Title'),para(result['gaps'][0]),Spacer(1,12)]
 if kind=='docx':
  pkg=wire['document']['package']
  for _,entry in pkg['headerEntries']:story += [para('Header','Heading2'),para(' '.join(leaves(entry)))]
  for block in pkg['document']['content']:
   typ=block['type']; ordinal=block.get('sourceOrdinal')
   if typ=='table':
    rows=[[para(' '.join(leaves(c))) for c in row['cells']] for row in block['rows']]
    t=Table(rows,colWidths=[150]*3);t.setStyle(TableStyle([('GRID',(0,0),(-1,-1),.5,colors.grey),('BACKGROUND',(0,0),(-1,0),colors.HexColor('#DDEEE9')),('VALIGN',(0,0),(-1,-1),'TOP'),('TOPPADDING',(0,0),(-1,-1),7),('BOTTOMPADDING',(0,0),(-1,-1),7)]));story += [t,Spacer(1,10)]
   else:
    tx=' '.join(leaves(block))
    if tx:story += [para(tx),para(f'word/document.xml source ordinal {ordinal}','Italic'),Spacer(1,8)]
    for im in images(block):
     occ=next(o for o in result['occurrences'] if o['picture_id']==str(im['id']));story+=picture(occ,d)
  for _,entry in pkg['footerEntries']:story += [para('Footer','Heading2'),para(' '.join(leaves(entry)))]
 else:
  for i,slide in enumerate(wire['slides']):
   if i:story.append(PageBreak())
   story += [para(f"Slide {i+1}: {slide['partPath']}",'Heading1')]
   for shape in slide['shapes']:
    if shape['kind']=='picture':
     occ=next(o for o in result['occurrences'] if o['picture_id']==str(shape['id']) and o['part']==slide['partPath']);story+=picture(occ,d,350)
    else:
     tx=' '.join(leaves(shape))
     if tx:story += [para(tx),para(f"{slide['partPath']} shape {shape['id']}",'Italic'),Spacer(1,8)]
     if shape['kind']=='graphicFrame':story += [para('OMITTED VISUAL: native chart is not rendered here. Inspect the retained original slide for values and visual relationships.','Heading2')]
   if slide.get('notes'):story += [para('Presenter notes','Heading2'),para(slide['notes'])]
  story += [para('Chart parts without adapted visual preview: '+', '.join(result['charts_without_visual_preview']))]
 SimpleDocTemplate(str(R/f'adapted-{kind}.pdf'),rightMargin=40,leftMargin=40,topMargin=35,bottomMargin=35).build(story)
for k in ['docx','pptx']:output(k)
for src in [R/'original-docx'/'harbor.pdf',R/'original-pptx'/'harbor.pdf',R/'adapted-docx.pdf',R/'adapted-pptx.pdf']:
 subprocess.run(['pdftoppm','-scale-to','1300','-png',str(src),str(src.parent/(src.stem+'-page'))],check=True,env={**os.environ,'FONTCONFIG_FILE':'/private/tmp/brn-p1-fonts.conf'},stderr=open(R/'render.stderr.txt','a'),timeout=30)
 print(src.relative_to(R),len(PdfReader(src).pages),'pages')
