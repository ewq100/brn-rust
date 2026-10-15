#!/usr/bin/env python3
"""Check exact page/phrase/order and table associations against independent inventories.

Scan text and vector graphics are deliberately assessed through the saved visual
review and exact embedded-image pixels; this program never calls an OCR model.
"""
import hashlib,json,pathlib,re
ROOT=pathlib.Path(__file__).resolve().parent; REPO=ROOT.parent.parent; OUT=ROOT/'evidence'
fold=lambda s:' '.join(s.split())
def ordered_units(text,units):
    text=fold(text); cursor=0
    for unit in units:
        found=text.find(fold(unit),cursor)
        if found<0: return dict(passed=False,missing_or_out_of_order=unit)
        cursor=found+len(fold(unit))
    return dict(passed=True,units=len(units),exact_substantive_sequence=text==fold(' '.join(units)))
def page_lines(fixture,backend,page):
    return [fold(line) for line in (OUT/fixture/backend/f'page-{page}.txt').read_text().splitlines() if line.strip()]
results={}
source=json.loads((REPO/'crates/brn-threads-intake/tests/fixtures/harbor-study.inventory.json').read_text())
for backend in ['poppler','xberg']:
    checks={}
    for page in [1,2]:
        units=[x['needle'] for x in source['items'] if x['locator'].startswith(f'page:{page}')]
        checks[f'page{page}_ordered_inventory']=ordered_units(' '.join(page_lines('harbor-study',backend,page)),units)
    # Full row tuples, in their header/row order, rather than independent values.
    assert all(v['exact_substantive_sequence'] for v in checks.values())
    checks['table_rows']=ordered_units(' '.join(page_lines('harbor-study',backend,1)),['Route Median (min) Missed handoffs','Check first 12 1','Test first 19 3'])
    results[f'harbor-study/{backend}']=checks
source=json.loads((ROOT/'source-inventory.json').read_text())['process-scan']
for backend in ['poppler','xberg']:
    checks={'page1_ordered_inventory':ordered_units(' '.join(page_lines('process-scan',backend,1)),source['page1_source_units'])}
    lines=page_lines('process-scan',backend,2)
    units=source['page2_before_table']+[cell for row in source['page2_table'] for cell in row]+source['page2_after_table']
    checks['page2_raw_source_order']=ordered_units(' '.join(lines),units)
    # A hand-checked, COMPLETE permutation of untouched nonempty source lines,
    # using the existing schema-2 contract; no candidate text is rewritten.
    order=list(range(len(lines))) if backend=='poppler' else [0,1,2,3,4,8,5,9,6,10,7,11,12,13,14,15,16,17]
    assert sorted(order)==list(range(len(lines))), 'incomplete or duplicate source-line permutation'
    checks['source_line_order']=order
    reflow=' '.join(lines[i] for i in order)
    checks['page2_reflow_inventory']=ordered_units(reflow,units)
    assert checks['page2_reflow_inventory']['exact_substantive_sequence']
    assert checks['page1_ordered_inventory']['exact_substantive_sequence']
    checks['scan_has_no_native_text']=not page_lines('process-scan',backend,3)
    assert checks['scan_has_no_native_text'], 'scan-only page acquired nonempty native text'
    results[f'process-scan/{backend}']=checks
comparison=json.loads((OUT/'comparison.json').read_text())
assert next(f['sha256'] for f in comparison if f['fixture']=='process-scan')==source['sha256']
assert next(f['sha256'] for f in comparison if f['fixture']=='harbor-study')=='f5739675823b6900a212bdd10e27423f7fc29038c03fb7d103fa78430abd3fe0'
for fixture in comparison:
    pop=fixture['images']['poppler']; xb=fixture['images']['xberg']
    results[fixture['fixture']+'/images']={'counts_match':len(pop)==len(xb)==1,'ordered_pixels_equal':[(p['size'],p['rgb_sha256']) for p in pop]==[(p['size'],p['rgb_sha256']) for p in xb]}
(OUT/'inventory-checks.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
assert all(check['passed'] for key,checks in results.items() if '/images' not in key for name,check in checks.items() if isinstance(check,dict) and name!='page2_raw_source_order')
assert results['process-scan/poppler']['page2_raw_source_order']['passed']
assert not results['process-scan/xberg']['page2_raw_source_order']['passed']
assert all(checks['ordered_pixels_equal'] and checks['counts_match'] for key,checks in results.items() if '/images' in key)
