# Projects candidate evidence

**Scope/authority:** Owner requested implementation and confirmed the planned Projects feature on 14 October 2026. Implements stage two of the [accepted brief](../chats-and-projects/spec.md) using the [plan](plan.md). Release, real-data access and merge are unassigned.

**Baseline:** fetched `origin/main` at `85d291cddfa0f5b5a73e3d4f3b2b96c844445839`. Isolated `codex/projects-feature`; original checkout and unrelated experiments preserved.

**Candidate:** implementation commit `3569e6cb2ab8d05d552bcb022d0374749ac54581`, reviewed/tested tree `b53af0c9dfbf6dd1cddef10ab183a66dca31bc4b`. Subsequent evidence updates change documentation/receipts/screenshots only. macOS 26.5 (25F71), Rust 1.98.1, repository lockfile, `CARGO_TARGET_DIR=/tmp/brn-ui-repairs-target-20261010`; no dependency changes. Explicit canonical disposable data directories; synthetic fixtures; no live providers or embedding-model downloads.

| Gate | State |
|---|---|
| Defined / build ready | Satisfied: accepted stage-two decisions; owner confirmation; independent read-only plan review approved. |
| Implementation | Satisfied locally: checked core/app commands, context/retrieval/runtime, native project controls and CLI routes. |
| Automated verification | Satisfied: default workspace 539 passed / 3 subprocess-helper tests ignored; native widgets 144 passed; builds and strict lint passed. |
| Independent code review | APPROVED for candidate tree above against baseline; supported findings resolved. |
| Native observation | Representative synthetic create/settings/document Save/chat/delete/Trash Restore/restart observed; limits below. |
| Owner acceptance | Pending; concrete qualification below. |
| Hosted CI / merge | Pending and unassigned. No push, PR, merge or integration claim. |
| Release / real-data access | Unassigned. |

## Verification receipts

Exact commands, exit results and log SHA-256 hashes are in [receipts](checks/receipts.json). Tests are offline. The three ignored workspace tests are subprocess entrypoints invoked by their parent witnesses, not missing feature coverage. Native test count excludes two non-widget tests; the default workspace suite covers native-independent state.

| Command (common target directory above) | Result / retained log |
|---|---|
| `cargo test --workspace --locked --offline` | 539 passed, 0 failed, 3 helper entrypoints ignored. [Log](checks/workspace.log). |
| `cargo build --workspace --locked --offline` | Passed. [Log](checks/build.log). |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed. [Log](checks/clippy.log). |
| `cargo build -p brn-desktop --features native-ui --locked --offline` | Passed. [Log](checks/native-build.log). |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings` | Passed. [Log](checks/native-clippy.log). |
| `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline` | 144 passed, 0 failed. [Log](checks/native.log). |
| `python3 docs/work/active/projects/cli-witness.py /tmp/brn-ui-repairs-target-20261010/debug/brn` | 41 real CLI calls / 19 assertions passed. [Script](cli-witness.py), [receipt](checks/cli.log). |
| `cargo fmt --all -- --check`; `git diff --check`; `python3 scripts/check-markdown-links.py` | Passed on finalized evidence; links checked again after documentation completion. |

Eight Projects service journeys cover replay/key misuse, current membership, knowledge-off, shared identity, move/provenance/Undo, stale/dirty deletion, exact Trash conflicts, independent Keep/cancellation, retained source/figure access after unlink/deletion, restart/backup/restore and terminal Action privacy. Five runtime journeys cover real handlers/instructions/discussion filtering, live fencing and late completion, exact review after attention changes, preserved partial progress and model attempts to change context_epoch. Five actual native widget journeys cover dialogs/drafts, selected-project writing for shared documents, deletion/restore and displayed answer preservation with paused queued work.

The CLI witness creates projects/chats/private/general documents, shares the same Note, changes knowledge settings, moves a chat in/between/out, verifies scoped/general searches, previews/deletes/replays deletion, saves a later shared-document edit, restores relationships/settings without overwriting that edit, then backs up/restores/reopens into fresh storage. It uses only the CLI checked service, with no database writes outside it.

Native build/lint report the existing pinned `block 0.1.6` future-compatibility notice; current gates pass. Optional native-retrieval/model qualification and hosted cross-platform CI are not claimed.

## Native observation

The exact built native binary was copied into a temporary synthetic app bundle (`dev.brn.projects.candidate`) and opened with the explicit fresh `/private/tmp/brn-projects-native-zseb517j/data` directory. Through actual controls: create Japan planning; save “Use Estonian for beginners.” and knowledge off; create/save Trip itinerary; start Route discussion; inspect the concrete removal list; move to Trash; select Trash and Restore; reopen settings to verify retained instructions/off; quit and restart to verify the saved project/chat remain available. No prompt was sent. The synthetic preview remains available locally for owner review.

[Deletion preview](native-delete.png) and [restored project](native-restored.png) retain observed layouts. Document sharing/selected-project rewrite, unfinished Action choices and streaming fence cases have service/widget evidence, but were not exercised with live providers or in this representative native walkthrough. Owner daily-use, broad accessibility/IME and semantic model quality remain unqualified.

## Review corrections

Independent plan review approved scope. Whole-diff review found and verified fixes for schema-4 backup validation, attention-version coupling, moved-chat private provenance, cancelled/terminal Action retention, private unlink retention, shared-document writing project selection, Configure enforcement for context_epoch, source access after unlink and native public-answer preservation. Final review also corrected foreign scoped Actions entering a shared-document deletion preview and independently kept Actions re-entering a restored project's next preview. Regression assertions prove foreign obligations remain byte-for-byte unchanged and outside Keep choices stay outside. Final independent review returned APPROVED, with no remaining actionable requirement/invariant findings.

One earlier broad run was interrupted after discovering unindexed project-metadata scans; schema-5 partial indexes replaced those scans. Earlier zero-test filtered invocations are not evidence. The final default/native suites above qualify the corrected candidate.

## Owner qualification

1. Create a project; set instructions and turn BRN knowledge off. Create/link/import a document and start two project chats. Confirm earlier discussion remains distinguishable from accepted documents/decisions.
2. Move an existing chat between projects and back to standalone. Confirm its draft/history survive, active work stops and queued text stays paused.
3. Share one private document with a second project. Open it there and create a writing conversation: it uses that project's instructions. Save/review/Undo and see the same record through both links. Unlink the original project without promoting or copying it.
4. Delete a project with an exclusive document, a shared document and unfinished tasks. Check the removal/retention list and choose Keep outside or Cancel for each task. Restore: shared edits persist, cancelled/outside tasks remain cancelled/outside and work stays paused.

Next consequential gate: owner acceptance of this local candidate, followed by separately authorized publication/CI/integration. Current implementation authorization does not assign release, owner-data adoption or merge.
