# Simple safe Save — roadmap Stage 1

Baseline: clean `main@4bf78784a7923c4999a5f601f5974dc8f15c4656`, 2026-10-03.
The owner authorized sequential implementation and integration of BRN v1;
live accounts, original data, downloads and release remain outside that authorization.

## Outcome and scope

Give the simple app shared workflow/CLI/native manual Markdown Save, exclusive
Save Copy and recoverable unfinished edits. Preserve exact UTF-8 bytes, BOM,
line endings and frontmatter. Reuse the hardened macOS descriptor/coordination
adapter, retaining identity, ownership, attribute and durability checks.
Keep authority in Markdown and recovery/intents in WorkStore. No new datastore
or generic repository abstraction. Preserve legacy paths until verified replacement.

## Steps and contracts

1. Add additive WorkStore editor records, monotonic generations and bound save
   journals; persist buffer and intent before filesystem mutation.
2. Workflow observes fresh files, refuses conflicts/missing originals, stages
   and installs atomically, verifies installed/displaced identities and reconciles
   interrupted writes without replay. AppWorker drains admitted editor mutations.
3. Expose the same operations headlessly and in the native small editor. Preserve
   later typing on stale acknowledgements; flush recovery on guarded navigation/quit.
4. Independent read-only defect review; validate/fix findings; fresh checks;
   update contracts/status and integrate the slice.

## Acceptance and checks

- Exact-byte edit/Save survives reopen; empty and multilingual files work.
- External replacement, content change, missing file, symlink/hardlink and root
  changes cannot silently overwrite an occupant. Copy cannot overwrite any target.
- Restart recovers acknowledged unfinished edits; UUID replay never writes again.
- Interrupted Save is classified from exact artifact/identity proof; uncertain
  outcomes retain useful work and prevent another original-path write.
- Save/recovery acknowledgements retain newer native typing. Guarded quit drains
  admitted mutations. Dock/system termination limitations remain explicit.
- Targeted store/workflow/CLI/desktop tests, crash/failure seams, default
  workspace format/build/Clippy/tests, native desktop tests/build, synthetic
  end-to-end fixtures, local Markdown links and `git diff --check`.

Manual acceptance (pending owner): use a disposable vault/data directory, open a
note containing BOM/CRLF/Estonian text, correct it and Save/Cmd-S; compare bytes.
Edit again, navigate and restart to recover it. Change the disk file externally
before Save and verify refusal; Save Copy to an unused path and verify occupied
paths refuse. Repeat guarded Quit with pending recovery. Native IME/accessibility
and power-loss behavior are separate qualification.

## Implementation and verification — 2026-10-03

Implemented shared WorkStore editor records and a bound Save journal; workflow
Save/Copy/Reload/reconciliation; CLI `edit`; and native Save, recovery, comparison
and guarded navigation/quit. Hash/identity proofs retain uncertain work without
repeating filesystem writes. Completed older journals compact only after proven
artifact retirement; the latest applied original retains a recovery pair.
Retained AI readers reject results spanning a Save or unresolved outcome.

Independent read-only review against the exact baseline above identified worker
startup, copy-reservation, evidence-return and no-op replay defects. All were
validated, fixed with regression coverage and re-reviewed without remaining
findings. Later typing and unavailable-vault recovery regressions were also fixed.

Fresh final-tree checks on macOS arm64, pinned Rust 1.98.1, locked/offline:

- `cargo fmt --all -- --check`, `cargo build --workspace --locked --offline`,
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: passed.
- `TMPDIR=/private/tmp cargo test --workspace --locked --offline`: 728 passed,
  0 failed, 1 ignored private subprocess entry point. Includes 25 WorkStore
  editor tests, 5 CLI editor tests and 14 editor/worker regressions; crash tests
  exercise original/Copy interruption at seven checkpoints each.
- `cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline`:
  137 unit + 6 CLI tests passed. Native desktop build and all-target Clippy passed.
- `cargo test -p brn-workflow --features native-retrieval --lib --test models
  --locked --offline`: 93 library + 5 model tests passed, 1 private subprocess
  entry point ignored. Native CLI build passed.
- `TMPDIR=/private/tmp/brn-stage1-fixtures bash scripts/verify-end-to-end.sh
  --fixtures-only`: 47 assertions passed; synthetic vault + separate legacy fixture.
- `bash scripts/test-make-macos-app.sh`: passed with stub binaries.

macOS coordinated filesystem tests required execution outside the command sandbox;
all data was synthetic. Credential fixtures used a canonical explicit temporary
parent. No live provider, account, model download or original data was used.
The upstream `block v0.1.6` future-compiler warning remains. GUI/IME/accessibility,
owner acceptance, real native inference and power-loss qualification are pending.
Dock/system final quit drains admitted work; typing not yet admitted cannot be
claimed durable. This bounded implementation does not establish BRN v1 completion.

Local integration is the commit containing this record, on `main`; no remote
push, public release or owner native acceptance is claimed. Next: roadmap Stage 2.

## Publication checkpoint — 2026-10-04

The owner now authorizes milestone branches, PRs and automatic merges after
independent review, local verification and applicable exact-commit macOS/shared
CI. This checkpoint preserves `6609442` and merges hosted-CI parent `609d859` on
`codex/v1-stage1-save`. Independent read-only review of all five integration files
found no actionable defect. The conflict resolution retains Save's crate-visible
file helpers and CI's macOS test gates/kernel-aware pipe error assertion.

Fresh macOS arm64 / Rust 1.98.1 locked/offline
`TMPDIR=<fresh canonical owned parent> CARGO_TARGET_DIR=<isolated absolute target>
bash scripts/verify-end-to-end.sh` passed retirement, formatting, build,
all-target Clippy with warnings denied, **728 tests / 0 failed / 1 ignored** and
**47 end-to-end assertions**. The ignored crash entry point is exercised by the
subprocess matrix. No provider, model assets or original/private data were used.
Existing native evidence above remains separate from pending exact-PR CI and
owner GUI/IME/accessibility acceptance. No release is authorized by this record.

Resume from the exact PR head/merge commit recorded in its description. Next is
Stage 2 legacy removal, which must remove retired `brn-flow` CI commands and use
the supported startup/restart driver. Environment: macOS Apple Silicon, Command
Line Tools, pinned Rust 1.98.1/rustfmt/Clippy, cached locked dependencies (or an
explicit normal development fetch), Python 3/Bash, and a new canonical owned
synthetic TMPDIR outside Git. Native builds also need protobuf; native observation
needs an unlocked session. Do not copy task credentials or original data during a
later transfer to the Mac mini.

The first exact-head PR run at `f9c38bc` passed macOS Core, Native UI and Native
Retrieval. Ubuntu build/tests/help/fixtures passed, but Clippy correctly rejected
the new editor test helper `ok`, whose callers are all macOS-only. The helper now
uses the same macOS guard; fresh local CLI all-target Clippy, **5 editor process
tests / 0 failed**, formatting and diff checks passed. Windows retains the known
Unix-only API build failure. The corrected latest PR head must qualify again;
the initial run does not qualify a changed commit.
