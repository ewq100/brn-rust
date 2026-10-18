# NF-1 specification and real call-path tests (source-only)

18 October 2026, revised after Opus round 2 (case C evidence; the fixed rewrite prompt). A specification, not an implementation. No production edit, no compile, no test run. Facts below come from reading `work/int-w-20261017` (`d2d84e66`), `73ea9ca7`, and the simulated merged tree `deb060ee0e809b0b906bc9aaf97626a4181ef7be` (its `threads_ui.rs` positions: `start_run` 1899, `take_mcp_review` call 1914, `start_request` 1949, `reviewed.attach` 1983, `retry_rewrite`'s call 1860).

## The static finding

INT-W split the single X2 `start_run` into `start_run` and a shared `start_request`. D4's prompt-bound MCP review (`take_mcp_review` before preparation, `reviewed.attach(prepared)` after it) sat in the single function. After the textual merge `reviewed` is a local of `start_run` but is used in `start_request`: compile error E0425 under `--features native-ui`. This is a **static finding, not an executed red.** The default workspace build does not compile `threads_ui.rs`.

`start_request` has two callers: `start_run` and `retry_rewrite` (the saved-rewrite retry, `begin_run_start` then `start_request`).

## Required ordering (preserved from X2)

| Step | Behavior to preserve |
|---|---|
| 1 | `begin_run_start`: a running worker refuses with its busy notice **before anything is taken**, so a busy refusal leaves a pending review untouched. Then `refresh()`. |
| 2 | `take_mcp_review(&thread, &prompt, queued)`: consume the pending review (or `None`), clear query and review state, refuse on queue, stale stamp, other chat or different prompt with the existing message. Happens **before** `prepare_thread_run`. Preparation creates the Run; a refusal after prepare leaves a Failed Run with fence +1 (see `reviewed_snapshot_handoff_barrier`). |
| 3 | `prepare_thread_run(request)`, with its existing selection and failure handling. |
| 4 | `reviewed.attach(prepared)` **after** prepare, with its existing "MCP input authorization refused" handling. |

## The fix being specified

Move step 2 into `start_request` (after `begin_run_start` has already run in both callers) and thread `queued: bool` into it.

- **`queued` is the queue-text match**, `queued.is_some()` from `start_run`'s `pending.filter(thread, text == prompt, run.is_none())`. It is **not** `request.queue_claim.is_some()`: a matched queue item can have `dispatch == None`, and `take_mcp_review` refuses on the match alone.
- `retry_rewrite` passes `false`. Its prompt is the fixed string `Rewrite the saved document using the captured feedback and selected chat; prepare a whole-note proposal for review.`, the same string X2's `retry_rewrite` passed to `start_run` and INT-W's `rewrite_run_start` sets. A pending review for a different composer prompt (the usual case) is consumed and refused with the existing message and "No new run was started". A review whose prompt **exactly equals** that string, in that chat, may attach, as it did in X2, so behavior is preserved. `false` differs from X2's value only if a queue item's text equals that exact string. No production decision is made here.
- `take_mcp_review`'s predicate and message, the saved-policy check, the context stamp snapshot, the one-prepare rule, the queue lease and the post-prepare `attach` refusals are unchanged. Nothing weakens the saved review authority; no grant is created or widened.
- Alternatives (pass `None` for writing and leave a review pending; skip attach for writing) leave a grant-bearing state unconsumed and need a stated reason; not recommended.

## Existing controls that stay unchanged and must still pass

In `crates/brn-desktop/src/threads_mcp_tests.rs`: `native_exact_input_is_transient_once_and_cleared_on_scope_policy_or_run_change` (including its `take_mcp_review` queued and changed-prompt refusals and the direct prepare/attach positive), `native_review_policy_switch_before_prepare_is_not_reauthorized`, `native_review_permitted_project_move_before_prepare_is_not_reauthorized`. These call `take_mcp_review` and `attach` directly; **none reaches `start_run` or `start_request`**, so they do not test the moved code. They remain necessary and are not replaced.

## New real call-path cases (desktop widget tests, offline fixture)

Fixtures to reuse, not copy: the offline workspace (`focus_tests::configure_offline_fixture`, or the explicit nonexistent `auth_file` fixture used by `starting_a_replacement_run_clears_the_old_paused_notice`), the MCP `policy(chat)` helper, `begin_mcp_input` / `review_mcp_input`, and the writing fixture used by the B1b retry tests in `threads_writing_tests.rs` (`ensure_writing_context`, `snapshot_writing_request_with_feedback`). The B1b refusal test already shows the pattern: `retry_rewrite` on an offline fixture stops at provider selection with the exact notice `Rewrite request saved; AI has not started. Choose a provider and model in Settings before asking BRN. Retry rewrite uses these same saved inputs.` and leaves records and receipts byte-equal.

| Case | Setup | Expected |
|---|---|---|
| **A. `retry_rewrite` with a pending review** | MCP policy enabled with `McpAccess::Chat(<writing conversation>)`; a pending review made from the composer for that conversation; a saved RewriteNote request with unsent general feedback in the editor; snapshot records, receipts and `writing_request` before | After `retry_rewrite(request_message)`: worker `None`; **records and receipts equal the snapshot** (no Run created, so the refusal is before prepare); `mcp_view.pending` and query cleared; the saved RewriteNote request unchanged; the unsent feedback editor value preserved; notice contains `MCP input review no longer matches this exact owner request. No new run was started; review the destination, scope and input again.` inside the retry wrapper `Rewrite request saved; AI has not started. … Retry rewrite uses these same saved inputs.` (assert the whole string once the implementation fixes how `warn` composes `notice`; warn tone) |
| **A2. second retry makes progress** | Case A state, no review pending | `retry_rewrite` again: reaches `prepare_thread_run` (offline fixture notice: provider selection, as in the B1b test) with records and receipts still equal; shows the refusal was one-shot, not a permanent block |
| **B1. `start_run` with a mismatched review** | Pending review for prompt P in chat C; `start_run("different", C, …)` | warn notice (the same message), worker `None`, no Run record, pending cleared |
| **B2. `start_run` as a queued dispatch** | Pending review; a queue item with text equal to the prompt, `run == None`, once with `dispatch == Some(..)` and once with `dispatch == None` | Both refused: the queue-text match, not the claim, decides. No Run record. Verifies `queued` is the match bool. If a matched item with `dispatch == None` cannot be built through the current checked queue paths, record that limit; do not manufacture an unsupported fixture |
| **C. `start_run` with a matching review (positive)** | Pending review for prompt P in chat C, same prompt, no queue, a provider that prepares but cannot authenticate (explicit nonexistent `auth_file`, as the existing replacement-run test does) | Use only the **synchronous** control flow of `start_run`. Immediately after it returns assert: `worker.is_some()`; the notice contains neither the take refusal ("MCP input review no longer matches…") nor the attach refusal ("MCP input authorization refused…"); `mcp_view.pending` is `None`; exactly one new Run record, whose id equals the worker's run id. Do **not** assert Failed, Working or any terminal state: `spawn_prepared` runs in the background and the missing credentials may legitimately fail it at any time. A take refusal returns before prepare (no Run); an attach refusal fails before spawn (`worker == None`, the authorization notice), so these assertions are conclusive for the take and attach steps. `attach_external_input` on success only sets the in-memory `scope.external` and extends the prepared input string; the Run record has no attached field and the fence is unchanged, so no persisted MCP field exists to assert. The existing direct take/prepare/attach positive in `native_exact_input_is_transient_once_and_cleared_on_scope_policy_or_run_change` remains the evidence for attachment content. No new test API or code is invented |
| **D. busy does not consume** | A running worker present, pending review set | `start_run` and `retry_rewrite` each return with the busy notice; **the pending review is still present** and the same later start can still use it. Needs an existing way to place a worker (the steering tests set `ui.worker`); if none is usable, record the limit and verify the order by review of step 1 |

Non-goals: no GUI, no real MCP server or provider, no network, no new fixture framework. Each case must fail if the `take_mcp_review` call is removed from the start path or moved after `prepare_thread_run` (a post-prepare take would leave a Failed Run, breaking A and B1). Demonstrating that with a throwaway seeded omission is a mutation-style check and needs its own explicit allocation; no automatic campaign.

## Evidence rules for the implementer

Report the pre-fix compile error as a static finding. A compile failure is not a behavioral red. Show each case passing on the fix, record the executed count of the new tests (nonzero), and keep the unchanged MCP controls in the same run. The `queued` argument and the ordering above are review items for the independent Opus review.
