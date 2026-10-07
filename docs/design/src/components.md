# Components

Implemented components live in
[`native/ui.rs`](../../../crates/brn-desktop/src/native/ui.rs). Use them instead
of styling `div`s or `Button`s ad hoc. Toolkit controls (`Button`, `Editor`,
`Textarea`, `Input`, `Checkbox`, dialogs) come from `gpui-kit` 0.6.6 and are
themed by [`theme.rs`](../../../crates/brn-desktop/src/native/theme.rs).

| Component | Code | Use it for | Do not use it for | Status |
| --- | --- | --- | --- | --- |
| `section_label` | `ui::section_label(text, p)` | Naming a group of rows or fields | Page titles (use `view_header`) | Implemented |
| `view_header` | `ui::view_header(title, kind, identity, p).child(actions)` | Top of every document-pane surface: optional kind badge, human title, mono identity, right-aligned quiet actions | Sub-sections | Implemented |
| `badge` | `ui::badge(text, tone, p)` | Short state or kind: *Draft*, *Applied*, *overdue*, *Source · read only* | Sentences; counts in the navigator (use `nav_row` trailing) | Implemented |
| `callout` | `ui::callout(tone, body, p)` | Consequences and trust statements, errors, warnings, AI suggestions | Ordinary help text (use `hint`) | Implemented |
| `hint` | `ui::hint(text, p)` | Muted explanatory prose, empty-list notes | Errors | Implemented |
| `meta` | `ui::meta(text, p)` | Exact identity lines: paths, UUIDs, model, versions, times | Prose | Implemented |
| `empty_state` | `ui::empty_state(title, body, p)` | A surface or list with nothing to show: say why and what to do | Loading (use a hint) | Implemented |
| `toolbar` | `ui::toolbar()` | A wrapping row of buttons at natural width | Vertical stacks of buttons | Implemented |
| `nav_row` | `ui::nav_row(id, icon, label, trailing, p)` | Navigator destinations with optional count/shortcut | Lists of records | Implemented |
| `list_row` | `ui::list_row(id, title, detail, trailing, p)` | Selectable records: chats, notes, proposals, Actions, search hits | Commands | Implemented |
| Button tiers | `ui::primary`, `ui::secondary`, `ui::quiet`, or `Button` + `.primary()/.outline()/.ghost()/.danger()` | See below | — | Implemented |
| Count tile | inline in [`dashboard.rs`](../../../crates/brn-desktop/src/native/dashboard.rs) | Dashboard counts; tone only when non-zero | General statistics | Implemented (candidate for `ui`) |
| Decision bar | inline in [`review.rs`](../../../crates/brn-desktop/src/native/review.rs) | Persistent bottom bar for a proposal decision | Non-binding actions | Implemented (candidate for `ui`) |
| Chat exchange | `chat_exchange` in [`simple.rs`](../../../crates/brn-desktop/src/native/simple.rs) | Question block, BRN label with recorded provider/model/effort, status badge, answer | — | Implemented |
| Claim chip | prototype `.chip` | Inline provenance label on a claim | — | Prototype |
| Activity trail | prototype `.trail` | One-line summary of AI steps, time and tokens | Raw tool transcripts | Prototype |
| Review group list | prototype Inbox | Checkbox list of independent proposals + one approval | — | Prototype |

## Button tiers

| Tier | Toolkit | When |
| --- | --- | --- |
| Primary (filled cyan) | `.primary()` | The single main command of a region: *Ask*, *Save to Markdown*, *Review exact approval…*, *Process checked originals*, *Complete…* |
| Secondary (outline) | `.outline()` | Real alternatives: *Search*, *Rewrite*, *Comment…*, *Reject* |
| Quiet (ghost) | `.ghost()` | Toolbar and row actions, recovery tools, *Close*, refresh |
| Danger (filled red) | `.danger()` | *Stop* and confirming destructive operations only |

Use `.small()` in panes and rails; reserve default size for dialogs. Labels are
verbs; an ellipsis (…) means a dialog or form follows.

## States every component must show

| State | Pattern |
| --- | --- |
| Disabled | Toolkit disabled style plus a reason nearby (hint or tooltip) when the reason is not obvious |
| Selected | Toolkit `selected` (rows), `primary` for the chosen segment of a segmented control |
| Loading | A short hint (“Loading Dashboard…”) in place of content; keep prior content when refreshing |
| Empty | `empty_state` with what to do next |
| Error | `callout(Danger)` with the workflow message and the retry control |
| Provisional | `Tone::Ai` badge (“Streaming (provisional)”) |

## Element IDs

Every interactive element keeps a stable `id` (`open-inbox`, `review-approve`,
`dashboard-action-<uuid>`…). Widget tests, routing and accessibility depend on
them. Restyling must not rename or remove IDs; preserve accessible labels that
tests read (for example `dashboard-counts`).
