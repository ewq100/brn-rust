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

- [x] **D1:** allowlist a real `derived/` directory in `Store::open`, rejecting a symlink or non-directory there and symlinks in its ancestors. `Store::derived_directory()` creates it lazily, on first search. Core tests cover open, backup, restore and upgrade with and without `derived/`. Rollback caveat: a build older than this one refuses a workspace that has `derived/`; delete the folder to go back.
- [x] **Core reader (re-review B1–B3):** `Store::open_reader` gives a validated read-only `Snapshot`. It refuses schema 3 instead of upgrading it.
  - `watermark()` is `max(rowid)` of `revisions`. Every record write adds a revision in the same transaction, and nothing deletes or vacuums them.
  - `changed_since`, `workspace_identity` and `with_references` (the by-ID closure that `visible` follows).
  - `search_eligibility`, derived from `visible` itself. A property test proves it equals `records_for_chat` for random mixes of chat notes, chat Sources, imports, archived and superseded notes.
- [x] `brn-retrieval::search_index`: schema `BRNS` v1 with meta keys. `indexed_text` carries insert and delete triggers for **both** FTS tables, and passages are replace-only.
  - Adopted from S2 instead of the planned sibling swap: an advisory `search.lock` serializes opens across processes. An outdated index is rebuilt **in place** in one transaction, a damaged one is replaced, and a foreign file is refused unchanged.
  - The sync watermark and workspace identity live in `meta`.
- [x] Markdown chunker v1 (`markdown_chunk`, written by a helper and reviewed by the lead). A 2,500-document randomized property test covers coverage, UTF-8 and CRLF boundaries, the cap, determinism, heading starts and paths, and fence and table integrity.
- [x] Analyzer registry (`en` only) and a language column. An unregistered language is refused.
- [x] **D2:** a `brn-search` worker thread in `brn-threads-app::search`, owning the snapshot reader and the index writer.
  - Callers use a cloneable `SearchHandle`: `request` returns a non-blocking `SearchTicket` (dropping it cancels, per S3) and `search` blocks.
  - Each request syncs by watermark. A Source change re-derives the notes that import it.
  - Results are collapsed per note and re-checked with core `searchable_notes`. Snapshots are retried when another process moved the index.
  - Failures are errors, never empty results.
- [x] Tests:
  - Upsert, remove, title and content changes with no stale FTS terms (FTS5 `integrity-check`), and the links mirror.
  - Meta and version mismatch rebuild, corruption replacement, and refusal of foreign files and symlinks.
  - Concurrent handles, four concurrent index writers, and a second workspace view sharing the index.
  - Restart, deletion and re-creation of the index.
- [x] Harness retriever `worker-keyword` recorded in [evidence](evidence.md).

## Task 2 — hybrid query and app/UI/CLI switch (PR 2)

- [x] Query pipeline: eligibility filter, exact and stemmed FTS (plus dense when present, Task 5), RRF, collapse per document, core visibility re-check, and overfetch.
  - Added from the Task 1 finding: a third lane requires every query word (stemmed) in one passage. It is a subset of the stem lane and supplies a note's passage when it matches. Inflection recall@10 went from 0.30 to 0.75.
  - Lanes skip passage text; text is fetched once for the fused notes.
- [x] Evidence quota (`search::arrange`): when a page has more than 3 results, at least 30% of its slots go to matched evidence (imported notes, and targets of visible `source` links) while unplaced evidence remains. Pages keep rank order, and every result appears once.
- [x] Paging returns an exact `total` (SQL union of both lanes over eligible documents), `offset`, `reachable` (the fused list of at most 100 notes) and an opaque `cursor`. A cursor for another page size is refused.
- [x] `Workspace::search_handle` starts the worker lazily; `Workspace::search_page` takes the context, limit and cursor. `Workspace::search` returns the first 30 general results. The in-memory `NoteIndex` is gone from `Workspace`. `SearchHit` gained `heading_path`, `snippet`, `highlights`, `matches`, `inbound_links`, `protected`, `imported` and `evidence` as additive serde-default fields.
  - Inbound-link counts include only links from documents visible in the same context.
  - CLI: `search --query TEXT [--limit 1..50] [--cursor NEXT] [--thread CHAT]` prints the page with `total`, `shown`, `offset`, `reachable` and `cursor`.
  - Desktop (S3): requests are awaited on the background executor and never block typing. A newer query cancels the older one, and stale answers are ignored by generation. The index is prepared at startup ("preparing search…"). Results show exact counts (`N matches · 1–12`), 12 per page with Previous/Next, heading path, passage count, imported/protected labels, inbound links and highlighted matching context.
  - A `derived/` directory or index removed while running is re-created on the next request (`StoreReader::derived_directory`).
- [x] `user_scale` now covers 10,000 notes: reopen without rebuild, one-note sync < 50 ms and query p95 < 150 ms. Strict bounds apply with `BRN_SCALE_STRICT=1` on a release build; CI uses 40× bounds.
- [x] Visibility tests: chat-only notes stay out of general and other-chat results, snippets, counts and inbound-link counts. Keep-in-BRN eligibility after a version-checked Keep (and after its Undo) is covered through `Workspace::search` in `tests/chat.rs`.
- [x] Rerun Task 0 and record the deltas in [evidence](evidence.md#task-2--query-paging-and-app-switch).

## Task 3 — agent tools (PR 3, shipped with Task 4)

- [x] **D4:** `brn-ai` defines `RunNoteSearch` (named so it does not clash with the vault-era `NoteSearch`), and `ThreadRunRequest.search` carries it. `brn-threads-app` implements it for `SearchHandle` (a new `brn-threads-app` → `brn-ai` edge; `brn-ai` still does not depend on retrieval). The tools always pass the run's host-validated chat, never a model argument. When the handle is absent, the tools are not registered; the desktop shows a notice and the CLI reports `search: unavailable` with the reason.
- [x] `search_notes` `{query, limit 1–20, mode hybrid|keyword, cursor}` and `note_connections` `{id, hops 1|2, sources}` with strict schemas (`deny_unknown_fields`, nullable cursor required), byte and limit validation, compact output (`semantic: none` until Task 5) and activity strings. Tests cover registration only with a handle, binding to the run chat (an extra `chat` argument rejects the batch), validation and hidden notes refused like missing ones.
- [x] Guide updates on top of PR #129's bundle (bundle `brn-threads-guidance-4`):
  - `BRN.md` discovery and `maintain-notes` step 1 now search with `search_notes` (2–3 reformulations, cursor), use `note_connections`, then read sections; `read_records` stays for exact IDs and is the fallback when search is not offered.
  - #129's provenance model is unchanged; no citation syntax.
  - D7 relation names are recommended, other concise names allowed.
  - The root Topics note is explained (D6); the host names it in run context.
- [ ] Live before/after comparison with #129's `scripts/evaluate-wiki-guidance.py` (bundle 3 against bundle 4 with search, identical model and settings, at 1k/5k seeded notes). This is an owner-requested provider run and is not part of this build.
- [x] Scripted fixture journeys at 10,000 notes (`tests/agent_journeys.rs`, a scripted model through the real runtime tools and the real worker, behind the test-only `scripted-runs` feature):
  - Retrieval: search, then read the target section, within 3 rounds.
  - Maintenance: search, read, prepare and apply within 8 rounds; the applied change is found by search in the same run.
  - A stale candidate is reported Stale, refreshed from current search results and applied.
  - Another chat's private note is invisible to search and Connections.

## Task 4 — topic navigation support (PR 4, shipped with Task 3)

- [x] Native Connections tile: the open note's bounded neighbourhood (8 per hop, deliberate second hop, optional sources) through the worker, as a readable list with exact shown/available counts. Known relations get labels and filters ("other" groups the rest); unknown names are shown as written. Clicking a note opens it and recenters. Every node is visibility-checked (`Snapshot::visible_records`, `searchable_notes`). A drawn graph remains a separate native-shell UI task.
- [x] **D6, root Topics note:**
  - `WorkspaceSettings.topics_note` (serde default, omitted when unset; no SQL change), set only through owner configuration (`Store::choose_topics_note`): it must name a current general note, is replay-checked, preserved by `configure`, and deferred while a run works.
  - "Create Topics note" in the notes tile (and CLI `topics-create`) creates the note as an owner action. The Topics section lists the topic pages the note links to, in the order the note mentions them, with link counts and descriptions.
  - The host passes the note's ID, version and title in run context when the run's chat can see it.
  - Tests: old settings serialize unchanged; missing, archived and chat-only targets give an honest empty state or are refused; replay; Undo of the note's content keeps the pointer.
- [x] **D7:** `brn_threads_app::relations` constants used by Connections, the evidence quota and the knowledge check. Core does not validate relation names.
- [x] Search results and the notes list show inbound-link counts (`← N`), and a "Most linked" section sorts subjects by them.
- [x] Owner-triggered **Knowledge check** (desktop overview and CLI `knowledge-check [--seed-chat yes]`) over general knowledge only: broken links, stale recorded inputs, stale candidates, coverage gaps, orphans, duplicate titles and content, contradictions, unapplied supersession, relation-name variants and pages that look like unlisted topic pages. Counts are exact; 25 findings are kept per kind. The findings seed one chat for `maintain-notes`'s bounded review. Tests cover each finding, that no chat-only material appears, and the seeded chat.

## Task 5 — dense upgrade (PR 5)

- [x] **D3:** a pinned `multilingual-e5-small` profile: `Xenova/multilingual-e5-small` at revision `761b726d…` (ONNX of MIT-licensed `intfloat/multilingual-e5-small`), exact sizes and digests, `query: `/`passage: ` prefixes, 512 tokens. The exclusive installer and its consent rules are unchanged; the owner runs `brn model-install --destination …` in a native build. The previous MiniLM pin stays recognised and verified. The installer tests check the new manifest and keep the MiniLM one. The Xenova card declares no licence of its own (as for the MiniLM pin); the base model is MIT.
- [x] Embedding queue inside the worker: idle-time batches of 16 passages, each followed by a check for queued requests (so one waits at most one batch). Progress is reported (`SearchHandle::embedding_progress`), vectors are stored per model as unit-length little-endian `f32`, and an in-memory contiguous matrix is scanned by brute force. The index schema is now v2: passage IDs are `AUTOINCREMENT`, so a cached vector can never be attributed to a later passage (a test caught this).
  - The dense lane is fused by RRF with the keyword lanes. Dense-only notes need cosine ≥ the model's floor and count towards `total`.
  - `semantic` is `none`, `partial(n/m)`, `complete`, `off (…)` in keyword mode, or `failed: …`. It reaches the agent tool, the CLI page and the desktop status line. A failing model is reported while keyword lanes still answer; there is no fallback between models.
  - Hosts use the folder named by `BRN_EMBEDDING_MODEL_DIR`. An unusable folder is an error in the CLI and a notice on the desktop.
  - Tests use deterministic fake models: paraphrase found only by the dense lane, prefixes, coverage, visibility, re-embedding after an edit, the floor keeping abstention, a failing model, and keyword mode.
- [x] Owner-requested qualification (approved 14 October 2026): `brn model-install`, then `retrieval_eval --retriever hybrid` at 1k and 10k notes. Recorded in [evidence](evidence.md#qualification-with-the-pinned-model). The measurements changed the design:
  - The dense lane now only adds notes the keyword lanes missed; boosting keyword matches cost inflection and section ranking.
  - A semantic match also needs the best passage to stand 0.017 above the background; the 0.82 floor alone let unrelated queries through at 10k.
  - `matching_keywords` became one query; it had raised hybrid p95 to 263 ms.

## Task 6 — cleanup (PR 6, shipped with Task 5)

- [x] Retired the vault-era `NoteIndex` (metadata, UUID, edge-proof, reader and embedding caches, about 1,700 lines plus six test files) and the old fixed-size chunker, once nothing outside `brn-retrieval` used them. The query helpers moved into `search_index`, the `Embedder` trait into `embedding`. Error variants and `serde`/`serde_json` dependencies used only by that code were removed; the lockfile only loses those edges.
- [x] Assessed the legacy `ReadTools::search_notes` path in `brn-ai/src/tools.rs`. It belongs to the offline provider-format qualification runtime (`provider_fixture_runtime`, test- or `capability-spike`-only exports, about 6,700 lines of format tests) and does not touch any index. Shipped hosts never reach it; agent runs use `search_notes` through `RunNoteSearch`. Removing it means retiring or porting those provider-format tests, which is a separate owner decision rather than retrieval cleanup, so it is kept unchanged.

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

Round two (independent, after the #129 and #130 merges) found no broken decision. Three blocking findings were adopted in Task 1:
- B1: re-check visibility over a by-ID closure, not a full scan.
- B2: eligibility must include the import Source's chat, and archived and superseded notes are excluded.
- B3: a core-owned read-only reader.

Significant findings:
- S1 and S2 (cheap watermark, cross-process writers) were adopted in Task 1.
- S3 (the UI must not block) is split: the non-blocking ticket exists, and the UI switch is Task 2.
- S4 (D1 caveats) was adopted.
- Task 3 notes: name the brn-ai trait distinctly (for example `RunNoteSearch`), make tool registration conditional, and update the nine-tool assertions.

## Gate record

- Defined: the owner requested this plan after the research, the English-first decision, and the move to Chats and Projects.
- Accepted: D6 and D7, then D1–D5, by the owner on 13 October 2026, with direction to build. #129 and #130 merged the same day (D5); the branch was rebased onto `56c28879`.
- Merging any slice still needs separate owner authority.
