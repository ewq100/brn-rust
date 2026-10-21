# Independent complete-candidate source review

Result: **no actionable findings** (P0–P3). Source review passes the selected requirements and correctness/responsibility lenses. This is not runtime qualification, native observation, owner acceptance, integration or merge approval.

## Reviewed identity and boundary

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch: `codex/import-target-owner`
- Candidate: `2ecf634d7de93fbf7381317168109b6d199531e4`
- Base and verified merge base: `bd60131dfd8b4652b17c0b142c411bfe87550570`
- Working tree clean at opening and final source inventory. Full actual candidate is the five-file committed delta, 23 insertions and 12 deletions; no staged/unstaged/untracked additions were present.
- Authority: `ordinary-import-target-ownership` in the lead checkout's `docs/work/active/vacation-campaign/campaign.json`, including its explicit Option design and resource/action limits. The older read-only design memo's enum preference is superseded by this selected task design.
- Prior Run candidates are frozen dependencies and excluded from this selected delta.

Read the repository AGENTS/status/index, desktop README, architecture/invariants, relevant attachment behavior, workflow, verification, strategic-programming, reference-import ownership record, selected lead task/design and review/resource-policy instructions. Used reviewing-product-candidates and the Rust review checklist; no nested reviewer was created.

## Findings and requirement evidence

No actionable findings. The complete diff and production consumers support these conclusions:

1. Ambient ownership is retired: `ThreadsUi.attaching_chat`, its initializer, both assignments and the picker helper's `.take()` are absent. Repository-wide Rust-source inventory found no remaining `attaching_chat` route.
2. All six ordinary picker calls are migrated. `threads_chat.rs:793` passes `Some(id)` after the same composer/chat setup. `threads_projects.rs:201` passes the same captured project scope via `Some(import.clone())`. The two `threads_tiles.rs:183,191` and two `threads_writing.rs:891,906` general calls pass `None`. The project ID retains its established scoped-import meaning; the checked service already accepts an active Project or Open Thread.
3. `threads_ui.rs:3068-3095` exposes the named owned `attachment_chat: Option<String>` through the existing chooser/helper. It is captured before the native prompt at the same stage as the former `.take()`. `async move` keeps that owned request through picker completion; later navigation does not replace its target.
4. The reference caller at `threads_ui.rs:4191` explicitly passes ordinary target `None`; the existing helper still normalizes any reference request to no ordinary destination. The separate captured `ReferenceImportIntent` and its post-picker target validation remain intact.
5. Pre-picker admission (`threads_ui.rs:3085`), post-picker admission (`:3107`) and conversion admission (`:3146`) remain in the same order with the same notices/early returns. Picker cancellation still executes the empty `Ok(Ok(None))` arm and drops the captured request when the callback ends. Empty-path, failed-picker and disappeared-view paths introduce no retry state.
6. `attach_chat_path` (`threads_chat.rs:736-774`) remains unchanged and directly supplies `Some(chat)` to `convert_file`; its one-path and worker guards remain intact. Rust-source inventory includes the two unchanged document-test conversion callers as well as every production call.
7. The conversion implementation (`threads_ui.rs:3138-3273`) and event/worker contracts are unchanged. Conversion still moves `attachment_chat` into `WorkerEvent::Converted.chat` at `:3257`; completion at `:2859-2969` uses that captured value in `import_full_scoped_with_conversion`, opens a general imported note only for `chat.is_none()`, and keeps reference/cancel/error/queue policy. No fallback to current navigation or conversion-default change was added.
8. Actual checked consumer `brn-threads-app/src/intake.rs:292-389` retains settings/provenance checking, operation binding/replay and active Thread/Project scope validation. UI forwarding does not become a second persistence or authorization owner.
9. Responsibility cost is proportionate: the two attachment actions now supply destination and intent together, the established Option-valued conversion/worker boundary is reused, and no enum adapter, newtype, request framework, extra layer or duplicate policy is introduced. The existing ordinary chooser wrapper predates this change and continues to give ordinary callers a reference-free interface.

No bug is claimed. No new forwarding-only tests are necessary for this migration. Existing consequential tests remain unchanged: `b3_reference_disconnect_clears_intent_before_the_next_chat_attachment` injects captured `chat_b` completion and verifies canonical attachment to that exact chat, preserved writing setup and paused queue; cancelled-reference conversion verifies no canonical save/attach; document conversion tests cover actual selected helper and captured useful-information project behavior. I inspected their relevant source; I did not execute them or credit them as fresh passing results.

## SHA-256 of reviewed changed files

| Repository-relative path | SHA-256 |
|---|---|
| `crates/brn-desktop/src/threads_chat.rs` | `3b027f5660d33c1a0c94a7fda8d9ddea20db30c91c0fdd60617b09b76e55f5e0` |
| `crates/brn-desktop/src/threads_projects.rs` | `f52e21cdcbf140a7b839f733a83996d7ce7f9888b4cef93900a7d268bce24994` |
| `crates/brn-desktop/src/threads_tiles.rs` | `7d6cc3b55375d1ceb3822b2e95f8208b0484cbd3f958fe0f659bdba2632f7572` |
| `crates/brn-desktop/src/threads_ui.rs` | `aa421dbd3aab66a7f055b01f6b4ce68088c6930c40db6c561078a02a35634ab1` |
| `crates/brn-desktop/src/threads_writing.rs` | `5aa0ea3231c0cc2b5836b591ee3665c0f283fc170ab1a1399a28be6fffa4dce1` |

## Limitations and next gate

Read-only Git/source inspection and hashes only. No source/Git/shared-record edits, builds, tests, network/provider/account/native activity, owner data access or nested agents. Only this task-owned `/tmp` receipt was written. Runtime checks are lead-owned and their outcomes are not inferred here. A real native picker/navigation/cancellation interaction remains separate; source and headless tests cannot establish native usability. The lead must reconcile prior dependencies and fresh qualification/integration gates on the actual candidate before publication/integration.
