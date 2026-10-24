#!/usr/bin/env python3
"""Deterministic synthetic images: exact native JPEG in DOCX and bounded PDF."""
from pathlib import Path
import zipfile
root = Path(__file__).resolve().parents[4]
out = Path(__file__).resolve().parent
original = root / 'crates/brn-intake/tests/fixtures'
jpeg = (original / 'quay-map.jpg').read_bytes()
for name, image in [('jpeg-figure.docx', jpeg), ('invalid-image.docx', b'not a JPEG image')]:
    with zipfile.ZipFile(original / 'harbor.docx') as src, zipfile.ZipFile(out / name, 'w', zipfile.ZIP_DEFLATED) as dst:
        for entry in src.infolist():
            payload = src.read(entry.filename)
            target = entry.filename.replace('image1.png', 'image1.jpg')
            if entry.filename == 'word/media/image1.png':
                payload = image
            elif entry.filename.endswith(('.xml', '.rels')):
                payload = payload.replace(b'image1.png', b'image1.jpg').replace(b'Extension="png" ContentType="image/png"', b'Extension="jpg" ContentType="image/jpeg"')
            info = zipfile.ZipInfo(target, (2026, 10, 23, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            dst.writestr(info, payload)
# A hand-authored one-page PDF, no external assets or fonts. Four RGB pixels.
import zlib
pixels = bytes([255,0,0, 0,255,0, 0,0,255, 255,255,255])
image = zlib.compress(pixels)
content = b'BT /F1 12 Tf 30 160 Td (Synthetic retained figure) Tj ET\nq 100 0 0 100 30 30 cm /Im0 Do Q\n'
objects = [
 b'<< /Type /Catalog /Pages 2 0 R >>',
 b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
 b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 4 0 R >> /XObject << /Im0 5 0 R >> >> /Contents 6 0 R >>',
 b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
 b'<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length '+str(len(image)).encode()+b' >>\nstream\n'+image+b'\nendstream',
 b'<< /Length '+str(len(content)).encode()+b' >>\nstream\n'+content+b'endstream',
]
pdf = bytearray(b'%PDF-1.4\n'); offsets = [0]
for i, obj in enumerate(objects, 1):
    offsets.append(len(pdf)); pdf += f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n'
xref = len(pdf)
pdf += f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode()
for offset in offsets[1:]: pdf += f'{offset:010d} 00000 n \n'.encode()
pdf += f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode()
(out / 'four-pixels.pdf').write_bytes(pdf)
