# Workspace shell and design system

Date: 1 October 2026

Status: Implemented (slice 1); automated verification recorded in the workspace shell evidence. Manual native verification and user acceptance pending.

Inspected baseline: local `main` at `4f059881f5a34d22ffdf1bb53e79954c94c8608f`.

Reference: the BRN UI/UX handoff at `/Users/evokessler/repos/brn/brn-ui-handoff` (UI-SPEC, INFORMATION-ARCHITECTURE, COMPONENT-INVENTORY, DESIGN-SYSTEM, DECISIONS, OPEN-QUESTIONS, RUST-UI-MIGRATION-NOTES and the recommended workspace mockup). The handoff preserves behaviour and information architecture, not its TypeScript stack, and it assumes a Pi SDK provider. This repository uses the Codex App Server. Every provider-facing handoff element must be mapped to existing `brn-workflow` capabilities, not re-implemented in the UI.

This is the first of several UI slices. It does not authorize new workflow, store, CLI or provider capabilities, live provider calls, vault migration, merge or release.

## 1. Outcome and decisions

Replace the single-column page switcher in `brn-desktop` with the handoff's combined workspace. History is on the left, the document and chat share the centre, and the vault is on the right, all in the refined-terminal visual language. Capabilities that exist today move into their new regions with unchanged workflow behaviour.

| Decision | Choice |
| --- | --- |
| Decomposition | Slice 1 is the shell and design system. Later slices are chat polish, the vault rail and note editor, the history/diff UI, and review and publication. |
| Toolkit | Stay on GPUI Kit 0.6.6 (still provisional per the architecture baseline). |
| Handoff elements with no backend | Hide them in the UI and track each in `docs/ui/feature-backlog.md`. |
| Layout persistence | A versioned JSON file in the explicit data directory. Unsent composer text is not persisted across relaunch. |
| Appearance | Dark and Light, following macOS system appearance by default, with a Settings override. |
| Narrow windows | Rails auto-collapse (vault first, then history). The centre falls back to Document \| Chat tabs. |
| Header | A transparent titlebar: traffic lights and the BRN header in one strip. |
| Structure | Restructure in place (approach A) with a GPUI-free layout module. |

Rejected structures: one GPUI entity per region (rewrites state ownership and risks dirty-state and stale-response guards) and a parallel new binary (doubles maintenance).

## 2. Architecture and modules

`Desktop` stays the single owner of workflow worker handles, `Phase`, generation-aware response handling and dirty-state guards. Region modules are render functions over `&mut Desktop`; they hold no workflow state.

| Module | Responsibility | Features |
| --- | --- | --- |
| `layout.rs` | `LayoutState` stores `history_w`, `vault_w`, `doc_share`, `history_collapsed`, `vault_collapsed`, `focus` (with saved prior state) and `appearance` (`System \| Dark \| Light`). A pure `resolve(window_width) -> ResolvedLayout` handles clamping, auto-collapse and `CentreMode::{Split, Tabs(Doc \| Chat)}`. Also does versioned `load`/`save` of `layout.json`. | Default (no GPUI) and unit-tested |
| `theme.rs` | Maps the handoff dark and light tokens onto the gpui-component `Theme`. Sets zero radii, a monospace chrome font, a serif editorial reading font and a 2px cyan focus ring. | `native-ui` |
| `shell/header.rs` | Transparent titlebar strip: brand, phase status, Cancel while running, Focus, Vault toggle, and a gear when history is collapsed. | `native-ui` |
| `shell/history_rail.rs` | New chat, conversations, drafts list, Settings footer. | `native-ui` |
| `shell/centre.rs` | Chat-first state, document plus chat split with a draggable divider, and tabs fallback. | `native-ui` |
| `shell/vault_rail.rs` | Sources, Working drafts, import and index actions. | `native-ui` |
| `shell/settings.rs` | Settings dialog. | `native-ui` |

The existing `drafts.rs` and `comments.rs` views are kept and hosted in the centre document pane. Splitting `native.rs` (about 2,000 lines) along these boundaries is an intended, targeted improvement. Workflow calls, actions and messages are moved, not rewritten. Use gpui-component `resizable`, `tree`, `title_bar` and `theme` where they fit. Keyboard-adjustable dividers may need custom handling.

## 3. Region content (existing capabilities only)

- **Header:** BRN, the phase status (`Ready`, `Opening workspace…`, `<label> · Ns elapsed`, `Cancelling … awaiting safe stop`, `Workspace failed to open · …`), Cancel shown only while running, Focus, Vault toggle, and a Settings gear only when history is collapsed.
- **Status line** under the header: the current `message`. A failed workspace open keeps its existing recovery text.
- **History rail:** New chat (the existing new session), saved conversations (selecting one resumes it in the centre chat), Drafts (selecting one opens it in the document pane) and Settings in the footer.
- **Centre:**
  - Chat-first when no document is open. It contains the transcript and saved answers with citations, plus a composer with the Keyword, Semantic and Hybrid profile, Search, Ask and Save as candidate.
  - Document open:
    - document and chat panes with a draggable divider;
    - a **draft** shows the existing editor, Save working copy, Save checkpoint, Discard edits, the comments panel and revision compare;
    - a **source** shows a read-only reader with title, approval, revision and bytes, plus Approve and Withdraw (the existing approval actions).
  - Closing the document returns to chat-first.
- **Vault rail:**
  - Sources, with an approval tag in text and never colour alone.
  - Working drafts.
  - Choose file, Import and approve, Build index and Refresh.
- **Settings dialog:**
  - **Workspace:** appearance (System, Dark or Light), panel sizes and Reset layout.
  - **Connection:** data directory, Codex executable, retrieval model directory and the managed sign-in note.
- **Navigation:** `Page` is replaced by a centre state `{ doc: Option<Draft(id) | Source(id)>, tab }`. Opening Settings, resizing, collapsing or entering Focus preserves composer text, selection, the open document and scroll position in memory.

Hidden until their slices land: Review queue and pending count, Notes, Archive, model and effort controls, context usage, explicit context chips, evidence inspection with Return, publication, and richer Connection status. Each is listed in the feature backlog.

## 4. Layout behaviour, keyboard and appearance

### Defaults and bounds (points)

| Region | Default | Minimum | Maximum |
| --- | --- | --- | --- |
| History | 220 | 180 | 320 |
| Vault | 260 | 180 | 360 |
| Document | 58% of centre | 360 | — |
| Chat | remainder | 260 | — |

The handoff mockup values (144/158 px, 53/47 split) are illustrative and not used.

### Reflow

`resolve` is pure and deterministic:

1. When the window is narrower than visible rails plus document and chat minimums, collapse the vault. If it is still too narrow, collapse history.
2. When it is narrower than document plus chat minimums, use `CentreMode::Tabs` with Document and Chat tabs.
3. With no document open (chat-first), the centre needs only the chat minimum, and the tabs fallback does not apply.
4. A collapsed rail stays visible as a 28 pt expansion rail, except in Focus.
5. Auto-collapse is a resolved display state. It never overwrites the user's saved collapse preference, so widening restores the rails.

The window minimum changes from 800×600 to 480×480.

### Focus

Focus hides both rails, remembers their prior collapsed state and widths, and restores them on exit. Divider movement inside Focus affects only the centre split.

### Persistence

- Write `layout.json` with `{"version": 1, ...}` to the explicit data directory on change.
- A missing file loads defaults silently.
- A corrupt file or unknown version loads defaults and shows a status note. Rewrite the file only after the next user layout change.
- The file holds no credentials or content. Transient state (selection, dialogs, composer text) is not persisted.

### Appearance

System by default. Observe window appearance changes and switch between the handoff dark and light token sets. Settings can override with Dark or Light. State colours are always paired with text or an icon.

### Keyboard and menus

Add View and Navigate menus that carry these shortcuts:

| Shortcut | Action |
| --- | --- |
| ⌘0 | Toggle history |
| ⌥⌘0 | Toggle vault |
| ⇧⌘↩ | Toggle Focus |
| ⌘L | Focus composer |
| ⌘N | New chat |
| ⌘, | Settings |
| ⌘. | Cancel running action |
| Esc | Close dialog |

Dividers are focusable. ← and → move 8 pt and ⇧ with an arrow moves 32 pt, within bounds. ⌘Q and the dirty-close guard are preserved.

### Accessibility limitation

GPUI's VoiceOver support is limited. This slice provides keyboard reachability for every region and action, a visible focus ring and text paired with every status colour. VoiceOver hierarchy and announcements remain unqualified and are recorded as open.

## 5. Testing and verification

Unit tests (default features, `layout.rs`):

- resolve tables across width bands, collapse order, the tabs fallback, and that saved preferences are untouched by auto-collapse;
- Focus enter and exit restoring the prior state;
- clamping of widths and document share;
- JSON round trip, plus missing, corrupt and unknown-version files falling back to defaults;
- appearance resolution for System with dark or light, and both overrides.

Checks, following [verification](../../development/verification.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p brn-desktop --locked
bash scripts/verify-desktop-shell.sh --native
```

Manual native evidence on the unlocked target Mac with fresh disposable data:

- resizing, collapsing, Focus, reflow down to 480 pt and the tabs fallback;
- the keyboard map and divider keys;
- a system appearance flip;
- relaunch restoring the layout;
- existing flows in their new homes: import and approve, index build, search and ask, conversation resume, draft edit, save and the dirty guard, comments, and revision compare;
- composer text surviving Settings and resizing.

Record results and limitations per the verification guide. A build or headless test does not establish native usability.

## 6. Documentation

- New `docs/ui/feature-backlog.md`: every hidden handoff feature, with its slice or roadmap item, prerequisite and handoff source reference.
- [Roadmap](../../roadmap.md): pointer to the UI slices and feature backlog.
- `crates/brn-desktop/README.md` and the [architecture overview](../../architecture/overview.md): desktop module layout.
- [Status](../../status.md): when implementation starts or completes.
- A dated decision record under `docs/architecture/decisions/` for GPUI retention for the shell, the JSON layout file, System appearance and the hybrid titlebar.

## 7. Out of scope

New workflow, store or CLI capabilities. Also out of scope:

- vault Notes and Archive (see the [Markdown-notes design](2026-10-01-open-and-safely-edit-markdown-notes-design.md));
- model and effort controls and context usage;
- review and publication (roadmap 13 and 14);
- diff UI changes;
- VoiceOver qualification;
- persisting unsent text across relaunch;
- choosing a final toolkit.
