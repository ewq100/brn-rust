# Combined C0 evidence

Worktree: `/Users/evokessler/.t3/worktrees/brn-rust/t3-ceb6e16c`
Branch: `work/build-c0-20261016`
Baseline: `8f768943964a77711b26d5235df146f8efe1dc6c` (fetched origin/main, 16 October 2026).
Initial checkout clean at `9b47b285d23bb5f89db02ba398244827032fab6b`; created a new branch at the instructed baseline. No existing branch or worktree reset. Sole C0 writer; lead owns integration and external ledger.

C0 local deliverables prepared and Opus round-1 corrections recorded; exit gates are audited below. No unconditional C0 completion, independent review, native observation, publication or merge claimed.

## Exact-baseline hosted evidence

Lead-provided run [38908335265](https://github.com/ewq100/brn-rust/actions/runs/38908335265), head exactly baseline above. Local source JSON: `/Users/evokessler/.codex/brn-build-orchestration/2026-10-16/baseline-ci.json`; failure log beside it.

- [Documentation and tooling](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997503571): **success**, 2026-10-16T13:30:17Z–2026-10-16T13:30:27Z.
- [Native retrieval and combined build (windows-2025)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997503797): **failure**, 2026-10-16T13:30:17Z–2026-10-16T13:32:32Z.
- [Native retrieval and combined build (macos-15)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997503847): **success**, 2026-10-16T13:30:25Z–2026-10-16T13:36:19Z.
- [Native retrieval and combined build (ubuntu-24.04)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997503871): **failure**, 2026-10-16T13:30:17Z–2026-10-16T13:31:13Z.
- [Native UI build and state (windows-2025)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997503941): **failure**, 2026-10-16T13:30:17Z–2026-10-16T13:32:23Z.
- [Native UI build and state (macos-15)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504049): **failure**, 2026-10-16T13:30:23Z–2026-10-16T13:34:31Z.
- [Core and CLI (ubuntu-24.04)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504087): **success**, 2026-10-16T13:30:17Z–2026-10-16T13:34:07Z.
- [Native UI build and state (ubuntu-24.04)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504119): **success**, 2026-10-16T13:30:17Z–2026-10-16T13:32:16Z.
- [Core and CLI (macos-15)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504186): **success**, 2026-10-16T13:30:23Z–2026-10-16T13:37:47Z.
- [Core and CLI (windows-2025)](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504318): **failure**, 2026-10-16T13:30:17Z–2026-10-16T13:32:09Z.

Required native UI macOS job failed in `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline`: `threads_ui::writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard`, `threads_writing_tests.rs:846`, expected None, got Some((edit-id, 1)); 305 passed, 1 failed, 1 ignored, 3 filtered, 13.19 s. This predates C0. Cause and flakiness are not established; C0 has no authority to repair it. Three other required lanes passed; four-green claim is false. Optional Windows/Linux failures are separate.

## Local verification and environment

All 21 exact commands/elapsed seconds/environment are in [results.json](c0/results.json), with [per-crate/feature counts, command links and slow-suite limits](c0/baseline-results.md) and [initial local test-binary inventory](c0/test-suites.json) plus [complete exact-baseline hosted/default and local feature/doc inventory](c0/complete-test-inventory.md). Task-owned target `/private/tmp/brn-c0-20261016-target`, `CARGO_INCREMENTAL=0`, fresh explicit synthetic `TMPDIR`, model paths explicitly empty. [Read-only preflight](c0/preflight.json): installed pinned compiler and native compiler/protoc checked; no toolchain install. Maintained helper built first. Poppler `pdftotext`/`pdfimages`: `/Users/evokessler/.local/Cellar/brn-poppler-tools/26.10.0/bin/`; native `pdftoppm` absent at PATH and searched standard/local locations. No replacement wrapper or fixture suppression.

- Helper build/strict lint/test: **64 passed**, 78.081 s across three commands.
- Default/core+CLI local lane: build/strict all-target lint/fmt passed; workspace test command **1012 passed, 1 failed, 6 ignored** observed before Cargo stops at `brn-threads-intake` qualification. Failure `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch`, `qualification.rs:470`, `Pdf preparation failed: Unavailable`. Local environment gap, not new C0 defect or a clean default pass. Capability fixtures: **346 passed, 1 ignored** (343 library + 3 example); strict lint passed. Offline Save/recovery/search/export/backup/startup/refusal and contract-3 CLI journeys passed. Core seven-command lane 177.566 s, excluding helper preparation.
- Native UI build/strict lint/state: **95 passed**. Native-test-support strict lint and filtered `threads_ui::` widget command: **306 passed, 1 existing ignored, 3 filtered**, no skip option added; five-command lane 179.951 s. Does not erase hosted failure above.
- Native retrieval+combined: retrieval library/model-download refusal tests **35 passed**; app **210 passed, 1 existing ignored**; CLI/combined desktop build and strict lint passed; combined desktop **95 passed**. Six-command lane 168.134 s. Download tests are cancelled/occupied-target refusal witnesses, not model acquisition. [Combined native headless startup](c0/native-startup.json) twice passed on fresh synthetic data; no GUI interaction.
- [Default normal graph](c0/default-tree.txt): **zero GPUI entries**; default build log compiles no GPUI. Hosted Linux default lane passed at exact baseline. F22 future writing crate remains a mandatory integration rule, not a shipped new crate.
- [Tooling](c0/tooling.log): **156 tests passed**. Markdown links and diff whitespace checked on final content; final receipts recorded below. No required check weakened, tests deleted or new required tests skipped. All production Rust/root manifests/lock/CI unchanged. [Final content checks](c0/final-checks.json) passed (root/experiment formatting, local links, diff whitespace and shell syntax).

Wall times include compilation and warm-cache effects; these are local scheduling receipts, not performance promises or Linux timings. [Artifact sizes](c0/artifact-sizes.json) identify exact unstripped final-feature binaries, not release size comparisons. [Individual slow-test sample](c0/slow-tests.md) (30 exact tests passed; raw argv/output in `c0/slow-tests.json`) and [sample environment caveat](c0/slow-environment.json) supplement harness slow-binary durations. No full individual-test ranking or flake rate is established. CLI samples ran after native build, so the underlying `brn` executable has native-retrieval enabled; do not compare those wall times directly to default-feature suite time.

## Inventory and preserved work

[Worktree inventory](c0/worktrees.json): all 25 prior checkouts preserved plus this isolated task checkout; dirty/staged/untracked paths recorded without reading owner payloads. Parent `.vscode/` and `experiments/`, writing-ux-research's five staged docs, and reset-worktree `.DS_Store` files untouched. No other worktree wrote C0 paths. Initial process inventory found no cargo/rustc build (app/server sessions were unrelated); lead confirmed no competing heavy build, and every C0 heavy command was serial.

Lead's read-only H0 scratch inventory `/Users/evokessler/.codex/brn-build-orchestration/2026-10-16/h0-inventory.{md,json}` records 20 active folders, 164 remote branches (symbolic HEAD excluded), 25 prior worktrees. It grants no removal authority. Uncertain branches retained; no observed consequential path overlap besides the documented future `threads_ui.rs` serialization seam. Open PR inventory still #149 draft do-not-merge, #112 draft archive, #84 H4, #82 H1, #80 H5; no PR created/worked on by C0.

## Decisions, experiments and CL

[Slice 0 plan](../agent-capabilities/plan.md) records the four choices and their qualification limits. [Harness report](../../../../experiments/r0-rig-reuse/REPORT.md): initial 4 credential-free tests passed; corrected 5-test suite passes twice with strict lint; explicit token policy/stub compactor with source bytes unchanged; process-local summary reconstruction; actual scripted HTTP prefix stability; actual in-process rmcp 2.2 adapter conversion/cancel/list refresh/disconnect; format-2 AgentRun field inventory/roundtrip/direct-driving caveat. The initial harness compile error is preserved and fixed within owned experiment paths. Production semantic C01–I02 gates are not claimed.

[Route trace and prior-experiment applicability](c0/routes.md) record exact X1/Z1/#149 snapshots, actual moved surfaces and reopen triggers. X1 helper/adapter code unchanged; app keyed/chat reuse moved, so future production integration must reconcile it. Z1 negative upstream qualification remains applicable; no adoption. #149 UI host moved substantially, so its prototype cannot be claimed rebased/qualified; PC stays deferred. No duplicated rerun or production cutover.

**CL removal candidates: empty.** No exact path/symbol passed all no-caller, no-saved-data and unchanged-behavior regression proof requirements. Preserve recovery/compatibility and CW2-owned replacements. CL may be a recorded no-op gate. Future concrete destructive removal is pending **explicit owner approval**, not covered by old cleanup permission. [Precise CL/CW1/B1a starts](c0/start-messages.md) are prepared for the principal lead; no later slice started.

## C0 exit audit

| Criterion | Result |
|---|---|
| Correct fresh branch/fetched baseline, existing state preserved | Satisfied; exact paths and inventory above |
| Stage 0 prerequisites landed | Satisfied by ancestor checks `81129f07`, `192cd765`, `36f2c733`, #157 main merge |
| Current real routes/dependency/schema/model/owner map | Corrected one-owner map includes backup/restore/upgrade/schema and CLI routes plus all-other-paths principal custody; route trace retained |
| Section 7.2 and F01–F22 applicability/F22 graph | Recorded; new semantics pending their CW/B slices |
| Required-lane baseline executed, failures explicit | Satisfied as baseline disclosure; local PDF tooling gap and hosted native UI failure remain principal-owned carry-forwards, **not C0 exit prerequisites**; no four-green claim |
| Test counts/times/slow evidence/ignored cases | Complete per-crate/feature/binary/doc inventory from exact-baseline hosted logs plus local lanes; original Cargo-stop and individual timing-sample limits retained |
| Credential-free executable reuse plus four decisions | Satisfied within proof limits; no live/semantic/HTTP deployment qualification |
| X1/Z1/#149 snapshots and applicability | Recorded; retained outcomes, moved surfaces and no rerun/adoption claims |
| O1 and consequential gates | O1 CW-first owner-authorized; D-S/D-M unresolved, T/PC deferred, X retains Poppler, L no live qualification |
| CL list and start messages | Satisfied: defensible empty removal list, no-op proposal; destructive approval still required for any future concrete list |
| Common committed integration ref | Satisfied by the exact local implementation commit and receipt below |
| No new failing/skipped required tests | Production tree unchanged; baseline failures retained, corrected five-test suite passes without ignores; whole-baseline/correction diff checks pass in correction receipt |
| Independent actual-diff review | Opus round 1 **CHANGES_REQUIRED**; [implementation dispositions 1–10](c0/review-corrections.md) supplied; fresh principal-owned correction review pending, not approved |

C0 deliverables and review corrections are locally implemented/deterministic-verified within the stated baseline disclosures. The pre-existing PDF tooling gap and hosted native-widget failure **do not block C0 exit** under master §6/build-sequence step 8. Principal owns their follow-on disposition; B1a waits before moving the implicated writing-test file. C0 exit now awaits fresh correction review and the principal’s exit decision. Empty CL may then be recorded as no-op without claiming four green lanes; CW1 may carry failures forward. No baseline repair or remote rerun is authorized here. Native-observed, live-provider-verified, owner-accepted, published, merged and released are all **not performed** by this worker. No schema/data migration introduced; local rollback is omission/revert of these docs/standalone experiment commits, with production baseline unchanged.

## Local integration receipt

Implementation integration commit: `c3447893dde8d5456b6a9cb6af79a767d3a01856`, parent `8f768943964a77711b26d5235df146f8efe1dc6c`. It contains both plans, contract revision C0-20261016-1, standalone proof source/lock and captured baseline evidence. The following receipt commit records this immutable reference plus log-ending normalization only; no self-referential final SHA is embedded. Integrate both commits from `work/build-c0-20261016`; final HEAD is reported in the worker handoff.

Initial staged/full-commit `git diff --check` flagged terminal blank lines in captured Cargo logs ([failure receipt](c0/initial-diff-check.log)). Terminal blank lines were normalized; no test output content, command, count or failure was removed. Final validation checks the **whole diff against baseline**, including the new tracked evidence; the earlier unstaged check had not included untracked logs.

**H0 inventory update:** after the C0 implementation commit, the owner separately authorized the principal lead to remove old remote branches/worktrees, archiving merged branch tips first. C0's 26-checkout inventory is a dated **pre-cleanup** snapshot, not the live post-H0 inventory. Root completed **129** fully merged remote-branch deletions after verified `archive/<original-name>` tags and **11** clean inactive worktree removals; **35 remote branches / 15 worktrees** remain, local branches/docs/source untouched. Root owns `cleanup-report.md`, `cleanup-candidates.json`/`cleanup-results.json` and ledger under `/Users/evokessler/.codex/brn-build-orchestration/2026-10-16/`; this worker performs no housekeeping. Current C0 checkout, unmerged/dirty/open-PR/current-program work are preserved by that separate assignment. This does not amend the empty CL production deletion list or authorize later slices.

## Opus round-1 correction receipt

Resumed clean at `dfe2fa8f57820cb576a9651f2718cf4953cafd1e`. Full [finding dispositions, actual resume/Stop observations and named-owner diagnostic carry-forward](c0/review-corrections.md) are supplied for principal's fresh correction review. Complete hosted default inventory is **1,013 passed / 0 failed / 6 existing ignored**, with six actually executed zero-test library doc targets; local original **1,012 / 1 / 6** remains partial and unchanged. Explicit retirement scripts/CLI help pass; five corrected experiment tests pass twice and strict lint passes, with Git unaffected by reruns. The initial root-seed narrative is qualified as unproven transient provenance rather than inferred from final pins. No production/root-manifest/root-lock/CI/pack edits, later slices, publication or independent approval by this worker.
