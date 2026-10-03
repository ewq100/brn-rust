# brn-desktop

Desktop entry point, GPUI views and transient interaction state. Simple operations go through AppWorker; legacy local operations keep their existing Worker. Also retains sample headless shell checks.

## Simple workspace (native default)

The default is **`~/Library/Application Support/BRN-simple`**, with the exact
**`BRN-simple.credentials`** sibling. Startup never inspects/copies/migrates the
old BRN folder, signs in, discovers provider models, chooses a provider/model,
or downloads a model. Data directory creation/canonicalization and layout
loading precede GPUI; AppWorker alone opens authority, scans the vault and
loads/uses retrieval resources off GPUI. [`ai.rs`](src/ai.rs) holds only
presentation DTOs and operation/generation correlation.

Choose Vault uses the native folder chooser and sends BindVault to the owner.
Notes are **saved-file readers**, not editors: there is no simple Markdown Save
in this step. Refresh/search report keyword-only results, unreadable notes and
exact embedding progress. History resumes from WorkStore even without a vault
or a valid current selection.

Settings provides independent ChatGPT/Copilot account status, explicit Connect/
Disconnect, model discovery and provider/model selection. A connected cache with
no display name stays “account name unavailable”; failed/cancelled Connect
refreshes actual status. Codes/links exist only in the active transient login
dialog; cancel, dismissal and every ending clear it and target its exact UUID.
Code expiry/reconnect are explicit retry states, never automatic login.
ChatGPT live chat remains conditionally qualified; a quota reset alone does not
establish availability.

Each Ask freezes selection. Only one Ask is active; notes/search/account/history
diagnostics are independent. Text/tool progress is provisional. Stopped/Failed
partials retain their terminal status and provider/model, including historical
selections. Only Finished establishes durable finalization. PersistenceFailed
retains an explicitly **not saved** partial in memory, with an explicit Copy
action. Further Ask is blocked while finalization is unacknowledged; copy that
partial before closing/restarting. Account/history diagnostics remain available.
Navigation keeps owned progress correlated by exact request UUID/generation;
reopening the active conversation shows its full partial answer even after
visiting other history. Its live Running history row is hidden in favor of the
provisional stream. Finished replaces that row once, and older queued history
snapshots cannot restore Running over a known terminal result. A different
conversation or new blank chat does not adopt that result. Stop intent survives
pre-admission cancellation acknowledgements. Composer edits invalidate only
search results, not the current answer or its follow-up conversation.

Native retrieval offers a one-time prompt per stored consent decision, showing
pinned source, bytes/cost and destination. Decline makes no network request.
Later Download requires fresh explicit approval; its destination is not passed
as a startup load path. Cancel Download targets the active installation UUID.
Downloaded is not Installed; activation/indexing errors end progress honestly.

`--legacy` explicitly opens `~/Library/Application Support/BRN`. An explicit
`--data-dir` classifies database/sidecar/backup markers; mixed modes refuse before
opening. Empty directories default to simple unless `--legacy` is supplied.
`--legacy`/`--vault` conflicts refuse before database work. Legacy retains local
editing/recovery/history and its note guards, but AI and executable controls are
retired. `--codex` is unknown, not accepted configuration. The legacy Config
contains only a local model directory. No App is opened in a legacy folder.

## Interfaces and source

[Entry point](src/main.rs), [layout model](src/layout.rs) and [tokens](src/tokens.rs) (GPUI-free, default-feature tests), [native shell](src/native/mod.rs) with [theme](src/native/theme.rs) and [regions](src/native/shell/mod.rs), [draft UI](src/drafts.rs), [comment UI](src/comments.rs), [CLI tests](tests/cli.rs). Layout and appearance persist to `layout.json` in the data directory. See the [workspace shell decision](../../docs/architecture/decisions/2026-10-01-workspace-shell.md) and the [UI feature backlog](../../docs/ui/feature-backlog.md).

The native workspace has History and Vault rails around a document/chat centre.
Narrow windows collapse rails and use Document/Chat tabs; Focus hides the rails.
Closing a document retains draft edits, and late draft-open results cannot replace
newer document navigation. Settings uses the toolkit modal host for appearance,
rail widths and layout reset. Layout preferences are stored separately from the
authoritative workflow data.

The three dividers support pointer dragging and keyboard resizing: Tab among
chrome controls to focus, then ←/→ for 8 pt or ⇧←/→ for 32 pt. Grabs preserve
the pointer offset within the divider; only changed layouts persist when a drag
ends, including when the window deactivates. Tab/Shift-Tab inside multiline
composer/draft/comment editors indent/outdent (toolkit behaviour); leave editors
via ⌘L, menus, Escape or other applicable shortcuts.
The BRN, View and Navigate menus expose Settings
(⌘,), Quit (⌘Q), History (⌘0), Vault (⌥⌘0), Focus (⇧⌘↩), New Chat (⌘N),
Focus Composer (⌘L) and Cancel Running Action (⌘.). Legacy New Chat is idle-only; simple history navigation stays independent of Ask.
Quit still respects dirty drafts. Escape remains local to editors and the
toolkit Settings dialog.

## Legacy local Markdown notes (`--legacy` or legacy markers)

The native vault-rail **Notes** section chooses a local vault, opens existing
`.md` files by chooser or vault-relative path, and reopens registered notes/recovery
after restart. Notes open as centre documents with interim editing controls.
A workspace binds one vault; a different root requires a separate data directory.
Save/Cmd-S in the note editor explicitly writes Markdown through the workflow.
Search approval is a separate
saved-snapshot action, not Save or publication approval. Existing source rows
retain their last-validated current-state labels and refresh after note changes.

[`notes.rs`](src/notes.rs) keeps exact UTF-8 text (including BOM, mixed line
endings and frontmatter), editing generations and durable acknowledgements
separate from fresh disk observations. Save and recovery receipts acknowledge
only their submitted generation; later typing stays in the live GPUI editor.
Copy receipts validate a distinct destination without switching the original
editor or resolving an uncertain original operation. The UI shows Unsaved,
Recoverable in BRN, Saving to Markdown, Saved to Markdown, External conflict
and Save outcome uncertain, alongside missing/root-unavailable/owned-elsewhere
availability and typed failure details.

Recovery is scheduled after **500 ms** without an edit and coalesces to the
latest text. This is not a durability deadline: the single worker can be busy
with local model/index work. Only an acknowledged commit establishes recoverability.
Window close, the application Quit action/menu/Cmd-Q and note-switch/actions
defer for pending note mutations and flush the latest buffer asynchronously.
Failed recovery keeps work accessible until explicit retry or confirmed discard
of unrecovered typing.
Other note-action failures, including save conflicts and uncertain
reconciliation, do not pause automatic buffer recovery or deferred-close flushes.
That discard preserves acknowledged recovery and uncertain operations; confirmed
reload is a separate workflow decision that discards local text in favor of disk.
Reload refuses an inode change. Atomic-save editors such as TextEdit replace the
inode; use confirmed Relink to the same path first, then Reload with explicit
discard if desired. Relink alone retains local edits.
Standalone draft/comment close guards remain independent.

Direct macOS termination, such as Dock Quit, does not pass through those action
guards in the pinned GPUI implementation. Its final `on_app_quit` hook cannot
veto termination. It dispatches no new recovery flush: even an idle worker's
SQLite commit has no guaranteed completion bound, and an admitted critical job
must be joined rather than abandoned at GPUI's 200 ms quit-future deadline.
Guarded close/Quit drain and join local work asynchronously before closing,
including admitted critical notes, chat finalization, installers and ordered
layout writes. Finalization failure blocks closing and retains the unsaved
partial; a separate explicit close-without-saving confirmation is offered.
The defensive system-termination hook preserves the legacy Worker's synchronous
drain of already accepted critical note jobs before returning its timed future,
without waiting for layout preferences. Simple AppWorker joins stay off GPUI;
that path cannot veto termination or guarantee completion at GPUI's
quit-future deadline.
It does not flush the latest coalesced UI submission if it was not already
admitted, and cannot keep the window open on recovery failure. Unacknowledged
typing can be lost. Restart reconciliation classifies interrupted save intents
without replaying their filesystem writes; it cannot recover typing that was
never durably recorded. This limitation does not extend to the guarded routes
listed above, and no protection is claimed for unadmitted typing on termination.

Presenter notices are drained even while a job is running, then coalesced into
idle worker observations. Root/rescan notices observe all registered notes;
moves/deletions never retarget buffers. Editor focus, window activation and note
actions request fresh observation; notifications cannot prove eligibility.
Comparison shows baseline/local/disk text or deletion, with unexpected
displacement retained by the operation recovery. Relink explicitly confirms
identity and retains edits. Copies use an explicit user-entered vault-relative
`.md` destination and cannot overwrite an occupant. Recovery inspection,
reconciliation and accepting reviewed current disk state retain the original
outcome and protected recovery; none silently replay a write.

## Dependencies and features

Always depends on `brn-core` and workflow DTOs for GPUI-independent AI state tests.
Default features are empty; `native-ui` enables GPUI; `native-retrieval` includes
native UI and workflow native retrieval. Normal native builds enable both.

## Verification

Run from the repository root:

```sh
cargo test -p brn-desktop --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
bash scripts/verify-desktop-shell.sh --native
```

Native interaction requires macOS Apple Silicon and an unlocked session. Use explicit disposable data. Preserve dirty-state guards, focus/selection, exact original quotes and generation-aware response handling.

Note-state tests exercise the GPUI-kit Rope text backend programmatically,
including BOM/CRLF/Unicode and byte limits, plus receipt/close/retry scheduling.
They do not establish widget rendering, IME/accessibility behavior, OS chooser
usability or human acceptance. Those still require native observation.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
