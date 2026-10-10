from pathlib import Path
import json,sys,datetime,sqlite3,hashlib
r=Path('/private/tmp/brn-ui-repairs-macbook-20261010')
case,checks,status,note=sys.argv[1:5]
source=(r/'cases'/case) if case=='N' else Path('/private/tmp/brn-full-macbook-20261009/cases')/case
journal=r/'ui-observations.json'; events=json.loads(journal.read_text()) if journal.exists() else []
event={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'case':case,'checks':checks,'status':status,'source_case_path':str(source),'fresh_candidate_fixture':case=='N','native_observation':note,'runtime_manifest':str(r/'runtime-manifest.json')}
receipts=r/'receipts'/case; receipts.mkdir(parents=True,exist_ok=True)
snap=receipts/f'native-{len(events)+1}.sqlite'
conn=sqlite3.connect(f'file:{source}/data/brn.sqlite?mode=ro',uri=True); dst=sqlite3.connect(snap); conn.backup(dst); dst.close(); conn.close()
conn=sqlite3.connect(f'file:{snap}?mode=ro&immutable=1',uri=True)
event['snapshot']=str(snap); event['integrity']=conn.execute('pragma integrity_check').fetchall(); event['foreign_keys']=conn.execute('pragma foreign_key_check').fetchall()
event['counts']={t:conn.execute('select count(*) from '+t).fetchone()[0] for t in ['actions','proposals','proposal_applies','findings','conversations','inbox_items']}; conn.close()
event['vault_sha256']={str(f.relative_to(source/'vault')):hashlib.sha256(f.read_bytes()).hexdigest() for f in (source/'vault').rglob('*') if f.is_file()}
events.append(event); journal.write_text(json.dumps(events,ensure_ascii=False,indent=2)+'\n'); print(case,checks,status,event['counts'])
