# Legacy production removal — roadmap Stage 2

Baseline: clean `main@6609442d60f23445cfb6ca380a584dd92bbbdb16`, 2026-10-03.
Stage 1 Save/recovery is automated verified and locally integrated; owner native
acceptance remains pending. The current mission authorizes this dependency slice.

Remove the superseded Store/Workspace/worker, sample core, brn-flow, legacy CLI
commands and native legacy state/rendering. Keep exactly the six frozen crates,
AppWorker, WorkStore, current vault/search/AI/editor contracts and the hardened
macOS Save adapter. Narrow shared filesystem DTOs without changing Save proof.
Remove obsolete tests and product script branches; preserve historical records,
standalone experiments and all existing data. Legacy marker refusal remains;
there is no export, migration, credential action or original-data inspection.

Implementation cuts: storage owns WorkStore and filesystem proof DTOs; workflow
owns current application and private file adapter; CLI and desktop consume only
AppWorker. Retrieval retains current note indexes and native embedding/downloads.
Fixed-boundary helpers implement storage, CLI/scripts and native removal while
the lead integrates workflow/retrieval and documentation.

Acceptance: six workspace crates; no shipped legacy execution path; current
Save/recovery/read/search/chat state checks still pass; old/mixed markers refuse
without modification. Fresh default format/build/Clippy/tests, optional native
workflow/desktop tests/build, synthetic end-to-end and launcher checks, independent
read-only review and corrected findings. Repair current links/contracts, retain
historical references, record results and integrate before provider spikes.

Manual acceptance (pending owner): launch with fresh disposable data and synthetic
vault, edit/Save/Copy/restart recovery and view history/settings. Attempt retired
flags and a synthetic old-data marker; expect refusal without opening authority.

## Results — 2026-10-03

The legacy production paths, source modules and obsolete tests are removed.
Cargo metadata verifies six workspace crates and only `brn`/`brn-desktop` binaries.
Seven file-proof DTOs moved unchanged to `brn-store::files`; the private macOS
adapter moved to `brn-workflow::files`. Current WorkStore migrations, buffers,
Save proof/replay, retrieval and owned chat/account/model lanes remain.
Desktop's real AppWorker startup check replaces the sample shell; current Rope
and admitted-Save final-quit proofs replace useful old native tests.

One independent read-only review of the complete diff against the baseline
found no actionable defects. Initial integration checks exposed two stale moved
references (the native installer cancellation helper and subprocess test name)
and the narrowed failure type's missing equality derive; all were corrected.
No provider, model asset, original/private data or migration was used.

Fresh locked/offline checks on macOS arm64, pinned Rust 1.98.1:

- Workspace format/build/all-target Clippy passed. Full tests: **391 passed,
  0 failed, 1 ignored** private crash entry point, exercised by subprocess tests.
- Native desktop `native-ui,native-retrieval`: **84 unit + 7 CLI passed**;
  build and all-target Clippy passed.
- Native workflow `--lib --test models`: **67 passed, 1 ignored**; native
  retrieval suite: **42 passed**. Synthetic assets/vectors are not real ONNX
  qualification; the local-model test self-skips without its explicit fixture.
- End-to-end `--fixtures-only`: **52 assertions passed**, covering current vault,
  Save/recovery/replay/Copy/conflict and unchanged synthetic old-marker refusal.
- Stub macOS launcher checks and shell syntax passed. Native CLI build passed.

Commands: `cargo fmt --all -- --check`; `cargo build --workspace --locked --offline`;
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`;
`TMPDIR=/private/tmp cargo test --workspace --locked --offline`;
`cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline`;
`cargo test -p brn-retrieval --features native --locked --offline`;
`cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline`;
`cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline`;
`cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets --locked --offline -- -D warnings`;
`cargo build -p brn --features native-retrieval --locked --offline`;
`TMPDIR=/private/tmp/brn-stage1-fixtures bash scripts/verify-end-to-end.sh --fixtures-only`;
`bash scripts/test-make-macos-app.sh`; `bash -n scripts/*.sh`.

Filesystem coordination tests and the Save fixture required execution outside
the command sandbox; synthetic paths used canonical explicit temporary parents.
Native interaction/IME/accessibility, real inference, live accounts, power-loss,
other-volume support and owner acceptance remain pending. Upstream `block v0.1.6`
retains its future-compiler notice. Historical records/trials/data are preserved;
no public distribution or remote push is claimed.

Local integration is the commit containing this record. Next: scoped provider
capability preparation and explicitly authorized actual-route checks (Stage 3).
