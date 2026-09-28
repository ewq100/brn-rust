#!/usr/bin/env bash
# Exercise the local, provider-free source-to-search workflow.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  if [[ "$CARGO_TARGET_DIR" != /* ]]; then
    printf 'CARGO_TARGET_DIR must be an absolute path when set: %s\n' "$CARGO_TARGET_DIR" >&2
    exit 1
  fi
else
  export CARGO_TARGET_DIR="$repo_root/target"
fi

cd "$repo_root"
cargo +1.98.1 fmt --all --check
cargo +1.98.1 build --workspace --locked --offline
cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test --workspace --locked --offline

binary="$CARGO_TARGET_DIR/debug/brn-flow"
if [[ ! -x "$binary" ]]; then
  printf 'Expected executable not found: %s\n' "$binary" >&2
  exit 1
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/brn-flow-e2e.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
data_dir="$scratch/data"
mkdir "$data_dir"
fixture="$scratch/source.txt"
printf 'The syntheticterm bluejaytheta identifies this private workflow fixture.\n' > "$fixture"

expect_failure() {
  local label="$1"
  shift
  if "$@" >"$scratch/output" 2>&1; then
    printf 'Unexpected success for %s: %s\n' "$label" "$*" >&2
    exit 1
  fi
}

"$binary" --help >/dev/null
expect_failure 'unknown command' "$binary" bogus --data-dir "$data_dir"
expect_failure 'unknown option' "$binary" sources --data-dir "$data_dir" --unknown value
expect_failure 'missing option value' "$binary" search --data-dir "$data_dir" --query
expect_failure 'invalid search profile' "$binary" search --data-dir "$data_dir" --query term --profile invalid
expect_failure 'missing source UUID' "$binary" approve --data-dir "$data_dir" --source bad --version bad --state approved
expect_failure 'bad history session UUID' "$binary" history --data-dir "$data_dir" --session bad

"$binary" import --file "$fixture" --approve yes --data-dir "$data_dir" > "$scratch/import.json"
"$binary" import --file "$fixture" --approve yes --data-dir "$data_dir" > "$scratch/reimport.json"
python3 - "$scratch/import.json" "$scratch/reimport.json" <<'PY'
import json
import sys

first, again = (json.load(open(path, encoding="utf-8")) for path in sys.argv[1:])
assert first["source_id"] == again["source_id"], (first, again)
assert first["version_id"] == again["version_id"], (first, again)
assert again["changed"] is False, again
PY

"$binary" sources --data-dir "$data_dir" > "$scratch/sources.json"
"$binary" sessions --data-dir "$data_dir" > "$scratch/sessions.json"
python3 - "$scratch/sources.json" "$scratch/sessions.json" <<'PY'
import json
import sys

assert len(json.load(open(sys.argv[1], encoding="utf-8"))) == 1
assert json.load(open(sys.argv[2], encoding="utf-8")) == []
PY

"$binary" build --data-dir "$data_dir" > /dev/null
"$binary" search --data-dir "$data_dir" --query syntheticterm --profile keyword > "$scratch/search.json"
python3 - "$scratch/search.json" syntheticterm <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["evidence"], result
assert any(sys.argv[2] in item["quote"] for item in result["evidence"]), result
PY
expect_failure 'semantic search without native provider' "$binary" search --data-dir "$data_dir" --query syntheticterm --profile semantic

printf 'The syntheticterm copperbadger identifies the updated workflow fixture.\n' > "$fixture"
"$binary" import --file "$fixture" --approve yes --data-dir "$data_dir" > "$scratch/changed-import.json"
expect_failure 'search with stale index' "$binary" search --data-dir "$data_dir" --query copperbadger --profile keyword
if ! rg -q 'stale' "$scratch/output"; then
  printf 'Expected stale-index failure, got: ' >&2
  cat "$scratch/output" >&2
  exit 1
fi
"$binary" build --data-dir "$data_dir" > /dev/null
"$binary" search --data-dir "$data_dir" --query copperbadger --profile keyword > "$scratch/updated-search.json"
python3 - "$scratch/updated-search.json" copperbadger <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["evidence"], result
assert any(sys.argv[2] in item["quote"] for item in result["evidence"]), result
PY

printf 'End-to-end workflow verification passed (local fixture; no provider, credentials, or native model used).\n'
