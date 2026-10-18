# Shipped BRN UI workflow inventory (provisional)

Snapshot: campaign pilot status after the shared Note and Review lifecycle owners
were integrated. This remains an explicit provisional denominator of 25 coherent
native desktop workflow domains found by source search, not a completeness claim.
Reconcile against live callbacks, optional features,
OS prompts, and the final release surface before reporting coverage. Every row is
**UNQUALIFIED** unless noted. Durable business state belongs to checked
`WorkspaceStore`/SQLite (`docs/architecture/invariants.md`); UI state below means
only workflow/presentation state. Async identity and failure notes are source-level
observations, not qualification evidence.

GPUI owns editor buffer mechanics, selection/IME, widget focus internals, layout,
and rendering. BRN owns editor-to-record/session coordination, navigation/focus
intent, target identity, prompts, and recovery decisions. Stateless controls are
not listed as separate workflows. App startup parsing and static icons are excluded;
headless startup/data-folder behavior is covered by `main.rs` tests/CI but has no
stateful UI lifecycle. Other CLI, agent, and runtime operations are outside this
desktop inventory. No OS-intent registration was found in the bounded pass;
platform file/folder prompts are included, and any additional routed OS entry remains
an explicit denominator gap.

| # | Workflow / real routes | BRN owner and current lifecycle | Effects, identity, failure / recovery | Evidence and status |
|---|---|---|---|---|
| 1 | Startup/data root: `main.rs::parse/launch`, `--data-dir`, startup check | `Workspace::open`; no long-lived UI state | explicit absolute root; refuse incompatible/unsafe roots before mutation; startup errors returned | `main.rs` tests + CI startup; stateful UI N/A, **UNQUALIFIED** |
| 2 | Workbench layout, tile show/hide/maximize/move, focus traversal; title/nav menus, layout keys | `ThreadsUi.layout`, focus handles, reveal/focus intent; shell adapter in `threads_shell.rs` | clicks/actions/shortcuts update layout and focus; unavailable destination falls back to first control; no async work | focus/tiled/pages tests; **UNQUALIFIED** |
| 3 | Launcher and command routing: ⌘K, query, arrows/Enter, navigation commands | `ThreadsUi.launcher_*`, `lists::LaunchItem`; `threads_shell.rs` | synchronous query/selection; command runs against current records; close/escape restores focus | focus/pages tests; **UNQUALIFIED** |
| 4 | Note create/open/archive/protection from Notes lists and context menus; ⌘N | `ThreadsUi` + `Workspace`; rows/menus in `threads_tiles.rs` | checked create/change; protection/archive rules enforced at service; stale/refused action notice | pages/context-action tests; **UNQUALIFIED** |
| 5 | Note read/edit/save/recover/discard/compare/close; toolbar/menu, ⌘S, navigation | Shared private `NoteLifecycle` owns current record, clean/edit/blocked state and Read/Write; `WritingPanel` and desktop use its load/save/discard/refresh/input effects; Store owns durable note/edit session | base version + generation; persisted dirty guard/recovery; stale save preserves typed text and offers compare/discard; reopening reloads canonical note; duplicate desktop mutation routes retired | `brn-writing-ui` owner and independent host tests, plus desktop writing/document tests. Implemented pilot; independent Sol source review passed. Final production Linux X11 input/recovery/Save/close/reopen passed at source609af544 (production8c951d64), exact v3 content and empty recovery; see verification ledger. Full native/owner acceptance remains open. |
| 6 | Note formatting/read mode/marks/reveal | GPUI editor mechanics; BRN `ThreadsUi::set_note_reading/format_note/reveal`, `WritingPanel` | formatting allowed only in editable state; saved source ranges map to reading view or fall back to editor | `threads_document_tests.rs`; **UNQUALIFIED** |
| 7 | Passage selection, comment draft/post, ask-about-selection; ⌘/, context/toolbar | toolkit owns selection; BRN `ThreadsUi::track_selection/comment_on_selection/post_comment`, composer drafts | quote/range bound to note version; Save-before-comment and stale anchor refusal preserve draft; posting only on explicit action | writing/document tests; **UNQUALIFIED** |
| 8 | Writing setup create/edit/recheck/save/replace | `ThreadsUi` setup form and `brn-writing-ui::setup`; checked Workspace settings | target + observed view token; stale save preserves form; revoked scope fails closed | writing tests incl `b3_stale_setup_save...`; **UNQUALIFIED** |
| 9 | Scoped reference search/import/attach and saved reference management | `ThreadsUi.reference_import`, generation/cancel token; app search/import service | target/setup/context token and generation fence delayed results; cancel never attaches; errors keep canonical result/intent actionable | writing tests (`b3_delayed_scoped_search...`, `b3_cancelled_reference_conversion...`); **UNQUALIFIED** |
| 10 | Rewrite request, retry saved rewrite, requirements/exhaustive review request | `ThreadsUi::rewrite_note_with_requirements/retry_rewrite`, saved writing request in Store | request snapshot binds note/version/thread/inputs; wrong displayed context rejected; worker/run receipt retains outcome and retry route | writing tests; shared run tests; **UNQUALIFIED** |
| 11 | Candidate review, review edit/compare, Apply/Approve/Dismiss | Shared private `ReviewLifecycle` owns review/candidate transitions and effects; desktop and independent writing host use it; previous desktop-owned mutation routes retired | exact displayed candidate + expected versions; stale/superseded/deferred outcomes visible; owner edits block apply; recovery reloads current note | `brn-writing-ui` owner and independent host tests, plus desktop review/document/writing tests. Implemented pilot; independent Sol source review passed. Final integrated source/native acceptance pending. |
| 12 | Receipt Undo and protection changes from note/history/action menus | `ThreadsUi::undo_receipt/toggle_protection`; Store receipt/compensation | operation ID and version checks; compensation preserves later changes and refuses conflicts; notice links follow-up | review/backup/core tests; **UNQUALIFIED** |
| 13 | Chats: new, select, note-link, move, archive/reopen | `ThreadsUi.chat`, `writing_threads`, `threads_writing.rs`; Store threads/links | selected thread/note identity; archived chat blocks run until unarchive; service checks lifecycle | chat/projects/writing tests; **UNQUALIFIED** |
| 14 | Chat composer, destination drafts, attachment selection, message send | toolkit composer; BRN `ComposerDestination/Drafts`, `ThreadsUi::ask` | draft keyed by Home/Thread/Note; saved message precedes run; failed send keeps draft; attachments scoped to destination | chat/intake tests; **UNQUALIFIED** |
| 15 | Run start/queue/stream/stop/continue/retry/recovery card | `ThreadsUi::Worker` (`thread`, synchronous `run_id`), `threads_state::ContinuePath`; Store run/queue | worker events polled; identity includes invocation and thread; stale completion must not retarget view; failed/interrupted/cancelled notice, queue resume or retry | chat/steering/shared-run tests; **UNQUALIFIED** |
| 16 | Task/action create, edit, complete/cancel, evidence, disclosure | `ThreadsUi` actions tile/`threads_tiles.rs`; Store Action records | checked state transitions; completion evidence required; archived/protected constraints; validation notice | pages/review/core tests; **UNQUALIFIED** |
| 17 | History/receipt details and per-operation Undo | `ThreadsUi` receipts + `threads_tiles.rs::history_tile/receipt_row` | receipt-specific operation identity; Undo creates compensation; conflicts are visible | backup/review tests; **UNQUALIFIED** |
| 18 | Projects: create/rename/settings, attach/unlink/move chats/notes, delete/Trash/restore | `threads_projects.rs` dialogs + `ThreadsUi` selected project/trash | checked project/record versions and explicit keep/cancel choices; stale delete/change refused; trash/restore route retains records | projects tests; **UNQUALIFIED** |
| 19 | Search results, filters, paging, open/reveal result | `threads_search.rs`, `ThreadsUi.search/results/hits` | query/view token and cancel-on-drop; only matching current query publishes; errors/retry remain visible | app search tests + pages tests; **UNQUALIFIED** |
| 20 | Connections/topics/knowledge inspection and refresh/check | `threads_connections.rs` view state/tasks; app search/retrieval | async task tied to current view/input; canceled/stale result ignored; refresh/error state visible | no dedicated desktop test module found in bounded map; **UNQUALIFIED** |
| 21 | Settings: provider/model/effort, delegation, theme, data/export choices | `ThreadsUi.settings/settings_section`, `threads_tiles.rs` + shell | explicit persisted settings through Workspace; configuration errors notice; credentials selection separate from model choice | chat/pages/backup tests; **UNQUALIFIED** |
| 22 | Skills: discover/refresh/install/remove/select for runtime | `skills_view::SkillView`, `threads_skills.rs`; AI/app package services | package identity/digest and workspace path checks; refresh/install errors visible; exact removal | skills desktop/AI/core tests; **UNQUALIFIED** |
| 23 | MCP: discover, inspect, approve exact tool, disable, authorize one input | `mcp_view::McpView`, `threads_mcp.rs`; runtime/store policy | discovery task + digest; approval binds exact tool digest; input approval one current request; failure/withdrawal retains safe state | MCP tests; **UNQUALIFIED** |
| 24 | File import/conversion: useful info/full note, Xberg modes, 8 kinds, conversion settings | `ThreadsUi::choose_import/convert_file`, `threads_documents.rs`, `Worker` | conversion/worker target and intent; cancel token; conversion report exposes gaps; failure leaves source external and no false completion | documents/intake tests; **UNQUALIFIED** |
| 25 | Export/full backup and restore destination | `ThreadsUi::choose_output/output/choose_backup_folder`, `Worker`; app/core backup services | OS-selected destination + worker completion; export/backup report success only when complete; errors keep destination/result actionable; restore safety is core-owned | backup tests + end-to-end scripts; **UNQUALIFIED** |

Status counts: 25 provisional domains; 0 fully qualified under the full campaign
acceptance standard; 2 implemented lifecycle pilots (Note and Review) with final
integrated source/native acceptance pending; 23 unqualified. The final production Linux X11 Note journey passed on the committed pilot
material (exact contentv3 and empty recovery); broad native qualification remains open. OS route completeness and the denominator remain open.
Counts describe current evidence, not inferred coverage from module or test names.

Source map: `crates/brn-desktop/src/threads_ui.rs` (state, lifecycle, workers,
bindings), `threads_state.rs`, `threads_shell.rs`, `threads_tiles.rs`,
`threads_writing.rs`, `threads_documents.rs`, `threads_search.rs`,
`threads_projects.rs`, `threads_connections.rs`, `threads_skills.rs`,
`threads_mcp.rs`, `threads_context_actions.rs`; reusable editor surfaces in
`crates/brn-writing-ui/src/`. Initial bounded search: `rg --files
crates/brn-desktop/src crates/brn-writing-ui/src`; `rg -n 'KeyBinding|fn
bindings|fn .*open_note|fn .*save|cx.spawn|std::thread::spawn|\.on_click|\.dropdown_menu'
crates/brn-desktop/src/threads_*.rs`. Re-enumerate menus, callbacks, file prompts,
optional feature routes, and integration/native drivers before treating this as the
completion denominator.
