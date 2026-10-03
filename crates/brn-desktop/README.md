# brn-desktop

Desktop entry point, GPUI views and transient interaction state. All operations go through AppWorker. The headless startup check opens and joins the same current workflow without GPUI.

## Current workspace (native default)

The default is **`~/Library/Application Support/BRN-simple`**, with the exact
**`BRN-simple.credentials`** sibling. Startup never inspects/copies/migrates the
old BRN folder, signs in, discovers provider models, chooses a provider/model,
or downloads a model. Data directory creation/canonicalization and layout
loading precede GPUI; AppWorker alone opens authority, scans the vault and
loads/uses retrieval resources off GPUI. [`ai.rs`](src/ai.rs) holds only
presentation DTOs and operation/generation correlation.

Choose Vault uses the native folder chooser and sends BindVault to the owner.
Notes support small corrections with explicit **Save to Markdown / Cmd-S**.
Fresh disk text and protected recovery remain separate; later typing survives
older acknowledgements. Compare baseline/local/disk, observe disk, confirmed
reload, exclusive Save Copy and uncertain-save reconciliation use AppWorker.
An unused vault-relative `.md` destination creates a copy without switching or
resolving the original editor. Refresh/search report keyword-only results, unreadable notes and
exact embedding progress. History resumes from WorkStore even without a vault
or a valid current selection.

After 500 ms without an edit, recovery submits the latest exact buffer to
WorkStore; only its acknowledgement establishes recoverability. Note-switch,
document close and guarded window close/Quit wait for the latest recovery and
admitted mutations. Recovery failure retains text and offers explicit retry;
pending navigation can be cancelled. These routes do not save Markdown.
Dock/system termination cannot veto exit and can lose unacknowledged typing.
The editor is implemented and automated verified; native/IME/accessibility
acceptance remains pending in [status](../../docs/status.md).

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

`--legacy` is unknown. Legacy database, sidecar and mixed authority markers
refuse before opening authority or writing a startup probe; empty directories
use the current workspace. Startup never opens or migrates old data.
`--codex` is unknown. `--model-dir` supplies an explicit local retrieval model
path; account and chat model selection remain independent Settings actions.

## Interfaces and source

[Entry point](src/main.rs), [layout model](src/layout.rs) and [tokens](src/tokens.rs) (GPUI-free, default-feature tests), [native shell](src/native/mod.rs) with [theme](src/native/theme.rs) and [regions](src/native/shell/mod.rs), [CLI tests](tests/cli.rs). Layout and appearance persist to `layout.json` in the data directory. See the [workspace shell decision](../../docs/architecture/decisions/2026-10-01-workspace-shell.md) and the [UI feature backlog](../../docs/ui/feature-backlog.md).

The native workspace has History and Vault rails around a document/chat centre.
Narrow windows collapse rails and use Document/Chat tabs; Focus hides the rails.
Closing a document waits for its latest buffer recovery acknowledgement. Later
edits survive older Save and recovery acknowledgements. Settings uses the toolkit
modal host for appearance,
rail widths and layout reset. Layout preferences are stored separately from the
authoritative workflow data.

The three dividers support pointer dragging and keyboard resizing: Tab among
chrome controls to focus, then ←/→ for 8 pt or ⇧←/→ for 32 pt. Grabs preserve
the pointer offset within the divider; only changed layouts persist when a drag
ends, including when the window deactivates. Tab/Shift-Tab inside multiline
composer/note editors indent/outdent (toolkit behaviour); leave editors
via ⌘L, menus, Escape or other applicable shortcuts.
The BRN, View and Navigate menus expose Settings
(⌘,), Quit (⌘Q), History (⌘0), Vault (⌥⌘0), Focus (⇧⌘↩), New Chat (⌘N),
Focus Composer (⌘L) and Cancel Running Action (⌘.). History navigation stays
independent of Ask. Quit waits for latest note recovery and joins admitted
workflow mutations. Escape remains local to editors and the
toolkit Settings dialog.

Dock/system termination cannot veto exit in the pinned GPUI toolkit. The final
hook submits no new recovery flush. It synchronously drains admitted AppWorker
mutations before returning GPUI's timed future; this can block the UI during
defensive system termination. Guarded close joins on the background executor.
Guarded window close, Quit, document close and note switch retain the current
buffer until recovery is acknowledged. Unacknowledged typing may be lost during
system termination. Restart reconciliation does not replay filesystem writes.

## Dependencies and features

Depends on workflow DTOs for GPUI-independent AI/editor state tests.
Default features are empty; `native-ui` enables GPUI; `native-retrieval` includes
native UI and workflow native retrieval. Normal native builds enable both.

## Verification

Run from the repository root:

```sh
cargo test -p brn-desktop --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
bash scripts/verify-desktop-shell.sh --native
# Existing disposable absolute data directory only:
cargo run -p brn-desktop --locked --offline -- --data-dir /absolute/disposable/data --headless-check startup
```

Native interaction requires macOS Apple Silicon and an unlocked session. Use explicit disposable data. Preserve dirty-state guards, focus/selection, exact original quotes and generation-aware response handling.

Editor-state tests cover exact BOM/CRLF/Unicode text, byte limits and
receipt/close/retry scheduling. Native-feature tests round-trip the GPUI-kit
Rope backend through real AppWorker Save and restart recovery, and verify that
admitted Save drains before the defensive timed quit future.
They do not establish widget rendering, IME/accessibility behavior, OS chooser
usability or human acceptance. Those still require native observation.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
