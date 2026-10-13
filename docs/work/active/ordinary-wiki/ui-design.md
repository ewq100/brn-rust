# Notes UI design recommendation

**Scope:** design and prototype first, as selected by the owner. No native UI changed. The question is how maintained ordinary knowledge should expose its evidence, connections and changes while keeping the note readable and editable.

## Selected direction

**Owner selected B — Subject overview with evidence chronology on 13 October 2026** for the later native UI pass. Keep the subject account beside dated evidence and expose related work and exact accepted changes. Keep protection/version/unsaved status beside the title. Source reports and protected references can disagree without either silently replacing the other.

The horizontal note strip in the first B sketch does not scale to hundreds of notes. Replace it with search and bounded browsing: global title/text search with result context, Recent, recorded relationships, protected/imported references and All A–Z. Keep the current subject open behind navigation. Optional local connections and chronology details can complement this structure. The [navigation research](../../../research/notes-navigation-at-scale.md) compares subject index Notes, shallow collections/tags and bounded local connections; none is an accepted metadata or native implementation change yet.

Purpose chips in the demo are illustrative writing conventions; the current Note schema has no role field. A native pass should omit automatic chips unless a separate product decision justifies them.

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

Primary source: separate branch [`codex/ordinary-wiki-ui-prototype`](https://github.com/ewq100/brn-rust/tree/codex/ordinary-wiki-ui-prototype), commit `43d7c3bcd11c9f29a9851af9dcb58a2da9320226`, based on `65119a73`. [Prototype and run instructions](https://github.com/ewq100/brn-rust/blob/43d7c3bcd11c9f29a9851af9dcb58a2da9320226/crates/brn-desktop/prototypes/README.md). Assets remain outside the implementation branch.

On this machine the preview is [A](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=A), [B](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=B) and [C](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=C). The single-file demo uses synthetic content and memory-only state; it never reads/writes BRN, calls a model or sends a message.

Browser preview on 13 October 2026 inspected all three structures at 1440×1000 and 720×1000. Narrow layouts stack context while preserving note navigation. Checked retained-source and unavailable-original drawers, protected reference navigation, linked conflict discussion, per-note draft preservation, arrow keys inside a textarea, disabled Undo with an unsaved draft and mock compensation as v5. Temporary viewport override was reset. Screenshots are saved with the prototype.

The controlled runtime pilot also found one wiki sample that names source dates/paths without visible inline source-record links. Its typed relationships and exact inputs passed the host checks. This supports making recorded evidence reachable in context; it does not establish that this specific UI has superior usability.

B is owner-selected. Native implementation, keyboard/focus/IME behavior and real usability remain unobserved. Rewrite a selected design using native components and existing checked paths; do not promote throwaway HTML to production.

The 500-note follow-up prototype uses four worked examples and 496 clearly marked navigation placeholders. It demonstrates bounded paging, literal title/text search with snippets, Enter to open, ⌘K, Recent, linked/protected views and starting-letter browsing. This is navigation design evidence, not proof of native search relevance or performance. The current Notes tile already has title filtering, corpus keyword text search and bounded Recent/Linked to open threads/Protected & imported/A–Z lists. Note-centered links, a consolidated library overlay and any folders/tags remain proposed UI work.

The researched recommendation is search plus a few useful subject index Notes, with optional bounded local connections. Index Notes can link a subject from several entry points using existing Note/Link records. New folder/tag metadata and owner taxonomy are separate product choices; a whole-corpus graph is optional. The research identifies these as design hypotheses, not measured usability wins.

Independent follow-up review caught an abbreviated demo index that missed visible body text. Full worked-note bodies are now indexed separately from matching-context snippets; Tuesday and Medical emergencies both find and open the comparison. The scoped correction has no remaining material review findings.
