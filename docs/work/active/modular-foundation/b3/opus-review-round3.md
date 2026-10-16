# B3 correction review, round 3: **APPROVED** (source only)

B3-1b is closed. An import completion no longer discards unsaved edits in any form, including a hidden form for another Note or scope. I found no new blocking defect, and no previously accepted item is reopened.

**Review limits:** I read the actual diff `def88185..93b92d7b`, with `f2b060bb..93b92d7b` as context, plus the B3 evidence docs. I made no edits and ran no Cargo, build, test, lint or mutation. I read the retained before-fix log only.

**Pins and scope check out:**
- `HEAD` is `fc0ce49b` and the worktree is clean.
- The tree at `93b92d7b` is `c2838856`. Its crates tree `0afb215e` equals the crates tree at `HEAD`, so commits after the pin change only docs.
- The source delta touches only `threads_ui.rs`, `setup.rs` and `threads_writing_tests.rs`. All of them are within the original eight allocated paths.
- No manifest, lockfile, script, CI, app, core, AI or CLI file changed.

## Verified

- **The guard (`threads_ui.rs:4241-4244`).** `if self.writing_setup_dirty(cx)` no longer depends on the import's target.
  - It runs after the canonical import and receipt resolution.
  - It runs before the already-attached check, attachment validation and `replace_writing_setup`.
  - So a dirty form for any Note or scope keeps its target, brief and references. The import is still retained and no setup is written.
  - The clean path is unchanged: the already-attached check, fresh `validate` against the expected setup version, and `set_writing_setup`.
- **Dirty detection (`threads_ui.rs:3869`, `setup.rs:168`).** It compares the whole value (brief plus pending references) with `saved`, so it covers edits in a hidden form. Any doubt, such as whitespace, counts as dirty, which errs on the safe side.
- **Every automatic replacement path.** `writing.setup` and the brief box are only set inside `replace_writing_setup` (`:3933`, `:3953`, `:3959`). Its five callers:

  | Caller | Replaces when |
  |---|---|
  | `load_writing_setup` | Clean form only; a dirty form is rechecked instead |
  | `recheck_writing_setup`, different context | Clean form only; a dirty form gets the return-to-original-context notice |
  | `save_writing_setup` | Only after an explicit, checked Save succeeds |
  | Import completion | Only after the guard; dirty forms return before this |
  | Reload request | Explicit "Reload saved (discard edits)" click |

  Navigation, refresh and Stop clear only `reference_import`; they never touch the form. The response's audit table is accurate.
- **Privacy.** The notice is generic and names no Note, brief or reference. The fixture checks that "Note B", B's brief and the pending reference title are absent. While A is displayed, the brief, reference rows, attach and Save controls are all hidden.
- **Wording.** All three changed warnings now say "Recheck and keep edits", which is the real button label (`setup.rs:302`). The advice is coherent:
  - If the dirty form belongs to the displayed context, Recheck keeps the edits.
  - If it belongs to another context, Recheck shows a non-destructive return-there notice.
- **Fixture.** The previous 148,018 test bytes are an exact prefix (`cmp` passes) and there are no removed lines. The fixture name is unique.
  - The sequence is real throughout: real typing, purpose and attach clicks, the `Converted` channel and `poll_worker`.
  - It then returns to B, clicks Recheck then Save, checks the persisted values, and ends with the labelled explicit discard.
- **Before-fix evidence.** The retained log's SHA256 `dc3877de…` matches the receipt. It fails at line 3745 with "A completion must retain the other Note's hidden dirty brief": `left: "Clean A setup"` (A was attached and its setup reloaded into the form), `right: "Private B brief typed…"`. That is exactly the B3-1b path. The receipt records 0 passed / 1 failed / 287 filtered, exit 101, on a separate source and target.

## Non-blocking (Low, optional)

1. Two transient-race messages still end in "Reload": `setup.rs` `attach_match` ("Reference changed during inspection; Reload"), and `SetupDraft::load` ("Writing setup changed during inspection; Reload"). "Recheck and keep edits" would be more consistent there too. They weren't in the round-2 list.
2. With a dirty form, the completion notice always says "Find the imported Note to attach it", even if the Note is already attached in A. This wording predates round 3, from the same-target case, and is harmless.
3. The fixture covers a different Note but not a different scope. Because the guard no longer looks at the target, both are covered structurally.

## Still open (not claimed)

- Native human, IME and real-picker observation
- Live converter fidelity
- The PDF failure from missing pdftoppm
- Hosted CI and full F21
- Mutation (none this round)
- D1B/D2 integration
- Owner, program and principal acceptance
