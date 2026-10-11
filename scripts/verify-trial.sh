#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Resolve Cargo through the repository toolchain pin even when called elsewhere.
cd "$repo_root"
starter_manifest="$repo_root/Cargo.toml"
trial_manifest="$repo_root/experiments/codex-app-server/Cargo.toml"
cargo=(cargo)

expect_status() {
    local expected="$1"
    shift
    local actual=0
    "$@" >/dev/null 2>&1 || actual=$?
    if [[ "$actual" -ne "$expected" ]]; then
        printf 'expected exit %s, got %s: ' "$expected" "$actual" >&2
        printf '%q ' "$@" >&2
        printf '\n' >&2
        return 1
    fi
}

printf '%s\n' '== Starter offline locked build, format, Clippy, and tests =='
"${cargo[@]}" build --manifest-path "$starter_manifest" --workspace --offline --locked
"${cargo[@]}" fmt --manifest-path "$starter_manifest" --all -- --check
"${cargo[@]}" clippy --manifest-path "$starter_manifest" --workspace --all-targets --offline --locked -- -D warnings
"${cargo[@]}" test --manifest-path "$starter_manifest" --workspace --offline --locked

printf '%s\n' '== Provider harness offline locked build, format, Clippy, and tests =='
"${cargo[@]}" build --manifest-path "$trial_manifest" --offline --locked
"${cargo[@]}" fmt --manifest-path "$trial_manifest" -- --check
"${cargo[@]}" clippy --manifest-path "$trial_manifest" --all-targets --offline --locked -- -D warnings
"${cargo[@]}" test --manifest-path "$trial_manifest" --offline --locked

printf '%s\n' '== Starter CLI subprocess smoke cases =='
expect_status 0 "${cargo[@]}" run --quiet --manifest-path "$starter_manifest" --offline --locked -- --help
expect_status 0 "${cargo[@]}" run --quiet --manifest-path "$starter_manifest" --offline --locked -- --version
expect_status 1 "${cargo[@]}" run --quiet --manifest-path "$starter_manifest" --offline --locked --
expect_status 1 "${cargo[@]}" run --quiet --manifest-path "$starter_manifest" --offline --locked -- --unknown
expect_status 1 "${cargo[@]}" run --quiet --manifest-path "$starter_manifest" --offline --locked -- --help extra

printf '%s\n' '== Provider harness argument and failure smoke cases =='
expect_status 2 "${cargo[@]}" run --quiet --manifest-path "$trial_manifest" --offline --locked -- invalid
expect_status 2 "${cargo[@]}" run --quiet --manifest-path "$trial_manifest" --offline --locked -- live extra

scratch_dir="$(mktemp -d "${TMPDIR:-/tmp}/brn-trial-verify.XXXXXX")"
trap 'rmdir "$scratch_dir"' EXIT
expect_status 1 env CODEX_BIN="$scratch_dir/no-such-codex" "${cargo[@]}" run --quiet --manifest-path "$trial_manifest" --offline --locked -- sidecar-check
expect_status 1 "${cargo[@]}" run --quiet --manifest-path "$trial_manifest" --offline --locked -- persist-resume "$scratch_dir/no-such-state.json"

printf '%s\n' 'Verification completed. No live or credentialed command was invoked.'
