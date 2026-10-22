# Independent complete recovery-copy privacy candidate review

## Disposition and exact scope

**Changes required before qualification: two P2 test-validity/effectiveness findings below. No independent production privacy defect found in the selected implementation.** Reviewer authored neither source, test, nor adapter. No tests/builds/workloads/native/account/network/cache actions or source/Git edits were performed; this report is the sole write. Corrected source needs an affected independent review refresh and lead-owned qualification before approval/integration.

Verified clean branch `codex/recovery-copy-privacy` in architecture-status-route, exact candidate `049819a4ae77494dc5da2b0f34e07c3fd1525ca7`, verified merge base actual main211 `aded398a0c1960d1feabe48166284ec0824ee4e1`. Read **all four actual changed files/diff**, surrounding service/replay/editor/Apply code, author report and saved before-source log/completion. Complete task diff is 718 insertions/4 deletions, SHA-256 `04972c36f8c9170e6adf83c4cedb4190895a4c0ec20addfe9f4671d4450da4c5`, matching the saved author patch despite the reconciled CI-only ancestry.

Whole final file hashes:

| File | SHA-256 |
| --- | --- |
| desktop `threads_polish_tests.rs` | `81768f47b5165b30bb6d1c8b5822afa4b7beb93659555c379069c22ee5c5f726` |
| desktop `threads_ui.rs` | `4442ae0032cee811c50c28302eefafd51beff825aa8a51cb3540bd68ca130d44` |
| app `writing.rs` | `816e51279483226e4ef17ddda6af4632e37528719398d87fdc0ed06d454698f7` |
| app `tests/writing_context.rs` | `d2ad4d22306ca6b7691d16f9a364fe2751a6dec247d8f78e40d5c3df5dc3aac0` |

## Findings requiring correction

### P2 — Recovered-private-basis UI fixture violates the existing dirty guard

Location: `crates/brn-desktop/src/threads_polish_tests.rs:812–847`, in `stale_recovery_from_private_basis_is_refused_in_general_scope`.

The fixture calls `begin_edit` and `update_buffer` on the private Note, then attempts an owner change on that **same guarded Note** to clear its chat scope. Core Apply discovers an open edit session for every written Note (`store.rs:1966–1978`) and returns `ApplyOutcome::Deferred` when guarded (`:2328–2329`). Owner authority does not bypass this guard. Therefore the fixture's `Applied` assertion at line 847 cannot hold, and it never reaches the intended newer-public-Record/older-private-session SaveCopy boundary. This is a deterministic source finding; no failed execution was manufactured or run to prove it.

Small correction: perform the checked promotion while no edit guard is open, then call the existing `begin_edit(note, private_basis.version)` and update its buffer. `edit.rs:169` reads that exact immutable historical revision and intentionally supports stale bases. Recover/open that session normally, retaining the explicit assertions that the displayed Record is newer/public while the session basis is older/private. Do not bypass dirty guards, rewrite SQL, or weaken the `Applied` precondition.

### P2 — Actual emitted Current fence and privacy-before-replay boundary lack the available consequential witness

Location: `crates/brn-threads-app/tests/writing_context.rs:317–397`; author report's request-inspection limitation.

The new public-stale case asserts exact copy text and `needs_refresh` after a later source edit, but that derivation assertion still passes if the service emits **only Historical provenance and omits the Current input**. The other new tests make privacy changes before the service call, not after preparation. They therefore cannot expose a regression that leaks the old general snapshot through an Apply after the source becomes private. This is the consequential race the selected service must guard. The current implementation contains the right Current input, but the available cheap boundary evidence is missing.

The claimed lack of a public inspection seam is incorrect: `Store::prepared(&receipt.operation)` remains public and available after Apply (`store.rs:611`, `1513–1528`). Existing app production code itself inspects an applied operation this way. Extend the existing public-stale case rather than adding a hook/framework or duplicate policy test:

- obtain the **actual emitted request**, assert Historical at the old basis and Current at the observed newer source version;
- clone that actual request under a fresh operation, updating its new creation ID; prepare through existing owner preparation;
- change the source to private through an independent checked connection before Apply;
- assert actual Apply is Stale, with no new Note or accepted receipt for that pending copy and unchanged private source.

This uses the real request plus existing checked seams, and would fail if Current were removed or incorrectly bound. It tests Apply refusal, not a manually authored approximation of the new service. Update the author report accordingly.

Strengthen the **same existing completed-replay case** by reopening Workspace and retrying the original unchanged key while the source is now private. The current test advances only public markdown and stays on the same Workspace, so it does not distinguish replay before privacy eligibility from replay after it, nor witness a lost-response/reopen boundary. Source order is currently correct; the stronger case verifies the intended durable contract without new replay machinery. Preserve changed-intent rejection and exact original copy/receipt identity.

## Production assessment

The chosen boundary fits the accepted general-only slice. App `save_recovery_copy` reuses existing `writing_operation`, `writing_replay`, and `writing_commit`; it adds no core persistence/publication framework. Intent binding includes source identity, actual basis, captured scope, title, and exact text. A completed receipt or immutable prepared request is replayed before new mutable eligibility checks. Existing helper errors propagate; unprepared operations are distinguished from malformed/error state. Changing intent under the same host key fails through the existing bound allocation.

New operations reject captured scoped context, unavailable/archived/superseded/non-Note historical or current records, and private historical/current Note scope. General visibility checks add fail-closed admission; the Current source version closes races between separate reads and Apply. A general Note has scope-independent general visibility here, so accepting a general source does not silently implement guessed project-copy membership. Private duplication remains explicitly unavailable. A public v1 edit can copy while saved v2 is still general: Historical binds v1, Current binds observed v2. No equality between edit basis and current source is imposed.

Core Apply, unchanged by this candidate, rechecks Current inputs inside its SQLite writer transaction before effects, and validates historical revision existence. Source changes after observation cannot pass the stored current version. Exact input expectations survive immutable preparation and derivation recording. Source metadata/import/history, edit sessions, and accepted state are not rewritten by the copy. A refusal may allocate a bound operation, but creates no accepted copy receipt or Note; that bookkeeping is not a claim of zero SQL activity.

The same-version public path is source-compatible: Historical and Current may name the same ID/version, existing validation permits both expectations, and derivation storage has no uniqueness constraint that would reject their rows. Existing core current-input coverage already includes both expectations for one ID/version. A short same-version positive case would be useful if the corrected app case is already being extended, but this is not a separate production finding or a reason for another abstraction.

Desktop now obtains the owned `EditSession.base_version`, rather than the latest displayed Record version. That is the correct source of a recovered buffer's basis. The key derives from session/generation/scope and the app separately hashes exact intent; repeated same-key altered content cannot silently overwrite an immutable copy. No new lifecycle field, per-keystroke DB poll, or privacy bool owned by the UI was added. Refusal only reports failure and notifies; removal of refresh/reload preserves visible text/selection/context and the retained buffer. Success normally opens the receipt's target through existing current context admission and refresh behavior.

Replay-target notice ordering was checked: the generic saved-copy message is set **before** `open_note`, whose unavailable/archived target admission overwrites it with a warning. It does not overwrite that later warning with a new-success claim. Replaying an accepted copy whose target is later unavailable creates no second copy. No actionable notice defect found in the inspected route; this ordering is source-reviewed, not a separately executed UI result.

## Test effectiveness, baseline, and remaining gates

The retained initial baseline at exact `2ff42451adf97268884afd3a0b91c3ec97922e3f` actually compiled desktop and ran the selected real-input/click test: **0 passed/1 failed, child exit 101**, at `private recovery copy entered general Notes`. The receipt records `baseline_reproduced=true`, `qualified=false`, unchanged checkout, leader reaped/group absent, and no cleanup errors. Earlier original-source/session assertions were reached and passed; later search/export assertions were not reached. This establishes that one original privacy defect, not every newly added boundary or native behavior.

The strengthened first UI witness now checks private refusal text, unchanged visible draft/selection/scope, exact kept session, canonical source and no accepted receipt, plus general records/search/export exclusion. That is useful final coverage distinct from the recovered historical-base case, once the latter's setup is corrected. Existing public stale-copy positive control is unchanged. App tests cover scoped/current/historical privacy, exact Unicode text, generic unavailable source cases and changed intent. The archived/superseded examples exercise the **current** source while using an earlier basis; do not report them as executed archived/superseded historical-basis cases.

Test setup volume is mostly explicit checked fixture state, not new production layering. Two actual UI transitions plus shared service tests are proportionate for privacy and durable replay, but run the corrected meaningful boundaries rather than accepting a count of tests. No test result is claimed for `049819a4`; author reports formatting/diff only. After correction, refresh affected review, run focused app/UI witnesses first, then the lead's appropriate existing core/current-input, writing/app/desktop, strict lint/fmt and ordinary native-build gates. No provider/account/native launch is required to qualify this selected deterministic boundary. The failing test-only baseline and all actual failures remain retained; qualification and normal integration are still pending.
