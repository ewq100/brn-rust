# B1a pure-helper move: reviewed local implementation

**16 October 2026:** the current formatting and tracked-diff helpers are adopted by the pending desktop imports in `brn-writing-ui`, with one production implementation each. This is the first B1a helper slice, **not panel extraction or B1a completion**. [CW2 acceptance](../cw2/principal-disposition.md) is the local foundation; [refreshed panel proposal](panel-scope.md) remains unimplemented.

## Identity and exact scope

Checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`, branch `work/b1a-cw2-20261016`, unchanged HEAD/base `b617d7842c1e668292b4f23c7b321ec195a76a1d`. Principal created this branch, preserved the old B1a branch/helpers, supplied root/desktop manifests and two imports, and removed old helper sources atomically before this turn. The worker did not reapply that patch or change shared UI. Worker changes in this turn are only the permitted lock update and these B1a documents; prepared helper bytes remain exact.

- Move `crates/brn-desktop/src/threads_format.rs` → `crates/brn-writing-ui/src/formatting.rs`: 714 lines, **30** original tests, six necessary `pub(super)` → `pub` promotions.
- Move `crates/brn-desktop/src/threads_diff.rs` → `crates/brn-writing-ui/src/diff.rs`: 373 lines, **4** original tests, eight necessary public promotions. The reconstruction helper remains test-only and retains its narrower visibility.
- New `src/lib.rs`: two public modules only. No panel/native types, editor or host trait, new behavior, copied run path or app dependency. Full test modules and all non-visibility helper logic are byte-identical to their foundation sources. [Provenance](helper-provenance.json) records SHA-256 and exact counts.
- New crate manifest has default=[] and optional existing `gpui-kit =0.7.3` under native-ui. Desktop's default dependency disables default features and its native feature forwards native-ui. Root gains one member; all other configuration/pins remain.
- Principal already replaced exactly two module declarations with `use brn_writing_ui::diff` / `formatting`. No shim, remaining old copy or cross-crate path include.
- Offline `cargo update --workspace --offline` added exactly one local package and desktop edge. **Principal reviewed and approved this exact lock diff** during the checks. No external package/version/checksum changed. Lock SHA-256: `8dd42e874ca7a7751bf9aabab608d2a7c46310435a73da76e2ab57cddf5e68b6`.

## Verification on this pending candidate

[Machine receipt](verification.json) retains exact commands, current source hashes, UTC/durations, explicit environment, log hashes and artifact identities. Raw logs/source snapshot are retained at `/private/tmp/brn-b1a-20261016-evidence/`. Toolchain is repository Rust1.98.1; target `/private/tmp/brn-b1a-20261016-target` did not exist before this task and was created by these commands. No shared C0 target artifacts were reused. `CARGO_INCREMENTAL=0`, fresh task-owned synthetic TMPDIR, native/model paths empty; locked/offline after lock update. Exact heavy-process inspection was empty before starting.

| Check | Actual result |
|---|---|
| Existing helper library tests | **34 passed / 0 failed / 0 ignored / 0 filtered**; all30 formatting +4 diff tests ran. |
| Helper strict all-target Clippy | Passed with `-D warnings`. |
| `cargo fmt --all -- --check` | Passed. |
| Default `cargo tree --workspace --edges normal --locked --offline` | Passed; **zero GPUI entries**, no normal helper dependency. |
| Default `cargo build --workspace --locked --offline` | Passed, fresh task target, **no GPUI compilation**; 28.548s process. |
| Native `cargo build -p brn-desktop --features native-ui --locked --offline` | Passed; moved native imports/types compiled, 39.164s process. Existing external block0.1.6 future Rust compatibility warning reported. |

F22 default graph/build is established for this pending helper candidate. Default workspace Clippy/tests and native widgets/state/strict all-target desktop lint were **not** rerun for this unchanged helper move; no full-lane pass is implied. There were no failed build/test commands in this run. No new guard/stress/mutation campaign, provider/model/GUI/data access or delegation. Known PDF-tooling gap, original widget-B causal uncertainty and CW2 qualification limits remain; compilation is not human native observation, hosted CI or owner acceptance. A/B corrections and CW2 disclosure/cache/typing test source remain unchanged.

## Commit proposal and release

Proposed title: **`refactor(writing-ui): move formatting and diff helpers`**.

Production content is pinned by SHA-256 **`279a618a6d9dbfbb6506f91dbe0e2b3ce9b1abe3bcfc843e11e973b00b038619`**, frozen scratch patch `helper-production-279a618a6d9dbfbb6506f91dbe0e2b3ce9b1abe3bcfc843e11e973b00b038619.patch`. It contains exactly10 production paths: root/desktop/new-crate manifests, lock, two desktop module imports, two old-file deletions and the three new Rust sources. `production-candidate.json` in scratch records every exact file blob/hash and the parent. These B1a receipt/proposal documents accompany that production change. No source change is needed to manufacture a review pin.

**No commit or staging has occurred.** Principal check-receipt review and explicit commit allocation remain pending; fresh independent Opus actual-diff review/integration remain principal-owned. The immutable production patch plus final complete candidate fingerprint in scratch support that review without claiming a nonexistent commit SHA.

**Heavy slot explicitly released** at `2026-10-16T18:53:33.958326+00:00`: every started build child completed; precise cargo/rustc/clippy/rustfmt/mutant/desktop process check empty. No further Cargo/build/test by this worker without a new allocation. D1A's separate source/research work and later principal-serialized lock reconciliation are unaffected.

## Principal disposition after the receipt

The preceding pre-commit receipt preserves the actual chronology. The principal checked its source/log hashes and committed the atomic helper move at `01fba9a59b51ea13b01e88f176c3dda1ad9b0961`. Fresh independent [Copilot Opus review](helper-opus-review-round1.md) approved that exact source. The stale desktop README link is corrected and this disposition resolves the two nonblocking documentation findings. No source or tests changed after approval, and no repeat build is claimed.

The helper unit is accepted locally as a B1a prerequisite. Full B1a remains in progress: panel/state extraction, real-editor F15 witness and affected verification/review are still required. Default/native compilation is not hosted CI, native human observation or owner acceptance. The original receipt omits the desktop manifest from its per-command hash map; that manifest is retained in the initial snapshot and production candidate record. No contemporaneous hash is invented. D1A owns the sole heavy slot; subsequent B1a source preparation cannot invoke Cargo/tests until explicitly allocated.
