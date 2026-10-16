import os,json,subprocess,sys,tempfile
from pathlib import Path
# Both locations are explicit; the witness creates/removes only its own synthetic child.
# Usage: cli-witness.py [ABSOLUTE_BRN_BINARY] (else CARGO_TARGET_DIR/debug/brn); documented CLI calls only.
exe=(Path(sys.argv[1]) if len(sys.argv)>1 else Path(os.environ['CARGO_TARGET_DIR'])/'debug/brn').resolve(strict=True)
fixture_parent=Path(os.environ['TMPDIR']).resolve(strict=True)
with tempfile.TemporaryDirectory(prefix='chat-cli-',dir=fixture_parent) as temp:
    root=Path(temp);data=root/'data'
    # Conversion scratch stays inside the witness root, so the parent ends empty.
    (root/'tmp').mkdir();env=dict(os.environ,TMPDIR=str(root/'tmp'))
    def call(*args,ok=True):
        r=subprocess.run([str(exe),'--data-dir',str(data),*map(str,args)],capture_output=True,text=True,env=env,timeout=30)
        if ok: assert r.returncode==0,r.stderr
        else: assert r.returncode!=0;return r.stderr
        return json.loads(r.stdout)
    # A test workspace (B1 phase 1): created explicitly, before the first write.
    call('init','--purpose','test')
    ordinary=call('note-create','--title','Historical note','--text','Unchanged ordinary knowledge')['id']
    # The CLI cannot produce an old format (invariant 17); old-format adoption is
    # covered by chat_storage.rs::adopted_upgrade_requires_external_backup_and_preserves_the_v3_snapshot.
    # Upgrading a current workspace records nothing.
    last=call('history','--limit',1)
    backups=root/'backups';backups.mkdir();call('upgrade','--backup-directory',backups)
    assert call('history','--limit',1)==last
    assert call('read','--id',ordinary)['data']['Note']['markdown']=='Unchanged ordinary knowledge'
    chat=call('thread-start','--title','Standalone chat')['id'];source=root/'attachment.md';source.write_text('# Retained attachment\n\nprivate chat phrase')
    receipt=call('intake','--source',source,'--intent','chat','--thread',chat)
    note=next(w['after'] for w in receipt['receipt']['writes'] if 'Note' in w['after']['data']) if 'receipt' in receipt else next(w['after'] for w in receipt['writes'] if 'Note' in w['after']['data'])
    source.unlink();assert call('read','--id',note['id'])['data']['Note']['markdown'].endswith('private chat phrase')
    assert call('search','--query','private chat phrase')['hits']==[]
    queued=call('queue','--thread',chat,'--text','Ask about the retained file')
    call('archive','--thread',chat)
    records=call('records');control=next(r['data']['ChatControl'] for r in records if 'ChatControl' in r['data']);assert control['paused'] and control['queue'][0]['text']=='Ask about the retained file'
    assert next(r for r in records if r['id']==chat)['archived'];call('unarchive','--thread',chat)
    call('keep-in-brn','--note',note['id'],'--base',note['version']);assert call('search','--query','private chat phrase')['hits']
    call('backup','--destination',root/'complete-backup.sqlite3')
print('Chat CLI current-workspace upgrade, attachment after unlink, private search, queue/archive pause, Keep and complete backup witness passed')
