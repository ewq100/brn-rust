# Actual Mac acceptance: follow-ups, inbox and Restore

**Unperformed.** This checklist records no Mac access, native observation or owner
acceptance. Run only in a separately authorized Mac session against the reviewed
integrated candidate. Linux observations, toolkit/state tests and CLI results stay
separate. This is an interaction checklist, not another runtime specification.
Control names were inspected in the Actions, Inbox and Restore source drafts;
reconcile them with the candidate before the session.

## Session and synthetic fixture

- [ ] Record commit/tree, app/helper identities, macOS version, display scale,
  accessibility method, actor and observation time. Mark each item pass/fail/blocked
  with a screenshot or short observation; record owner acceptance separately.
- [ ] Use a fresh synthetic workspace only: an Action with unknown follow-up fields,
  another with a distinct due date, a private Project/chat with a waiting queue,
  and a small inbox containing valid documents, a malformed file, an unsupported
  file, a subfolder and a symlink. Record inbox file hashes beforehand.
- [ ] Prepare a synthetic full backup containing notes/figures/history, an unsaved
  recovery buffer and a paused queue. Keep a corrupt backup and a separate recovery
  parent directory. Record backup hashes and current workspace content/queue identity.
  Use deterministic conversion settings; no live provider or external delivery is
  authorized by this checklist. Keep queued messages inert.

## Actions: visible facts and safe editing

- [ ] Open **Task actions → Edit details…**. Inspect **Task follow-up details**:
  **Owner**, **Waiting on**, **Follow up**, **Save**, **Cancel**. Verify initial Owner
  focus, logical Tab/Shift-Tab order, keyboard save/cancel, readable field errors,
  accessible names and return focus. Inspect at a small usable window size.
- [ ] Unknown values stay blank/unspecified. Enter a synthetic owner, waiting-on text
  and a valid `YYYY-MM-DD` follow-up date; Save and reopen. Row facts show all three,
  with the existing **Due** date distinct. Saving neither completes the Action nor
  claims a reminder was scheduled. Clear each field and verify persistence.
- [ ] Try an invalid date; verify visible refusal and retained entries. Cancel edits
  and verify no accepted change. With the dialog open, change that Action through
  a second authorized synthetic host; stale Save must preserve newer accepted data
  and the local entries, with an understandable reopen/retry instruction.

## Inbox: explicit General consent and manual work

- [ ] In Settings, inspect **Document inbox**, **Choose General inbox folder…** and
  **Scan inbox**. Use the actual folder picker; cancel it once. Select the synthetic
  folder while a private Project/chat is active. Inspect the General-knowledge
  disclosure and **Use this folder for General knowledge?** confirmation.
- [ ] Cancel confirmation: no folder adoption/import. Confirm **Use General knowledge
  inbox**: the chosen path is visible, but no import or scan occurs. Inspect focus,
  accessible names and disabled Scan state before configuration. Verify that neither
  choosing nor confirming the folder dispatches the private chat's waiting queue.
- [ ] Explicitly choose **Scan inbox**. Observe progress and per-file Pending/Imported/
  Failed status with useful detail. Valid retained figures are readable; malformed
  and unsupported files have honest outcomes. Subfolders/symlinks are not followed.
  The destination stays General; private chat content is not sent to intake.
- [ ] Scan unchanged files again, retry a corrected failure, and cancel a sufficiently
  large synthetic scan. Inspect partial/pending results and safe explicit retry;
  no duplicate imported note is silently created. Reopen and inspect saved status.
  The private queue remains unchanged through completion, cancellation and failure.
  Compare original-file hashes/names: nothing moved, overwritten or deleted.

## Restore: separate validated copy, then explicit opening

- [ ] In Settings, inspect **Restore a full backup**. Exercise actual **Choose backup…**
  and **Choose recovery folder…** pickers, including cancellation. A recovery parent
  inside the current workspace and a corrupt backup must yield understandable
  refusal without damaging either source. Destination is a fresh `BRN-restored-…`
  child outside the current workspace, not an overwrite target.
- [ ] Observe validation and **Cancel validation** on a sufficiently large synthetic
  backup. Cancellation may wait for cleanup; it must not publish a recovery copy or
  claim success. After successful validation, inspect the exact destination, included
  content and exclusions, and the notice that the recovered inbox needs a new choice.
- [ ] At review, **Cancel restore** leaves no published copy. Repeat and use **Restore
  this copy**; observe publishing and completion. Verify controls/focus/accessibility
  while busy and after errors. Do not infer cancellation after publication began.
- [ ] Choose **Open recovered workspace** only after successful publication. Inspect
  the separate **BRN — Recovered workspace** window, saved notes/figures/history and
  unsaved recovery. Queues stay paused until explicit Resume; inbox scans require a
  new folder choice. Close/reopen the recovered copy. The original workspace remains
  openable with unchanged accepted content/queue, and the backup bytes are unchanged.

Record failures and blocked items as remaining gates. A passing session does not
by itself approve merge/release or establish broad live-model usefulness.
