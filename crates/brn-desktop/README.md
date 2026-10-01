# brn-desktop

Desktop entry point, GPUI views and transient interaction state. Also retains sample headless shell checks. Integrated operations go through the workflow worker.

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
Focus Composer (⌘L) and Cancel Running Action (⌘.). New Chat is idle-only;
Quit still respects dirty drafts. Escape remains local to editors and the
toolkit Settings dialog.

## Local Markdown notes

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
with provider/model work. Only an acknowledged commit establishes recoverability.
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
The existing hook drains and joins already-admitted critical note jobs,
including queued saves/copies/buffer commits; they never use the 250 ms detached
reaper. That synchronous defensive join can exceed the GPUI future deadline.
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

Always depends on `brn-core`. Default features are empty; `native-ui` enables GPUI and `brn-workflow`; `native-retrieval` includes native UI and workflow native retrieval.

## Verification

Run from the repository root:

```sh
cargo test -p brn-desktop --locked
cargo test -p brn-desktop --features native-ui notes:: --locked
cargo build -p brn-desktop --features native-ui --locked
bash scripts/verify-desktop-shell.sh --native
```

Native interaction requires macOS Apple Silicon and an unlocked session. Use explicit disposable data. Preserve dirty-state guards, focus/selection, exact original quotes and generation-aware response handling.

Note-state tests exercise the GPUI-kit Rope text backend programmatically,
including BOM/CRLF/Unicode and byte limits, plus receipt/close/retry scheduling.
They do not establish widget rendering, IME/accessibility behavior, OS chooser
usability or human acceptance. Those still require native observation.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
