# B1a panel source candidate — awaiting compile allocation

Source-only preparation on `work/b1a-cw2-20261016`, starting from clean exact `df5e48e38549ad19a70daeaf47f9b6b185ef6d0e` in `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`. HEAD remains unchanged. The [approved helper move](helper-move.md) and [CW2 disposition](../cw2/principal-disposition.md) remain the foundation. The current principal allocation supersedes the historical unallocated wording in [panel-scope](panel-scope.md).

## Actual source preparation

- `brn-writing-ui/src/native.rs` owns the plain WritingPanel, its existing eight editor/view entities and local Note/comment/review/draft/layout state. A short-lived WritingView borrows the host's real Workspace and checked display maps; concrete WritingRequest values call a function pointer on the existing host Context. No panel entity event queue, host-operation trait, new runtime or second Workspace.
- `writing.rs` owns the actual Note/comments/review/Compare render bodies, existing disclosure/early refusal and exact review controls. `document.rs` owns Read/format controls, real editor formatting/selection/decorations and the checked begin_edit/update_buffer operation. `workbench.rs` owns the current status/proposal/result queries and rendering. Old desktop document/workbench files are deleted; calls use the library. The original chat/composer and linked-tile implementation remains desktop, with panel-field forwarding.
- One shared markdown policy, Palette/CURRENT and the allocated generic components move to the library; desktop imports/re-exports those implementations. Pure document fit values live in the default, toolkit-free layout module; host Layout/TileKind/panel widths remain desktop. Existing outline/unlocated/exact-location helpers move once and their current mixed-state tests remain desktop.
- Small necessary transitive closure: existing `row_text`, `pill`, `scroll`, `terminal_notice`, typography/title/short and the current Compare-open-edit dialog presentation move once. Desktop's full section header/list/forms remain desktop; the panel uses the existing no-count/no-trailing section appearance. No additional framework or product behavior is introduced.
- Host owns subscriptions and their original order. First InputEvent::Change synchronously calls the moved `note_changed(&mut Workspace, text)` before derived Read/selection/notify. The existing typed EditSessionOpen outcome/notice stays in the host. Save/comment/run/Apply/Dismiss/receipt Undo and context/runtime/worker/queue handlers stay single existing checked handlers. Host still coordinates root/tile focus before editor focus.
- CW2 cache stays host-owned, including its complete token/Note/scope/conversation/exact Vec<Prepared> key, clear-before-reload/error behavior and before/after token check. Shell render still refreshes it first, once. The panel borrows checked maps and obtains caller-scoped image maps only at consuming render sites; absent checked message scope resolves empty. Display data does not authorize fresh Apply/read. No selection fallback is added.

The three approved helper Rust files are unchanged except `lib.rs` gains real module gates/exports/documentation. Formatting and diff source bytes remain exact. No AI/core/app, shared framework, CI/script, root manifest or Cargo.lock change is included. Desktop README changes only its affected implementation links.

## Test source preservation

Existing 30 formatting +4 diff tests remain unchanged. Ten existing pure test bodies/portions relocate: theme3, markdown2, workbench3 and layout2. The two layout tests split their pure threshold assertions into the library and keep all host Layout assertions in desktop, retaining their original names. No assertion is duplicated or dropped to manufacture a pass. The native library now contains44 test annotations;36 are in the default source graph and8 require native-ui. These are static source counts, not executed results.

Desktop native test edits are field/API forwarding and necessary imports/formatting. Existing assertions, real clicks/input, fixtures and the two reviewed public reduce-motion calls survive. CW2's privacy/context/access/cache/real-typing witness preserves all original assertions and events. No cache-load or scope assertion is weakened.

One new F15 journey in `threads_document_tests.rs`, `local_editor_undo_after_apply_and_refresh_cannot_resurrect_or_save_stale_text`, first proves real local editor Undo on real simulated input. It saves owner typing, prepares an actual checked rewrite candidate, clicks Apply, then presses Cmd-Z and attempts Cmd-S. It repeats after a real saved-record change and `reload_open_note`. It asserts exact saved/editor text, absent dirty session, unchanged saved revision and unchanged receipt count after attempted Save. No durable receipt Undo is substituted. This test is unexecuted; no failing-before/corrected result is claimed. The pinned editor's existing set_value path clears its undo manager; this slice does not change that policy or add a guard fix.

## Static results and identity

Direct repository-pinned `rustup run 1.98.1 rustfmt --edition 2024` parsed/formatted the changed source; direct `--check` and `git diff --check` are the source-only checks. Scratch `/private/tmp/brn-b1a-panel-20261016/` retains the starting tracked-file hash inventory, original sources, preparation scripts, static audits, final source manifest and frozen patch. The final source fingerprint is recorded below after freezing. No Git staging or commit has occurred.

No Cargo, build, lint/type check, tests, executable reuse, mutations, stress, GUI, provider/model access, slot polling or delegation ran in this source window. D1A retains the sole heavy slot. This is a compile-window candidate, not B1a source acceptance/completion, hosted CI, human native observation or owner acceptance. Existing PDF native-pdftoppm gap, widget-B original-cause uncertainty and CW2 global qualification limits remain open.

## Manifest/lock and next serialized checks

The allocated new-crate manifest adds the actual local brn-threads-app dependency. GPUI retains existing exact0.7.3, optional only under native-ui. `native-test-support = ["native-ui", "gpui-kit/test-support"]`; desktop's existing native-test-support additionally forwards `brn-writing-ui/native-test-support`. No new external crate/version, direct AI dependency or root workspace member change. Anticipated lock delta is only the existing local brn-writing-ui package adding its brn-threads-app edge. Cargo.lock is untouched; principal serializes reconciliation and actual delta review before locked commands.

After explicit slot/lock allocation, use repository Rust1.98.1, task target `/private/tmp/brn-b1a-20261016-target`, fresh explicit synthetic TMPDIR and `CARGO_INCREMENTAL=0`, retaining exact source/command/log/artifact identities. Inspect precise running heavy processes once at allocation; do not reuse shared C0 artifacts. Required commands to schedule (not executed here):

```sh
cargo test -p brn-writing-ui --lib --features native-ui,native-test-support --locked --offline
cargo clippy -p brn-writing-ui --all-targets --features native-ui,native-test-support --locked --offline -- -D warnings
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop local_editor_undo_after_apply_and_refresh_cannot_resurrect_or_save_stale_text --locked --offline
bash scripts/verify-desktop-shell.sh
cargo fmt --all -- --check
cargo tree --workspace --edges normal --locked --offline
bash scripts/verify-storage.sh
bash scripts/verify-end-to-end.sh --fixtures-only
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets --locked --offline -- -D warnings
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
```

Full desktop widgets must retain the A/B/CW2 witnesses; report actual relocated/split test accounting instead of assuming the old suite count. Default tree/build/lint/tests must demonstrate F22 with zero GPUI. Existing helper and affected service/headless lanes run once at a coherent source pin, with real prerequisite helper/Poppler identities and the carried PDF gap reported honestly; no wrappers/skips to simulate success. No new mutants for unchanged guards; any actual guard correction first needs failing-before evidence and a principal-bounded affected mutation allocation.

Principal owns CI/script edits: the first two commands above are the exact additional native library lane. Building desktop as a consumer does not execute moved library tests. Principal also owns fresh independent Opus actual-diff review and final source/check acceptance. No commit is proposed as verified until those gates have evidence.

## Frozen source scheduling identity

Production source manifest SHA-256: `37a10fd99f26e1e07fff6a9927e710f429230a2e5e36ace4f6959f96cc23f748` (`34` production paths). Frozen patch SHA-256: `40728a06e22360acc6d8972d2d6774ee9c1da1c5fc5162947fe1c573dcb0cc46`. Machine receipt: `/private/tmp/brn-b1a-panel-20261016/source-candidate.json`; that receipt records each original/current file hash and exact successful static command/log identities. This fingerprint excludes prose so documenting it does not alter the source pin. It is an uncompiled WIP identity, not a commit or accepted build.

## Bounded deferred-display correction after principal inspection

The first fingerprint `37a10fd9…` remains the historical uncompiled source candidate above. Principal identified an introduced eager-construction regression: every `writing_view()` created scoped asset maps for every checked conversation and the Note, scanned receipts for latest Run, evaluated Edit-note/Stop display policy and cloned progress/run identity. Cheap predicates and per-candidate identity filters paid for unrelated display work.

The correction changes only native.rs, writing.rs, workbench.rs, desktop threads_ui.rs and threads_chat.rs. WritingView now borrows progress and worker Run ID. It borrows the concrete host plus pointers to the existing scoped asset resolver, Edit-note/Stop presentation queries and original `state::latest_run`; construction invokes none of these. Their implementations remain single and unchanged. The same Note-to-chat image-scope derivation is single in WritingPanel and reused by desktop `note_assets`.

Assets resolve only when Read, Compare, saved-version or actual checked-message rendering asks for them. Message resolution retains the exact previous checked-request-conversation membership gate; absent/None scope yields an empty map without calling the resolver. Note assets retain the exact original Note.chat/record_in_chat policy. No all-assets map, second cache, generic host-operation trait or copied latest-run algorithm. Latest Run is looked up only by the existing status/result consumers; Edit-note/Stop policy and progress text ownership are likewise deferred to their original consuming controls. The unused view-only note-editor-visibility wrapper/field is removed, so construction also avoids Layout::visible() allocating a tile vector; the real host editor guard still calls the single panel guard at its original action/subscription sites. Pure comment/toolbar/state/identity adapters now only assemble borrowed references and read the constant-time maximized-tile fact before doing their own original query work.

No existing test source, IDs, labels, original assertions/events, F15 test or CW2 cache/authority logic changes in this correction. Before/after path hashes and the five-file correction diff are retained in scratch. Source-only direct pinned rustfmt and static diff/provenance checks remain the only qualification.

When compilation is allocated, add one focused behavioral work-bound witness using the existing native fixture and checked-display/cache witness: instrument actual scoped asset resolutions and deferred latest-run reads in test support, repeat cheap comment/toolbar/identity queries and per-candidate filtering, and assert no unrelated display loads. Then exercise actual checked Read/Compare/message consumers and retain existing scope/cache assertions. Measure invoked work, not constructor fields or a broad timing benchmark. This witness is planned, not written/executed in the no-tests window. Compiler qualification of the borrowed identity/progress and concrete function-pointer lifetime/coercions is also pending.

Principal separately supplied pending `.github/workflows/ci.yml` and `scripts/verify-desktop-shell.sh` hunks for explicit native writing-library build/lint/test. This worker did not write or execute either path. Their exact deltas/hashes are recorded separately in the revised material-source receipt and await principal inclusion approval. Cargo.lock/root manifest remain untouched. No Cargo/tests, staging/commit or slot polling occurred; D1A retains the sole heavy slot.

### Revised source scheduling identity

Owned source fingerprint: `ebe7628bbd57a51b535296e8b8e944afba9dc158df96009b5adc88ffd6db5bef` (34 paths; only the five listed Rust files differ from `37a10…`). Including the two principal-supplied verification files, material-source fingerprint: `db515af6dbb024c1e160f5cb7a0e0e205a6761ed07d2423ab2337bd21019659a` (36 paths). Machine receipt: `/private/tmp/brn-b1a-panel-20261016/source-candidate-deferred.json`. It records both exact frozen patches, the bounded correction, root verification diff/hashes and successful direct rustfmt/diff/reverse-patch checks. Root verification inclusion remains pending principal approval; the worker neither edited nor executed those files. Earlier receipt/patch identities remain retained unchanged. All identities remain uncompiled WIP.

## First granted compile window

The historical source-only identities above are superseded for current qualification by [panel-window1](panel-window1.md) and its [machine receipt](panel-window1.json). The allocated local lock edge, ordinary compile/import/visibility corrections, two existing asset assertion API forwards and the meaningful work-bound witness are included there. Library44/full native270 passing results, one existing ignore, all failed attempts and deferred gates are recorded without extending them to global acceptance.
