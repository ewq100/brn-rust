# Independent rejected-Settings-Save review

Result: **no actionable findings** in frozen candidate `f67599f96ac54943a22f42c4038804c9b439be73`. Source satisfies the selected error-only draft-retention behavior without changing canonical ownership or other refresh policies. Final execution qualification remains pending and lead-owned.

## Source identity

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`; branch `codex/settings-save-rejection`.
- Baseline/verified merge base: `6fb5323fa37c1def0f46b2979a480c173a70b0f1`.
- Final candidate: `f67599f96ac54943a22f42c4038804c9b439be73`; clean working tree verified.
- Complete actual delta: only `crates/brn-desktop/src/threads_ui.rs` (12 additions/3 deletions) and the existing helper Save journey in `threads_pages_tests.rs` (38 additions). No additional test case, production caller, dependency, schema, configuration or runtime prompt changed.
- Final SHA-256: `threads_ui.rs` = `456ef65314e44e0ac871c2387ee5fd886c4dcebccf51687d2f4e0f40efbcca7b`; `threads_pages_tests.rs` = `ac8858436fd3fcadfc5d80e6b47457752045aa19851f42aab360ef835966ba92`.

Read the complete baseline-to-final diff and resulting configure/form/refresh/test source, applicable AGENTS/status/desktop README/architecture invariants/strategic-programming guidance and existing review/Rust guidance. Checked the selected task and design in the lead campaign record and `/tmp/brn-settings-rejected-save-preflight.md`. Inspected the actual Save listener, runtime/form ownership, app validation, checked Store settings writer, notice handling and the pinned widget API relevant to the fixture correction.

## Requirements and correctness

At `threads_ui.rs:3583-3596`, only an actual `Workspace::configure` `Err` captures the current `ProviderForm`. Error reporting still runs first; the ordinary `refresh()` still runs unconditionally; then the editable form alone is restored. `fail()` changes notice state, so taking the snapshot after it does not change attempted selections. The four form fields are provider, parent effort, optional helper provider and helper effort. Model, auth path, credentials path and helper model are separate existing `InputState` entities; refresh does not replace their values. Together these preserve the complete attempted Models form.

The native active `self.settings` cache is still assigned only from canonical refresh or the pre-existing explicit settings paths. No rejected form is copied into it. Retaining selections therefore does not activate them or bypass `Workspace::configure`, app validation, SQLite transaction/fencing or authority checks. The unchanged Save listener in `threads_tiles.rs:1693` remains the sole configure caller and retains its busy-worker gating. There is no asynchronous yield or deferred continuation between the configure result, refresh and restoration that would overwrite later user input.

All `Ok(ApplyOutcome)` variants retain the prior report-then-refresh behavior and receive no restoration. Explicit toolbar/Connect paths, navigation, worker completion, helper Off reset and other refresh callers have identical source. This deliberately selected Save-error retention policy does not generalize navigation or worker behavior. The small local option is sufficient; it duplicates neither shared validation nor a general form framework.

The concrete relative credential-path refusal is rejected at `brn-threads-app/src/settings.rs:89-110`, before operation allocation or Store access. Thus the witness can legitimately require unchanged full canonical settings and history for that rejection. More general service errors can arise during operation allocation, preparation or the immediate Store writer/commit path (`brn-threads-core/src/store.rs:1011-1317`). The candidate correctly does not infer that every `Err` means nothing happened: it preserves the existing canonical reread instead of restoring an old settings snapshot or reporting success. A later explicit retry continues through the existing checked service with the refreshed canonical base.

## Witness and fixture correction

The existing helper Save journey makes real segmented clicks, retaining its original Low → Off → Codex reset interaction. It additionally chooses parent High, changes the model, enters a relative synthetic auth path and clicks the real Save button. It then checks complete canonical settings and receipt history are unchanged, the absolute-path error is visible in notice state, and attempted model/path text remains. The helper effort control must still exist after rejection. Only the invalid path is corrected before another real Save. The reopened settings must contain parent High, the changed model, the corrected auth path and the original exact helper expectation (Codex, gpt-6-luna, High, corrected auth, no credentials directory). Existing busy-worker button assertions remain. These expected values come from the chosen interaction, not the new restoration implementation.

The test-only `2ea1c375` negative in `/tmp/brn-settings-rejected-save-qualification/baseline.log` actually failed with 0 passed/1 failed/342 filtered at the missing `helper-effort-high` control. Preceding canonical/history/error/text assertions passed before that failure. This demonstrates the original partial reset at a visible boundary.

The first `07e3b533` focused run in `/tmp/brn-settings-rejected-save-qualification/first-final-fixture-failure.log` reached the retained helper control, then failed because `.selected()` returned `None` rather than `Some(true)`. Final `f67599f9` changes only those two unsupported observation assertions into control visibility checks. I verified the pinned `gpui-component` 0.7.3 button implementation: `.selected()` controls selected styling and does not set `aria_selected`; note/list rows separately opt into that metadata through `aria_selected`. The correction is justified and does not discard a valid selection failure. Parent High and helper Codex/High retention still have meaningful end-to-end proof through unchanged successful-retry saved-value assertions; visibility alone is not claimed to prove the selected segment.

The fixture remains a fresh canonical temporary root with offline provider configuration and automatic polling disabled. Synthetic paths are strings; no credential contents, sign-in or provider is accessed. Test count is unchanged. No waits, external dependencies or new timing assumptions were added.

## Evidence limits

Read-only `git diff --check` passed for the complete final candidate. I ran no build, Rust test or native action. Neither the original negative nor the first fixture failure is a final passing result; the lead must qualify final `f67599f9`. Earlier PR 196 results do not qualify this follow-up. No network, provider/account action, credential-content read, source/shared-record edit or nested work was performed.

The witness exercises deterministic pre-write validation refusal and corrected retry, not every storage/serialization/commit error, concurrent external settings update, failed canonical reread or every editable field value. `refresh()` returns unit and may itself fail or replace the original notice with a refresh error; retaining its existing reporting/reread path does not establish successful canonical reloading in that case. The change makes no all-errors rollback guarantee. Its narrower guarantee is retention of editable selections after service `Err`, with the existing authoritative refresh/error policy preserved. Actual native interaction remains deferred, with no owner acceptance, merge or release claim.
