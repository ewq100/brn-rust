# Pre-release cleanup evidence

## Scope and identity

Owner-authorized aggressive cleanup, 11 October 2026. Baseline: `3a3aa7ac6f2a70815ffc778a694663f729203a79` (PR #120). Isolated branch: `work/pre-release-cleanup`. The [plan](plan.md) defines deletion boundaries and verification. No release, provider calls, live data access or native Computer Use observation is included.

The owner additionally requested chat/writing UX research during this work. That investigation uses a separate worktree and does not change this cleanup candidate's runtime scope.

## Independent plan review

- Original plan approved at SHA-256 `ed88f06232e9995a1327edf21372efc6a603b9e20cde653713240ff67a1afa8f` by a read-only GPT-6.1 Sol reviewer, independent of the production, tooling and docs implementers.
- Historical-artifacts addendum approved at plan SHA-256 `81cf6ac67e29170eb2171781e06dfa1778f7c175a3cc21c2bff44453723efe2f`. Reviewer confirmed 55 tracked files / 7,807,273 blob bytes, only historical documentation references, no current executable/build consumer. SQLite snapshot payloads were not opened. The current PDF `.gitattributes` rule remains.
- Review conditions: all twelve fixture bytes identical, no dependency version changes, preserve active provenance/security assertions, preserve optional features and CI contexts, validate current documentation links including links into the archive.

## Candidate and integration gates

Implementation and local candidate checks are satisfied. Independent candidate review is satisfied. Publication, required PR CI, merge and required main CI are pending and will be recorded on the pull request. Native usability and owner acceptance remain explicitly unverified; existing deferral is unchanged. Final commit/tree and hosted CI receipts will be recorded on the pull request so they identify the immutable tested result.


## Local candidate verification

All checks below passed on macOS arm64 with pinned Rust 1.98.1. Tests use synthetic temporary workspaces and installed helpers; no provider calls or downloads. The default build cache was `../threads-macbook-checkout/target`, native cache `../threads-macbook-local/target`, with four build jobs. Intake environment selected the built `brn-intake-helper` and installed Poppler `pdftotext`/`pdfimages` explicitly.

```sh
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo test -p brn-intake --features helper --all-targets --locked --offline
cargo clippy -p brn-intake --features helper --all-targets --locked --offline -- -D warnings
cargo fmt --all --check
cargo build --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test -p brn-ai --features capability-spike --all-targets --locked --offline
cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline -- -D warnings
cargo build -p brn-desktop --features native-ui --locked --offline
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings
cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline
cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline
cargo test -p brn-threads-app --features native-retrieval --locked --offline
cargo build -p brn --features native-retrieval --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets --locked --offline -- -D warnings
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check-provider-retirement.py
python3 scripts/check-intake-retirement.py
bash scripts/test-make-macos-app.sh
python3 scripts/check-markdown-links.py
python3 scripts/check-markdown-links.py --all
git diff --cached --check
```

Current shell scripts also passed syntax checks. Passing, overlapping test counts: intake helper 37; workspace 435; provider capability fixtures 208; native widgets/state 39; native retrieval 15; native app 36; combined desktop state 18; tooling 16. Do not sum these as unique tests. The upstream `block v0.1.6` future-incompatibility notice remains. Production logs were retained locally under `/tmp/brn-pre-release-cleanup-production/`; hosted PR checks are the durable independent rerun.

Parent independently verified all twelve relocated fixture bytes against the baseline, unchanged lockfile package names/versions/sources/checksums, and unchanged CI workflow/setup action and `.gitattributes`. The lockfile removes only seven direct dependency edges. Four retired materialization-only tests were removed; current raw attachment/provenance, decoder and security assertions remain. The 201-line obsolete implementation removal does not touch the checked writer.

Both Markdown modes passed: 35 current Markdown files / 127 local links and 45 including archived files / 148 local links, zero failures before this evidence expansion. Current-to-archive missing-file and missing-fragment behavior remains regression-tested.

## Checkout reduction

Baseline: 563 tracked files, 32,291,271 blob bytes. Candidate: 195 files, approximately 3.2 MB, including relocated live fixtures and new cleanup evidence. Counts come from `git ls-tree -rl`, exclude Git history/build caches, and do not measure installed app size. Removed history remains at the fixed baseline linked by the archive index.

Removed: 162 standalone experiment files, 55 old campaign artifact files, four trial scripts, the completed Rig comparison harness, obsolete documentation/screenshots, seven unused dependency declarations and retired source-note materialization. Twelve active fixtures moved into their owning crate unchanged. Nine selected Threads evidence reports remain locally archived; current docs describe eight maintained crates and remaining release gates.


## Independent candidate review

Approved without actionable findings against staged tree `8b81e2d6323b7c52df0796b4c16483c6ccd92a3a`, baseline `3a3aa7ac6f2a70815ffc778a694663f729203a79`. Reviewer independently confirmed all twelve fixture byte identities, all 972 unchanged locked package identities, no live removed-API consumer, legacy-only removed tests, preserved CI/native gates/toolchain, retained substantive content of nine archived reports, and current authority/provenance. The reviewer reran all 16 tooling tests, both Markdown modes and staged whitespace checks successfully. Only review/evidence/status bookkeeping changed after that review; production, tests, tooling and requirements stayed frozen.
