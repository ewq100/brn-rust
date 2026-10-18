#!/usr/bin/env python3
"""C0 only: serialized offline checks, synthetic fixture parent, no app UI/provider/model access."""
import json, os, pathlib, subprocess, time, platform
root = pathlib.Path(__file__).resolve().parents[5]
out = pathlib.Path(__file__).resolve().parent
os.environ.update(CARGO_TARGET_DIR='/private/tmp/brn-c0-20261016-target', CARGO_INCREMENTAL='0', BRN_NATIVE_MODEL_DIR='', BRN_EMBEDDING_MODEL_DIR='', RUST_BACKTRACE='1', CARGO_TERM_COLOR='never', BRN_INTAKE_HELPER='/private/tmp/brn-c0-20261016-target/debug/brn-intake-helper', BRN_PDFTOTEXT='/Users/evokessler/.local/Cellar/brn-poppler-tools/26.10.0/bin/pdftotext', BRN_PDFIMAGES='/Users/evokessler/.local/Cellar/brn-poppler-tools/26.10.0/bin/pdfimages', TMPDIR='/private/tmp/brn-c0-20261016-fixtures')
pathlib.Path(os.environ['TMPDIR']).mkdir(exist_ok=True)
lanes = {
'helper': ['cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline', 'cargo clippy -p brn-intake --features helper --all-targets --locked --offline -- -D warnings', 'cargo test -p brn-intake --features helper --all-targets --locked --offline'],
'core': ['cargo fmt --all -- --check', 'cargo build --workspace --locked --offline', 'cargo clippy --workspace --all-targets --locked --offline -- -D warnings', 'cargo test --workspace --locked --offline', 'cargo test -p brn-ai --features capability-spike --all-targets --locked --offline', 'cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline -- -D warnings', 'bash scripts/verify-end-to-end.sh --fixtures-only'],
'native-ui': ['cargo build -p brn-desktop --features native-ui --locked --offline', 'cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings', 'cargo test -p brn-desktop --features native-ui --locked --offline', 'cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings', 'cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline'],
'native-retrieval': ['cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline', 'cargo test -p brn-threads-app --features native-retrieval --locked --offline', 'cargo build -p brn --features native-retrieval --locked --offline', 'cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline', 'cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets --locked --offline -- -D warnings', 'cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline']}
results=[]
for lane, commands in lanes.items():
 for command in commands:
  start=time.monotonic(); name=f'{len(results):02}-{lane}.log'
  with (out/name).open('w') as f:
   code=subprocess.call(command.split(),cwd=root,env=os.environ,stdout=f,stderr=subprocess.STDOUT)
  entry=dict(lane=lane,command=command,exit=code,seconds=round(time.monotonic()-start,3),log=name)
  results.append(entry)
  (out/'results.json').write_text(json.dumps(dict(baseline='8f768943964a77711b26d5235df146f8efe1dc6c',platform=platform.platform(),environment={k:os.environ[k] for k in ['CARGO_TARGET_DIR','CARGO_INCREMENTAL','BRN_NATIVE_MODEL_DIR','BRN_EMBEDDING_MODEL_DIR','BRN_INTAKE_HELPER','BRN_PDFTOTEXT','BRN_PDFIMAGES','TMPDIR']},results=results),indent=2)+'\n')
  print(json.dumps(entry),flush=True)

raise SystemExit(1 if any(row['exit'] != 0 for row in results) else 0)
