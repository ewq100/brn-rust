# One file, one Source: canonical local file locators (round 5 slice B)

Slice B (`02bd457b`, on top of slice A `e07354db`) makes a local file supplied under two path spellings (`/tmp` vs `/private/tmp`, or a symlinked folder) land in one chat and one Source. A new core helper, `canonical_locator`, turns an absolute path to an existing regular file into its `realpath` on Unix and returns every other locator unchanged; `same_locator` lets a Source stored under an older spelling still match. The runtime canonicalizes host-supplied references in `prepare_run`. `begin()` and full import look up the exact real path first, then `same_locator`, and record the real path on a legacy match as a new, version-checked revision. `source_intake_chat` and Keep in BRN match through `same_locator`.

Watch for: the legacy-spelling fallback has a test in `begin()` only. Full import's fallback and respelling, and both Keep in BRN lookups, could be reverted without any test failing (confirmed). A locator-only respell also counts as a meaningful input change, so it flags derived notes for refresh (confirmed). And every intake now resolves stored Source paths on the filesystem, including model-named ones (likely).

**Verdict**: NEEDS_CHANGES

## High-level view

Canonicalization happens at the service entry points (`prepare_run`, `import_full_scoped`, `source_intake_chat`, `keep_in_brn`), so neither host has a production change. The rule is narrow: an absolute path to an existing regular file, UTF-8 result, Unix only. `synthetic://`, URLs, relative, missing and directory locators pass through unchanged, and `same_locator` skips the filesystem unless both sides are absolute paths.

Legacy Sources are respelled in place, not merged. A Source saved under an old spelling keeps its ID and gets the real path as a new revision inside the checked start or full-import change, with `expected_version` set and history kept. Old duplicates stay as they are, and `source_intake_chat` counts all of them so they lead to one chat. The respell has two costs. The core treats a locator change as new evidence, so derived notes are flagged in `needs_refresh`. And coverage issues recorded under the old spelling lose their source scope.

Scope boundaries hold. `begin()` matches only within its `source_chat` scope, full import within its chat, and Keep in BRN among global Sources. `source_intake_chat` matches Sources across all scopes, as it did before, but reuses a chat only if it passes the unchanged project and open-chat filter. Equal `realpath`s mean the same file at lookup time, so nothing merges two distinct files. The edge cases are hard links, which stay separate (documented), and a retargeted symlink, which matches whatever it points to now.

The new filesystem dependency is broad. `source_intake_chat` resolves every stored absolute-path Source that is not an exact match, on every call, and `begin()` does the same within its scope whenever its exact match misses. Stored locators include Sources the model named.

The evidence holds where tests exist. Each of the seven integration and end-to-end tests failed at `e07354db` with a recorded assertion, four post-fix mutations tie the core checks to named tests, and the counts reconcile: core lib 35→39, app `source_intake_chat` 2→7, desktop 152→153, workspace 742→752. The gap is the legacy path outside `begin()`. Ownership is clean. Desktop changes are in `threads_intake_tests.rs` only, and there is no change to `main.rs`, `docs/status.md`, `docs/architecture`, the schema, leases or dirty guards.

<details>
<summary>Issues (6)</summary>

1. **Legacy fallback untested in full import and Keep in BRN** (blocking, confirmed). Add tests that seed a Source under an alias spelling through a checked owner change. Full import through the real path should reuse that Source ID, record the real path as a new revision with revision 1 kept, and report "Observed previously imported full note" when a prior full note exists. Keep in BRN, with a chat Source and a global Source that differ only in spelling, should create no second global Source. Record a fail-before for each by removing its `same_locator` fallback. Covering `begin()`'s report-bearing branch with a legacy Source, the branch both hosts use, would also help.
2. **Respell flags derived records for refresh** (non-blocking, confirmed). `meaningful_input_change` counts a locator-only change, so a respell lists derived notes in `needs_refresh`, and the desktop's "may need refresh" notice follows. Exclude a pure respell from meaningful changes, or record it as a limitation.
3. **Realpath fan-out over stored, model-named locators** (non-blocking, likely). Limit the `same_locator` fallback scan to host-recorded Sources (named in a Run's `sources`, or a full import's `import.source`), or record the limitation, including the risk of blocking on unreachable volumes.
4. **Retained coverage scope lost after a respell** (non-blocking, likely). In `retained_useful_report`'s scope recovery, also match an issue prefix whose canonical form equals a Source's locator, or record this as a limitation for notes written before the upgrade.
5. **Keep in BRN is not exact-first** (non-blocking, confirmed). Prefer an exact match before `same_locator` in both global lookups, as `begin()` and full import do (plan Decision 3).
6. **Unrecorded side effects** (non-blocking, confirmed). Add three limitations: the provider prompt now carries the resolved path, a stored symlink path matches its current target, and two spellings of one file in a single request are rejected.

</details>

<details>
<summary>Details</summary>

### Side effects of canonicalizing at `prepare_run`

Replay is unaffected. The start operation key is unique and not bound to the sources, full import still hashes `converted` as given, and Keep in BRN replays a prepared operation before it touches the filesystem. Two side effects are unrecorded (confirmed). First, the provider prompt now carries the resolved path. That can name a different location than the one the owner typed, for example the target of a symlinked folder. Second, if one request holds two spellings of one file, they collapse into one `BTreeMap` key and `begin()` rejects the run with its generic rejection. Both hosts build one-source requests today (`main.rs:851`, `threads_ui.rs:2982`).

### Legacy spellings: exact-first lookup and the respelling revision

```
begin(): exact (chat scope, real path) ──miss──▶ same_locator in chat scope
            │ hit                                   │ hit on old spelling
            ▼                                       ▼
   no write unless report             report: full Source Put (locator = real)
                                      no report: clone, locator + observed_at only,
                                                 expected_version, archived kept
```

The no-collision argument holds because `Store::records()` includes archived rows. The unique index is `(COALESCE(chat,''), locator)`, and both `begin()` and full import scope their exact lookup the same way. So an exact real-path Source is always chosen before a respell could collide with it.

The respell is a meaningful input change in the core (confirmed). For Sources, `meaningful_input_change` ignores only `observed_at` and `label`. A locator-only respell therefore puts every current note derived from an earlier revision of that Source into the receipt's `needs_refresh`. This fires on the first new-build supply or full import of a legacy-spelled file. The desktop formats any applied receipt with "N directly derived records may need refresh" (`threads_state.rs:152`), so a full import is likely to show that notice for a pure spelling change. CLI and desktop always send a report, so `begin()` already rewrote the Source on every re-supply. The respell is what turns that write into a meaningful change.

Retained coverage scope breaks after a respell (likely). `retained_useful_report` recovers an issue's source with `strip_prefix("{Source.locator} · ")` over current Source locators. An issue that an older build recorded as `/tmp/a · page 3` no longer matches once Source A says `/private/tmp/a`, so it falls back to the previous note's primary Source. Single-source hosts can reach this. A note built from A and later updated from B keeps A's issues under A's old spelling, with B as primary. If A is re-supplied (and respelled), A's omission is shown under B, and requalifying B later drops it. Only notes written before the upgrade with non-canonical paths are affected.

Keep in BRN does not prefer the exact match (confirmed). Both global lookups are a single `find(same_locator)` over records in ID order. With legacy duplicate global Sources for one file, the old-spelling one can win, and a global Source it creates copies the chat Source's spelling as is. It is the same file, so nothing is corrupted, but this departs from plan Decision 3: "every Source lookup prefers an exact match".

### Filesystem fan-out on stored locators

`source_intake_chat` applies `same_locator` to every Source record with no pre-filter, so each stored absolute-path locator that is not an exact match costs one `realpath` per intake. `begin()`'s fallback does the same within its scope whenever the exact match misses, which is every first supply of a file. The verification section records the cost. Two things go beyond cost (likely). First, the runtime lets a model name a Source (`threads_sources.rs:308`), so locators the host never received are now resolved automatically. Second, a stored path on a stalled network volume or a removed drive blocks the intake call inside `realpath`. `source_intake_chat` only needs Sources that some Run's `sources` names. Filtering to those, plus full-import `import.source` targets for `begin()` and full import, keeps the fix and drops model-named paths from the scan.

### What the tests do not reach

All of the following are confirmed:

- `full_import_of_one_file_through_two_spellings_observes_one_source` imports through the alias first. That first import already stores the real path, so the second import is an exact match. Removing full import's `.or_else(same_locator)` fallback, or its `source.locator = locator` respell, leaves the test green. That fallback is all that protects a full import of a file whose Source an older build stored under another spelling. Without it, the import creates a second Source and a second protected full note.
- No test exercises either `keep_in_brn` lookup with two spellings.
- `begin()`'s report-bearing branch with a legacy Source is untested, and it is the branch CLI and desktop use. The older-spelling test uses `TransientSource::text`, which has no report and so takes the new clone branch.
- `non_file_locators_are_unchanged` has no mutation that fails it. It is a guard, and its folder case uses an already-canonical path, so dropping `is_file` leaves it passing.

The manual CLI store at `/private/tmp/brn-dwr-data/r5-b-locator/` exists, with `alias`, `other`, `src` and `data`, which matches the recorded check.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/locator.rs`: new `canonical_locator`, `same_locator` and four unit tests.
- `crates/brn-threads-core/src/lib.rs`: module and re-exports.
- `crates/brn-ai/src/threads.rs`: `prepare_run` canonicalizes references; `begin()` does an exact-then-`same_locator` lookup and writes the no-report respell Put.
- `crates/brn-threads-app/src/intake.rs`: `source_intake_chat`, `import_full_scoped` and both `keep_in_brn` global lookups use the helpers.
- `crates/brn-threads-app/tests/source_intake_chat.rs`: five real-file tests.
- `crates/brn/tests/source_locator.rs`: new CLI two-spelling test.
- `crates/brn-desktop/src/threads_intake_tests.rs`: desktop two-spelling test.
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice B section.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show 02bd457b`.

</details>
