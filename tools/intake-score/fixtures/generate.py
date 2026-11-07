#!/usr/bin/env python3
"""Create small CC0 synthetic inputs with only the Python standard library.

Manifests are authored separately. This never calls or imports BRN or Xberg.
"""
from pathlib import Path
from zipfile import ZipFile, ZipInfo, ZIP_DEFLATED
from xml.sax.saxutils import escape
import struct
import zlib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def package(name, entries):
    with ZipFile(HERE / name, "w") as out:
        for path, data in sorted(entries.items()):
            info = ZipInfo(path, (2026, 11, 7, 0, 0, 0))
            info.compress_type = ZIP_DEFLATED
            out.writestr(info, data)


def from_template(path):
    with ZipFile(ROOT / path) as source:
        return {n: source.read(n) for n in source.namelist()}


def paragraph(text, style=""):
    props = f'<w:pPr><w:pStyle w:val="{style}"/></w:pPr>' if style else ""
    return f'<w:p>{props}<w:r><w:t>{escape(text)}</w:t></w:r></w:p>'


def cell(text):
    return f'<w:tc>{paragraph(text)}</w:tc>'


def table(rows):
    return '<w:tbl><w:tblGrid><w:gridCol w:w="2400"/><w:gridCol w:w="2400"/></w:tblGrid>' + ''.join(
        '<w:tr>' + ''.join(cell(c) for c in row) + '</w:tr>' for row in rows) + '</w:tbl>'


def nested_docx():
    entries = from_template("crates/brn-intake/tests/fixtures/harbor.docx")
    nested = table([["Inner gate", "Owner"], ["Inspect", "Iris"]])
    outer = table([["Stage", "Detail"], ["Prepare", "Nested schedule"]])
    outer = outer.replace(paragraph("Nested schedule"), paragraph("Nested schedule") + nested + paragraph("After nested table."))
    body = paragraph("Island handoff", "Heading1") + paragraph("Use the inner gate before dispatch.") + outer
    entries["word/document.xml"] = ('<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>' + body + '</w:body></w:document>').encode()
    # Remove unrelated template headers, figures and their relationships.
    entries = {p: v for p, v in entries.items() if not p.startswith(("word/media/", "word/header", "word/footer"))}
    entries["word/_rels/document.xml.rels"] = b'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>'
    package("nested.docx", entries)


def slides():
    entries = from_template("crates/brn-intake/tests/fixtures/harbor.pptx")
    for i, (title, body) in enumerate([
        ("Island inspection", "Inspect the north gate before noon."),
        ("Island dispatch", "Dispatch only after Iris signs the log.")], 1):
        entries[f"ppt/slides/slide{i}.xml"] = f'''<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{title}</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{body}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>'''.encode()
    entries = {p: v for p, v in entries.items() if not p.startswith(("ppt/media/", "ppt/charts/", "ppt/notesSlides/", "ppt/slides/_rels/"))}
    package("island.pptx", entries)


def workbook(name, sheet_names, rows):
    entries = {"[Content_Types].xml": '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>' + ''.join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' for i in range(1, 3)) + '</Types>',
        "_rels/.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        "xl/workbook.xml": '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>' + ''.join(f'<sheet name="{s}" sheetId="{i}" r:id="r{i}"/>' for i, s in enumerate(sheet_names, 1)) + '</sheets></workbook>',
        "xl/_rels/workbook.xml.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' + ''.join(f'<Relationship Id="r{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1, 3)) + '</Relationships>'}
    for i, content in enumerate(rows, 1):
        xml = '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>'
        for r, row in enumerate(content, 1):
            xml += f'<row r="{r}">' + ''.join(f'<c r="{chr(65+c)}{r}" t="inlineStr"><is><t>{escape(v)}</t></is></c>' for c, v in enumerate(row)) + '</row>'
        xml += '</sheetData>' + ('<mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells>' if i == 1 else '') + '</worksheet>'
        entries[f"xl/worksheets/sheet{i}.xml"] = xml
    package(name, entries)


def pdf(name, title, left, right):
    # Two independent text columns. Helvetica sizes encode the authored headings.
    def escaped(s):
        return s.replace('\\', '\\\\').replace('(', '\\(').replace(')', '\\)')
    ops = [f'BT /F1 20 Tf 50 760 Td ({escaped(title)}) Tj ET']
    for x, lines in [(50, left), (320, right)]:
        for i, line in enumerate(lines):
            size = 16 if i == 0 else 11
            ops.append(f'BT /F1 {size} Tf {x} {710-i*28} Td ({escaped(line)}) Tj ET')
    stream = '\n'.join(ops).encode()
    objects = [b'<< /Type /Catalog /Pages 2 0 R >>', b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>', b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>', b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>', f'<< /Length {len(stream)} >>\nstream\n'.encode() + stream + b'\nendstream']
    result = b'%PDF-1.4\n'; offsets = [0]
    for i, obj in enumerate(objects, 1):
        offsets.append(len(result)); result += f'{i} 0 obj\n'.encode() + obj + b'\nendobj\n'
    start = len(result)
    result += f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode() + b''.join(f'{o:010} 00000 n \n'.encode() for o in offsets[1:])
    result += f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n'.encode()
    (HERE / name).write_bytes(result)


def main():
    nested_docx(); slides()
    workbook("merged.xlsx", ["Island budget", "Staff"], [
        [["Island budget", ""], ["Item", "EUR"], ["Lamp", "45"], ["Rope", "12"]],
        [["Name", "Role"], ["Iris", "Inspector"], ["Omar", "Lead"]]])
    workbook("reserve.xlsx", ["Reserve stock", "Dispatch"], [
        [["Reserve stock", ""], ["Part", "Units"], ["Seal", "17"], ["Valve", "6"]],
        [["Day", "Owner"], ["Friday", "Lena"], ["Monday", "Kai"]]])
    pdf("columns.pdf", "Island field report", ["Measurements", "Morning tide was 3 metres.", "Wind held steady at 4 knots."], ["Limitations", "Only two stations were sampled.", "No storm observations were made."])
    pdf("reserve.pdf", "Reserve field report", ["Inventory", "Seventeen seals were counted.", "Six valves passed inspection."], ["Constraints", "Counts exclude the south shed.", "Repeat the count on Monday."])
    sources = {
        "notes.txt": "Island log\n\nIris checked 12 lamps at 09:30.\nKeep the gate closed until noon.\n",
        "unicode.txt": "Café observations\r\n\r\nTide Δ = 3.2 m; water 8 °C.\r\nNo final newline Ω",
        "reserve.txt": "Reserve audit\n\nLena counted 17 seals and 6 valves.\nThe south shed remains unchecked.\n",
        "outline.md": "# Island log\n\n## Gate checks\n\n- Inspect the lamp.\n- Sign the log.\n\n| Gate | Owner |\n| --- | --- |\n| North | Iris |\n| South | Omar |\n\nDo not open before noon.\n",
        "literal.md": "# Literal examples\n\n## Safety\n\n1. Read the label.\n2. Retain the serial number.\n\n```text\n# This is code, not a heading\n- This is code, not a list item\n```\n\nMeasured Δ = 3.2 m.\n",
        "reserve.md": "# Reserve audit\n\n## Parts\n\n- Count seals.\n- Count valves.\n\n| Part | Units |\n| --- | --- |\n| Seal | 17 |\n| Valve | 6 |\n\nThe south shed remains unchecked.\n",
        "structure.html": "<!doctype html><html><body><h1>Island log</h1><h2>Gate checks</h2><ul><li>Inspect the lamp.</li><li>Sign the log.</li></ul><table><tr><th>Gate</th><th>Owner</th></tr><tr><td>North</td><td>Iris</td></tr><tr><td>South</td><td>Omar</td></tr></table><p>Do not open before noon.</p></body></html>",
        "reserve.html": "<!doctype html><html><body><h1>Reserve audit</h1><h2>Parts</h2><ol><li>Count seals.</li><li>Count valves.</li></ol><table><tr><th>Part</th><th>Units</th></tr><tr><td>Seal</td><td>17</td></tr><tr><td>Valve</td><td>6</td></tr></table><p>The south shed remains unchecked.</p></body></html>",
        "reserve.eml": "From: Lena <lena@example.test>\nTo: Kai <kai@example.test>\nSubject: Reserve audit\nMIME-Version: 1.0\nContent-Type: text/plain; charset=utf-8\n\nLena counted 17 seals and 6 valves.\nThe south shed remains unchecked.\n"}
    for name, content in sources.items():
        (HERE / name).write_bytes(content.encode())


if __name__ == "__main__":
    main()
