# Bounded Codex writing smoke route (read-only research)

## Finding

There is no normal public CLI route that captures a writing comment and dispatches `OwnerRequest::RewriteNote`: `brn ask` accepts ordinary chat, note-edit, or decision authority, while native Rewrite constructs the typed writing request in `brn-writing-ui`. The smallest supported non-UI route is a small throwaway developer harness using the public `brn-threads-app::Workspace` and `brn-ai::threads` runtime APIs. It would exercise the production runtime and account transport, not the scripted-model test transport. This is a route proposal only; no account/model/native/provider call or build was made.

## Public API sequence

Use a new absolute synthetic data directory on a fresh workspace and short synthetic content. In a throwaway harness depending on `brn-threads-app` and `brn-ai` (ordinary/default production features; no `scripted-runs`, `capability-spike`, or test-only Rig features):

1. `Workspace::open(fresh_data_dir)` initializes the ordinary workspace and derives its Codex auth-file location from `CODEX_HOME`, falling back to `HOME/.codex/auth.json`. It stores only that location. Do not print or inspect its contents. If the harness instead uses `open_without_default_credentials`, set only the synthetic workspace's normal `WorkspaceSettings` provider/model/effort/auth-file fields via `configure`, pointing at the already-existing approved account location; never copy credentials.
2. `new_note_with_key` (or `new_note`) creates one short synthetic Note. `ensure_writing_context` creates/selects its normal linked Writing conversation. `add_writing_comment` saves one exact-range `WritingCommentInput` whose range is UTF-8 byte offsets into the saved Markdown and whose quote matches that slice.
3. Call `writing_comments`, take the one pending comment ID, and call `snapshot_writing_request_with_feedback` (or `snapshot_writing_request`) with the Note ID/version, linked conversation, and that ID. This persists the exact immutable input snapshot before generation, as the UI does.
4. Call `Workspace::prepare_thread_run(ThreadRunStart { thread: snapshot.thread, prompt: "Rewrite the saved document using the captured feedback and selected chat; prepare a whole-note proposal for review.", request_message: None, queue_claim: None, owner_request: Some(OwnerRequest::RewriteNote { request_message: snapshot.request_message }), sources: vec![], max_tool_rounds: Some(4), trace: TraceLevel::Off, offered_copilot_models: None })`, then pass `prepared.prepared` to `brn_ai::threads::run_prepared` with a `CancellationToken` and an event sink that does not log payloads. Wrap the call with a hard 180-second client deadline/cancel guard. `prepare_thread_run` resolves provider/model/effort from workspace settings; use the authorized existing Codex account, Luna/low where available, and no fallback.

The direct harness needs a Tokio runtime; `brn-ai` already depends on Tokio (`rt-multi-thread`, `macros`, `time`, `sync`) and `brn-threads-app` depends on the production `brn-ai` crate. `run_prepared` itself persists normal Run/proposal receipts in the explicit workspace. The tested scripted APIs are explicitly inappropriate as evidence for this live route.

## Evidence and guards for the actor

Before dispatch, the lead/actor must reserve a new attempt in the campaign ledger and check current account allowance against the campaign floor. The 19 October session used all three attempts; the 20 October ledger currently has no attempts. Respect six attempts per Tallinn date, 180 seconds, at most four rounds when passed as above, and one actor at a time. A deadline, provider/account failure, or uncertain launch consumes the reserved attempt. Do not retry a launch merely to get a pass. The campaign expires 1 November 2026.

Expected successful outcome: terminal Completed; one useful v2 whole-note proposal tied to the frozen request; exactly one captured comment included; canonical Note ID/version/Markdown bytes unchanged; unrelated synthetic Note unchanged; comment remains pending/unaddressed; candidate visible for later ordinary UI review. No Apply/Undo in this smoke route. Preserve hashes/IDs and minimal sanitized run outcome; no auth material or raw provider response.

Refusal/failure assertions: stale or mismatched snapshot must refuse before dispatch; missing account bootstrap or unavailable selected model must produce a diagnosed failure and consume this attempt; timeout/interruption must be recorded as such, with no claim of upstream cancellation. A completed answer without a candidate, a changed Note/unrelated Note, or addressed feedback is a failed qualification requiring diagnosis, not another automatic run.

## Source pointers

- `crates/brn/src/main.rs`: `ask` calls shared `prepare_thread_run`, but constructs no `RewriteNote` owner request.
- `crates/brn-writing-ui/src/native.rs`: `WritingPanel::rewrite_run_start` is the existing UI's checked request constructor; it validates saved request context and supplies `RewriteNote` authority.
- `crates/brn-threads-app/src/writing.rs`: `ensure_writing_context`, `add_writing_comment`, `snapshot_writing_request_with_requirements`, and `writing_request_in_context`.
- `crates/brn-threads-app/src/run.rs`: public shared selection and `prepare_thread_run`.
- `crates/brn-ai/src/threads.rs`: public `run_prepared`, account bootstrap selection, and ordinary runtime.
- `docs/work/active/vacation-campaign/evidence/codex-ai-smoke-qualification.json` and `exact-note-reading-qualification.json`: prior synthetic smoke receipts; ledger records the three completed attempts on 19 October.
