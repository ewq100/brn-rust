#!/usr/bin/env bash
# Explicit runtime qualification against a completed synthetic trial state.
set -euo pipefail
if [[ $# -ne 1 || ! -d "$1" || ! -f "$1/COMPLETE" ]]; then
  printf 'Usage: %s COMPLETED_SYNTHETIC_STATE_DIR\n' "$0" >&2
  exit 1
fi
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$repo_root/experiments/retrieval-trial/target/debug/brn-retrieval-trial"
state="$(cd "$1" && pwd)"
"$binary" evaluate --state "$state"
"$binary" reopen --state "$state"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/brn-retrieval-qualification.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
copy="$scratch/state"
cp -R "$state" "$copy"
expect_rejection() {
  local expected="$1"
  shift
  if "$binary" "$@" >"$scratch/output" 2>&1; then
    printf 'Unexpected success: %s\n' "$*" >&2
    exit 1
  fi
  if ! rg -q "$expected" "$scratch/output"; then
    cat "$scratch/output" >&2
    printf 'Missing expected error: %s\n' "$expected" >&2
    exit 1
  fi
}
expect_rejection 'missing or incomplete' reopen --state "$scratch/absent"
expect_rejection 'new state directory required' build --state "$copy"
# Every mutation below is made only in the newly created disposable copy.
mv "$copy/model/tokenizer.json" "$scratch/tokenizer.json"
expect_rejection 'native retrieval failure' reopen --state "$copy"
mv "$scratch/tokenizer.json" "$copy/model/tokenizer.json"
printf '\n' >> "$copy/model/tokenizer.json"
expect_rejection 'model file mismatch' reopen --state "$copy"
cp "$state/model/tokenizer.json" "$copy/model/tokenizer.json"
printf '\n' >> "$copy/documents.json"
expect_rejection 'document snapshot hash mismatch' reopen --state "$copy"
cp "$state/documents.json" "$copy/documents.json"
# A changed format with otherwise complete metadata exercises compatibility validation.
sed 's/"format": 1/"format": 999/' "$state/manifest.json" > "$copy/manifest.json"
expect_rejection 'incompatible retrieval state' reopen --state "$copy"
cp "$state/manifest.json" "$copy/manifest.json"
db_file="$(rg --files --hidden "$copy/lancedb" | head -n 1)"
if [[ -z "$db_file" ]]; then
  printf 'No database file found for corruption check\n' >&2
  exit 1
fi
printf '\n' >> "$db_file"
expect_rejection 'LanceDB file inventory mismatch' reopen --state "$copy"
printf 'Runtime evaluation, separate-process reopen, and seven failure checks passed.\n'
