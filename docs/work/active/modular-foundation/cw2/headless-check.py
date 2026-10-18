#!/usr/bin/env python3
"""Two fresh synthetic startup/reopen checks; startup flag exits before GUI."""
import pathlib,subprocess,tempfile
binary=pathlib.Path('/private/tmp/brn-cw2-20261016-target/debug/brn-desktop')
with tempfile.TemporaryDirectory(prefix='brn-cw2-startup-',dir=pathlib.Path(tempfile.gettempdir()).resolve()) as root:
 data=pathlib.Path(root)/'data';observed=[]
 for label in ['fresh','reopen']:
  command=[str(binary),'--data-dir',str(data),'--headless-check','startup']
  result=subprocess.run(command,text=True,capture_output=True,check=True)
  assert result.stdout.startswith('Threads ready: '),result.stdout
  observed.append(result.stdout)
  print(label,result.stdout.strip(),flush=True)
 assert observed[0]==observed[1],observed
print('2 passed / 0 failed; synthetic headless startup, no GUI observation')
