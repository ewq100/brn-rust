# B3 independent review (read-only): **CHANGES_REQUIRED**

The review found one material defect: unsaved brief and reference edits in the setup form can be lost with no owner action. Everything else is either a proportional residual or a qualification that remains open.

**What I checked.** I read the actual diff `f2b060bb..ba7161cc`, the specs and the B3 evidence docs.
- **Pins:** tree `3754ad17…` and crates tree `5c774f49…` match. `ed8574b5` contains the same crates tree.
- **Scope:** only the eight allocated Rust paths changed, plus B3 docs and the status link.
- **Production frozen:** production code is unchanged since `29c9cf68`. The two later commits change tests only.
- **Test prefixes:** the pre-B3 test bytes are identical (72,479 bytes in the native writing tests; 5,919 in `import_coverage.rs`).
- **No execution:** I made no edits and ran no builds, tests or mutation. The worktree is clean.

## Blocking finding

**B3-1 · Medium · unsaved setup form edits are lost without owner action**

Where:
- `crates/brn-desktop/src/threads_ui.rs:3869-3892`: `load_writing_setup` always overwrites the brief box and replaces the draft.
- `threads_ui.rs:4213`: a successful import completion calls `load_writing_setup` automatically.
- `crates/brn-writing-ui/src/setup.rs:130`: `displayed()` compares a store-wide `change_token`.
- `setup.rs:287`: when the token changes, the drawer hides every control, including Save.

What happens:
1. **Import completion.** Starting an import requires a saved, clean form, but the drawer stays editable during conversion. On completion, the attach step still succeeds because the setup version hasn't moved. The automatic reload then overwrites the typed brief and drops pending reference changes, and the notice reports only success.
2. **Any unrelated write.** The token (`store.rs:229`) is store-wide: the latest revision, `data_version` and pending operations. Any canonical write hides the form, including run progress, a chat message or another process. The only control left is Reload, which discards the owner's text.

The stale-save fixture asserts "form retained" by calling `save_writing_setup` directly on a hidden form. The UI doesn't offer that path, so the stated guarantee ("failed save must retain the owner's draft") doesn't hold for an actual owner.

**Correction:** never discard owner-entered unsaved brief text or pending reference edits without an explicit owner action. Either fix is routine and needs no product decision:
- **(a) Keep edits on same-context reload.** When Reload or the import completion reloads the same Note and scope, keep the dirty brief and say the saved setup changed. Save then checks against the new version. Recheck pending references through `checked_label`, or drop them with "no longer available". Never carry this across a different Note or scope.
- **(b) Don't auto-reload a dirty form.** On import completion with a dirty form, skip automatic attach and reload, keep the import and tell the owner to attach explicitly. Make Reload state that it discards edits.

Add one native fixture: edit the brief during conversion, complete the import, and assert the text is kept with no lost write.

## Verified correct (no defect)

- **Intake resolver** (`full_import_note_in_scope`):
  - The receipt payload must equal the persisted Applied receipt. Only the persisted copy is used afterwards.
  - It requires exactly one Source write.
  - Source scope is checked on both the receipt snapshot and the current record, using the single existing visibility predicate.
  - The requested SHA selects the exact full-import link, so it never falls back to `external_version`, title or locator.
  - Notes must be live, not superseded and in scope, and the result must be exactly one Note.
  - The store token is checked before and after the read.
  - Every failure returns one generic error. The importer, dedup and storage policy are unchanged.
- **Desktop import path:**
  - Reference imports use the existing picker, `convert_file`, worker, `import_full_scoped` (captured scope) and `set_writing_setup` with the captured setup version. There is no duplicate converter or store route.
  - Stop turns the result into an error before saving. A dropped connection clears the pending reference intent.
  - Attachment is refused after the target or access changes, and the legitimate import is kept.
  - There is no fallback to the global Chat: the attach chat is forced to none.
- **Rewrite path:** the configured rewrite runs saved inputs → `ensure_writing_context` → the requirements snapshot → `retry_rewrite` → `rewrite_run_start` → `start_request`, which is the one B1b start. Plain rewrite keeps the default binding byte-equivalent.
- **Review UI:** Sources used renders only after the existing early-review gate and private-source disclosure, and uses only the existing checked review evidence. Import coverage, retained-text reading, model claims and quotations are shown separately. "Attached" is never presented as "read", and old evidence is never rebound.
- **Async search:** replies are fenced by generation, target and token, and cancellation is on drop. Labels come only from the scoped service's exact versions.
- **Tests:** six app scenarios and twelve native fixtures, with meaningful assertions. The before-proof is a real behavioral failure on the accepted base.

## Proportional residuals (non-blocking)

| ID | Severity | Location | Issue | Correction |
|---|---|---|---|---|
| R1 | Low | `threads_ui.rs:3941` | "Next" reads the current query box. The cursor is a plain offset (`search.rs:751`), so an edited query silently skips its first page. | Store the query with the cursor in the draft, or clear `next` when the query changes. |
| R2 | Low (usability) | store-wide token | The drawer is hidden during any canonical write, such as active runs or other processes. This fails closed but is disruptive. | Mostly addressed by B3-1. Narrowing the token is optional later. |
| R3 | Low | `references.rs:100` | Quotations show the raw record ID instead of the checked source title. There's no privacy leak. | Map to `evidence.sources` titles. |
| R4 | Low | `references.rs:32` | `writing_setup` adds a full `records()` load on every review render. This matches existing per-render queries. | Optionally derive it alongside the existing review cache. |
| R5 | Low | `threads_ui.rs:4157`, `4214` | Notices can mislead: an observation-only re-import with no eligible Note says "Imported Note retained". Re-importing the working Note's own file is refused by core self-reference validation with a "Target…changed" message. | Adjust the wording. |
| R6 | Low (test meaning) | delayed-search and configured fixtures | (1) `load_writing_setup` bumps the generation, so the target/observed comparison in the search reply handler is never independently tested. (2) The configured fixture saves via a real click, but the run is prepared manually (no worker started), relying on B1b for dispatch. (3) "Must be current" is set by direct mutation rather than a click. (4) The match count includes the working Note itself and archived hits. | Optional. |

## Mutation proposal assessment (not executed)

Both proposed omissions are well targeted, and the 600-second scope is proportionate.
- **R1 omission:** without the receipt filter, the persisted receipt is used. The altered-payload call then succeeds and fails the `c1eeeb19` assertion.
- **R2 omission:** using the captured target with A's still-current setup would attach to A. That fails the "A references unchanged" assertion.

Neither omission covers B3-1. That needs a normal fixture, not a mutant.

## Still open

These are not claimed by this review: native-human/IME/picker observation, hosted CI and the full F21 check, program and owner acceptance, live converter/provider fidelity, the carried missing-pdftoppm PDF failure, D1A/D1B integration, publication, and principal acceptance.
