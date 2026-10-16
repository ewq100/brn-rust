# UI polish: handoff for the next polish pass

**Written:** 16 October 2026 by the UI polish lead, at the end of the owner's assignment. **State:** the UI polish (`work/ui-polish`) and the daily-workflow reliability work (`work/daily-workflow-reliability`) are combined on `work/integration-ui-reliability`. The owner authorized one PR to `main`, merged after the four required checks. The PR records the CI receipts and the merge.

Use this file to pick up the remaining UI work. The current target, behavior contract and invariants still own product requirements: [threads-target](../../../architecture/threads-target.md), [threads-behavior](../../../architecture/threads-behavior.md) and [invariants](../../../architecture/invariants.md).

## What exists

- **Design direction:** [plan.md](plan.md). It covers the critique, tokens, shared components (`threads_components.rs`), layout, copy, small-window and focus rules, and the owner decisions.
- **Slice plan and review checklist:** [impl-plan.md](impl-plan.md).
- **Evidence:**
  - [UI evidence](evidence.md) and [UI verification](verification.md), which keep implemented, automated, native-observed and not observed apart.
  - The [integration evidence](../integration-ui-reliability/evidence.md), with its [open items](../integration-ui-reliability/evidence.md#open-items).
  - The [reliability evidence](../daily-workflow-reliability/evidence.md).
- **Screenshots:** [before/](before/) is the baseline, [after/](after/) holds the UI branch rechecks, and [native/](../integration-ui-reliability/native/) holds the combined build.
- **Native observation tooling:** [observation/](observation/).
  - `input.swift` posts real CGEvent input and needs Accessibility trust for the host app.
  - `wincap.swift` captures windows with ScreenCaptureKit.
  - `seed-fixture.rs.txt` seeds synthetic data through the shared service.
  - `observe-harness.rs.txt` is a fallback that drives views without input. It is evidence tooling only: never compile it into the product.
  - Use fresh `/private/tmp/...` data folders (BRN refuses the `/tmp` symlink), the offline-fixture provider, and empty scratch `HOME`/`CODEX_HOME` for any store set to Codex.

## Decisions to preserve

- **Earlier owner-accepted behavior** (listed in [impl-plan.md](impl-plan.md#hard-constraints)):
  - ordinary clicks reuse the panel, and Open beside makes splits
  - every writing interaction stays in the Note panel
  - chat queue, Steer, Stop/Resume, recovery and Quiet
  - ⌘K, the five themes, Projects with checked Trash, protected notes, exact-candidate review, version checks, replay and Undo
- **16 October 2026 owner decisions** (recorded in [plan.md](plan.md#open-questions-conservative-defaults-apply) and the integration evidence):
  - Theme switching lives only in Settings → General and ⌘K, not the status bar.
  - Narrow Note panels use an in-panel stacked comments section.
  - Launch receipts fold under "Background · N" in Changes.
  - No desktop Restore in this pass.
  - A notice clears when the next Run starts. Archive and the owner's Stop add no new notice.
  - Archived chats' Messages appear in owner search with an Archived label.
- **Text floor:** in the busiest 900×600 Note state the text may drop to 48 px, and banners, failure reasons and the writing bar are never hidden to meet the budget. The owner accepted this. The lead extended the same floor to an open comment draft (R1); the owner can overrule that.

## Remaining work, in suggested order

1. **Native findings on the combined build, not fixed** ([integration evidence](../integration-ui-reliability/evidence.md#native-observed-recheck-of-6db39420)):
   - An "In chats" hit title truncates to "R…" beside the Archived chip at 170 pt.
   - A ⌘K snippet shows raw Markdown table text.
   - Archived chats show the global "Resume queue" strip. Decide whether it belongs there under U11.
   - One click on a hit's snippet text didn't open the chat, while its title line did. It was seen once.
2. **Open layout question:** the busiest Note with a recorded notice. In test fonts the writing bar ends 2.5 px below the Note at 900×600 with the text already at 48 px. Look at it natively first; if it reproduces, either let `writing-run-detail` give way in the Note or ask the owner. The guard test `failed_rewrite` needs a recorded-notice variant.
3. **Review findings that may still be open.** Many were fixed in the fix pass (INT-1 to INT-6). Re-check these against the code before working on them:
   - Panel Close/Maximize accessible names don't say which panel.
   - ↵ on a focused consequence-dialog confirm is untested.
   - The launcher's keyboard selection can scroll out of view.
   - Two tooltips use the word "record", which the copy rules forbid.
   - Disabled-reason wording drifts between surfaces.
   - The one-primary rule is tested by row, not by the filled button. Kit 0.7.3 doesn't expose button variants.
   - The several-notes review bar wraps to two rows at 1260×860.
   - Home keyboard routes.
   - The U10 test seeds the older run's notice first, so it doesn't prove the record-ID match.
4. **Not natively observed yet:**
   - live streaming, Stop/Steer, and the owner's Stop of an unattached or killed Run (these need a provider)
   - ⌘K ↓↵ onto a Message, search paging and ⌘L
   - R2/R3, U3 rows with Protected, and the stale-edit, newer-version, import and recoverable-edit banners
   - attachments, Compare, Trash restore, export and file pickers
   - drag-resize, 1050×700, Solo, the Läänemeri and Paber themes, VoiceOver, IME, contrast measurement and keyboard-only ⋯ menus
5. **Smaller known gaps:**
   - The All chats day grouping uses UTC while times are local.
   - The recovery card's announced name includes its progress line.
   - The unscoped Notes search lists no note hit from an archived chat.
   - A Finder launch with an unusable data folder shows no window; the error goes only to stderr. A startup error window would be a product change.
6. **Needs another branch or the owner:**
   - **U3:** Review rows can name their targets only while the candidate is still prepared. The service would need to put target note IDs and a protected flag on the attention.
   - **CLI round 3** (`work/cli-agent-contract`): caller-bound edit sessions change the desktop's begin/buffer/Save/Discard path, recovery buffers and U4's Discard of an edit opened elsewhere. Agree the API before that design is approved.
   - **Launch receipts:** ChatControl receipts are created on every launch. Not creating them would be a service change.
   - **Separate assignments:** a desktop Restore flow, and the desktop review UI and grants for owner-authority phases 2–3 (B1).
   - **U5:** verbatim imports show "Partial". That is an owner rule; don't change it.
   - **`resume_chat_queue`** stops a queue-claimed Working Run without a lease check.
   - **Windows:** native-ui was first compiled in this PR's CI. Check the PR for the result.

## Working rules that helped

- Read [plan.md](plan.md) before changing UI, and extend its tokens and components instead of adding new styles.
- Run `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh`, which builds the helper and desktop, runs clippy with `-D warnings` and all native tests serially. Also run the other desktop lanes listed in [impl-plan.md](impl-plan.md#commands).
- Keep automated checks, native observation, owner acceptance and merge apart in evidence. Never claim native observation from tests.
- If another agent or the owner may be driving the Mac, agree a lock before opening any BRN window.
