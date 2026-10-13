# Navigating hundreds of BRN notes

**Sources checked:** 13 October 2026. **Inspected checkout:** `codex/ordinary-wiki-guidance`, HEAD `237e460d`; recorded `origin/main` baseline `65119a73`. **Scope:** research for the owner-selected B subject overview with evidence chronology; design/prototype only. Recommendations below are design inferences, not accepted schema changes or demonstrated usability improvements.

Use B as the reading destination, with a searchable library and a few subject index Notes as entry points. Add an optional bounded local connections view. Folders and tags can be useful later if repeated navigation tasks justify their maintenance; a global graph should remain optional.

## What the sources establish

Navigation involves different tasks: finding a remembered note, browsing a subject without knowing its title, and discovering relationships. One mechanism need not handle all three.

Obsidian's official documentation establishes capabilities: file/folder browsing, keyword and property search, multiple tags, backlinks, and global/local graphs. It does not provide a comparative usability study showing that graphs or categorization improve retrieval across hundreds of notes. Its graph filters and depth controls are mechanisms for reducing displayed scope, not proof of scalability. [Graph view](https://obsidian.md/help/plugins/graph), [Search](https://obsidian.md/help/plugins/search).

A 2005 study of project organization found that folders represented people's evolving understanding of projects, while also accumulating overloaded meanings and tensions between current use and reuse. This supports taking the owner's organization seriously; it does not establish that a folder tree is best for BRN. [Jones et al., university repository abstract](https://digital.lib.washington.edu/researchworks/items/806438cb-e92d-4bbf-b842-8f57bf1a27a7).

Microsoft's first-party Phlat report describes combining keyword search with metadata browsing to accommodate remembered topic, person, location, and time. It reports deployment with evaluation still underway, so it supplies a design precedent rather than a measured win for this proposal. [Cutrell and Dumais, *Exploring Personal Information*](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ExploratorySearchAndPIM.pdf).

## Three approaches

### 1. Searchable library plus subject index Notes

Keep the library beside B's subject overview. Search results should show titles and matching text, with visible result counts and ways to reach omitted results. Retain Recent, Linked to open threads, Protected & imported, and A–Z as useful entry points. A paged 500-entry synthetic library can exercise this presentation without displaying 500 items simultaneously.

Create a small number of ordinary index Notes only where useful: for example, **Harbor project**, **Procurement**, and **Estonia operations**. Each can explain its scope and link to relevant subject Notes. The same note can be reachable from several indexes without duplication. A meaningful link says more than a bare category membership: why does this note belong here, and what should the reader open next?

This fits BRN's existing Note and Link records. Corpus keyword search already indexes current note titles and Markdown; the Notes tile also filters titles. Index Notes introduce no mandatory page-role field. Native navigation from recorded Links would still need presentation work; current Markdown rendering does not make generated `brn://` links functional. [Search implementation](../../crates/brn-threads-app/src/lib.rs), [Notes lists](../../crates/brn-desktop/src/threads_lists.rs), [UI mapping](../work/active/ordinary-wiki/ui-design.md).

**Tradeoff:** indexes can become stale or excessively broad. Maintain only indexes that answer recurring questions, and keep direct search available when a note has no index membership.

### 2. Shallow folders/collections with optional multi-topic tags

A proposed folder hierarchy would give each note one primary home, such as **Projects → Harbor**. This provides a stable browsing path when the owner remembers where something belongs. Obsidian's file explorer supports nested folders, moving notes, sorting, revealing the active file, and searching a folder. [File explorer](https://obsidian.md/help/plugins/file-explorer).

One home becomes awkward for a note spanning Harbor, procurement, and Estonia. Multiple tags or collection memberships can express those overlapping topics. Obsidian supports multiple tags and nested tag names; properties provide typed values including lists, dates, and links, and Search can combine text with tag/property filters. [Tags](https://obsidian.md/help/tags), [Properties](https://obsidian.md/help/properties), [Search](https://obsidian.md/help/plugins/search).

For BRN, start with a few owner-defined collections and optional facets rather than requiring every note to be classified. Keep unclassified notes accessible. Avoid mixing topic, document purpose, protection, and truth in one hierarchy: **Procurement** is a topic; **protected** is an editing boundary; **imported** describes provenance.

**Tradeoff:** naming, aliases, merges, moves, and stale membership all need maintenance. BRN has no folder/tag/role metadata contract today. A future implementation must define membership, ordering, deletion, export, and checked writes. Prototype chips remain synthetic; Markdown hashtags should not silently become authoritative metadata.

### 3. Bounded local connections, with an optional global map

Obsidian's global graph can display the vault's notes and links; its local graph starts from the active note and expands by depth. Backlinks list incoming references and distinguish explicit linked mentions from unlinked name occurrences. [Graph view](https://obsidian.md/help/plugins/graph), [Backlinks](https://obsidian.md/help/plugins/backlinks).

For BRN, offer **Connections** from the subject overview: direct incoming/outgoing Links first, relation labels, source/subject distinctions, a disclosed display limit, and deliberate expansion to a second hop. Provide the same connections as a readable list so navigation does not depend on interpreting spatial layout.

Use existing Link endpoints and relation strings; those strings are not a closed ontology. A connection is recorded context, not proof that one claim supports another. Keep evidence inspection tied to actual source/revision provenance.

**Tradeoff:** unlinked relevant notes remain undiscoverable through this view, and many links can crowd even a local neighborhood. A global map may help inspect isolated notes or broad clusters, but its layout and node prominence should not imply semantic importance or evidence strength. Treat any navigation benefit as a hypothesis to test.

## Recommended hybrid and boundaries

Combine approach 1 with the local part of approach 3: **library → subject overview → evidence chronology / related Notes**. Use a few index Notes for browsing; offer direct Connections when the reader wants to follow relationships. Consider approach 2 only after tasks expose a missing, stable organization dimension.

The chronology must distinguish evidence effective/document dates, source observation time, and BRN revision time. An old document imported today is older evidence. Unknown dates stay unknown; unavailable originals and retained excerpts remain visibly different. [Evidence mapping](../work/active/ordinary-wiki/ui-design.md).

AI can propose index updates, memberships, and links with explanations. Its taxonomy may split synonyms, merge different meanings, overclassify passing mentions, or present generated organization as owner intent. These are design risks, not observed results from this research. Keep proposals bounded and inspectable under existing authority; do not mass-reclassify the corpus to populate a visualization.

SQLite remains authoritative. Organization changes must use the shared checked service, applicable protection/authority, exact candidates, version checks, receipts, and Undo. Navigation must preserve per-note drafts. Guidance roles remain writing conventions. [Invariants](../architecture/invariants.md), [Assignment boundaries](../work/active/ordinary-wiki/spec.md).

Compare synthetic 100-, 500-, and 1,000-note sets using concrete tasks: reopen a remembered note, find a multi-topic note, trace a disputed statement to evidence, and locate an unclassified note. Record completion, wrong turns, discovery failures, and organization upkeep. Browser prototype observations establish design behavior only; native usability and owner acceptance remain separate.
