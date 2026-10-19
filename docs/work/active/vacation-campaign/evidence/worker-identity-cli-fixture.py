import json, os, pathlib, subprocess, hashlib
root=pathlib.Path('/tmp/brn-campaign-identity/cli-desktop-qualified').resolve()
root.mkdir()
data=root/'data'
source=root/'synthetic-source.md'
source.write_text('# Synthetic worker identity evidence\n\nThe harbor team meets on Monday. Preserve the exact chat instruction during desktop reuse.\n', encoding='utf-8')
cli=pathlib.Path('/workspace/brn-environment/target/debug/brn')
receipt={'cli_sha256':hashlib.sha256(cli.read_bytes()).hexdigest(), 'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest()}
def call(name,args,expected):
 result=subprocess.run([str(cli),'--json','--data-dir',str(data)]+args, capture_output=True,text=True)
 assert result.returncode==expected,(name,result.returncode,result.stderr,result.stdout)
 receipt[name]=json.loads(result.stdout)
 assert result.stderr.strip() == ('Reading the document…' if name == 'intake' else ''), result.stderr
call('init',['init','--purpose','test'],0)
call('configure',['configure','--provider','Codex','--model','gpt-6.1-sol','--effort','medium','--auth-file','/var/tmp/brn-campaign-identity-explicitly-missing-auth-20261019.json'],0)
call('intake',['intake','--intent','useful','--source',str(source)],7)
assert receipt['intake']['result']['terminal']=='Failed:ReconnectNeeded',receipt['intake']
assert receipt['intake']['result']['reused_chat'] is False
pathlib.Path('/workspace/brn-environment/logs/vacation-identity-cli-fixture.json').write_text(json.dumps(receipt,indent=2)+'\n')
(root/'chat').write_text(receipt['intake']['result']['thread'])
print(json.dumps({'data':str(data),'source':str(source),'chat':receipt['intake']['result']['thread'],'cli_sha256':receipt['cli_sha256'],'intake':'Failed:ReconnectNeeded (expected missing fixture credentials)'}))
