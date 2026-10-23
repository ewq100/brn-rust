import pathlib,sys,time,json,subprocess,hashlib,shutil
sys.path.insert(0,'/workspace/brn-environment/logs/deferred-linux');import driver as d
out=pathlib.Path('/workspace/brn-environment/logs/rig045-migration/native');out.mkdir();d.ROOT=out
root=pathlib.Path('/tmp/brn-rig045-native-20261023');root.mkdir();data=root/'workspace'
baseline=pathlib.Path('/workspace/brn-environment/rig045-baseline')
shutil.copytree(baseline/'workspace',data)
assert d.snap(data,'old-store-reopened')==json.loads((baseline/'snapshot.json').read_text())
d.cli(data,'pause')
before=d.snap(data,'before');pid=d.launch(data,'first-launch');d.open_note('GPUI 0.8 smoke');d.ax('focus',name='Note text');d.xd('key','ctrl+a');expected='Rig 0.45 native — café, 日本語, λ.\nSaved and reopened exactly.\n';d.paste(expected);d.wait_draft(data,expected);d.capture('dirty');rows=d.saveax('dirty');disabled=next(r for r in rows if r.get('name')=='Rewrite with AI, unavailable: Save your edit first');assert not disabled['actions'];(out/'disabled-observation.json').write_text(json.dumps({'node':disabled,'unix_enabled_sensitive_caveat':all(s in disabled['states'] for s in ['enabled','sensitive'])},indent=2))
d.xd('key','super+s')
for _ in range(60):
 a=d.snap(data,'saved');note=next(r for r in d.notes(a) if r['data']['Note']['title']=='GPUI 0.8 smoke')
 if note['version']==3 and not a['recovery']['result']:break
 time.sleep(.25)
else:raise AssertionError('Save not settled')
assert note['data']['Note']['markdown']==expected;d.capture('saved');d.saveax('saved')
# Accessible menu trigger, item activation, repeated opens and focus/keyboard dismissal.
d.ax('action',name='Open beside');time.sleep(.5);assert any(r.get('name')=='Projects' and r.get('role')=='menu item' for r in d.saveax('menu-open'));d.ax('action',name='Projects',role='menu item');d.capture('projects-open');d.ax('action',name='Close Projects')
for key in ['Return','space']:
 d.ax('focus',name='Open beside');d.xd('key',key);time.sleep(.5);assert any(r.get('role')=='menu item' for r in d.saveax('menu-'+key));d.xd('key','Escape');time.sleep(.3);assert not any(r.get('role')=='menu item' for r in d.ax())
d.xd('mousemove',1190,81);time.sleep(.4);d.xd('click',1);time.sleep(.5);assert any(r.get('role')=='menu item' for r in d.ax());d.xd('key','Escape')
d.xd('key','super+q');d.wait_exit(pid);pid=d.launch(data,'reopened');d.open_note('GPUI 0.8 smoke');d.ax('focus',name='Note text');d.xd('key','ctrl+a','ctrl+c');time.sleep(.7);assert subprocess.check_output(['xclip','-selection','clipboard','-out'],text=True)==expected;after=d.snap(data,'reopened');assert after==a;d.capture('reopened');d.saveax('reopened');d.xd('key','alt+F4');d.wait_exit(pid)
assert next(r for r in d.notes(before) if r['data']['Note']['title']=='Upgrade sentinel')==next(r for r in d.notes(after) if r['data']['Note']['title']=='Upgrade sentinel')
(out/'result.json').write_text(json.dumps({'result':'pass','rig':'0.45.0','old_rig':'0.44.0','journeys':['old 0.44 store exact snapshot, native Unicode edit/save v3/quit/reopen/OS close','AT-SPI menu item activation','repeated menu Return/Space/Escape/pointer','disabled Rewrite has no action; Unix enabled/sensitive caveat retained'],'binary_sha256':hashlib.sha256((d.BIN/'brn-desktop').read_bytes()).hexdigest()},indent=2)+'\n')
print('PASS Rig 0.45 old-store native smoke')
