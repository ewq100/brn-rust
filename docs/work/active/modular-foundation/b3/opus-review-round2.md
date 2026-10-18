# B3 correction review, round 2: **CHANGES_REQUIRED**

The fix closes the paths B3-1 named. One narrower route to the same defect is still open: an import completion can silently discard unsaved setup edits that belong to a different Note or scope. The fix is one condition plus one fixture.

**Review limits:** I read the actual diffs `ba7161cc..def88185` and `f2b060bb..def88185` and the B3 evidence docs. I made no edits and ran nothing: no build, test, lint or mutation. The finding below comes from reading the code and was not executed.

**Pins and scope check out:**
- The `HEAD` crates tree `181659aa…` equals the crates tree at `def88185`. Commits after `def88185` change only docs.
- Only the eight allocated Rust paths change. Manifests, the lockfile, app, AI, core, CLI, CI and scripts are untouched by the correction.
- The pre-B3 test prefixes are byte-exact (72,479 and 5,919 bytes). The test diff has zero removed lines, so nothing was weakened.

## Blocking finding

**B3-1b · Medium · an import attaching to one context discards a dirty form bound to another**

Where: `threads_ui.rs` in `complete_writing_reference_import`. The new dirty guard (around line 4239) only fires when the form's target equals the import's target. When attachment succeeds, the `Ok(())` branch (around line 4298) always calls `replace_writing_setup`. That call sets the form to `None` and clears the brief box.

How to reach it (navigation isn't blocked during an import worker):
1. Start an import on Note A. The form must be saved and clean to do this.
2. Switch to Note B, or to A in a project scope. Open the setup: because A's form is clean, it is replaced by B's.
3. Type a brief or change references in B.
4. Return to A. B's form is now hidden.
5. The conversion completes. The dirty check is skipped because the form belongs to B. `intent.target.validate` passes because A is displayed and its setup is unchanged. The reference is attached, then `replace_writing_setup` wipes B's edits.

The only notice says the reference was attached. This breaks the round's own requirement: never discard owner edits without an explicit action, and never move a draft across contexts implicitly.

The new fixture only covers a dirty form for the same target, so this gap is untested.

**Bounded fix (either):**
- Make the guard independent of target: if `self.writing_setup_dirty(cx)` is true, keep the import, skip attachment and show the existing honest notice.
- Or keep the attachment but call `replace_writing_setup` only when no dirty form exists; otherwise say the other form was kept.

Add one native fixture for the sequence above. It should assert that B's brief and references are kept and that A's setup matches whichever option is chosen.

## Verified correct

- **Same-context B3-1 fix:**
  - A dirty completion keeps the import, skips attachment and the automatic reload, and writes no setup.
  - Opening the setup while dirty rechecks instead of replacing the form.
  - Recheck reloads the exact Note, scope and current Note version through the token-fenced `SetupDraft::load`.
  - Recheck keeps the original expected setup version, so `validate` refuses a newer saved setup and no receipt is written.
  - A failed recheck keeps the form. Pending additions stay unnamed and unchecked.
  - In another context or after access loss, the brief, labels, reference controls and Save are all hidden.
  - The Reload button reads "Reload saved (discard edits)", with an explanation visible before the click.
- **Fixture:** it uses real typing, purpose, remove and attach clicks, the actual Converted channel and `poll_worker`. It covers an unrelated write, a different scope, access loss and restore, an explicit discard and the newer-setup conflict. Setting the worker and search matches directly is disclosed.
- **LOW1 (query cursor):** Next uses the stored query, Find uses the box, and the generation, target and token fences are unchanged. The paging fixture is meaningful.
- **LOW3 and LOW5:** quotation titles come from the same checked source and version, with a generic fallback. The new observation-only and self-reference notices are accurate, and the new message still contains the substring the existing assertion checks.
- **Guard recheck:** the second variant genuinely attaches after navigation and fails at the earlier notice assertion. Reporting it as caught while disclosing the "NO VERDICT" controller result is accurate, and it does not claim the later state assertions ran.
- **Command records:** 14 records, consistent with three disclosed failures followed by 11 passes.
- **Still holding:** the intake resolver, the shared start path, versions, replay, Undo and search fences are unchanged.

## Non-blocking (Low, wording)

Several warnings still say "Reload" where a non-destructive Recheck now exists:
- Search: "Writing context changed; Reload before searching references."
- Attach: "Writing context changed; Reload before choosing references."
- `attach_match` in `setup.rs`: "Reference search changed; Reload before attaching."

Pointing these at "Recheck and keep edits" would avoid steering owners toward discarding. The Save-error text is accurate.

LOW2, LOW4 and LOW6 are accepted as disclosed.

## Still open (not claimed)

Native human, IME and real-picker observation; hosted CI and full F21; live converter fidelity; the missing-pdftoppm PDF failure; D1B integration; owner, program and principal acceptance.
