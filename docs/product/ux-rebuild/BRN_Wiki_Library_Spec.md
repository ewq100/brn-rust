# BRN Wiki Library Specification

**Spec ID:** BRN-WIKI-001<br>
**Date:** 4 November 2026<br>
**Purpose:** Build-agent handoff<br>
**Requested outcome:** A Library of useful, maintained wiki articles, with new information improving existing knowledge.<br>
**Repository reviewed:** `ewq100/brn-rust` at [`098dc5563b27`](https://github.com/ewq100/brn-rust/commit/098dc5563b27f04a1527bf61c83317a787d0cab1).

## 1. Product goal

Make consolidated articles the main Library experience. The owner should open a subject and understand its current state without assembling the answer from many loose notes.

**Default rule: update the existing article first. Create another article only when it has a useful, separate subject.**

Consolidation means rewriting information into a coherent account while preserving important facts and evidence. Grouping fragments under a heading, adding tags, or hiding them from the list does not meet this requirement.

There is no target article count or mandatory article length. A source can improve several articles, one article, or none. Personal writing, quick notes, drafts and selected full document imports remain supported.

## 2. Starting point and implementation approach

BRN already documents maintained wiki behaviour, topic navigation, shared checked writes and agent maintenance of ordinary knowledge. The existing `maintain-notes` and `intake` skills already instruct the agent to read related knowledge and prefer updates. Extend and verify this foundation. [1], [2], [3]

An article remains an existing Note with a stable identity. A topic index remains a Note linking related articles. Use existing Sources, Links, revisions, protection, review and Undo.

The build agent must first inspect its actual branch, current task, relevant source and tests. Record a short gap table: **already works / partly works / missing / not yet observed**. Documentation and a successful build do not prove that a real agent produces good articles or that the UI is usable.

Use the active frontend and current design direction. This spec does not select a new toolkit or require a frontend migration. Coordinate the Library layout with any active design-council work and preserve its applicable design gate. Older prototype documents are context, not authority to restore an outdated layout. [1], [4], [5]

## 3. Required knowledge behaviour

### R1. Find the existing subject before writing

Search within the current access scope, read likely articles and follow relevant recorded connections. Use alternate subject names when needed. A ranked search page is not proof that no other article exists.

Choose the destination by meaning and scope, not title similarity alone. Two projects with similar names may need separate articles. The same article can appear in several topic indexes without copying its content.

If discovery is incomplete, preserve that uncertainty. Do not claim a complete search or force an uncertain merge.

### R2. Choose the right result

| Situation | Required result |
|---|---|
| New information changes or extends an existing subject | Update the existing article, retaining its identity. |
| Information supports a distinct, reusable subject | Create a focused article and useful links. |
| Input repeats information already represented | Avoid another article, repeated passages and unnecessary rewriting; a source observation may still be recorded. |
| Repeated source includes a meaningful revision | Update affected content and provenance while preserving unrelated details. |
| Information is temporary or has no durable value | Keep it in the relevant thread unless the owner requests a saved note. |
| Owner requests a full document import | Keep the faithful protected reference through the existing import path; use its information in articles when useful and authorized. |
| Sources disagree or evidence is incomplete | Preserve the disagreement or gap and use the existing attention/review mechanism where needed. |

Repeated wording is not sufficient proof of duplicate identity. A new independent source may add useful evidence even when its claim is already present.

### R3. Write readable articles

Use a clear title, a short orientation and sections appropriate to the subject. For example:

- A project article can explain current status, relevant decisions, constraints and open questions.
- A process article can explain the procedure, conditions, exceptions and examples.
- A comparison can explain agreements, differences and supported conclusions.

These are writing conventions, not compulsory templates or new record types.

Rewrite the affected explanation so the article remains coherent. Do not endlessly append source summaries. Preserve relevant owner wording and unique details. Split an article when its subjects are independently useful and separation improves reading; retain a useful overview and links where appropriate.

### R4. Preserve meaning and evidence

Material claims must remain traceable to their supporting source or exact accepted revision. Preserve relevant dates, quantities, units, exceptions, conditions, negative facts and minority evidence.

Distinguish reported claims, owner decisions and AI interpretation. Ingesting an old source today must not make it more current than newer evidence. Do not treat a rewritten summary as an independent confirming source.

Show unresolved contradictions clearly. Do not silently turn a tentative plan into a decision, remove a qualification, or use an ordinary article to replace a protected commitment.

Retain source identifiers, locations and supporting excerpts where available. Original files can remain in their existing systems. Selected full imports remain readable in BRN. If exact evidence is unavailable, show the actual limitation; do not manufacture quotations or promise recovery of an original BRN never retained. [1], [2], [3]

### R5. Follow current authority and scope

Preserve the current BRN model: ordinary knowledge may be maintained under the owner's selected delegation; review-first work remains a candidate; protected material requires applicable authority. Do not restore the historical two-vault approval model or require approval for every ordinary delegated update. [1]

Consolidation cannot move private or project-only information into a broader article. All reads, candidates, search results and writes must retain current access boundaries. Source content and retrieved instructions cannot authorize a change.

Do not change an Action's commitment or claim a real-world task is complete merely because its article was rewritten.

### R6. Make changes understandable and recoverable

Use the shared checked change path, exact input versions, stable identities, immutable candidates, editing guards and grouped commits.

A newer owner edit or unsaved draft must not be overwritten by an earlier consolidation result. A stale group must be reread and reassessed.

Report meaningful changes from actual receipts, for example: “Updated the delivery article from three sources; one conflict remains.” Show which articles changed, why, their evidence and the relevant history/Undo action. Do not claim “updated” when only a candidate was prepared.

## 4. Library experience

### R7. Open into topics and current articles

Make useful topic indexes and current articles the normal entry point. Provide search and direct access to unindexed notes, personal writing, sources and selected full references.

Use existing topic links to establish the initial article navigation. Do not make all notes depend on an automatically inferred role, title convention or mandatory taxonomy. Protection and import status do not determine whether something is a useful article.

Current articles should be easy to find in relevant results. Clearly identify references and superseded material when they appear. Preserve current search scope rules and access to underlying evidence; UI emphasis must not silently change agent retrieval authority.

### R8. Keep the article central

An article view must make these available without overwhelming the text:

- Its current explanation and useful section navigation.
- Related articles.
- Supporting sources and recorded evidence dates.
- Relevant unresolved questions or conflicting evidence.
- Last change, pending review where applicable, history and Undo.

Use real record navigation. Do not show decorative source links, unsupported internal URLs or controls that appear functional but do nothing. A typed relationship is not evidence that a claim is true.

Preserve existing editing, comments, unsaved drafts, review and keyboard navigation. Narrow layouts must keep the article usable and supporting information reachable. A global graph is not needed for this outcome.

## 5. Consolidating existing loose notes

### R9. Work on a bounded subject

Start with a selected topic or a clearly scoped instruction. Discover its related notes and identify a suitable existing article before creating another.

Prepare a coherent change containing the resulting article, source attribution, relevant link changes and any proposed retirement of ordinary notes. Explain which details were incorporated, which remain in separate references and which conflicts remain unresolved.

Keep stable article identities where possible. Preserve useful distinct notes, personal writing and full imported references. Do not archive something merely to reduce the visible note count.

### R10. Retire material only after preserving its useful content

Superseded ordinary notes may leave the current view only when their relevant information has been accounted for and current authority permits the operation. Preserve recoverable history; no hard deletion is required.

Repair affected current knowledge links and topic indexes in the same coherent group where supported and authorized. Preserve historical citations to the revision they actually cite. Do not silently retarget recorded contradictions, protected relationships or project-document links with separate owner rules.

Old references must open the retained record/history or clearly identify an applicable replacement; they must not become unexplained dead ends.

Undo must cover the article, note status and link changes in that group. If later edits prevent safe automatic Undo, retain the current data and use the existing conflict/compensation path.

Development uses synthetic workspaces. Do not automatically migrate or consolidate the owner's real Library as part of installing this feature.

## 6. Build sequence

1. **Inspect and map the gap.** Follow `AGENTS.md`, identify the active task and branch, inspect the existing intake/maintenance guides, Library navigation, change services and relevant tests. Preserve unrelated work. [5]
2. **Establish a small baseline.** Exercise representative synthetic subjects and inspect saved articles, sources, links and changes. Record what actually fails.
3. **Improve the existing agent guidance and tools where evidence warrants it.** Prefer small changes to `agent/skills/intake/SKILL.md`, `agent/skills/maintain-notes/SKILL.md` and existing services. Avoid duplicating rules across growing prompt files.
4. **Implement the Library presentation.** Work from a concrete article-view mockup within the active design direction. Reuse current components and resolve any applicable design gate before production UI changes.
5. **Complete bounded consolidation and recovery.** Reuse the normal change/review path. Add only missing capabilities demonstrated by the acceptance cases.
6. **Verify and document.** Run focused checks and the applicable repository gates; update the selected task record, behaviour documentation and evidence. Report implemented, tested, native-observed and merged status separately.

Agent judgement owns topic matching, useful synthesis and merge/split decisions. Application code owns access, protection, versions, checked writes, history and recovery. Do not create a deterministic “semantic deduplication” engine to pretend to prove that two subjects mean the same thing.

## 7. Acceptance cases

Use small synthetic fixtures with relevant facts and expected outcomes defined before evaluation. Judge the saved state, not only the assistant's final message.

| ID | Scenario | Pass condition |
|---|---|---|
| A1 | Several inputs about an existing project | The same article becomes a coherent current account; relevant unique facts and source references remain accessible. |
| A2 | Exact repeated input, then a revised version | Repetition creates no unnecessary article or duplicated passage; the meaningful revision updates affected content without losing unrelated details. |
| A3 | Similar names but different entities or scopes | Distinct subjects remain distinct; private/project information does not appear in broader articles or search. |
| A4 | Older evidence arrives after a dated correction | Current and historical claims are treated by their evidence dates and meaning, not arrival order. |
| A5 | Contradiction and a protected owner decision | The decision is preserved; competing evidence remains visible; any required decision follows existing authority. |
| A6 | Full imported reference supports an article | The article gains useful knowledge with sources; the protected reference remains intact and readable. |
| A7 | Several existing fragments consolidated | Relevant details are preserved; appropriate ordinary notes are superseded; current links work; historical citations retain their original meaning. |
| A8 | Undo, later owner edit and dirty editor | Safe Undo restores the coherent group; stale or guarded writes refuse without partial damage or lost edits. |
| A9 | Temporary question and a genuinely new topic | Temporary material need not create an article; a distinct reusable subject can create one without forced merging. |
| A10 | Library navigation and article reading | Topics/search lead to readable articles; evidence, related pages and changes open correctly; unindexed notes and references remain reachable. |

No rule requires exactly one article per input batch. A coherent set of linked articles can be the correct result.

## 8. AgentBench and improvement loop

Extend existing AgentBench cases and the native CLI evaluator where needed; do not build a separate runner or move the benchmark to another repository under this assignment. Its current documentation distinguishes controlled/offline checks from live model qualification. [6]

For model-dependent article quality:

1. Freeze the baseline, fixture and criteria.
2. Change one meaningful behaviour or UI issue at a time.
3. Compare equivalent runs with the same provider/model/effort, authority and resource limits.
4. Inspect frozen saved records and changes; use independent review for factual support and readability.
5. Keep an improvement only when it helps the intended outcome without losing facts, evidence, scope or recoverability.

Assess readability, factual coverage, attribution, temporal correctness, consolidation and successful updates separately. Record failures, latency and measured usage where available. Lower note count alone earns no success.

Use existing wiki-only reader diagnostics where useful to check whether the saved articles answer the questions without relying on loose source notes. Keep underlying sources available to graders checking support. A scripted reader or mocked model cannot establish live semantic quality.

Keep evaluation examples separate from untouched challenge cases. Honour current model/account/spending authorization; historical pilot permission is not a new budget. If live access is unavailable, finish the deterministic work and clearly name the unobserved semantic checks.

## 9. Scope limits

This assignment does not require a new store, mandatory metadata taxonomy, global graph, original-file archive, background scheduler, provider gateway, automatic whole-Library migration or a second approval system.

Reuse maintained libraries and current BRN components before adding custom machinery. Change a schema only when a concrete required outcome cannot reasonably use the existing model, and document the reason and compatibility impact.

Keep testing proportional: add tests for actual missing behaviour or regression risks, reuse existing integrity checks, and avoid tests that merely assert prompt wording.

## 10. Definition of done and handoff

The build agent provides:

- The initial gap table and a concise account of what changed.
- Relevant implementation and focused regression checks.
- Representative before/after articles with source and change evidence from synthetic data.
- Acceptance results for A1–A10, separating deterministic checks, semantic observations and UI observations.
- Evidence that protection, privacy, unique information, links and recovery remain intact.
- Updated repository task/behaviour documentation and reviewable commits or PR according to current task authority.
- Remaining gaps stated precisely; no claim of owner acceptance, live quality, merge or release without evidence.

**User-facing success:** the owner opens BRN and finds understandable articles that improve as information arrives, while sources, personal writing and history remain easy to reach.

## Repository references

These references describe the inspected baseline. Reconcile them with the active branch before implementing.

[1]: https://github.com/ewq100/brn-rust/blob/098dc5563b27f04a1527bf61c83317a787d0cab1/docs/architecture/threads-target.md#ordinary-knowledge-as-a-maintained-wiki "Canonical architecture and maintained wiki policy"
[2]: https://github.com/ewq100/brn-rust/blob/098dc5563b27f04a1527bf61c83317a787d0cab1/agent/skills/maintain-notes/SKILL.md "Existing maintain-notes skill"
[3]: https://github.com/ewq100/brn-rust/blob/098dc5563b27f04a1527bf61c83317a787d0cab1/agent/skills/intake/SKILL.md "Existing intake skill"
[4]: https://github.com/ewq100/brn-rust/blob/098dc5563b27f04a1527bf61c83317a787d0cab1/docs/work/active/ordinary-wiki/ui-design.md "Historical notes UI design; reconcile with current target and active design"
[5]: https://github.com/ewq100/brn-rust/blob/098dc5563b27f04a1527bf61c83317a787d0cab1/AGENTS.md "Repository agent instructions"
[6]: https://github.com/ewq100/brn-rust/blob/098dc5563b27f04a1527bf61c83317a787d0cab1/benchmarks/brn_agent/README.md "AgentBench implementation and qualification boundaries"
