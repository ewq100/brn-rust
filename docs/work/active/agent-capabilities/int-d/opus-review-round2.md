# INT-D milestone review, round 2: corrected material `b9390407`

**Verdict: APPROVE_WITH_QUALIFICATIONS.** Both round-1 blockers, S1 and P1, are fixed. I found no new regression and no mandatory change. What remains are honest residual qualifications, plus three optional low-severity hardenings that block nothing.

This was a read-only review. I made no edits, builds, tests, live/provider/auth calls or git mutations, and delegated nothing.

## What I checked
- **Identities:** branch `work/int-d-20261017`. Material `b9390407` has tree `c33504319…` and crates `5993744077…`. HEAD `bc5eb5da` adds only 4 files under `opus1/`, all docs.
- **Diff hashes:** `c6d..b939` = `7a78100d…` and `c2ec..b939` = `dd9e468b…`, both as stated.
- **Material hashes:** I re-derived all 333 entries in `opus1/material-hashes.json` from git objects and every one matches. Against the original 333, exactly 4 changed: the CLI `int_d.rs`, its `support/int_d.rs`, `threads_int_d_tests.rs` and the app `int_d.rs`. The other 329 are identical. Relative to `c2ec`, `threads.rs` still has only the original +7-line test cfg registration.
- **What else changed in `c6d..b939`:**
  - the three qualification scripts, with hashes matching `script-hashes.json`;
  - the verbatim round-1 report, SHA `05a0b834…`;
  - new files under `opus1/`.
  - No `live-window1`, preflight or original evidence file changed.
  - No manifest, lock, CI, production, auth, core, native or API change.
- **Receipts:** the AI, app and CLI tests, both Clippy runs, fmt and doc-link checks each bind all 333 final hashes with `source_unchanged=true`.
- **S2 before-run:** the raw log is a real behavioural failure, not a compile error. It panicked at `support/int_d.rs:311`, exporting current v2 against prepared v1, with 0 passed, 1 failed and 3 filtered out. Its source is a separate `/private/tmp` archive and target.
- **Fake observer harness:** binds the current observer (`ceafbe43…`) and checker (`3d74c579…`) hashes. All 10 named cases are present.

## Axis 1: Standards

**S1 · resolved.** `crates/brn/tests/int_d.rs:5` now gates only the `candidate_export` module (`support/int_d.rs`) to macOS.
- Nothing in that module refers back through `super::` or `crate::`, so Linux has no dangling or unused items.
- The Linux corpus test defaults to `Root::new()` under `temp_dir()` (`int_d.rs:89-108`). `/private/tmp` is required only when `BRN_INT_D_FIXTURE_ROOT` is set explicitly.
- Strict qualification containment in `owned_root` is unchanged.
- **Qualification:** no Linux or hosted build was run. The gate is verified from source only.

**S2 · resolved.**
- `support/int_d.rs:95-97` refuses when the current target version differs from the write's `expected_version`, before any output is created.
- `:345` tests: positive export first, then a real checked owner edit, refusal, no output, and the Note and prepared candidate unchanged. A pre-occupied output is refused and its sentinel preserved; that refusal comes from `owned_root`, not the version guard, which is fine.
- `:420` tests that real Applied and Retired (via `review_dismiss`) operations are refused with no effect.
- The exporter never applies anything. The before-red is shown only for the stale case, and the docs do not claim more.

**S3 · resolved.**
- **Preparer:** now records the poll argv actually run (`--timeout 2`) and a Python timeout of 3.
- **Observer:**
  - It runs that exact recorded argv and checks it before configure. The expected path is the pinned binary path, which equals `a.binary.resolve()` in the preparer.
  - `TimeoutExpired` in a poll, a local inspection or the export now records a safe event. `finally` then reaps the owned ask, and the run exits with no Continue.
  - Non-Stop Continue runs are now labelled as such.
  - Corpus, manifest, case-in-manifest, initial DB and both prompts are bound before configure.
- The 10 fake-process cases include the timeout case and the 4 hash refusals.

**S4 · retained and acceptable.** The tracked `diff_sha256` caveat is still disclosed. Per-command 333-hash receipts govern.

**Rust quality:** test-only `unwrap`/`panic`. Helpers are reused without visibility changes. Rig 0.44's `CompletionRequest` has no `preamble` field (system text travels as `Message::System` in `chat_history`), so the app's no-system-promotion check (`tests/int_d.rs:41-47`) is complete.

## Axis 2: Spec

**P1 · resolved, and the assertions are meaningful.**
- **AI test** (`threads_int_d_tests.rs:494-503`, `:580-584`, `:626-635`, `:667-678`):
  - The full selected body appears in the first cold `input`, which comes before any tool call. The warm run contains only a summary and a text reply, so the body cannot have come from history.
  - The body is absent from `instructions`.
  - Persisted `SKILL.md` id and package digest are asserted on the stopped Run and on the fresh Continue Run.
  - This matches the source: `threads.rs:3593-3610` adds `selected_context()` to the input per run and records identities at initial admission.
- **App test** (`:183-194`, `:273-285`): the same delivery and identity assertions run on the initial abandoned run and again after restore, missing-package refusal, exact reinstall and fresh Continue.
- Every earlier assertion is preserved: Stop after durable prepare, raw prefix, Apply/replay/Undo and NeedsReview.

**L1–L5:** handled honestly.
- **L1:** stop-on-any-unsuccessful was chosen before activation.
- **L2:** Continue-prompt verification is labelled post hoc only.
- **L3:** the rubric is explicitly not prospectively pinned, and the dated labels only map existing prose.
- **L4:** no live Stop or Continue ran, and the future label is defined.
- **L5:** synthetic MCP-on summary refreshes are counted and charged, and not relabelled as live.

**Live-window honesty:** the assessment matches what happened.
- Exactly 2 starts; 6 cases unstarted.
- Control Completed with no candidate; the selected arm hit the tool limit (exit 7) with no candidate.
- The cause of the generic prepare errors is UNKNOWN.
- Parent and summary usage are reported separately.
- The selected-skill identity counts as an admission record only.
- There is no superiority or causal-benefit claim, and the 10 unused starts are not treated as authority.
- The round-1 "zero live" statements are correctly kept as time-qualified.

**Removed-path audit:** bounded and source-located. It does not claim exhaustiveness.

## Mandatory changes
None.

## Optional residuals (low; do not block)
1. **O1 · LOW · observer database binding covers only the main file** (`observe-workflow.py:101`, `prepare-qualification.py:114`). The store runs in WAL mode (`store.rs:154`), so frames left in `threads.sqlite3-wal` would not be detected. A future preflight could record that `-wal`/`-shm` are absent or hash them. Until then, the "6 unstarted DBs unchanged" result should read as main-file bytes only.
2. **O2 · INFO · `instructions` checks can pass vacuously.** At `threads_int_d_tests.rs:500` and `:632`, `!chat[0]["instructions"].to_string().contains(..)` would also pass if the key were missing. The warm-run check uses `.as_str().unwrap()`, which would catch that.
3. **O3 · INFO · the corrected preparer has never produced a preflight.** It is checked statically only. Any mismatch with the observer would make the observer refuse before configure, so it fails safe.

## Remaining gates (unchanged and still open)
- **Not executed:** hosted Ubuntu, Windows and macOS lanes; a Linux build of the cfg gate; native-human/GUI.
- **Live:** no Stop/Continue run, no repeats, no live review/Apply/replay/Undo. Prepare cause unknown. Any further live window needs new principal authority.
- **Carried failures:** workspace 1234/1/6 with `PDFUnavailable`; the original workspace 1224/5/6; SDK 474/4/1 and widget 314/1/1, still unexplained; AI 479/0/1 and other original qualifications, not relabelled after the correction.
- **D4 qualifications:** SDK timeout, NACK proven by config only, F3 render scans deferred, earlier compatibility-source association.
- **Prior campaigns:** mutation campaigns closed; D3b live caveat.
- **Integration and acceptance:** INT-W is separate and not merged. Principal, owner, program and main acceptance are all pending.
