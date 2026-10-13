import os,json,sqlite3,subprocess,tempfile
from pathlib import Path
# Both locations are explicit; the witness creates/removes only its own synthetic child.
exe=(Path(os.environ['CARGO_TARGET_DIR'])/'debug/brn').resolve(strict=True)
fixture_parent=Path(os.environ['TMPDIR']).resolve(strict=True)
with tempfile.TemporaryDirectory(prefix='chat-cli-',dir=fixture_parent) as temp:
    root=Path(temp);data=root/'data'
    def call(*args,ok=True):
        r=subprocess.run([str(exe),'--data-dir',str(data),*map(str,args)],capture_output=True,text=True,env=os.environ,timeout=30)
        if ok: assert r.returncode==0,r.stderr
        else: assert r.returncode!=0;return r.stderr
        return json.loads(r.stdout)
    ordinary=call('note-create','--title','Historical note','--text','Unchanged ordinary knowledge')['id']
    db=data/'threads.sqlite3'
    with sqlite3.connect(db) as c:
        controls=[row[0] for row in c.execute("SELECT id FROM records WHERE json_type(body,'$.data.ChatControl') IS NOT NULL")]
        for control in controls:
            operations=[row[0] for row in c.execute('SELECT operation FROM revisions WHERE id=?',(control,))]
            c.execute('DELETE FROM records WHERE id=?',(control,));c.execute('DELETE FROM revisions WHERE id=?',(control,))
            for op in operations:
                c.execute('DELETE FROM receipts WHERE operation=?',(op,));c.execute('DELETE FROM operations WHERE id=?',(op,))
        c.executescript("DROP TABLE chat_drafts; DROP INDEX one_chat_control; DROP INDEX source_locator; CREATE UNIQUE INDEX source_locator ON records(json_extract(body,'$.data.Source.locator')) WHERE json_type(body,'$.data.Source') IS NOT NULL; PRAGMA user_version=3;")
        old=c.execute('SELECT body FROM records ORDER BY rowid').fetchall()
    before=db.read_bytes();assert 'pre-upgrade backup' in call('notes',ok=False);assert db.read_bytes()==before
    backups=root/'backups';backups.mkdir();call('upgrade','--backup-directory',backups)
    files=list(backups.glob('*.sqlite3'));assert len(files)==1
    with sqlite3.connect(files[0]) as c:
        assert c.execute('PRAGMA user_version').fetchone()[0]==3
        assert c.execute('SELECT body FROM records ORDER BY rowid').fetchall()==old
    assert call('read','--id',ordinary)['data']['Note']['markdown']=='Unchanged ordinary knowledge'
    chat=call('thread-start','--title','Standalone chat')['id'];source=root/'attachment.md';source.write_text('# Retained attachment\n\nprivate chat phrase')
    receipt=call('intake','--source',source,'--intent','chat','--thread',chat)
    note=next(w['after'] for w in receipt['receipt']['writes'] if 'Note' in w['after']['data']) if 'receipt' in receipt else next(w['after'] for w in receipt['writes'] if 'Note' in w['after']['data'])
    source.unlink();assert call('read','--id',note['id'])['data']['Note']['markdown'].endswith('private chat phrase')
    assert call('search','--query','private chat phrase')==[]
    queued=call('queue','--thread',chat,'--text','Ask about the retained file')
    call('archive','--thread',chat)
    records=call('records');control=next(r['data']['ChatControl'] for r in records if 'ChatControl' in r['data']);assert control['paused'] and control['queue'][0]['text']=='Ask about the retained file'
    assert next(r for r in records if r['id']==chat)['archived'];call('unarchive','--thread',chat)
    call('keep-in-brn','--note',note['id'],'--base',note['version']);assert call('search','--query','private chat phrase')
    call('backup','--destination',root/'complete-backup.sqlite3')
print('Chat CLI upgrade, exact old backup, attachment after unlink, private search, queue/archive pause, Keep and complete backup witness passed')
