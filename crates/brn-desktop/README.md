# brn-desktop

Desktop entry point, GPUI views and transient interaction state. Also retains sample headless shell checks. Integrated operations go through the workflow worker.

## Interfaces and source

[Entry point](src/main.rs), [native views](src/native.rs), [draft UI](src/drafts.rs), [comment UI](src/comments.rs), [CLI tests](tests/cli.rs).

## Local Markdown notes

The native **Notes** page chooses a local vault, opens existing `.md` files,
and reopens registered notes/recovery after restart. A workspace binds one vault;
a different root requires a separate data directory. Save/Cmd-S on this page
explicitly writes Markdown through the workflow. Search approval is a separate
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
Window close, the application Quit action/menu/Cmd-Q and note-switch/actions defer for pending note
mutations and flush the latest buffer asynchronously. Failed recovery keeps work
accessible until explicit retry or confirmed discard of unrecovered typing.
That discard preserves acknowledged recovery and uncertain operations; confirmed
reload is a separate workflow decision that discards local text in favor of disk.
Standalone draft/comment close guards remain independent.

Direct macOS termination, such as Dock Quit, does not pass through those action
guards in the pinned GPUI implementation. Its final `on_app_quit` hook cannot
veto termination: it defensively joins admitted note mutations, but does not
flush unadmitted typing or keep the window open on recovery failure. This route
still needs a cancellable native termination-request integration before all
intentional close paths can be qualified as recovery-safe.

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
