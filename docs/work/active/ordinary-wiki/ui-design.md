# Notes UI design recommendation

**Scope:** design and prototype first, as selected by the owner. No native UI changed. The question is how maintained ordinary knowledge should expose its evidence, connections and changes while keeping the note readable and editable.

## Selected direction

**Owner selected B — Subject overview with evidence chronology on 13 October 2026** for the later native UI pass. Keep the subject account beside dated evidence and expose related work and exact accepted changes. Keep protection/version/unsaved status beside the title. Source reports and protected references can disagree without either silently replacing the other.

**Follow-up owner choice, 13 October 2026:** topic pages and search are the main navigation; a bounded local graph lives under **Connections**. B now shows adjacent **Topics & Search**, **Topic page / Subject overview**, and **Evidence / Connections / Changes** tiles. A topic page is an ordinary index Note explaining its scope and linking useful subjects; the same subject can appear in several indexes. Search remains available for notes without index membership. The [navigation research](../../../research/notes-navigation-at-scale.md) supports this as a design hypothesis; the owner accepted the direction, not a demonstrated usability improvement.

The owner requested the tiled UX of [Omarchy](https://omarchy.org). Its [official navigation manual](https://omarchy.org/manual/navigation/) describes adjacent windows, focus, rearrangement, resizing and maximization. B adapts these interactions within BRN's accepted Omakase shell: independent tile scrolling, visible focus, header swap/maximize/close controls and gap resizing. On narrow windows the row scrolls; selecting a subject reveals its tile, and maximize uses the available width. Keep the reading path and Previous note reachable.

Try **Harbor project → Harbor · current delivery plan → Evidence → Connections → Loading requirements**. Alternatively search **Jaan** and open the matching subject with Enter. Connections starts with direct recorded links, offers a deliberate second hop and optional source records, and discloses an eight-neighbor display limit with exact counts. Nodes open/recenter the selected subject; source nodes inspect retained passages. A readable connection list shows actual direction and relation strings. Link prominence or spatial position must not imply evidence strength. No folder/tag schema or global graph is required by this choice.

Purpose chips in the demo are illustrative writing conventions; the current Note schema has no role field. A native pass should omit automatic chips unless a separate product decision justifies them.

| Variant | Information hierarchy | Practical tradeoff |
|---|---|---|
| A | Subject list → readable account → evidence/related/history rail | Most direct extension of the current tiled workbench; rail stacks below text at narrow widths. |
| B | Topic pages/search → subject account beside evidence chronology → optional Connections | Adjacent navigation/context uses width; tiles can be resized or maximized. |
| C | Local evidence and related-note navigation → account with passage margin | Makes support/navigation visible; consumes more reading space and can overstate relationships if unqualified. |

## Current implementation and proposed presentation

| Proposed control | Existing data or checked path | Required later UI work |
|---|---|---|
| Topic pages and search | Existing Note/Link records, current corpus keyword search and bounded Notes lists | Present useful index Notes and matching-context results; preserve direct access for unindexed notes. No new role/folder/tag field. |
| Working/protected vN and unsaved writing | `Record.version`, `Note.protected`; editor/dirty guards in `threads_ui.rs` and `threads_writing.rs` | Retain the current header and Save/Discard behavior. Protection is independent of role, import and truth. |
| Evidence rail | Recorded `Link` endpoints/relation, `Note.import.source`, `Source` label/locator/observed_at/outcome/report; saved imported Note text | Add a bounded note-centered projection and source/detail navigation. Ordinary sources have metadata, not guaranteed retained bodies. |
| Inspect a passage | Current/historical Note revisions and existing coverage details | Display retained text and its actual revision/location when available; open an external locator separately. A quoted excerpt is not a full retained original. |
| Original availability | Source observation outcomes and coverage gaps when recorded | State the specific recorded observation/time and scope. “Original unavailable” requires evidence of an attempted access; metadata alone cannot imply availability, freshness or complete coverage. |
| Connections | Existing `Link` endpoints and relation strings, Source references | Project a bounded local neighborhood with shown/available counts, direct/second-hop distinction and a readable list. Open notes and retained evidence through actual record navigation. No inferred ontology or embedding graph. |
| Ask / decision thread | `Workspace::writing_threads`, `link_writing_thread`, existing per-note composers and authority/review path | Reuse the current linked chat; a conflict banner must come from supported context. Opening or drafting a thread supplies no decision authority. |
| What changed / Undo | `Workspace::history`, receipt before/after versions, `Workspace::undo`, existing exact review and refresh follow-up | Filter receipts by affected note; inspect the entire coherent operation group. Undo may affect several records, records compensation and may require reviewed compensation after later changes. |
| May need refresh | Receipt `needs_refresh`, existing refresh records/follow-up | Surface recorded dependencies, not a universal “fresh” badge. Rechecking versions does not establish world freshness. |

Source inspected: [core types](../../../../crates/brn-threads-core/src/types.rs), [workspace/history/Undo](../../../../crates/brn-threads-app/src/lib.rs), [writing conversations](../../../../crates/brn-threads-app/src/writing.rs), [bounded Notes lists](../../../../crates/brn-desktop/src/threads_lists.rs), [native editor and link handling](../../../../crates/brn-desktop/src/threads_ui.rs), [existing history tile](../../../../crates/brn-desktop/src/threads_tiles.rs) and [accepted Omakase scope](../omakase-ui/spec.md).

The current Markdown renderer opens HTTP(S) links only. Generated `brn://` text links should not be presented as working native navigation. Typed relationships provide a concrete way to make evidence discoverable even when a note names its sources without inline internal links; implementing their UI navigation remains a later pass.

The native note remains **always editable**, as required by the accepted Omakase scope. The prototype's Writing context tile only demonstrates dirty-draft coordination. It does not propose a Read/Edit mode switch. Keep existing per-note drafts, selected passage comments, in-place exact review and checked Save. A dirty note must not be overwritten by a queued maintenance result, and context switching must preserve the draft.

## Prototype and observations

Primary source: separate branch [`codex/ordinary-wiki-ui-prototype`](https://github.com/ewq100/brn-rust/tree/codex/ordinary-wiki-ui-prototype), commit `61f33b954591036c7e5b3230fcf8159442f31de0`, based on `65119a73`. [Prototype and run instructions](https://github.com/ewq100/brn-rust/blob/61f33b954591036c7e5b3230fcf8159442f31de0/crates/brn-desktop/prototypes/README.md). Assets remain outside the implementation branch.

On this machine the preview is [A](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=A), [B](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=B) and [C](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=C). The single-file demo uses synthetic content and memory-only state; it never reads/writes BRN, calls a model or sends a message.

Browser preview on 13 October 2026 first inspected all three structures at 1440×1000 and 720×1000. The accepted tiled B follow-up was inspected at those sizes and the default 374×853 viewport. Checked topic → subject → evidence, literal remembered-detail search/Enter, graph recentering, depth/source controls, the eight-neighbor bound, retained passages, protected references, separate note/thread drafts, dirty Undo guards, header swapping, gap resizing, close/reopen and maximized keyboard focus. Narrow subject tiles fit the available width and scroll independently. Temporary viewport overrides were reset; screenshots and run instructions are saved with the prototype. This is browser design evidence, not native qualification, search performance evidence or a usability study.

The controlled runtime pilot also found one wiki sample that names source dates/paths without visible inline source-record links. Its typed relationships and exact inputs passed the host checks. This supports making recorded evidence reachable in context; it does not establish that this specific UI has superior usability.

The direction is owner-selected; native implementation and interaction acceptance remain separate. Rewrite it using native components and existing checked paths; do not promote throwaway HTML to production. Preserve always-editable notes, per-note writing/comment drafts, per-thread replies, exact review, protection and grouped receipt Undo. Context navigation supplies no writing or decision authority.

The 500-note demo now contains seven worked Notes (three topic indexes and four subject/reference pages) plus 493 marked placeholders. Full worked-note text is indexed independently from matching-context snippets. It demonstrates twelve results per page, counts, search/Enter, ⌘K and Recent. Current native Notes already provides title filtering, corpus keyword text search and bounded Recent/Linked to open threads/Protected & imported/A–Z lists. Topic-first presentation, recorded-link navigation and the local graph remain later native UI work; new folders/tags are outside this selected pass.

Scoped independent reviews corrected incomplete demo body indexing, thread replies lost on rerender and focus cycling to a hidden tile during maximization. The current B candidate has no remaining material review findings. These corrections affect only the throwaway prototype and its evidence; packaged runtime guidance and the provider comparison are unchanged.
