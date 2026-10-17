# B4 local verification and review handoff preparation

**Current correction:** [Opus1 response and new checks](opus1-response.md) supersede the original action-wiring/lint-harness claims. The table below retains c95 evidence; it is not relabelled as corrected-source execution. [Native lane](principal-native-lane.md) now records verified `test = true` automatic example coverage.

Bound `work/b4-20261017` at accepted B3 base `c5b257fb643794e0802dd1cb4d3fee5ff3ee0edf` in `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`. Provider OpenAI/Codex, model gpt-6.1-sol. [Original requirements/plan and exact ownership](plan.md), [all actual commands and source/artifact hashes](checks.json), [whole lock reconciliation](lock-reconciliation.json), [exact lock delta](lock.patch).

## Implemented independence witness

The native-only `writing_host` example opens a real Workspace in its own fresh TempDir and renders the existing `WritingView::note_tile`/toolbar over the existing WritingPanel/editor. There is no brn-desktop dependency, imported desktop module/test code, navigation/task system, host trait, new service, parser or production writer. Existing library/desktop/app/core/AI/CLI source is unchanged.

One fixed synthetic working Note has a canonical read-only reference imported by the unchanged import_full service from explicitly synthetic completed conversion data. Original coverage is Partial; no real converter/picker fidelity is claimed. The optional brief/reference setup is persisted by existing checked services. Sources used, host-verified quotations, import/read/claim distinctions, comment rendering and exact review controls are the same library code as BRN.

UI Rewrite captures the actual active writing context and saved feedback, calls panel.rewrite_run_start and Workspace.prepare_thread_run, then feeds a scripted MockCompletionModel into the existing run_prepared_scripted implementation. Real read_note/prepare_changes tools produce the ReadLedger, immutable candidate and evidence. Host code never prepares a candidate directly or copies selection/search/provider/runtime/permission/queue orchestration. One normal Tokio executor runs this bounded offline script synchronously; there is no worker/channel or background child outliving the action/scratch directory. This synchronous development limitation is explicit; it is not a production async/Stop qualification.

Thin handlers bind public requests to existing service operations and toolkit entities. The contextual display batch is freshly checked per render, cleared before fallible reload, token-fenced and never an Apply grant. Apply rechecks the exact displayed candidate through writing_context_review and review_apply_in_context. Editor input calls the shared panel.note_changed before derived input work/notify, retaining the existing begin_edit/update_buffer guard. Programmatic reload uses the current editor's set_value. Unsupported host actions show an explicit unavailable notice; no success is fabricated. Only this disposable fixed-document host is supplied, without settings/navigation/sync or owner-data arguments.

## Actual gates

Rust/cargo 1.98.1, locked/offline after reconciliation, CARGO_INCREMENTAL=0 and fresh explicit synthetic TMPDIR per command. Task target `/private/tmp/brn-b4-20261017-target` was fresh and is used only by this same literal bound source root. No cross-root artifacts, GUI, provider/auth access, model assets/download or owner data. No before-copy is needed: absence of the new example at base is not behavioral red proof.

| Gate | Actual outcome on final material |
|---|---|
| Native example build (native-ui, without test-support) | Passed; actual development binary and dep-info hashes retained |
| Explicit example native/test-support test lane | 3 passed /0 failed /0 ignored |
| Native writing library | 44 passed |
| Native desktop regression | 287 passed /0 failed /1 existing ignored |
| Native writing all-target strict Clippy at c95 (ordinary example binary only) | Passed, -D warnings; journey test harness was not linted (Opus1 LOW) |
| Native desktop strict all-target Clippy | Passed, -D warnings |
| Host normal+dev dependency tree | Passed; no brn-desktop |
| Default workspace normal/features trees | Passed; zero GPUI |
| Default workspace build and strict all-target Clippy | Passed |
| Full default workspace tests, --no-fail-fast | 1115 passed /1 failed /6 ignored; sole carried qualification PDF Unavailable |
| Workspace fmt / local Markdown links / diff whitespace | Passed |

The new real-owner-event journey selects a saved passage with keyboard events, clicks the shared comment/Post/Rewrite/Review/Apply/Undo controls, checks persisted feedback and exact scoped request, actual scripted model input, host evidence/quotation, unchanged saved document until Apply, exact applied Markdown and durable Undo/editor refresh. The source has a complete retained-text read but remains partial-original coverage; normal Rewrite does not falsely claim an exhaustive requirements check. The other native witness checks real typing -> recoverable dirty guard -> owner Save, then notify-only access loss hides the entire panel. It asserts after run_until_parked; it does not prove first-edit ordering before render. That ordering is a source-code observation only. The ordinary fixture witness drops/reopens the real Workspace, verifies setup/context persistence, then drops only its own TempDir and checks cleanup. Real macOS GUI quit/destructor cleanup remains unqualified.

All pre-B4 tests/assertions/IDs/events remain byte-identical. The 326 protected original material inputs match accepted B3; only the allocated library manifest and lock change, plus four new example sources. Final 332-input fingerprint `85dbe2e458e7d080f6a70d923436de67d6e5bcc6c67ab9e8d0e6965c1e882ae7`. The actual precommit HEAD and material maps, not an invented future SHA, associate every execution with source.

## Retained failures and corrections

22 runner receipts plus two offline metadata commands are retained. Earlier candidate failures are not baseline or before proof:

- First compile SIGBUS during recursive GPUI test macro expansion. A wildcard import exposed the test macro alongside Rust's built-in test attribute. Explicit imports removed the failure; no dependency/compiler patch or upgrade.
- First compiled journey expected exhaustive completeness from a normal Rewrite. Existing service code requires an explicit exhaustive request. The corrected fixture asserts that request/overall completeness are false while retaining complete source-read, quotation and partial-original assertions. No existing test assertion was changed.
- Initial all-host test could not observe host-unavailable because its test-support marker was missing. The marker was added; refusal/hidden-panel assertions remain. The earlier claim here that keyboard/context-menu forwarding was complete was incorrect: Opus1 found missing key context/listeners at c95. See the [bounded correction response](opus1-response.md) for the actual correction and later evidence.
- Initial strict lint found an unnecessary borrow in fixture Workspace::open. Removed in fixture/reopen code, with final host tests/build/lint rerun on the corrected bytes.
- The full default suite failed only prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch at qualification.rs:470: Pdf preparation failed: Unavailable. Actual pdftotext/pdfimages identities are retained; pdftoppm is absent. No skip, wrapper or rerun-to-green.

The first cargo metadata --no-deps succeeded but left the lock unchanged; the audit's initial expectation of a resolved lock was wrong. A second offline resolution with the actual target resolved exactly the six approved existing-package edges. All 953 package identities/source/version/checksums remain identical. The receipt collector also initially named the wrong historical PDF fixture; raw output was inspected and the collector corrected to the actual carried failure, without executing another test. Originals remain in scratch evidence. These evidence-tool assumptions are not production/fixture failures or red proofs.

## Applicable scope and pending gates

F18 is locally characterized by the real host journey and actual built native binary, with the shared panel and shared run/context/evidence/Apply/Undo path. F22 default graphs/build/strict lint exclude GPUI; default tests retain the sole disclosed baseline failure. This local Mac proof does not replace the required hosted Linux lane. F04/F06/F09/F10/F12/F15/F16 remain shared contracts with existing current native regressions; these three host fixtures are not a full app acceptance suite or full F21.

No native-retrieval/combined/E2E/headless broad repeat: no existing production source, feature definitions, external package identity or retrieval contract changed; the newly used development features already exist in the desktop test closure. INT-W and any stronger combined qualification belong to principal. Native-human/IME/real picker/actual GUI launch, live provider/converter, hosted/full F21, owner/program/principal acceptance and PDF tooling remain open. D1B/D2 are not integrated. No mutation was granted or run; B3's 217.265250/600 allocation is closed and unused time is not transferred.

Historical c95 CI/script request (applied by principal at 50fa; superseded only after automatic example-test coverage is verified in the [correction response](opus1-response.md)): explicitly run `cargo test -p brn-writing-ui --example writing_host --features native-ui,native-test-support --locked --offline` in the native lane. A desktop dependency build or ordinary library test does not execute the example's three tests. Keep default gates with no native features; example required-features is native-ui. Launch instructions for later authorized human work: `cargo run -p brn-writing-ui --example writing_host --features native-ui --locked --offline`; this was not launched during verification.

Fresh principal-owned Opus actual-diff review and local acceptance remain pending. Compiled source/test pin and final heavy/source ownership release will be added separately after this receipt; no self-acceptance/publication/integration claim.


## Compiled pin and completed release

Source/test pin c95b17a8642353641b92e2801fc242e263290bff, tree 41034f07d7d3c688e6b286396b936dfe66c7649d, crates f2b22572b8c12c8553929cba3db0a7c294a26c95. [Final handoff](handoff.md) records full acceptance/ownership map and principal-owned native CI example lane. [Release proof](heavy-release.json) confirms all 332 material hashes unchanged, reaped own children and empty heavy/controller inventories. Heavy slot explicitly released; all B4 source/test/manifest/lock hunks frozen and handed to principal for fresh Opus1/disposition. Only final evidence changed after that pin; no source acceptance or integration claim.
