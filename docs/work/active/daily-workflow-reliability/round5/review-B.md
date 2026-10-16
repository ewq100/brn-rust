# One file, one Source: review round for canonical local file locators (round 5 slice B, pass 2)

Slice B (`02bd457b`) records a local file Source at its real path and still matches Sources saved under older spellings. The review-round commit `28c4a757` answers the six pass-1 findings, kept in [review-B-pass1.md](review-B-pass1.md). Two new core helpers carry the lookup rule: `find_source` (exact spelling first, then an older spelling of a host-recorded Source, both inside the caller's scope) and `matching_sources` (every such Source, any scope). The core's apply no longer flags derived records for a pure respell, and retained coverage canonicalizes an issue's source prefix when no exact prefix matches.

Watch for: two limitation sentences, "Paths that only a model named are never resolved" and "A Source only a model named, under an older spelling of a file, is not matched", overstate the B-R3 fix. A model can put its own Source into the candidate set by naming it in a Note's `import.source` (likely). `respelled_source` also runs `realpath` on both locators of any locator-only Source edit, a model's included, inside apply's write transaction (likely). Neither blocks, and both fixes are limitation wording.

**Verdict**: APPROVED

## High-level view

All six pass-1 findings are addressed. Full import, both Keep in BRN lookups and `begin()`'s report-bearing branch have older-spelling tests that fail when their fallback is removed (B-R1). Keep in BRN prefers the exact real path and creates global Sources at the real path (B-R5). A pure respell leaves `needs_refresh` empty, while a locator change to another file still fills it (B-R2). Retained omissions keep their source scope after a respell (B-R4), and the side effects are recorded (B-R6). The fallback scan resolves only host-recorded Sources (B-R3), with the gap in Watch for.

`find_source` collects candidate IDs from all records but applies the caller's scope to both steps. So `begin()` still matches only within `source_chat`, full import within its chat, and Keep in BRN among global Sources. `matching_sources` crosses scopes as `source_intake_chat` always did, chat reuse still passes the unchanged project and open-chat filter, and its result is unchanged because `source_intake_chat` only uses Sources a Run names. Only equal real paths match. `synthetic://` and URL locators skip the filesystem, since the candidate set is empty for a non-absolute canonical locator and `same_locator` needs both sides absolute.

Every respell is a Put inside the existing checked start or full-import change, with `expected_version`, and history keeps the old spelling. Apply computes `needs_refresh` once and stores it in the receipt, so the new filesystem read cannot change a replay. An Undo back to the old spelling also counts as a respell. No lease, dirty-guard, candidate-review, schema or `Cargo.lock` code changed.

The evidence reconciles. Workspace 752 → 759 matches the seven new tests (core lib +1, core `core` +1, app `source_intake_chat` 7 → 11, brn-ai lib +1). Desktop stays at 153 because the review round changed no desktop file. The recorded mutation outputs match the code. The figure-lookup mutation gives `[<new global>, <new global>]` because `keep_both` keeps the figure note first, and the attachment then finds that new real-path global Source exactly. The commit touches no desktop file, `main.rs`, `docs/status.md`, `docs/architecture`, `upgrade.rs` or `Cargo.lock`.

<details>
<summary>Issues (3)</summary>

1. **A model can make its own Source a fallback candidate** (non-blocking, likely). `older_spelling_candidates` trusts any `Note.import.source`, and a model's `prepare_changes` can create a Note whose import names a Source the model created. Reword the two limitations in verification.md to say a model can still bring a Source it named into the older-spelling match this way.
2. **`respelled_source` reads the filesystem inside apply's write transaction** (non-blocking, likely). It runs `canonical_locator` on both locators of any Source whose only meaningful change is its locator, a model's edit to a Source it named included. Add to the limitations that a stalled path there holds the shared store's write lock for every host, and that a model's locator edit can trigger it.
3. **Longest-prefix choice in the retained-scope fallback is untested** (non-blocking, confirmed). The B-R4 test's issue locator has one `" · "` separator, so replacing `.last()` with `.next()` passes the suite. Add a respelled issue locator with a second separator, or accept the gap.

</details>

<details>
<summary>Details</summary>

### How a model-named Source reaches the fallback

`older_spelling_candidates` treats a Source as host-recorded if a Run's `sources`, a Note's `import.source` or an Asset's `source` names it. Models cannot write Runs or Assets, because `prepare_changes` validation rejects `Run`, `Settings`, `Asset`, `Message` and `ChatControl` writes. A Note with `import` passes that validation (likely). `materialize_selected_sources` rewrites `note.import` only when a host-supplied source is selected, so in a Run without one the model's `import` survives. The core checks only that `import.source` names a Source record, and its full-import rule covers only `full_note: true`.

A model can therefore create a Source under some absolute path, plus a Note whose non-full import names it, and that Source counts as host-recorded. `source_intake_chat` and the fallback steps then run `realpath` on that path. And if the path is another spelling of a file the host later supplies in the same scope, `begin()` adopts the model's Source as the host's and respells it. An exact-spelling model Source was already adopted this way before the slice, so the impact is small. The two limitation sentences still claim the opposite.

Restricting candidates to `full_note` imports would break a real case. A useful note kept in BRN names the global Source that Keep in BRN created through a non-full import, and no Run names that Source. That is why the action is wording.

### `respelled_source` in the core's apply

`respelled_source` runs in `apply_on`'s refresh loop, inside the open write transaction, for every written Source that `meaningful_input_change` flags. It calls `canonical_locator` on both locators only once nothing but the locator, label and `observed_at` changed. On the host respell path the new spelling was just resolved by `prepare_run` and the old one names the same file, so the risk sits elsewhere (likely). A model may change the locator of a Source it named, since `materialize_selected_sources` blocks locator edits only for host-observed Sources, and an owner edit may change a locator too. If either path sits on a stalled network volume, `realpath` blocks with the write transaction open, and desktop, CLI and agent writes wait behind it. The current limitation says apply reads the filesystem, not that it holds the write lock while doing so.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/locator.rs`: `older_spelling_candidates`, `find_source`, `matching_sources` and a host-recorded unit test.
- `crates/brn-threads-core/src/lib.rs`: new re-exports.
- `crates/brn-threads-core/src/store.rs`: `respelled_source` skips refresh for a pure respell.
- `crates/brn-threads-core/tests/core.rs`: same-file versus other-file respell refresh test.
- `crates/brn-ai/src/threads.rs`: `begin()` uses `find_source`; retained-omission respell test.
- `crates/brn-ai/src/threads_sources.rs`: canonical issue-prefix fallback in `retained_useful_report`.
- `crates/brn-threads-app/src/intake.rs`: `source_intake_chat`, full import and both Keep in BRN lookups use the helpers; new global Sources record the real path.
- `crates/brn-threads-app/tests/source_intake_chat.rs`: four older-build tests (report-bearing supply, full import, two Keep in BRN cases).
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: review-round notes, tables and limitations.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show 28c4a757` (whole slice: `diff e07354db..28c4a757`).

</details>
