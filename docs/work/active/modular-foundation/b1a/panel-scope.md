# B1a panel allocation proposal after CW2

**Read-only proposal, 16 October 2026, pinned to `b617d7842c1e668292b4f23c7b321ec195a76a1d`.** No panel implementation is allocated or included in the helper move. Numbers below follow the current checkout, whose only principal-applied shared UI difference from this pin is the two helper imports. Symbols govern if line numbers shift.

CW2 source `12b0d329` is independently approved and [accepted for local dependent development](../cw2/principal-disposition.md). [Handoff](../cw2/handoff.md), [round 2](../cw2/opus-review-round2.md) and [round 3](../cw2/opus-review-round3.md) preserve the qualifications. Full F21/program qualification, hosted checks, human native/owner acceptance and the PDF-tool gap remain open. The former ownership/build blocker is released; full-panel **path and interface allocation** is still required.

## What changes from the 22624ac1 map

- `threads_ui.rs::start_run` already uses `Workspace::prepare_thread_run(ThreadRunStart)` and AI `spawn_prepared`; do not describe that as absent or copy it. B1a still emits a request to the existing host adapter. Later B1b can retire remaining adapter-specific writing selection/control without another runtime.
- `active_writing_context`, `ensure_writing_context`, `select_writing_conversation` and `new_writing_conversation` now own canonical context and durable explicit conversation use. The first-linked fallback is gone. `set_chat` and programmatic `open_thread` do not record owner selection. Preserve this distinction.
- Display safety now depends on one checked batch, its canonical change token and exact candidate content. Reusing only the operation ID or cached entity text is insufficient. First typing must not invalidate this batch on every edit-buffer update.
- CW2's named private-source disclosure is mandatory with Inputs collapsed. Review and Compare refuse before private content; saved inputs, feedback summaries, presence/status/result and worker display are context checked too. This is existing behavior to move intact, not permission to add B3.

## Exact current shared seam

| Source and symbol | Contract to preserve |
|---|---|
| `brn-threads-app/src/writing.rs:42 ActiveWritingContext` | Existing `{ note: String, scope: Option<String>, conversation: Option<String> }`. `None` is real canonical general scope, never selected Chat or an empty-string sentinel. |
| `writing.rs:48 active_writing_context` and `:57 active_writing_context_from` | Read-only resolution from Note/opening scope plus durable explicit-use receipts. Ignores undone/archived selection. Only the working Note gets the existing selected-conversation visibility exception. References retain canonical scope. |
| `writing.rs:166 select_writing_conversation`, `:205 ensure_writing_context`, `:221 new_writing_conversation` | Explicit owner selection is checked/replayable; repeating active selection is a no-write no-op. First comment/rewrite creates a conversation only if needed. No selection side effects from rendering, opening review or programmatic chat changes. |
| `writing.rs:593 writing_request_in_context` | Saved request inspection checks Note, canonical scope and current explicit conversation before disclosure, including before candidate creation. The batch supplies the same checked snapshots. |
| `review.rs:196 WritingCandidateReview`, `:206 WritingContextReview` | Existing display data: sources, host-verified quotations, model claims/explanation and retained-text completeness; maps of checked candidates and requests. Display results confer no Apply/read authority. |
| `review.rs:353 writing_context_review(&self, note, displayed, shown: &[Prepared])` | Returns `(ViewChangeToken, WritingContextReview)` after before/after token equality. Canonically validates exact currently pending candidates. Untrusted nested request routing hint excludes unrelated Notes before expensive validation; retained requests still get full validation. |
| `brn-threads-core/src/store.rs:229 change_token` | Revision watermark + external `data_version` + exact ordered pending operation/hash pairs. Local edit-buffer writes do not change it. The operations scan remains a disclosed shallow scan, not a constant-time guarantee. |
| `review.rs:22 review_apply_in_context(&mut self, id, canonical_scope)` | Fresh candidate/context inspection, then the one checked Apply transaction. Never authorize from display cache or prior review. |
| `brn-threads-app/src/run.rs` | Real `ThreadRunStart`, typed selection/preparation errors, `PreparedWorkspaceRun`, settings/search assembly and single-use prepare. Keep credentials, runtime worker/cancellation/lease/queue in host/runtime. |

### Concrete desktop preservation points

- `threads_ui.rs:472 WritingReviewCacheKey` is `(ViewChangeToken, displayed Note ID, canonical scope, selected writing conversation, Vec<Prepared>)`. The exact shown candidate content includes the current review candidate replacing its list entry. Keep this key intact.
- `threads_ui.rs:965 refresh_writing_review_cache`: token lookup, exact key compare, clear **before** fallible reload, canonical batch, observed-token compare, clear on every error. `threads_shell.rs:1652 Render::render` invokes it before any tile rendering, including Compare or changed/hidden Chat. Retain one cache owner and a single invocation per render.
- `threads_ui.rs:1361 open_note_in_context` resolves first and leaves the old Note/context intact on refusal. Only after successful opening does it adopt `context.scope/conversation`, derive Project, restore buffers/drafts and optionally initialize a blank Chat.
- `threads_ui.rs:1978 select_writing_thread`, `2013 new_writing_thread`, `2039 ensure_writing_thread`: consume the actual shared context methods. `1292 set_chat` has no durable-choice side effect; `1457 open_thread` passes opening context but never records a selection. `threads_projects.rs::open_project_note` remains a host scope caller.
- `threads_ui.rs:2610 open_review`: candidates with evidence cannot implicitly replace the canonical scope/conversation with snapshot context; legacy/v1 behavior remains distinct. `2770 apply_candidate` also requires the actual displayed Note target, then `review_apply_in_context` with `note_writing_scope`.
- `threads_writing.rs:2998 review_body`: evidence eligibility check is before reason/JSON/Inputs/headings/text; unavailable view offers Keep for later. `writing-private-disclosure` is outside the Inputs condition and names exact source titles/versions and owner review/provenance.
- `threads_writing.rs:3605 compare_tile`: same eligibility before versions/headings/current/candidate text. `note_bar:2562` excludes ineligible candidates before computing readiness; saved-input selection uses the checked request map.
- `threads_workbench.rs`: `writing_proposals:87` filters before feedback/count/newest/summary; `latest_note_writing_request:171` uses the checked request map; `writing_worker_for:31` checks exact Run → instruction → checked request → Note identity. `writing_run_outcome`, `writing_bar_row` and `writing_result_note` filter before suppression/presence. Recorded notice still leads and progress stays secondary. Display filtering does not cancel/change the real worker or queue.
- `threads_ui.rs:747` InputEvent::Change handler → `note_changed:1694` → begin_edit/update_buffer synchronously, then derived Read/selection/notify. `reload_open_note:2826` never replaces dirty content. Save/discard keep exact session/generation. Do not route this through an asynchronous notification, poll or debounce.

## Proposed concrete borrow/request interface

Use a **plain `WritingPanel` state value owned by the host**, containing the existing GPUI editor/view entities and local Note/comment/review/draft state. It is not a second Workspace, worker or datastore, and does not need a public host/editor trait. Keep the current toolkit editor. This avoids reentrant panel-entity updates and whole-desktop `Rc<RefCell<Workspace>>` conversion.

The following are proposed signatures/data flow for allocation review, not implemented or compiler-qualified APIs:

```rust
// Private host adapter, still the one owner of the real Workspace.
let Self { workspace, writing, .. } = self;
let result = writing.note_changed(workspace, text);
// Preserve current typed error handling, notice and cache-refresh policy.
```

`WritingPanel::note_changed(&mut self, workspace: &mut Workspace, text: String)` moves the existing checked edit/update logic exactly once. Return the current `AppResult<Option<(Tone, &'static str)>>` from its existing inner operation; keep edit-open state and the same typed `EditSessionOpen` consequence. The host consumes that result immediately, preserving notice/error text and required refresh. Other Save/comment/review helper methods borrow the same real service only for the duration of a synchronous call. No borrow is stored in a callback or crosses a provider/task boundary.

Render accepts short-lived `&Workspace`, borrowed canonical `Record`/`Receipt`/`Prepared`/`EditSession` slices, `ActiveWritingContext`, references to the host's checked candidate/request maps, caller-scoped assets and concrete width/visibility/worker display facts. The cache remains **host-owned initially**: one implementation of its reviewed invalidation/reload logic. The panel cannot refill denied entries from unscoped `writing_request`/raw records. If cache ownership later moves, move the complete key/batch/error logic together with no second cache.

A generic render function can use the actual host `Context<T>` and **one concrete UI request callback**, for example a function pointer `fn(&mut T, WritingRequest, &mut Window, &mut Context<T>)`. All event captures own their IDs/text/Prepared values; no `&Workspace` escapes rendering. Existing `cx.listener` dispatch invokes the host directly, preserving the present call stack. This is a UI callback, not a command bus/host-operation trait or orchestration framework.

Requests come only from existing controls:

| Request payload | Host destination |
|---|---|
| Rewrite with included comment IDs/current overall feedback; Retry exact request Message ID | Existing `rewrite_note` capture / `retry_rewrite` / `start_run`. Use app v2 capture and unchanged v1 readers; do not reserialize/reimplement request semantics in panel. |
| Stop current invocation; request Stop confirmation for exact saved Run ID | Existing cancel/Stop dialog and checked fence/queue pause; retain Note-vs-chat duplicate-control rule. |
| Explicit select `(note, thread)`; new conversation for Note | Existing host calls to shared checked selection/new context. Programmatic Chat/navigation never synthesizes these requests. |
| Open/Apply/Dismiss exact `Prepared`; Undo exact operation; prepare review edit/group selection | Existing single handlers shared with chat/Changes, including displayed-Note and fresh contextual Apply checks. No second Apply/Undo workflow. |
| Save/Save-copy/confirm Discard; post exact passage comment; mark captured subset addressed with actual Apply receipt | Existing service operations or moved panel methods borrowing the same Workspace. Preserve replay key, session/generation and saved quote. No post/run from selection alone. |
| Ask in chat with exact quote; Connections/Compare; New Note; Edit note; Export/open original | Desktop shell/composer/dialog/navigation actions. Keep authority and OS actions in host. |
| Format/Read/Write/Outline/reveal/comment draft/local disclosure toggles | Concrete panel state/editor updates from current handlers. No editor trait or new persistence. |

Retain subscriptions in the host during the first panel move: they refer to panel-owned toolkit entities but run the same immediate host handlers. First typing establishes the shared dirty guard before notify/render; local edit-buffer updates must retain CW2's cache hit. Lifecycle teardown keeps all subscriptions alive for the same window lifetime. Do not replace this with `cx.emit` unless its timing and borrowing are separately demonstrated; it is unnecessary for this proposal.

## Exact proposed source/support closure for later allocation

These paths are requests, **not granted edits**. Principal should allocate a bounded shared-support move before the full panel, then one exclusive panel/shared-caller window; no simultaneous UI writers.

| New writing-crate source | Existing source/hunks to move or consume once |
|---|---|
| `src/native.rs` | Plain panel state, existing OpenNote/Review/CommentDraft; current entity creation/initialization, typography/builders, concrete request callback. No new runtime or headless abstraction. |
| `src/writing.rs` | `threads_writing.rs::note_toolbar_needs` through `compare_tile` (967–3676), document/review helpers/floors and ProposalShape/review_footer at tail. Retain mandatory early gates and disclosure. Leave chat/composer block 15–966 and `linked_tile` in desktop. |
| `src/document.rs` | Current `threads_document.rs` whole implementation, adapted only for panel-owned state/real service borrow/current callback. Preserve formatted Read source mapping, formatting, saved-selection revalidation and programmatic-vs-user changes. |
| `src/workbench.rs` | Current document status/proposal/result renderer/derivations and inline 3 tests. Desktop chat keeps callable single shared queries for writing instruction/target association. |
| `src/layout.rs` | Only MarginMode/NoteNeeds/NoteFit/ReviewFit from `threads_layout.rs:98–216` plus the 2 threshold tests. Host Layout/TileKind/panel-width/margin-choice and cross-panel tests remain desktop. |
| `src/markdown.rs` | One existing LiteralHtml/external_link/stored_image/safe_markdown/markdown policy from `threads_ui.rs`. Desktop messages/linked work reuse it. Caller-visible asset map stays scoped, never replaced by all cached assets. |
| `src/components.rs` | Exact generic primitive set below; desktop re-exports same implementations. Leave desktop forms/list system there. |
| `src/theme.rs` | Whole existing ThemeId/Palette/pal/apply implementation and 3 inline tests, one CURRENT; desktop re-exports. No restyling or new theme system. |

Generic components, unchanged source boundaries: tokens and IconName; Tone/tone_icon; icon_button; segmented; empty_state; banner/banner_pinned and their floor constants; status_line; status_chip/short_status_chip/private chip; toolbar; prose_editor_state; unavailable; dialog sizing/body/footer; DialogAction/consequence_dialog. These are at `threads_components.rs:20–119,134–141,403–607,644–753,793–end`. Retain `form_keys` with desktop Ask action, field_error, list/header/loading/card/tone_mark/overflow helpers; they import/reuse the moved types/primitives.

Typography/builders (`MONO`, `UI_FONT`, text sizes, button/prose_editor/subheading/section/metadata) and unchanged `title`/`short` can live in `native.rs` and be re-exported by desktop to avoid two policies. `shell::item` is currently `WeakEntity<ThreadsUi>`: library menu construction uses the concrete request callback instead; keep shell panel menu sections/navigation. Outline `sections` and unlocated-label helper may move once with their existing tests; exact comment range validation already exists as app `exact_writing_range`.

Required existing-file allocations:

- `threads_ui.rs`: panel state/entity/subscription forwarding, editor/comment/review/view methods; retain exact cache implementation as above; unchanged host context/start/worker/queue/notice handlers with only necessary panel-field access. No cache/runtime rewrite.
- `threads_writing.rs`: actual bounded Note/review/Compare removal and shared helper imports. Never delete the whole mixed file.
- Delete `threads_document.rs` / `threads_workbench.rs` only when every caller uses the moved implementation. No duplicate route/shim or cross-crate path include.
- `threads_shell.rs`: Note/Compare/menu/layout/focus forwarding only; preserve pre-render cache refresh, shell navigation and panel body IDs. `threads_tiles.rs`: Note controls forwarding only.
- `threads_components.rs`, `threads_theme.rs`, `threads_layout.rs`: exact shared-support removal/re-exports above. `threads_state.rs`: only outlined document helper move if chosen, never the entire mixed state module.
- `threads_chat.rs`, `threads_context_actions.rs`, `threads_projects.rs::open_project_note`: minimal access forwarding only; preserve shared checked Stop, contextual Edit note, explicit opening scope and failed-open behavior. No expanded product logic.
- Manifest app dependency is justified **then** by actual Workspace/types. Gate all native modules/exports/dev dependencies on native-ui. Principal owns lock/feature/CI integration; explicit library native test jobs are necessary when tests actually move because building a dependency does not run its tests.

## Tests and next reviewable unit

The pure-helper move owns only its existing 30+4 tests. For panel extraction retain the full desktop integration journeys and their real clicks/assertions, including both test-specific reduced-motion setups and CW2's `private_writing_disclosure_is_mandatory_and_wrong_displayed_scope_hides_review_and_compare` (writing tests:349). That one witness includes repeated notify cache hits, actual typing/generation persistence with no reload, exact candidate-content mutation, wrong scope, local/external revocation and restoration, collapsed disclosure, Result/Compare/proposal/Saved-input/status checks and fresh refused/explicit Apply. Move or adapt its field access without weakening any proof. Keep generic review/Action/chat/project journeys desktop-hosted.

Preserve helper/editor selection cases F05/F06, dirty stale/delayed-generation guards F09, exact immutable Apply/receipt/Undo F12, drafts/restart F16 and unrelated F19. **F15 actual local editor Undo after Apply/refresh is an accepted later gap**: add a meaningful real-editor witness then, not export-mirroring tests. Native/IME/human acceptance remains separate.

Suggested next unit after helper review: exact shared-support implementation move (markdown/palette/generic primitives and native gating) with single implementations and desktop re-exports, followed by the exclusive panel state/render/caller move above. Principal may combine these if avoiding intermediate public APIs is simpler. No additional helper task is invented merely to fill capacity; no panel or support source is changed in the current slice.
