# Host-driven intake

**State:** PROPOSED prototype, measured once (see [evidence](evidence.md#host-driven-intake-prototype-14-october-2026)). Not accepted, not merged. Branch `work/host-intake`; guide bundle `brn-threads-guidance-8`.

## Problem

Intake is predictable: read the source, find related pages, decide update, create or keep in the chat, write. It runs as an open agent loop of 5–12 model turns and 60–230k input tokens, because every turn resends the preamble, playbooks and earlier tool results. The [compact-intake trace](evidence.md#compact-intake-s1-rerun-14-october-2026) shows the same opening in every intake: `read_source`, two or three searches, a listing, then reads.

## Idea

Mechanics are the host's job; meaning is the model's. Before the first turn of a run that has supplied sources, the host runs the retrieval a careful agent would run, deterministically and without a model, and appends the result to the first prompt. The model can then call `prepare_changes` in round 1 and `apply_changes` in round 2. Every tool stays available, so a model that needs more can search or read in the same run, with the same provider and model.

## What the host retrieves

Implemented in `crates/brn-ai/src/threads_intake.rs`, called from `run_prepared` and `run_prepared_scripted`. Both the CLI (`brn intake --intent useful`) and the desktop run through these, so neither needs a change.

1. **Source.** The same JSON `read_source` returns, so the Source record id and version, coverage, fingerprint and rewritten figure links are exact. Text is capped at 24 KB across all sources (at least 4 KB each); a marker gives the bytes shown and tells the model `read_source` returns the rest.
2. **Queries** (at most 6 across sources, deterministic, no model):
   - a *lead* query: the first 300 bytes of prose, cut at a word boundary, run as hybrid;
   - a *terms* query: the 24 most salient terms, run as keyword;
   - a *names* query: capitalised names and first-line tokens, run as keyword.
   Salience is log frequency, a small length bonus, +1.5 for a capitalised word that is not sentence-initial, +2 for a capitalised first-line word, −0.8 for pure numbers. Tokens under 3 characters and English and Estonian stopwords are dropped. The lead query is skipped when no salient term remains.
3. **Fusion.** Each query is searched with `RunNoteSearch::search` for the run's chat (8 results). Rankings are fused by reciprocal rank (k = 60); ties break by title, then id.
4. **Pages.** The top 3 live notes (not archived, not superseded) get their Markdown, at most 8 KB each and 20 KB together. A truncated page carries `next_start` and up to 24 headings, so `read_note` continues at that version. Each page also lists up to 8 one-hop neighbours from `RunNoteSearch::connections` (id, version, title, kind, relations, inbound links).
5. **Also found.** Up to 5 further hits as id, version, title, heading path and a 160-byte snippet.
6. **Status.** The queries run, the semantic lane status and whether retrieval was complete, partial or unavailable.

The whole context was 4–8 KB for the S1 sources and is bounded below 64 KB for any input (tested).

## Version pinning

Each page shows `id` and its current `version`. The header text tells the model to cite a page it relies on as a `prepare_changes` input at exactly that version, with the Source record at its shown version. Nothing here relaxes the checked boundary: `prepare_changes` still verifies input versions, `apply_changes` still rejects a stale candidate, and authority, approval and the protected-note rules are unchanged. The context is not a "read" in any tracked sense; it only saves the model the calls.

## Privacy and scope

- Search runs with the run's chat; the real handle applies chat and project visibility.
- Page text is read through `records_for_chat(Some(chat))` whatever the index returned. A note that chat cannot see never reaches the prompt or the trace, even if an index returned it.
- Trace `host_context` records counts, ids, versions, sizes and milliseconds at `Summary`; query text only at `Full`. It holds no note content.
- Retrieval never writes and never contacts a provider.

## Guide change

`brn-threads-guidance-8` adds step 0 to the intake playbook: when a Host intake context is present, work from it first; do not re-read what is shown unless marked truncated; skip catalog listings; search further only when the needed related page is missing; prepare in round 1 and apply in round 2; keep the source in the chat when it adds nothing durable. The 12-round default for source runs stays, as the fallback budget. The base guide and `maintain-notes` are unchanged.

## Fallback to tools

- No search handle: the source is still embedded and the prompt says retrieval is unavailable and to use `read_records`.
- A search error on one query: the others still count and the status is `partial`.
- A failure building the context: the plain prompt runs, exactly as before this change.
- The model can always call `search_notes`, `note_connections`, `read_records`, `read_note` and `read_source`. Absence from the context is never presented as absence from the wiki.
- No automatic switch of provider, model or account at any point.

## What stays model-decided

Whether a page is truly related; update versus create versus keep in the chat; which facts, qualifications and dates to keep; link relations; Actions and follow-ups; Topics changes; whether to search further. The host decides only which pages to show and how much of each.

## Tests

- Unit (`threads_intake.rs`): names and the first line beat filler; deterministic, bounded, empty text yields no query; Estonian one-liner.
- Integration (`crates/brn-threads-app/tests/host_intake.rs`, real search worker, scripted model): the first prompt carries the source and the related page at its version; a scripted model completes an intake in two rounds with no search or read; another chat's private note never reaches the prompt or trace; source and page sizes are capped with the rest reachable; no search handle leaves the source and the tools; a chat without sources gets no context.

## Risks and open questions

- The measured model still searched and listed records in four of five intakes (see evidence); the guide wording alone did not remove the habit. Guidance or tool shaping may be needed, for example making a catalog listing unavailable once context is present.
- There is no relevance floor: a weak match still fills a page slot. Showing fewer pages when scores fall away may save tokens.
- Estonian sources match English pages only through names and the dense lane; the measured arm ran without the embedding model.
- A very large Topics note could appear as a related page; the 8 KB page cap bounds it.
- The preamble is resent each turn, so turn count remains the main lever; this change cuts turns, not the floor per turn.
- Desktop uses the same path but was not exercised interactively.
