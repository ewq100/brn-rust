# Knowledge and retrieval implementation plan

**Goal:** Build the [knowledge and retrieval architecture](spec.md): a persisted, visibility-safe, English-first hybrid search; ranked search and Connections tools for the agent; topic-navigation support; and a better pinned embedding model.

**Baseline:** `main` at `56c28879`, which includes [PR #129](https://github.com/ewq100/brn-rust/pull/129) (ordinary wiki guidance) and [PR #130](https://github.com/ewq100/brn-rust/pull/130) (standalone chat stage 1), satisfying D5. **State:** ACCEPTED. The owner accepted D6 and D7, then D1–D5, on 13 October 2026, and directed the build.

**Stack:** Rust 1.98.1, rusqlite 0.40.2 (bundled; FTS5 `porter` built in), fastembed 7.1.0 behind `native`, Rig 0.44.0.

## Global constraints

- **No new crates.** Task 5 changes only pinned model assets.
- **No live provider calls in automated checks.** Downloads and semantic qualification are separate owner-requested runs.
- **Use synthetic data only,** with disposable explicit workspaces.
- **Preserve behavior:** checked writes, protection, grants, exact review, Undo, comments, chat queue/Steer/Archive, chat-only attachment isolation, export, backup/restore/upgrade, intake and delegation.
- **Core visibility is the authority.** Every result path (search, snippets, counts, connections) re-checks the core predicate. Context comes from the host, never from model arguments.
- **No role fields or taxonomy.** Topic pages stay ordinary linked notes, per the ordinary-wiki direction.
- **Ownership.** One lead owns `SearchHandle`, `search_index`, `NoteSearch` and integration. Coordinate guide edits with the ordinary-wiki bundle.
- **Delivery.** One reviewable PR per slice, each with independent review and required CI. Merging needs separate owner authority.

## Task 0 — evaluation harness and baseline (PR 1; can start now)

- [x] Deterministic synthetic English corpus at 1k, 5k and 10k notes. Notes include headings, lists, tables, long imported notes, topic pages with links, inflected forms and paraphrase pairs.
- [x] Chat-only notes and visibility cases: two synthetic chats with six chat-only notes each, probed from general, own-chat and other-chat contexts. Every returned ID is checked with core `record_in_chat`.
- [x] Golden queries (JSON, about 100) in these strata: exact name/ID, inflection, paraphrase, section-in-long-note, topic page, and abstention. Each query lists its relevant IDs.
- [x] An evaluation runner in `brn-threads-app` (the example `retrieval_eval` plus the integrity test `tests/retrieval_eval.rs`). It reports recall@10, recall@50 and MRR per stratum; query p50/p95; and build and sync timings. Output is JSON (format 2), including visibility violations and in-chat recall.
- [x] Record the current keyword baseline in [evidence](evidence.md).

## Task 1 — derived index and worker (PR 2)

- [ ] **D1:** allowlist a real `derived/` directory in `Store::open`, rejecting symlinks in it and in its ancestors. Add core tests for open, backup, restore and upgrade with and without `derived/`.
- [ ] `brn-retrieval::search_index`: schema `BRNS` v1, open-or-rebuild, atomic sibling swap, meta keys. Include `indexed_text` with insert and delete triggers for **both** FTS tables.
- [ ] Markdown chunker v1 with property tests: UTF-8 boundaries, complete coverage, stable output, and code and table integrity.
- [ ] Analyzer registry (`en` only) and a language column.
- [ ] **D2:** a search worker thread in `brn-threads-app` owning a read-only canonical connection and the index writer, plus a cloneable `SearchHandle` (request/reply). Handle shutdown, cancellation and worker failure; a failure is reported, never answered with empty results.
- [ ] Tests:
  - Upsert, remove, title-change re-chunk with no stale FTS terms, and links mirror.
  - Meta-mismatch rebuild, and refusal of foreign files.
  - Concurrent UI and agent requests.
  - CLI cross-process reader.

## Task 2 — hybrid query and app/UI/CLI switch (PR 2)

- [ ] Query pipeline: eligibility filter, exact and stemmed FTS (plus dense when present), RRF, collapse per document, core visibility re-check, and overfetch.
- [ ] Evidence quota: when a page has more than 3 results, at least 30% of slots go to matched imported references and source-linked notes. Paging returns `total`, `shown` and a `cursor`, with exact counts.
- [ ] `Workspace::search` uses the handle with the owner's context. `SearchHit` gains `heading_path`, `matches` and `inbound_links` as additive fields. Update the desktop notes filter and CLI `search` output.
- [ ] Extend `user_scale` to 10,000 notes. Check reopen without rebuild, one-note sync < 50 ms and query p95 < 150 ms; record on the target Mac and use lenient bounds in CI.
- [ ] Visibility tests: chat-only notes stay out of general and other-chat results, including snippets, counts and Keep-in-BRN eligibility after a version-checked Keep.
- [ ] Rerun Task 0 and record the deltas.

## Task 3 — agent tools (PR 3)

- [ ] **D4:** `NoteSearch` trait and `ThreadRunRequest.search`. The desktop and CLI bind a handle to the run's chat. When the handle is absent, the tools are not registered; there is no silent substitute.
- [ ] Add `search_notes` and `note_connections` to the runtime catalog, with strict schemas, byte and limit validation, compact output and activity strings. Add tests for scope binding: a model argument cannot widen the context.
- [ ] Guide updates on top of PR #129's bundle:
  - Rewrite `maintain-notes` step 1 and the `BRN.md` discovery paragraph so they say: search with `search_notes` using 2–3 reformulations, use `note_connections` for neighbourhoods, then read sections.
  - Keep #129's provenance model (input versions, prose attribution). Add no citation syntax.
  - Recommend relations `source`, `supersedes`, `contradicts` and `related`, and keep other concise names allowed (D7).
  - Explain the root Topics note: link every topic page from it, keep it current, and leave notes that are not topic pages out (D6).
  - Bump the guide identity once.
- [ ] Live before/after comparison by **reusing #129's `scripts/evaluate-wiki-guidance.py` harness**. The arms are the current tools and the tools plus search, with an identical selected model, effort, 8 rounds and repetitions. Add 1k- and 5k-note seeded scenarios so retrieval actually matters. Running it is an owner-requested provider run; the offline seed and structural checks are part of CI.
- [ ] Scripted fixture journeys at 10,000 notes:
  - Retrieval within 3 rounds.
  - Maintenance (search, read, prepare, apply) within 8 rounds.
  - A stale-candidate refresh.
  - The agent's own applied note is searchable in the same run.
  - A private attachment is invisible from another chat.

## Task 4 — topic navigation support (PR 4)

- [ ] Native Connections: bounded neighbourhood (8 per hop, deliberate second hop) through the handle, with a readable list beside the graph, following the ordinary-wiki B flow. The Connections graph also gets a separate UI task on the native shell. Known relations get labels and filters; unknown names are shown as written.
- [ ] **D6, root Topics note:**
  - Add an optional serde-default `WorkspaceSettings.topics_note`, with no SQL change. Setting it uses owner-only configuration.
  - The app creates the note on first use as an owner action. The Topics tile lists its links in order.
  - The host passes the note's ID in run context.
  - Tests: old settings serialize unchanged; missing, archived or invisible pointer targets produce an honest empty state; replay; Undo of note content.
- [ ] **D7, recommended relations:** shared constants in the app (`source`, `supersedes`, `contradicts`, `related`) used by Connections, the evidence quota and the knowledge check. Core does not validate relation names.
- [ ] Search results and the notes list show inbound-link counts, and subjects can be sorted by them.
- [ ] Optional **Knowledge check** (owner-triggered from the desktop and CLI). Its deterministic findings follow #129's list:
  - valid references and links to missing or archived notes;
  - recorded input versions that are no longer current;
  - version conflicts;
  - source coverage gaps;
  - orphans;
  - duplicate or near-duplicate titles and content.

  The findings seed one chat for `maintain-notes`'s bounded review. Test the findings and the visibility of the seeded chat.

## Task 5 — dense upgrade (PR 5)

- [ ] **D3:** a pinned `multilingual-e5-small` profile (revision, sizes, digests, prefixes, 512 tokens), keeping the exclusive installer and its consent rules. Update the installer tests with fixture bytes.
- [ ] Embedding queue inside the worker: bounded, cancellable, progress reported, vectors keyed by model, in-memory brute-force matrix.
- [ ] Owner-run qualification (requested separately): install the model, embed the 10k corpus, and run the dense and hybrid lanes. Record timings and memory.

## Task 6 — cleanup (PR 6)

- [ ] Retire the vault-era `NoteIndex` metadata, UUID, edge-proof and reader code once it has no callers.
- [ ] Separately assess the legacy `ReadTools::search_notes` path in `brn-ai/src/tools.rs`, which only provider-format tests use.

## Projects stage hooks (not built here)

When projects are planned, extend the following and add their tests in that plan:
- `visibility` with `project:<id>`.
- Indexing of `kind=message` for earlier-chat search.
- Eligibility for the "Use BRN knowledge" setting.

## Verification per PR

- `cargo fmt --all -- --check`.
- `cargo clippy` and `cargo test` for each affected crate with `--locked --offline`.
- The `native-retrieval` lane when the index or embeddings change.
- The native widget/state tests, with a nonzero count, when the desktop changes.
- `git diff --check` and `python3 scripts/check-markdown-links.py`.
- Record the commands, tree, features, results and limits in `evidence.md`. Native observation and owner acceptance are reported separately.

## Plan review record

Round one (independent, read-only) raised three blocking findings:
- FTS external content must index a stored `indexed_text` with triggers for both tables.
- Cross-thread search needs a worker that owns its connections.
- A core `Note.role` was sequenced too late and touched protection and strict request validation.

The first two are adopted (Task 1, D2). The third is removed: the owner-selected ordinary-wiki direction (PR #129) keeps roles as conventions. An added 8-round maintenance journey answers the tool-budget finding.

The PR #129 comparison led to these alignments:
- Provenance and relation strings follow #129.
- #129's live harness is reused.
- Its research added an evidence quota and exact-count paging.
- Knowledge-check findings match #129's deterministic list.
- Guide edits target #129's text.

A re-review is due before Task 1 implementation, now that D1–D5 are accepted and #129 and #130 are merged.

## Gate record

- Defined: the owner requested this plan after the research, the English-first decision, and the move to Chats and Projects.
- Accepted: D6 and D7, then D1–D5, by the owner on 13 October 2026, with direction to build. #129 and #130 merged the same day (D5); the branch was rebased onto `56c28879`.
- Merging any slice still needs separate owner authority.
