#!/usr/bin/env python3
"""Offline shipping entry-point witness using fresh explicit Threads data."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

runtime=Path(sys.argv[1]).resolve(strict=True)
parent=Path(os.environ['TMPDIR']).resolve(strict=True)
with tempfile.TemporaryDirectory(prefix='brn-threads-fixture-',dir=parent) as root:
    root=Path(root);data=root/'data'
    def call(*args,ok=True):
        result=subprocess.run([str(runtime/'brn'),'--data-dir',str(data),*args],capture_output=True,text=True,timeout=30)
        if result.returncode==0:
            if not ok: raise AssertionError('Unexpected success')
            return json.loads(result.stdout)
        if ok: raise AssertionError(result.stderr)
    # A test workspace (B1 phase 1): created explicitly, before the first write.
    call('init','--purpose','test')
    note=call('note-create','--title','Harbor','--text','Monday delivery')['id']
    token=str(root/'edit.token')
    session=call('begin-edit','--note',note,'--base','1','--token-out',token)['id']
    call('update-buffer','--session',session,'--generation','1','--text','Tuesday delivery','--token-file',token)
    assert call('recovery')[0]['markdown']=='Tuesday delivery'
    saved=call('save','--session',session,'--generation','1','--token-file',token)
    assert saved==call('save','--session',session,'--generation','1','--token-file',token)
    assert call('read','--id',note)['data']['Note']['markdown']=='Tuesday delivery'
    assert call('search','--query','Tuesday')['hits'][0]['note']==note
    call('export','--destination',str(root/'export'))
    call('export','--destination',str(root/'export'),ok=False)
    call('backup','--destination',str(root/'backup.sqlite'))
    call('backup','--destination',str(root/'backup.sqlite'),ok=False)
    for _ in range(2):
        subprocess.run([str(runtime/'brn-desktop'),'--data-dir',str(data),'--headless-check','startup'],check=True,capture_output=True,text=True,timeout=30)
    assert (data/'threads.sqlite3').is_file()
    assert not (data/'brn.sqlite').exists()
    old=root/'old';old.mkdir();(old/'brn.sqlite').write_bytes(b'old synthetic authority')
    refused=subprocess.run([str(runtime/'brn'),'--data-dir',str(old),'init'],capture_output=True,text=True,timeout=30)
    assert refused.returncode!=0 and (old/'brn.sqlite').read_bytes()==b'old synthetic authority'
print('Threads offline Save/recovery/search/export/backup/startup/refusal journeys passed')
