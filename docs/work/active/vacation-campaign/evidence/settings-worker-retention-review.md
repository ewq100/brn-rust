# Independent complete-candidate review

Result: no remaining actionable findings in frozen candidate `c01ce1993217063ac63a53ee90750c8e208c737d`. The earlier completion-placement defect is valid/fixed in source. Final execution qualification remains pending; this review does not establish native usability, owner acceptance, integration or release.

## Reviewed identity and scope

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`; branch `codex/settings-draft-isolation`.
- Baseline: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`.
- Final candidate: `c01ce1993217063ac63a53ee90750c8e208c737d`; clean working tree verified after the correction.
- Complete baseline-to-candidate delta: five files, 164 insertions and 44 deletions. Reviewed the entire actual delta, reconciled the prior isolation/helper-reset candidate and the worker-retention corrections, and inspected resulting source and relevant callers rather than relying on author summaries.
- Guidance: repository AGENTS/status/desktop README/architecture invariants and behavior/verification; reviewing-product-candidates, its resource policy, and the local Rust review checklist. Current selected task authorizes narrow retention during unrelated Finished processing; navigation retention remains unselected.

SHA-256 of final changed files (under `crates/brn-desktop/src/`):

| File | SHA-256 |
| --- | --- |
| threads_state.rs | `593871ef16669669b4877d098b084a835401156651401afe93beee626d0d7e86` |
| threads_tiles.rs | `7722d90fe3b2c342505d5d65edc4bc3578794966d5fa2a2fdaaa0a2dfa14e937` |
| threads_ui.rs | `a36ef22e5f8256afd00b582c59dee37aabc2a80d7441adfe5f6d60bb57433455` |
| threads_pages_tests.rs | `437e7053310935a0b7c032342f148b6b1ea9c590b09e8edd3409f774b7e8cfa2` |
| threads_tiled_tests.rs | `9c3922bdc7ec7975a78f2c98050fc5ad0ba05a41f12715c541a5cb04597f1c76` |

## Findings and dispositions

1. **P2, valid/fixed: restoration preceded a second completion refresh.** In superseded `140439f907edca2dc1d50c1c4d205a5b2ba2f3af`, `threads_ui.rs:2835-2845` restored the Models form immediately after the first refresh. Successful Finished processing subsequently called `dispatch_next` at old line 2958. `threads_chat.rs:328-353` refreshes even when dispatch returns `Ok(None)`, clearing the restored High choice to canonical Medium. Trigger: open Models, choose High without Save, finish a synthetic successful backup with an empty queue, then Save. Consequence: the selected effort is lost, contrary to the selected retention task. `/tmp/brn-settings-worker-retention-qualification/final.log` records the actual focused failure, 0 passed/1 failed/342 filtered, saved Medium versus expected High. The lead identified the omitted caller; I independently verified the source path and actual log. Final `c01ce199` captures the draft at `threads_ui.rs:2836-2839` but restores only at lines 2958-2965, after theme persistence, pending replacement/start and queue dispatch, with a second open-Models check and complete settings equality. This closes the defect in source; unchanged regression execution is still required.

2. **Valid/fixed fixture deadlock:** superseded `375ca1d5` used capacity one for two synchronous sends before polling the Connect receiver. Commit `140439f9` changes only that fixture capacity to two; final `threads_pages_tests.rs:1150` holds the exact two queued events before `poll_worker`. The correction preserves both event payloads and all assertions. No execution of the deadlocking version is claimed.

3. **Prior helper-reset P2 remains fixed.** `threads_tiles.rs:1640-1643` resets helper effort to High when Off is chosen; direct provider switches retain the existing effort. The existing helper Save journey still clicks Low → Off → Codex and expects High. Final worker-retention changes do not weaken or undo this compatibility correction.

No other actionable source finding was identified in the complete final candidate.

## Requirements, ownership and caller coverage

The four-field `ProviderForm` owns only uncommitted segmented selections. Startup and ordinary refresh derive it from canonical settings. Models controls render and change that form; active model/effort consumers and explicit toolbar/connection paths continue using `self.settings`. Save overlays the form and existing text inputs onto a clone of canonical settings and calls the unchanged checked `Workspace::configure`; failed/deferred writes do not first make a draft active in the canonical cache. Helper credential derivation and core validation/fencing remain at their existing boundaries.

The new optional snapshot is created only while handling Finished (including the existing RunFinished conversion), with Models open and a known non-Connect worker. Worker draining terminates at the first terminal event, so Converted/disconnected paths cannot accidentally populate the snapshot. After the entire synchronous completion path, the form alone is restored if Models remains open and the complete `WorkspaceSettings` value equals the captured cache. `WorkspaceSettings` derives full `PartialEq/Eq` over all fields, including helper, credentials, delegation, theme, conversion, skills and MCP. The guard is not a selected-field comparison and never restores canonical settings.

I traced `poll_worker` through its worker-kind/identity capture, terminal retirement, run acknowledgement, `persist_theme`, pending-send/start paths and `threads_chat::dispatch_next`, including its empty-queue and error refreshes. These operations and their queue/Run effects remain unchanged. Models visibility prevents the edit-note refresh helper from reopening a note in the ordinary retained-form case; the final visibility check also avoids restoring after a future completion path changes the displayed section. Explicit configure, Connect, navigation, Converted and disconnection continue their existing refresh/reset behavior. There is no generalized retention framework, new dependency, schema, runtime prompt, provider action or persistent draft state.

## Witness effectiveness and observed evidence

- The extended tiled Save journey makes a real High segment click, feeds Finished through an owned synthetic backup worker, checks saved canonical provider/model/effort remain unchanged before Save, then uses the real Save action and expects the independently specified High choice plus the existing model/backup/delegation/credential invariants. It catches a visible loss through the complete poll path rather than checking the new snapshot variable. The unchanged negative witness actually failed on test-only `72951e11`: `/tmp/brn-settings-worker-retention-qualification/baseline.log`, 0 passed/1 failed/342 filtered, Medium versus High. The same assertion also caught the incomplete `140439f9` correction.
- The synthetic Connect journey clicks High, proves canonical effort is still Medium, then sends CopilotConnected and Finished. It checks canonical Copilot configuration and form reconciliation to Copilot/Medium. This specifically catches accidental retention after Connect, where Connected has already updated the in-memory canonical cache before Finished. Capacity two is sufficient without threads, sleeps or timing dependence.
- Fixtures use fresh canonical temporary roots, offline configuration and disabled automatic polling. Credential values are synthetic absolute strings; the injected events do not perform sign-in, provider calls or credential-content reads. The helper Low/Off/On witness remains intact.
- I ran only source/read-only inspection and `git diff --check` for the final baseline-to-candidate diff; that check passed. I ran no build or Rust test. Earlier `fde04773` qualification is not qualification of this candidate. The lead's final candidate checks were pending at review completion.

## Limits

`refresh()` returns `()` and records errors through notice state. Equality therefore proves that the complete cached settings value remained equal after the refresh path, not that every refresh or a fresh settings read succeeded. A failure before the settings assignment can leave the old cache/form in place; a failure after that assignment can coexist with restoration when the reloaded settings are equal. This candidate preserves that existing refresh error policy. The author report now states this limit explicitly; the earlier claim that any failed refresh prevents restoration was unsupported and has been corrected.

The focused witness exercises successful backup completion and the Connect exclusion, not every worker kind, failed completion, concurrent external configuration change, theme-persistence branch or queue-start outcome. The full caller review supports the narrow guard across those paths, but no new execution claim is made for them. No native interaction, real account/provider, private workspace, credential content, cross-platform runtime, owner acceptance, merge or release was inspected or exercised. Lead-owned final checks and disposition remain required.

## Lead evidence-path clarification

The review read final.log during the failed140439 attempt. Its exact historical bytes are preserved as first-correction-failure.log in the raw archive; final.log now holds the subsequent c01 passing qualification. Review completed before the final execution result and therefore correctly left that separate gate pending.
