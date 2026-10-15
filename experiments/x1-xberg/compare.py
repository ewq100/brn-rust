#!/usr/bin/env python3
"""Run the two maintained synthetic PDFs; record raw outputs and source lines.

Requires installed Poppler and the Codex bundled Python (Pillow). Poppler XML
projection mirrors prepare.rs's isolated-row rule for these trusted outputs;
this script does not claim to execute the BRN helper or validate arbitrary XML.
"""
import collections, difflib, hashlib, json, os, pathlib, shlex, subprocess, time
import xml.etree.ElementTree as ET
from PIL import Image, ImageChops, ImageStat, ImageDraw
ROOT=pathlib.Path(__file__).resolve().parent; REPO=ROOT.parent.parent; OUT=ROOT/'evidence'
BIN=pathlib.Path('/private/tmp/brn-x1-target/release/xberg-probe')
FIXTURES=[('harbor-study',REPO/'crates/brn-threads-intake/tests/fixtures/harbor-study.pdf'),('process-scan',REPO/'crates/brn-threads-intake/tests/fixtures/luna/process-scan.pdf')]
COMMANDS=[]
def run(cmd, destination=None, **kwargs):
    start=time.monotonic()
    result=subprocess.run([str(c) for c in cmd],capture_output=True,timeout=60,**kwargs)
    COMMANDS.append(dict(command=shlex.join([str(c) for c in cmd]),returncode=result.returncode,seconds=time.monotonic()-start))
    (OUT/'comparison-commands.json').write_text(json.dumps(COMMANDS,indent=2)+'\n')
    if destination:
        destination.write_bytes(result.stdout); destination.with_suffix(destination.suffix+'.stderr').write_bytes(result.stderr)
    if result.returncode: raise RuntimeError(result.stderr.decode(errors='replace'))
    return result.stdout

def poppler_pages(xml):
    doc=ET.fromstring(xml)
    local=lambda e:e.tag.rsplit('}',1)[-1]
    pages=[]
    for page in (e for e in doc.iter() if local(e)=='page'):
        lines=[e for e in page.iter() if local(e)=='line']; texts=[' '.join(''.join(w.itertext()) for w in e if local(w)=='word')+' ' for e in lines]
        ids={id(e):i for i,e in enumerate(lines)}; flow={}; block={}
        for group, table in [('flow',flow),('block',block)]:
            for element in (e for e in page.iter() if local(e)==group):
                members=[ids[id(e)] for e in element.iter() if local(e)=='line']
                for i in members: table[i]=len(members)
        coords=[tuple(float(e.attrib[k]) for k in ['xMin','yMin','xMax','yMax']) if all(k in e.attrib for k in ['xMin','yMin','xMax','yMax']) else None for e in lines]
        anchors={}; joined=set()
        if all(c is not None for c in coords):
            rows=collections.defaultdict(list)
            for i,c in enumerate(coords): rows[c[1],c[3]].append(i)
            for members in rows.values():
                if len(members)<2 or any(coords[i][0]>=coords[i][2] or coords[i][1]>=coords[i][3] or flow.get(i)!=1 or block.get(i)!=1 for i in members): continue
                ordered=sorted(members,key=lambda i:coords[i][0])
                if any(coords[a][2]>coords[b][0] for a,b in zip(ordered,ordered[1:])): continue
                anchors[members[0]]=ordered; joined.update(members)
        order=[]
        for i in range(len(lines)):
            if i in anchors: order.extend(anchors[i])
            elif i not in joined: order.append(i)
        pages.append(dict(width=float(page.attrib['width']),height=float(page.attrib['height']),raw_lines=[t.strip() for t in texts],source_lines=[texts[i].strip() for i in order],text=''.join(texts[i]+'\n' for i in order)))
    return pages

def image_record(path):
    with Image.open(path) as im:
        rgb=im.convert('RGB')
        return dict(file=path.name,size=list(im.size),bytes=path.stat().st_size,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgb_sha256=hashlib.sha256(rgb.tobytes()).hexdigest())

summary=[]
for name,source in FIXTURES:
    folder=OUT/name; pop=folder/'poppler'; xb=folder/'xberg'
    pop.mkdir(parents=True,exist_ok=True); xb.mkdir(parents=True,exist_ok=True)
    sandbox=['/usr/bin/sandbox-exec','-f',ROOT/'runtime.sb','-D',f'BINARY={BIN}','-D',f'INPUT={source}','-D',f'OUTPUT={xb}']
    run(sandbox+[BIN,source,xb],folder/'xberg-run.log',env={'PATH':'/usr/bin:/bin','TMPDIR':str(xb)})
    run(['/opt/homebrew/bin/pdftotext','-bbox-layout','-enc','UTF-8',source,'-'],pop/'bbox.xhtml')
    run(['/opt/homebrew/bin/pdftotext','-layout','-enc','UTF-8',source,'-'],pop/'layout.txt')
    run(['/opt/homebrew/bin/pdfimages','-list',source],pop/'images-list.txt')
    run(['/opt/homebrew/bin/pdfimages','-png',source,pop/'image'],pop/'images-extract.log')
    pages=poppler_pages((pop/'bbox.xhtml').read_bytes())
    run(['/opt/homebrew/bin/pdftoppm','-f','1','-l',str(len(pages)),'-scale-to','1600','-png',source,pop/'preview'],pop/'render.log')
    (pop/'pages.json').write_text(json.dumps(pages,indent=2)+'\n')
    result=json.loads((xb/'result.json').read_text()); records=[]
    for n,page in enumerate(pages,1):
        (pop/f'page-{n}.txt').write_text(page['text'])
        xlines=[line.strip() for line in (xb/f'page-{n}.txt').read_text().splitlines() if line.strip()]
        plines=page['source_lines']
        (folder/f'page-{n}-lines.diff').write_text(''.join(difflib.unified_diff([x+'\n' for x in plines],[x+'\n' for x in xlines],fromfile='poppler prepared source',tofile='xberg plain page')))
        raw=Image.open(xb/f'render-{n}.png').convert('RGB'); rawsize=raw.size
        ratio=1600/max(raw.size); size=tuple(round(c*ratio) for c in raw.size)
        raw.resize(size,Image.Resampling.LANCZOS).save(xb/f'preview-{n}.png')
        candidate=Image.open(xb/f'preview-{n}.png').convert('RGB'); baseline=Image.open(pop/f'preview-{n}.png').convert('RGB')
        comparison=candidate.resize(baseline.size,Image.Resampling.LANCZOS) if candidate.size!=baseline.size else candidate
        difference=ImageChops.difference(baseline,comparison); stat=ImageStat.Stat(difference)
        records.append(dict(page=n,poppler_lines=len(plines),xberg_lines=len(xlines),exact_line_sequence=plines==xlines,whitespace_folded_text_equal=' '.join(plines).split()==' '.join(xlines).split(),xberg_render_raw_size=list(rawsize),poppler_preview=image_record(pop/f'preview-{n}.png'),xberg_preview=image_record(xb/f'preview-{n}.png'),mean_absolute_rgb_difference=sum(stat.mean)/3))
    images=dict(poppler=[image_record(p) for p in sorted(pop.glob('image-*.png'))],xberg=[image_record(p) for p in sorted(xb.glob('image-*.png'))])
    summary.append(dict(fixture=name,source=str(source.relative_to(REPO)),sha256=hashlib.sha256(source.read_bytes()).hexdigest(),pages=records,images=images,render_warnings=result['render_warnings'],warnings=result['warnings']))
for fixture in summary:
    for page in fixture['pages']:
        n=page['page']; folder=OUT/fixture['fixture']
        canvas=Image.new('RGB',(1640,1080),'#ddd'); draw=ImageDraw.Draw(canvas)
        for i,backend in enumerate(['poppler','xberg']):
            im=Image.open(folder/backend/f'preview-{n}.png').convert('RGB'); im.thumbnail((800,1035))
            canvas.paste(im,(i*820+10,35)); draw.text((i*820+10,10),f"{fixture['fixture']} page {n}: {backend}",fill='black')
        canvas.save(folder/f'visual-page-{n}.png')
(OUT/'comparison.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
