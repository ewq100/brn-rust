# Notes UI design recommendation

**Scope:** design and prototype first, as selected by the owner. No native UI changed. The question is how maintained ordinary knowledge should expose its evidence, connections and changes while keeping the note readable and editable.

## Recommendation

Use **A — Read with context** as the default: the existing note/editor stays central, with a contextual rail containing evidence, related notes and recent accepted changes. Keep protection/version/unsaved status beside the title. Show an unresolved discrepancy with its linked decision thread when supported by recorded context. A source report and a protected reference can disagree without either silently replacing the other.

Borrow **B — Subject overview**'s chronology for evidence details. Original/effective evidence dates and ingestion time must remain distinguishable; an older document ingested later is still older evidence. Offer **C — Connected reading** as an optional local connections view, rather than making every reading task start with a graph. Its arrows describe recorded relationships, not proof of truth.

This is a recommendation awaiting owner selection. Purpose chips in the demo are illustrative writing conventions; the current Note schema has no role field. A native pass should omit automatic chips unless a separate product decision justifies them.

| Variant | Information hierarchy | Practical tradeoff |
|---|---|---|
| A | Subject list → readable account → evidence/related/history rail | Most direct extension of the current tiled workbench; rail stacks below text at narrow widths. |
| B | Subject header → account beside dated evidence chronology | Makes chronology prominent; larger header pushes the substantive account down. |
| C | Local evidence and related-note navigation → account with passage margin | Makes support/navigation visible; consumes more reading space and can overstate relationships if unqualified. |

## Current implementation and proposed presentation

| Proposed control | Existing data or checked path | Required later UI work |
|---|---|---|
| Working/protected vN and unsaved writing | `Record.version`, `Note.protected`; editor/dirty guards in `threads_ui.rs` and `threads_writing.rs` | Retain the current header and Save/Discard behavior. Protection is independent of role, import and truth. |
| Evidence rail | Typed `Link` endpoints/relation, `Note.import.source`, `Source` label/locator/observed_at/outcome/report; saved imported Note text | Add a bounded note-centered projection and source/detail navigation. Ordinary sources have metadata, not guaranteed retained bodies. |
| Inspect a passage | Current/historical Note revisions and existing coverage details | Display retained text and its actual revision/location when available; open an external locator separately. A quoted excerpt is not a full retained original. |
| Original availability | Source observation outcomes and coverage gaps when recorded | State the specific recorded observation/time and scope. “Original unavailable” requires evidence of an attempted access; metadata alone cannot imply availability, freshness or complete coverage. |
| Related knowledge | Existing `Link` records (`relation` is a string), bounded Notes lists | Add navigation from actual links without inferring a new ontology or embedding graph. Separate direct relationships from indirect connections. |
| Ask / decision thread | `Workspace::writing_threads`, `link_writing_thread`, existing per-note composers and authority/review path | Reuse the current linked chat; a conflict banner must come from supported context. Opening or drafting a thread supplies no decision authority. |
| What changed / Undo | `Workspace::history`, receipt before/after versions, `Workspace::undo`, existing exact review and refresh follow-up | Filter receipts by affected note; inspect the entire coherent operation group. Undo may affect several records, records compensation and may require reviewed compensation after later changes. |
| May need refresh | Receipt `needs_refresh`, existing refresh records/follow-up | Surface recorded dependencies, not a universal “fresh” badge. Rechecking versions does not establish world freshness. |

Source inspected: [core types](../../../../crates/brn-threads-core/src/types.rs), [workspace/history/Undo](../../../../crates/brn-threads-app/src/lib.rs), [writing conversations](../../../../crates/brn-threads-app/src/writing.rs), [bounded Notes lists](../../../../crates/brn-desktop/src/threads_lists.rs), [native editor and link handling](../../../../crates/brn-desktop/src/threads_ui.rs), [existing history tile](../../../../crates/brn-desktop/src/threads_tiles.rs) and [accepted Omakase scope](../omakase-ui/spec.md).

The current Markdown renderer opens HTTP(S) links only. Generated `brn://` text links should not be presented as working native navigation. Typed relationships provide a concrete way to make evidence discoverable even when a note names its sources without inline internal links; implementing their UI navigation remains a later pass.

The native note remains **always editable**, as required by the accepted Omakase scope. The prototype's Edit drawer only demonstrates dirty-draft coordination. It does not propose a Read/Edit mode switch. Keep existing per-note drafts, selected passage comments, in-place exact review and checked Save. A dirty note must not be overwritten by a queued maintenance result, and context switching must preserve the draft.

## Prototype and observations

Primary source: separate branch [`codex/ordinary-wiki-ui-prototype`](https://github.com/ewq100/brn-rust/tree/codex/ordinary-wiki-ui-prototype), commit `63ac218c05bde67e00c45e53f32b6ed3be06b9ce`, based on `65119a73`. [Prototype and run instructions](https://github.com/ewq100/brn-rust/blob/63ac218c05bde67e00c45e53f32b6ed3be06b9ce/crates/brn-desktop/prototypes/README.md). Assets remain outside the implementation branch.

On this machine the preview is [A](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=A), [B](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=B) and [C](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=C). The single-file demo uses synthetic content and memory-only state; it never reads/writes BRN, calls a model or sends a message.

Browser preview on 13 October 2026 inspected all three structures at 1440×1000 and 720×1000. Narrow layouts stack context while preserving note navigation. Checked retained-source and unavailable-original drawers, protected reference navigation, linked conflict discussion, per-note draft preservation, arrow keys inside a textarea, disabled Undo with an unsaved draft and mock compensation as v5. Temporary viewport override was reset. Screenshots are saved with the prototype.

The controlled runtime pilot also found one wiki sample that names source dates/paths without visible inline source-record links. Its typed relationships and exact inputs passed the host checks. This supports making recorded evidence reachable in context; it does not establish that this specific UI has superior usability.

Owner variant selection, native implementation, keyboard/focus/IME behavior and real usability remain unobserved. Rewrite a selected design using native components and existing checked paths; do not promote throwaway HTML to production.
