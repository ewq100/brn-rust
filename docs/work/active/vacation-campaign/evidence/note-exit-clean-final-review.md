# Note exit preservation — complete final source review

**APPROVED: `174e381f29518f3ec903a5c166b13a87ac33fcfa`. No remaining actionable source findings.** The clean-checkpoint false refusal is repaired within the Note owner. Existing exact-source local receipts also pass; this disposition does not establish native termination, owner acceptance, hosted checks or integration.

## Identity, independence and review scope

Reviewed `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/note-exit-preservation-20261022`, clean HEAD `174e381f29518f3ec903a5c166b13a87ac33fcfa`, tree `be2b8858f672e4dcb96fab2a16c380d39d58cd7f`, against verified merge base `5c606c102d5d121105896c48dc7b39e78d26bdb3`. Complete task diff: six files, 678 insertions / 37 deletions. No unrelated dirty or untracked files were present.

I authored no candidate source. Reviewed the actual complete six-file scope and affected ownership/callers, reusing the complete `155bb` review in `/tmp/brn-note-exit-candidate-review.md` and the `660a` supported selection correction in `/tmp/brn-note-exit-ubuntu-setup-review.md` only after checking actual source equivalence. Read the entire `660a..174` delta, the complete new focused journey and surrounding Note lifecycle, original exit hooks, immutable revision reader/writer and both contracts. The desktop production file and native export are byte-identical to `660a`; the complete existing test source from the U4 journey onward is byte-identical. The only four-file delta is the new 150-line focused test, narrow Note-owner proof and two contract paragraph updates. I ran no build/test/fixture/runtime/native/account/network action; this report is my sole write.

## Requirement and correctness assessment

The prior clean-note trigger is real: ordinary refresh can retain an unchanged accepted vN editor while another checked Workspace Saves different vN+1. Current canonical/open recovery no longer match the old visible bytes, while the correct clean-text fast path does not create an edit. Requiring another Save or a discard consequence was therefore a false refusal.

`crates/brn-writing-ui/src/note.rs:88–121` first retains the existing current-canonical/open-recovery proof. Only if that does not prove durability does the new branch require all three conditions: lifecycle `EditState::Clean`; live text exactly equal to the held Note body; and the exact SQLite revision selected by the held ID/version equal to the **entire held Record**. Whole-record equality binds identity, version, archival state, Note metadata and content. Cached body equality alone is insufficient. A missing or mismatched revision falls through to the existing checked preservation/refusal path; SQL/decoding errors propagate through `?`.

This is a direct existing indexed read (`Store::revision`, `store.rs:632–643`), not a history scan. Checked writes insert current Record and immutable revision in the same transaction (`store.rs:2634–2645`); the new path neither writes nor adopts anything. The accepted old checkpoint remains historical evidence, never current authority. There is no reload, new guard, recovered-session substitution or permission to overwrite vN+1. Normal accepted-state immutability also avoids a mutable generation race in this proof.

The restriction to unchanged Clean is appropriate and economical. Dirty owned sessions, blocked first-dirty input, closed unsaved buffers and newer conflicting recovery do not gain historical admission. They retain the actual canonical/open-recovery proof, checked write/recheck or explicit local-discard route. No new public selector, lifecycle field, persistence mechanism or generalized exit framework is needed. Both contract paragraphs accurately name this additional narrow proof and keep Note-only/forced-termination limits explicit.

## Complete-candidate ownership and existing safeguards

The unchanged shared desktop installer is used by launch and the fixtures. Close and the registered deferred Quit action call the same preservation admission before continuation discard, worker cancellation and conversion cancellation. The previously valid P1 synchronous Quit/leased-window bypass remains fixed by App defer; the new checkpoint branch does not affect that ordering. Confirmed local discard still captures opaque Note intent and rechecks it immediately before deferred teardown. Cancel and Enter remain safe, changed intent refuses the stale confirmation, and no Store discard or bypass flag modifies another saved/recovery state.

The exported `NoteExitSnapshot` remains opaque to hosts. The prior PR209 non-explicit replacement generation/closed-session guard and validated same-Note Recover route remain intact; exit proof deliberately does not replace that navigation policy. The actual supported editor-range setup from `660a` remains unchanged, followed by the same real single-Unicode input. No provider, schema, dependency, privacy, runtime prompt or broader draft-retention changes are present.

## Test effectiveness and proportionality

The separate focused test at `threads_writing_tests.rs:1786–1932` is justified by a distinct Clean boundary and reuses the existing offline fixture and actual production installer. It does not duplicate a second framework or expand the already long U4 phase sequence.

Before Close, a second checked Workspace actually begins/updates/Saves different text and closes its session. The test proves the held original Record/editor, no owned or blocked state, different current Record in both refreshed UI records and SQLite, exact old immutable revision, absence of target open recovery and unchanged captured history. The installed predicate therefore cannot pass merely because current or open-recovery bytes happen to match. After admission it checks no consequence dialog, unchanged held/editor/session state, exact newer canonical and older revision, unchanged external closed session, no open target recovery and unchanged history; reopening independently checks canonical/revision/history again. It tests actual boundary behavior rather than mirroring an internal boolean.

Independently read and hash-verified the meaningful before-source baseline receipt/log: test-only HEAD `7a1a530f11ed7c06331b4439f4e92b2086b641be`, exit 101, actual 0 passed / 1 failed at the installed Close assertion `1870`, after the setup assertions. Log SHA256 `acb0a58b1b983122b33443ae1e3f47a8afc92c897d822a190c66c70c83e2df57`; source unchanged, leader reaped, group absent, no cleanup errors, 50.24s. This is a meaningful regression witness. Earlier compile/input/Cmd-Q/Ubuntu setup failures remain distinct historical evidence, not this baseline.

The existing full U4 remains the complementary negative/positive witness for live-only refused input, Close and registered Quit, Cancel/Enter, exact recovery admission, externally Saved closed cached session, newer recovery conflict, stale confirmation and explicit local-only teardown. Its bytes and assertions were not relaxed. No additional corrupt-database fixture or mirror test is warranted for this small direct read.

## Saved exact-source qualification inspected

Read existing `checkpoint-final-*` receipts and logs under `/tmp/brn-note-exit-qualification`; recomputed every raw log hash below. All five receipts identify exact HEAD/tree above, exit 0, unchanged source, reaped leader, absent group and no cleanup errors. Desktop includes actual compilation and both named journeys explicitly `ok`.

| Stage | Actual saved result | Log SHA256 |
| --- | --- | --- |
| writing-ui | 44 unit + 12 host tests PASS; 14.07s | `e3675004acee6e2d1731f7cfb5796d71c5a11914e4b948905384c28813dbc774` |
| separate writing-host | 12 PASS; 2.02s | `8980b3d6d4343f4e5d5922f7baf8dfa71c57ed0f54d9643162b987e71faf14ed` |
| desktop | 351 PASS / 0 FAIL / 1 existing ignored; focused Clean and U4 both PASS; 56.25s | `2ab1869b82ad76a746334785c0d64e09403c747c9095781e2b200b029c7b1a67` |
| combined all-target strict Clippy | writing-ui + desktop, `-D warnings`, PASS; 8.04s | `74de64582725a02c427c70324fd111e4b87fed1a75d66daf7c72845ca5af9c47` |
| ordinary shipping native-ui build | PASS, no launch; 8.03s | `7af993a9a0b5f11a303861085cfdf2e83775ba60ce8b27ecc37a1ab4c175910f` |

These are lead-owned locked/offline jobs-1 checks, independently inspected rather than rerun by the reviewer. They do not retroactively change previous failed results. Exact-head hosted/normal protected integration gates remain separate.

## Final source fingerprints and limits

- `threads_ui.rs`: `c650007d84392078af847e3009e878258d209c00c23b7de9f708a43b0d1c727c`
- `threads_writing_tests.rs`: `b4cbb3aa5a12027c9003c23c6c8b5b8256b87d4297b1a2347b5feaa26cb94ea7`
- writing-ui README: `d3b7b1e6890a9214777246cf702d50379a40ac9e39928d592cea125522a443bc`
- `native.rs`: `956ce480c7f47ffaa8c06b173acebe92b24660dc7fdb720e9f6ac214fb8d404e`
- `note.rs`: `26adacda1fec957c02e53f0bac9d1d6db584b2abd0d59a28bb16457a947e316a`
- behavior contract: `1461ae520a7318744fa196c9c9a3d26af7402ae2c81492b932b2eb2fc3d73086`

`simulate_close` executes the installed predicate without native window destruction. TestPlatform Quit is a no-op; registered action/dialog/source ordering and headless removal are not physical Cocoa termination or an active-provider cancellation observation. Native owner acceptance, forced termination and non-Note drafts remain outside this evidence. No remaining source blocker was found; normal exact-head checks and integration remain lead-owned.
