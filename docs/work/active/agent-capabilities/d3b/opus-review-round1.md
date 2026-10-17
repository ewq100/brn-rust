**Verdict: CHANGES_REQUIRED**

I reviewed the actual diff `000ed920..6ff0ccdb` in `/Users/evokessler/.t3/worktrees/brn-rust/t3-9527c407`. HEAD is `6ff0ccdb`, the tree is `dc12c1d0`, the crates are `9749d9bd`, and the worktree is clean (92 files, +6463/−90). Two small, bounded defects block approval. Everything else either meets the requirement in source or is nonblocking.

I did not build, test, run anything live, or read the artifact receipts. I did not check the reported pass counts myself.

## Blocking defects

**D1 — MEDIUM. Native inventory hides installed packages from other scopes.** `crates/brn-desktop/src/threads_skills.rs:69–76` (`refresh_skills`)
- **Problem:** A package gets a disabled inventory row only if `!settings.skills.iter().any(|b| b.id == info.id && b.project != project)`. Once a package has any binding in another scope, the native view hides it in every other scope. That includes a disabled binding left behind after a revoke.
- **Effect:**
  - The native owner cannot enable the same package in general and in a second Project, though core, app and CLI all allow it.
  - Native has no policy-row clear (native `SkillAction::Clear` only clears the selection, i.e. selects None), so the owner cannot recover inside native either.
- **Why the filter is wrong:** Disposition 4 makes the global inventory a deliberate owner contract, so the filter protects no privacy and only removes function.
- **Test gap:** `threads_skills_tests.rs` covers general scope only.
- **Fix:** For any inventory package with no row in the current scope, add a disabled/available row without looking at bindings in other scopes. Add a native test that enables the package in P1, then shows and enables it in general and in P2, while P1 policy stays hidden in those scopes.

**D2 — LOW–MEDIUM. A selection on an archived or deleted Project cannot be cleared.** `crates/brn-threads-core/src/store.rs:2071`, with `change_skills` `Select { id: None }` (~1270)
- **Problem:** The retained-binding exception uses `old.skills.contains(binding)`, which compares the whole struct, `selected` included.
  - Clearing the selection turns `{enabled:true, selected:true}` into `{enabled:true, selected:false}`.
  - That value is not in the old Settings, so `apply_on` rejects the write with "enabled skill requires a current active Project".
- **Where it fails:** The writer pre-check (~1170–1178) correctly skips `id: None`; only `apply_on` refuses. This breaks `skill-select --id none --project <archived>` and the native composer's None option.
- **Why it matters:** It contradicts disposition 2 and the app message that retained policy "may be inspected or cleared".
- **What still works:** `skill-clear` (removes the row) and disable (only enabled rows are checked).
- **Test gap:** `project_selection_is_exact_and_clear_remains_available_after_archive` covers only `Clear`.
- **Fix:** Treat a binding as retained if the old Settings has an equal binding ignoring `selected`, and only when the new binding is unselected. Add a core test that deselects after archive and still refuses a new select or enable there.

## Coverage gap

**G1 — LOW–MEDIUM. `read_skill` ranges are under-tested.** `crates/brn-ai/src/threads.rs` ~1033–1068, `BatchHook` ~2232
- **Missing tests:**
  - a start offset above zero;
  - a start that is not on a UTF-8 character boundary (rejected both in the batch and in the call);
  - partial delivery via `fit_prefix` with `next_start` and `complete:false`.
- **Mitigation:** The code reuses the tested `context_budget` helpers.
- **Provenance:** A partial read records the same whole-file `SkillLoad` identity as a full read, so stored details cannot tell partial from complete. The README wording just about covers this, so it is a precision preference, not a defect.
- **Fix:** Add one AI test for a non-boundary start, rejected before any effects, and one for a partial-then-continued read.

## Nonblocking

1. **Details recorded before the final context check.** `begin_context` stores `loaded_skill_details` after the begin-time preflight. Dispatch then adds host-intake and continuation context and preflights again (~2562). A run can therefore record admitted details and then fail with zero provider requests. This is disclosed as "not proof of upstream processing"; the reverse case cannot happen.
2. **Extra Run write.** The initial `update_run` runs even when the identity list is empty, adding a Run version and operation to every run. The fence is unaffected. Skipping it when the list is empty would remove the write.
3. **Built-in digest drift is undocumented.** Explicit built-in bindings pin a digest. After a BRN build changes a packaged SKILL.md, those bindings become unavailable. A selected one refuses runs, and a bound intake/maintain-notes preload disappears until re-selected or cleared. This fails closed and matches exact identity, but should be documented.
4. **Resolved chats show no loaded details.** The app's `loaded_skill_details` returns empty for non-Open chats. This may be stricter than needed for historical provenance; it is a question, not a defect.
5. **CLI `runs` does several record reads per run** (N+1) for details. It is bounded and off the critical path.
6. **Any policy change interrupts every Working run.** This includes no-ops and changes scoped to another Project, because any Settings write invalidates `live()` for all runs. It follows existing Settings fencing and is accepted under disposition 2; I note it as a residual.
7. **Test hygiene:**
   - `write.archived = false;` in the core test is a no-op line.
   - The raw-owner test asserts `is_err() || !Applied`, which is weak, but it does assert the record is unchanged.

## Verified in source

- **Disposition 1 (schema 8):** backup/restore/upgrade accept 3–7 plus current, and `open_existing` refuses 3..=7. The new fields are additive and default/empty-skipped. No SQL table or index changes. Mechanical test diffs only change 7→8, add initializers, or swap to an equivalent JSON assertion; no assertion is weakened.
- **Disposition 2 (settings writer):**
  - The policy change reuses the existing owner-only settings update with input-hash replay, and Configure preserves `skills` on both fresh and replayed writes.
  - `validate_skills` enforces at most 128 bindings, a unique scope/ID pair, selected implies enabled, one selection per scope, and a trimmed nonblank Project.
  - Raw owner writes are validated in `apply_on`, and Run details are bounded in `validate_request`.
  - Fencing, attention and leases are unchanged.
- **Disposition 3 (retention):** a missing or mismatched selected package refuses before the run is created, with zero provider contact (asserted by a CLI stub test). There is no name or path fallback.
- **Disposition 4 (inventory vs policy):**
  - Inventory stays global.
  - `--project` scopes only the policy projection.
  - Run details are filtered by scope before the 16-entry bound and counts, and are hidden after a chat move.
  - Other-Project policy is not disclosed.
- **Disposition 5 (legacy preloads):** `legacy_source_preloads` seeds only enabled, available built-in preloads at fence 1. New selected IDs are not seeded.
- **Disposition 6 (extra seams):**
  - The helper `BatchHook` has `skills: None`, and `read_skill` refuses helper calls.
  - The summary key now uses `run.guide_identity` (cache-reuse and refresh-on-removal tested).
  - Only the exact `skill_unavailable` error maps to the static notice and CLI `Unavailable`. HTTP spoofing stays generic (tested).
- **Disposition 7 (native):** no file I/O during render, refresh is stamp-based, actions are guarded against a stale scope, the unsupported-platform view is cfg-gated and static, and details sit in `conversation_detail`.
- **Catalogue freeze and `read_skill`:**
  - A bundle version plus metadata digest goes into the guide, trace and summary key, and stays stable within a run.
  - `read_skill` re-checks run state through `live()` and the digest through `current()`, and bounds output.
  - A whole invalid batch is rejected before any effects.
  - `remove_skill` revokes bindings across all scopes.
- **Reuse:** I found no second production implementation and no dead wiring; Rig and D3a packages are reused.

## Residual qualification limits

- No live or synthetic qualification results existed at the pin.
- Mutation testing has not been granted.
- No native-human or hosted checks; only aarch64-darwin has been exercised.
- The PDF/`pdftoppm` failure is carried over.
- D3a residuals remain disclosed.
- I did not check the receipt hashes or pass counts myself.

This verdict covers source acceptance only; it is not merge, owner or program authority.
