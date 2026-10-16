# Fix pass 1 on the combined desktop: launcher order, independent search lanes, U4/U10/U11 details

Fix pass 1 (`6db39420`, on `649494d6`) answers the nine findings of the first review. ⌘K now appends saved chat Message hits after every label match, so an arriving search answer never moves ↵. The two Notes search lanes wait only for their own request, using an explicit new-query flag. The Note's run status is the recorded notice alone, with the Run's progress as a muted line below it. `hold_worker` holds a `RunLease`, ⌘L in an archived chat lands on Unarchive, note and Message hits from archived chats share one Archived chip, and Changes resolves rewrite-request names on refresh. Everything stays in `crates/brn-desktop`, and the gate is recorded on this tree with the desktop and CLI from one build and the Poppler-only test reported as an environment gap.

Watch for: the busiest 900×600 Note guard test seeds a failed rewrite with no recorded notice, which the reliability service no longer produces. With a notice, the implementer measured the writing bar 2.5 px past the Note, and the new progress line is the one row in that stack the owner rule doesn't protect (confirmed test gap, non-blocking, pending a native look and the owner). Windows native-ui is still not compiled, and the evidence makes a manual `ci.yml` run on the PR branch a merge condition (confirmed, non-blocking).

**Verdict**: APPROVED

## High-level view

The blocking ⌘K defect is fixed at its root. `launcher` filters chats, notes and commands together, caps them at 20, then chains the Messages, so the rows the selection can already point at never move. The "raba" collision is pinned in a unit test (earlier rows unchanged after the answer) and in a window test (↵ switches the theme, ↓ ↵ opens the chat), and restoring the old order fails both.

The search lanes no longer infer a new query from "both lanes pending". `fresh` is set only by a both-lanes request and cleared when that request answers. Each lane's paging gates on its own `awaiting`, and "In chats" hides only while `fresh` is set. The paging test covers both interleavings the first review raised.

U10 now has the notice alone in `writing-run-status`, and `RunOutcome::detail` renders the progress as `writing-run-detail` below it only when the two differ. `threads-behavior.md` says the progress follows as a separate secondary line. The card's announced name still reads its progress line, and the evidence says so. That extra row is what sits behind the busiest-state concern below.

U4's Discard of the other edit has its own title and body. `focus_composer` routes an archived chat to the banner and then Unarchive. `archived_chip` serves both hit kinds. `refresh()` builds `refresh_names` once per history reload. Each has a test, and the changed assertions are replaced with equal or stricter ones.

<details>
<summary>Issues (2)</summary>

1. **Busiest Note with a recorded notice is not under test** (non-blocking, pending). `the_banner_stack_keeps_its_budget_above_the_text_at_900x600` seeds a failed rewrite without a notice. With one, the bar measured 2.5 px past the Note. Add the recorded-notice variant to that test, and either let `writing-run-detail` give way in the Note (the card keeps it) or take the owner's call after a native look.
2. **Windows native-ui compile before merge** (non-blocking, carried). Neither pass compiled native-ui for Windows, and PR CI runs native-ui on macOS only. Dispatch `ci.yml` on the PR branch and wait for the Windows and Ubuntu native-ui lanes before merging, or record the owner's explicit acceptance of the risk.

</details>

<details>
<summary>Details</summary>

### The busiest Note with a recorded notice

The committed guard for the busiest stale state seeds the failure through `failed_rewrite`. That helper writes a Failed `Run` whose `progress` is the reason string and a partial reply. It adds no Blocker attention, so the Note renders `RunOutcome::Failed(progress)`, one line, not the `Recorded` notice plus detail that a real failure now produces:

```rust
// threads_review_tests.rs, failed_rewrite: the Run is all there is
state: RunState::Failed,
progress: reason.into(),   // "Run failed: Synthetic provider stopped…"
// no attention item → writing_run_outcome() ≠ RunOutcome::Recorded
```

After the integration every failed or interrupted Run records a notice. A no-notice failure now exists only for legacy data, so the guard's "the bar ends the Note" assertion (confirmed) covers a state the owner will rarely see. The evidence measured the realistic state on this tree: the stale banner, open comments, a failed rewrite with a recorded provider notice and a 76-character progress, and a dirty bar. There the text is at its 48 px floor and the bar ends 2.5 px below the Note (8.5 px with `649494d6`'s inline form). The measurement used the GPUI test text system, which is wider than macOS fonts.

The evidence frames this as "which row gives way when every row is at its floor isn't settled by the owner rule". That is only partly true (likely). The rule protects banners, failure reasons and the writing bar. Since this pass, the failure reason is the notice, and `writing-run-detail` is the agent's own progress, which the rule doesn't name. So there is an option inside the rule: the progress line gives way in the Note when the bar would otherwise clip, and the recovery card, which isn't under the same height budget, keeps it. The contract sentence added in this pass ("the Note's run status show[s] it as a separate, secondary line") would need a "when room allows" qualifier. This isn't blocking: the clip is 2.5 px in a wider font, the evidence records it openly, and the busiest state's layout is already waiting on the owner. The test gap should close either way, so that the next layout change in this state fails a test, not a native check.

### Requirement coverage

1. Test expectations: done.
   - Settings stop → Interrupted.
   - Lease-free seeds count as crashed on Resume, Steer and Continue in desktop tests, and on Dispatch in the service's `run_recovery` tests.
   - `RunLease` is held for every live-worker seed, now including `hold_worker`. The workbench `run_fixture`'s exemption is justified in the evidence.
   - U1's lease-free Stop, decision 1 and the Review items on Stop/Archive/Resume each have a test.
   - The legacy notice survives only in recognition lists; 10 tools and `brn-threads-guidance-12` (main uses 11); search schema v4 with rebuild tests; canonical temp roots.
   - No assertion was weakened. This pass's changed assertions (`writing-run-status`, the U4 body, `RunOutcome::text`) each gain an equal or stricter replacement.
2. F-UI1: done.
   - "In chats" shows title, role, highlighted snippet and the Archived chip.
   - Paging is independent; activating a hit opens the chat at the Message; ⌘K shows the same service hits after the label matches.
   - `section_header`, `ListRow`, `loading` and `empty_state` are used as plan.md describes.
3. U4: done. The variant match goes through `AppError::Core`, never the text; Compare and Discard are offered; only `note` and `base_version` are read.
4. Data folder: done as an owner-terms startup message through `Display`, with a test. A Finder launch shows no window; the evidence lists that as an open product gap.
5. U10: done. All four surfaces show the recorded notice, and progress is a separate secondary line.
6. U11: done. The archived composer takes focus neither on open nor on ⌘L; Unarchive is the single next step; Continue is unavailable.
7. Contract and docs: done.
   - Decision 1 replaces the proposed sentence, and decisions 2 and 3 appear once.
   - The currency sentences at `threads-target.md:136` and `chats-and-projects/spec.md:70` are corrected.
   - `docs/status.md` gains one entry.
   - The evidence keeps implemented, verified, native-observed, owner-accepted and merged separate.
8. Gate: present with exact counts on this tree.
   - Workspace: 827 passed / 1 failed / 6 ignored; the failure is the Poppler-only test, reported as an environment gap.
   - Desktop shell: 294 / 1 ignored (two more than `649494d6`, matching the two new window tests).
   - `native-ui`: 80; `native-ui,native-retrieval`: 80; default features: 30.
   - Workspace and per-lane clippy, fmt, e2e fixtures, the CLI journey from the same build, headless startup, links, `git diff --check` and `Cargo.lock`: all clean.
   - Windows is not compiled; that is justified, with a merge condition (issue 2).

### Pending owner acceptance, not defects

- The 48 px text floor under a Note banner in the busiest 900×600 state is pending owner acceptance. `docs/status.md`, `plan.md` and the board record it as accepted ("i accept your offer", relayed by the lead on 16 Oct), but this review did not see that acceptance first-hand.
- R1, the same floor beside an open stacked comment draft, is a lead decision the owner can overrule.
- Nothing in this build is native-observed. F-UI1, U4, U10, U11, R1–R4 and this pass are automated-verified only.

### Carried, not from this diff

These are listed in the evidence and are unchanged by this pass:

- `resume_chat_queue`'s lease-free stop of a queue-claimed Working Run;
- the `project-delete` fence error;
- the backup-sweep edge;
- the slice H and G review notes;
- the U4 dependency on CLI round 3's caller-bound sessions.

The archived-note chip is dormant in the desktop: its owner search is unscoped, so no chat-only note from an archived chat reaches it. The evidence says so too.

</details>

<details>
<summary>File map</summary>

Reviewed `6db39420` on `work/integration-ui-reliability` against `649494d6` (fix pass 1), with the combined diff from `6f51d22c` as context. Inputs were the diff, the commit message, `evidence.md`, `ui-polish/plan.md`, the coordination board and the first review (`git show 6db39420:docs/work/active/integration-ui-reliability/review.md`). No suite was rerun and no spot-check was needed.

- `crates/brn-desktop/src/threads_lists.rs`: launcher order (label matches capped, then Messages) and the "raba" unit test.
- `crates/brn-desktop/src/threads_search.rs`: `fresh` new-query flag, per-lane paging gates, shared `archived_chip`.
- `crates/brn-desktop/src/threads_shell.rs`: "Searching chats…" as the launcher's last line.
- `crates/brn-desktop/src/threads_ui.rs`: `refresh_names` built in `refresh()`; archived guard in `focus_composer`.
- `crates/brn-desktop/src/threads_tiles.rs`: `refresh_name` reads `refresh_names`.
- `crates/brn-desktop/src/threads_workbench.rs`: `RunOutcome::text` returns the notice alone; `detail()`; `writing-run-detail` line.
- `crates/brn-desktop/src/threads_writing.rs`: separate Discard title and body for the other open edit.
- `crates/brn-desktop/src/threads_{chat,omakase,polish,tiled,writing}_tests.rs`: lease in `hold_worker`, ⌘L on archived chats, launcher collision, lane interleaving, archived note chip, U10 detail line, U4 body.
- `docs/architecture/threads-behavior.md`: progress as a separate secondary line.
- `docs/work/active/integration-ui-reliability/{evidence.md,review.md,review.json}`: fix pass record and the first review.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/integration-ui-reliability diff 649494d6 6db39420`.

</details>
