# Recovery-copy privacy guard candidate

## Identity and scope

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch: `codex/recovery-copy-privacy`
- Frozen failing witness parent: `2ff42451adf97268884afd3a0b91c3ec97922e3f`
- Candidate: `b1b04a2cb4a5c769449e3d661170a560c69cf963`
- Original source base: `13813f6ff827de776a663036f5d468500e7a5ed1`
- Changed paths: `crates/brn-threads-app/src/writing.rs`, `crates/brn-threads-app/tests/writing_context.rs`, `crates/brn-desktop/src/threads_ui.rs`, `crates/brn-desktop/src/threads_polish_tests.rs`
- Full patch: `/tmp/brn-recovery-copy-privacy.patch`
- Patch SHA-256: `04972c36f8c9170e6adf83c4cedb4190895a4c0ec20addfe9f4671d4450da4c5`

The pre-source witness run and its intended general-Notes failure are lead-owned evidence recorded in the campaign; I did not rerun it.

## Behavior

Added `Workspace::save_recovery_copy`, using the existing `writing_operation`, `writing_replay`, and `writing_commit` path. Its immutable input binding covers source ID, the exact edit-basis version, captured scope, title, and exact UTF-8 text. Completed/prepared operation replay occurs before fresh eligibility checks. A new attempt resolves the stored historical revision and separately observes the current source; private chat history/current state, any captured scope, unavailable/wrong-kind, archived, superseded, or non-general-visible sources fail closed. Eligible stale general edits create only a plain working Note. The prepared request records the exact basis as `Historical` and the observed current source version as `Current`, letting checked Apply reject a source change after observation while preserving public stale-copy behavior.

The desktop caller now uses the owned `EditSession.base_version`, preserving the actual visible text's origin when a newer Record is displayed beside a recovered buffer. Its stable key derives from session ID and generation and includes the captured scope so a later deliberate context change is a distinct intent. On refusal it sets the explicit recoverability notice and skips refresh/reload, retaining the visible editor, selection, open Note/context, and recovery session. On success it informs before opening the receipt target so a failed open of a later-retired replay target is not mislabeled as a newly saved copy.

The frozen UI witness now checks explicit refusal, unchanged source/current and historical revision, exact session/editor text and selection, unchanged scope, no new receipt, and absence from general Notes/search/export. A second real-control witness covers a private historical basis with a newer general current Record in a general context; using the current Record's version instead of the session basis would admit a copy and fail the assertion. Existing public stale-copy positive control is unchanged.

App service tests cover private historical/current/captured-scope refusal, public stale Unicode copy and same-intent replay after later source change, changed-intent rejection, missing/wrong-kind/archived/superseded failure, and unchanged records/history on refusal. A later source update asserts the accepted derivative appears in `needs_refresh`, witnessing durable derivation tracking. The emitted request in source explicitly binds both basis and current versions. There is no new public request-inspection seam; the tests do not independently introspect private derivation-input rows or orchestrate an inter-connection mutation between service observation and Apply. The `Current` input is enforced by the existing checked Store Apply path; the focused pre-existing core current-input suite should be included in lead qualification.

## Verification and limits

- `source /tmp/brn-launcher-focus-qualification/environment.sh && cargo fmt --all` completed successfully.
- `source /tmp/brn-launcher-focus-qualification/environment.sh && cargo fmt --all -- --check` passed.
- `git diff --check` passed; final candidate checkout is clean.
- Preserved `codex/indexed-settings-read` at `eae61dde04cda19c764246993966b13b997014eb`.
- No build or test was run by this implementation worker. The lead owns candidate qualification and independent review. No native acceptance is claimed.
