# Recovery-copy privacy guard candidate

## Identity and scope

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch: `codex/recovery-copy-privacy`
- Frozen failing witness parent: `2ff42451adf97268884afd3a0b91c3ec97922e3f`
- Corrected-review starting HEAD: `049819a4ae77494dc5da2b0f34e07c3fd1525ca7`
- Candidate: `40013cd1d094fa811a2f75591cd5ef6efc1e127f`
- Original source base: `aded398a0c1960d1feabe48166284ec0824ee4e1` (actual main211 merge base)
- Changed paths: `crates/brn-threads-app/src/writing.rs`, `crates/brn-threads-app/tests/writing_context.rs`, `crates/brn-desktop/src/threads_ui.rs`, `crates/brn-desktop/src/threads_polish_tests.rs`
- Full patch: `/tmp/brn-recovery-copy-privacy-corrected.patch`
- Patch SHA-256: `efc3a0dc2ee2d922d2629f1e000dc8501514407af833e0d0a3108502c940c887`
- Historical pre-correction report: `/tmp/brn-recovery-copy-privacy-implementation-initial.md`

The pre-source witness run and its intended general-Notes failure are lead-owned evidence recorded in the campaign; I did not rerun it.

## Behavior

Added `Workspace::save_recovery_copy`, using the existing `writing_operation`, `writing_replay`, and `writing_commit` path. Its immutable input binding covers source ID, the exact edit-basis version, captured scope, title, and exact UTF-8 text. Completed/prepared operation replay occurs before fresh eligibility checks. A new attempt resolves the stored historical revision and separately observes the current source; private chat history/current state, any captured scope, unavailable/wrong-kind, archived, superseded, or non-general-visible sources fail closed. Eligible stale general edits create only a plain working Note. The prepared request records the exact basis as `Historical` and the observed current source version as `Current`, letting checked Apply reject a source change after observation while preserving public stale-copy behavior.

The desktop caller now uses the owned `EditSession.base_version`, preserving the actual visible text's origin when a newer Record is displayed beside a recovered buffer. Its stable key derives from session ID and generation and includes the captured scope so a later deliberate context change is a distinct intent. On refusal it sets the explicit recoverability notice and skips refresh/reload, retaining the visible editor, selection, open Note/context, and recovery session. On success it informs before opening the receipt target so a failed open of a later-retired replay target is not mislabeled as a newly saved copy.

The frozen UI witness now checks explicit refusal, unchanged source/current and historical revision, exact session/editor text and selection, unchanged scope, no new receipt, and absence from general Notes/search/export. A second real-control witness covers a private historical basis with a newer general current Record in a general context; using the current Record's version instead of the session basis would admit a copy and fail the assertion. Existing public stale-copy positive control is unchanged.

App service tests cover private historical/current/captured-scope refusal, public stale Unicode copy, same-version positive binding, changed-intent rejection, and missing/wrong-kind/archived/superseded failure. The public-stale case now reads `Store::prepared` for the actual completed service operation and asserts exact Historical(old basis) and Current(observed newer source) inputs. It clones that actual request with a fresh operation and creation ID, prepares through `prepare_owner`, changes the source to private through a second checked Workspace connection, and asserts Apply returns Stale with unchanged private source, no created target and no receipt for the pending copy. The same test drops/reopens the original Workspace and retries the original unchanged key/intent while the source is private; it asserts the original receipt/target identity is returned and changed intent under the key creates no record or receipt. A same-version public case asserts both input expectations can bind the same source version.

The recovered-private-basis UI fixture now changes the current Note to general scope before opening an edit session at the earlier private revision; `Store::begin_edit` reads that exact historical revision and supports the stale base. This preserves the real UI route without writing a guarded Note. The actual Save Copy control is tested with the older private session basis and newer general displayed Record, so using `open.record().version` instead of `EditSession.base_version` would create a general copy and fail the test. These additions address both findings in `/tmp/brn-recovery-copy-privacy-review.md`.

## Verification and limits

- `source /tmp/brn-launcher-focus-qualification/environment.sh && cargo fmt --all` completed successfully.
- `source /tmp/brn-launcher-focus-qualification/environment.sh && cargo fmt --all -- --check` passed.
- `git diff --check` passed; final candidate checkout is clean.
- Only the two existing test files named below changed in the correction commit.
- Preserved `codex/indexed-settings-read` at `eae61dde04cda19c764246993966b13b997014eb`.
- No build or test was run by this implementation worker. The lead owns candidate qualification and independent review. No native acceptance is claimed.

Correction commit `40013cd1d094fa811a2f75591cd5ef6efc1e127f` changes only `crates/brn-desktop/src/threads_polish_tests.rs` and `crates/brn-threads-app/tests/writing_context.rs`. No production source changed. It corrects the dirty-guard fixture order, adds direct actual-request inspection and a checked second-connection privacy race, and makes the completed replay cross a Workspace reopen while current source is private. No build or test was run; only formatting and diff checks were used.
