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

Session labels show last-activity age from recorded chat activity. Historical
unknown activity and a timestamp ahead of the current clock have distinct labels;
opening history or restarting does not make a session newly active. Turn timing
is available through the shared workflow/CLI. Archive/Restore/Delete remain later
session lifecycle work.

After 500 ms without an edit, recovery submits the latest exact buffer to
WorkStore; only its acknowledgement establishes recoverability. Note-switch,
document close and guarded window close/Quit wait for the latest recovery and
admitted mutations. Recovery failure retains text and offers explicit retry;
pending navigation can be cancelled. These routes do not save Markdown.
Dock/system termination cannot veto exit and can lose unacknowledged typing.
The editor is implemented and automated verified; native/IME/accessibility
acceptance remains pending in [status](../../docs/status.md).

The Vault rail offers Current, Source, History and All browsing/search scopes.
Current is the default and opens the existing guarded note editor. Other scopes
open complete saved text in a separate read-only view, labeled with requested
scope/path and an exact Copy action; they expose no Save or proposal controls.
All can contain current notes but remains a read-only combined evidence view.
Scope changes update browsing/search without discarding an open document or
changing Ask's default current-knowledge behavior. Pagination/results and opening
replies are bound to scope/cursor/generation. Registered/recovered buffers retain
their direct guarded editing route regardless of the selected browsing scope.

**Sources** in the note editor and read-only evidence view expands saved-source
provenance in that document. It reads saved Markdown; unsaved editor changes are
explicitly excluded and remain intact. Each citation shows its UUID, observed
path(s), exact stored quote and Matched, Changed, Absent, Ambiguous or Incomplete
status. Changed/unavailable originals never replace the stored quote. Quotes are
read-only and **Copy exact quote** preserves their complete bytes. The bounded
panel offers Refresh and Close sources, with loading/empty/error states; it does
not navigate to a guessed source or expose a new mutation. Accepted document
navigation clears the panel; late inspection replies cannot reopen or replace it.

For manual source acceptance, use a fresh synthetic fixture from the
[CLI provenance scenario](../brn/README.md#durable-source-provenance), then launch
its native workspace. For the byte-preservation check, repeat capture/approval
with an additional fresh synthetic source containing BOM, CRLF and Unicode;
select byte range 0 through its complete UTF-8 byte count after identity assignment.
Open the saved knowledge note, type an unsaved correction
and select Sources: the saved quote/status must appear while typing stays intact.
Copy the quote and compare BOM/CRLF/Unicode bytes. Close/reopen the panel, then
change or duplicate the fixture source externally and Refresh; Changed/Ambiguous
must retain the original quote. Read the knowledge note through All and repeat
inspection in the read-only view. Navigate with an unacknowledged editor buffer
and confirm recovery is still required before the document changes. GUI and
owner acceptance remain separate from headless widget/state checks.

For manual acceptance, launch with a fresh synthetic data/vault pair containing
`current.md`, a `brn_kind: source` note with BOM/CRLF/Unicode, a
`brn_state: history` note and an archived original. Confirm Current lists only
current knowledge; select Source/History/All, search and open an original. Its
requested scope and Read only label must remain visible; typing must not change
the text, Copy must preserve full exact bytes, and Save must be absent. Switch
scopes rapidly during list/search/opening and confirm late replies do not change
the new view. Return to Current, type an unsaved correction and open evidence;
the existing recovery guard must acknowledge the latest buffer first. Repeat
with a full review/comment or unsent proposal form to check the retained-input
guard. Restart and compare fixture source bytes. GUI/IME/accessibility and owner
acceptance remain separate from state/widget checks.

Settings provides independent ChatGPT/Copilot account status, explicit Connect/
Disconnect, model discovery, provider/model selection and explicit low/medium/high
reasoning effort. Ask stays disabled until its effort choice is acknowledged.
A connected cache with
no display name stays “account name unavailable”; failed/cancelled Connect
refreshes actual status. Codes/links exist only in the active transient login
dialog; cancel, dismissal and every ending clear it and target its exact UUID.
Code expiry/reconnect are explicit retry states, never automatic login.
ChatGPT live chat remains conditionally qualified; a quota reset alone does not
establish availability.

Each Ask freezes provider, model and effort. Changing Settings affects new
requests; active and historical turns retain their recorded choice. Older history
shows unavailable effort rather than inventing a value. Only one Ask is active; notes/search/account/history
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

History also lists typed proposals. Full review shows every Create/Replace/Trash
member, exact captured before/proposed text, source versions and temporary
comments. Full edits recover after 500 ms through the same AppWorker boundary;
only acknowledgement establishes recoverability. Older replies preserve later
typing. Comments attach to the whole proposal or an exact UTF-8 selection;
unresolved anchors retain their old quote and require explicit reattachment.
Failed comment saves retain copyable draft text and guard leaving until it is
acknowledged or explicitly discarded. Rewrite freezes provider/model/effort and
supports Stop; a late result retains conflicting local text instead of replacing
it. These review operations keep vault knowledge unchanged. Exact approval opens
a full captured confirmation; group approval includes only its displayed records
and may stop after an earlier independent proposal. Controls remain guarded until
the application outcome and current review are acknowledged. Activity pages show
recorded successful changes and full historical approval snapshots; recovery
inspection/reconciliation reports actual pending/uncertain outcomes without
repeating installation. Failed application refreshes journals because an error
can follow recorded file effects. Activity exposes full historical Undo review;
an Applied snapshot exposes each original Trash member's exact restoration.
Interrupted operations offer separate full Finish/Restore review with captured
observations, retained comments and explicit direction. These requests are frozen
through confirmation; workflow can refuse later changed files. Errors retain
inspectable operation/attempt identities and never trigger automatic retry.
New proposal retains complete title/path/body widgets for Create, Replace or
Trash. Replace/Trash first load the exact saved source through AppWorker; stale
captures refuse creation. Switching to Trash preserves local body text until an
explicit discard. Creating review work freezes its full request and UUID; replay
may return a later edited review, and acknowledgements never replace later input.
Failed or unsubmitted input stays copyable and guards leaving. A changed payload
needs an explicit separate proposal; submission alone does not apply Markdown.
Unsubmitted form input is transient; only acknowledged creation is recoverable.
An acknowledged completed answer can explicitly prefill its full bytes and session
into this form. Failed, provisional or oversized answers cannot become truncated
drafts. AI writing uses a real stored seed Draft, comments and owned Rewrite.
GUI/IME/accessibility and owner acceptance are tracked separately.

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
cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
bash scripts/verify-desktop-shell.sh --native
# Existing disposable absolute data directory only:
cargo run -p brn-desktop --locked --offline -- --data-dir /absolute/disposable/data --headless-check startup
```

Native interaction requires macOS Apple Silicon and an unlocked session. Use explicit disposable data. Preserve dirty-state guards, focus/selection, exact original quotes and generation-aware response handling.

`native-test-support` enables the pinned toolkit's headless widget test context;
it is separate from normal native builds. Editor-state tests cover exact BOM/CRLF/Unicode text, byte limits and
receipt/close/retry scheduling. Native-feature tests round-trip the GPUI-kit
Rope backend through real AppWorker Save and restart recovery, and verify that
admitted Save drains before the defensive timed quit future.
They do not establish widget rendering, IME/accessibility behavior, OS chooser
usability or human acceptance. Those still require native observation.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
