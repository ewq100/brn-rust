from pathlib import Path
from zipfile import ZipFile,ZIP_DEFLATED
import shutil
p=Path(__file__).parent
shutil.copy('crates/brn-workflow/src/inbox_processing/fixtures/inline-png.docx',p/'sample.docx')
with ZipFile(p/'sample.docx') as z:
 image=next(z.read(x) for x in z.namelist() if x.endswith('.png'))
(p/'image.png').write_bytes(image)
pns='http://schemas.openxmlformats.org/presentationml/2006/main';ans='http://schemas.openxmlformats.org/drawingml/2006/main';rns='http://schemas.openxmlformats.org/officeDocument/2006/relationships'
rels=lambda body:f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>'
parts={
'[Content_Types].xml':f'<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="png" ContentType="image/png"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/></Types>',
'_rels/.rels':rels(f'<Relationship Id="rId0" Type="{rns}/officeDocument" Target="ppt/presentation.xml"/>'),
'ppt/presentation.xml':f'<p:presentation xmlns:p="{pns}" xmlns:r="{rns}"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>',
'ppt/_rels/presentation.xml.rels':rels(f'<Relationship Id="rId1" Type="{rns}/slide" Target="slides/slide1.xml"/>'),
'ppt/slides/slide1.xml':f'<p:sld xmlns:p="{pns}" xmlns:a="{ans}" xmlns:r="{rns}"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name="group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="text"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Slide sentinel</a:t></a:r></a:p></p:txBody></p:sp><p:pic><p:nvPicPr><p:cNvPr id="3" name="image" descr="Image alt" title="Image title"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10000" cy="10000"/></a:xfrm></p:spPr></p:pic></p:spTree></p:cSld></p:sld>',
'ppt/slides/_rels/slide1.xml.rels':rels(f'<Relationship Id="rId2" Type="{rns}/image" Target="../media/image1.png"/><Relationship Id="rId3" Type="{rns}/notesSlide" Target="../notesSlides/notesSlide1.xml"/>'),
'ppt/notesSlides/notesSlide1.xml':f'<p:notes xmlns:p="{pns}" xmlns:a="{ans}"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="2" name="notes"/><p:cNvSpPr/><p:nvPr><p:ph type="body"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Notes sentinel</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>',
'ppt/media/image1.png':image}
with ZipFile(p/'sample.pptx','w',ZIP_DEFLATED) as z:
 for n,b in parts.items(): z.writestr(n,b)
with ZipFile(p/'sample.docx') as src, ZipFile(p/'sample-with-gaps.docx','w',ZIP_DEFLATED) as dst:
 for n in src.namelist():
  b=src.read(n)
  if n=='word/document.xml':
   b=b.replace(b'</w:body>',b'<w:p><w:r><w:ruby><w:rt><w:r><w:t>RUBY_GUIDE_SENTINEL</w:t></w:r></w:rt><w:rubyBase><w:r><w:t>RUBY_BASE_SENTINEL</w:t></w:r></w:rubyBase></w:ruby></w:r></w:p><w:p><w:unknownWrapper><w:r><w:t>UNKNOWN_SENTINEL</w:t></w:r></w:unknownWrapper></w:p></w:body>')
  dst.writestr(n,b)
