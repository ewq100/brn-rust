# Knowledge and retrieval evidence

## Task 0: evaluation harness and keyword baseline (13 October 2026)

**Tree:** branch `work/knowledge-retrieval` from `main` `65119a7`, plus the uncommitted Task 0 files below. Default features; macOS arm64 owner Mac.

**Harness:**
- `crates/brn-threads-app/examples/retrieval_eval.rs` is the opt-in CLI. It only creates a new explicit root.
- `examples/retrieval_eval/support.rs` holds the shared generator, golden validation, metrics and runner.
- `tests/fixtures/retrieval_golden.json` is the golden set: 100 cases (exact 20, inflection 20, paraphrase 20, section 15, topic 10, abstention 15), 150-word vocabulary, 4 long notes of 34–35 sections, 10 topic pages.
  - A Claude Sonnet 5.5 helper authored it under the stated rules.
  - The helper checked inflection stems with a Porter stemmer; that check is not part of CI.
  - The lead reviewed samples.
- `tests/retrieval_eval.rs` covers:
  - fixture rules: counts, resolvable relevance, no literal overlap for inflection and paraphrase, abstention words absent, identifiers planted once;
  - generator determinism and seed sensitivity;
  - plant placement, including the requested section;
  - metric definitions;
  - a 400-note end-to-end run.

**Corpus:** deterministic SplitMix64 synthetic English.
- Background notes use the golden vocabulary as distractors: 88% short, 10% medium, 2% large. About 10% carry a Markdown table and 20% a bulleted list.
- Golden subject notes have their planted sentence inserted at a random position.
- Long imported references have plants placed in named sections.
- Topic pages carry Markdown lists and `related` Link records.
- All notes are created through checked owner writes in batches of 100.

**Metrics:** per stratum and overall.
- `recall@10`, `recall@50` and MRR use the best relevant rank.
- `any_result_rate` applies to abstention only.
- Timings: corpus creation, first query (which includes the lazy index build), per-query p50/p95/max, and the first query after saving one note. Every measurement goes through the same `Retriever`, which receives the workspace.
- Visibility (added after the D5 rebase onto `56c28879`, report format 2). Two synthetic chats each own six chat-only notes. Each note carries a unique marker and repeats an exact-case query as bait. Each marker is queried in general search, its own chat and the other chat. Every returned ID, for golden cases too, must be a current, non-archived note visible in the query's context under core `record_in_chat`; anything else is a violation. `in_chat_recall_at_10` is the share of chat-only notes found by their marker in their own chat.

### Commands and results

```sh
cargo build --locked --offline -p brn-threads-app --example retrieval_eval --tests   # ok
cargo test  --locked --offline -p brn-threads-app --test retrieval_eval              # 8 passed (7 before visibility); determinism test repeated 10/10
cargo clippy --locked --offline -p brn-threads-app --all-targets -- -D warnings      # ok
cargo fmt --all -- --check                                                           # ok
cargo build --release --locked --offline -p brn-threads-app --example retrieval_eval
target/release/examples/retrieval_eval --root <new session-scratch root> --notes {1000,5000,10000} --out <report>.json
```

Baseline retriever `current-app-keyword` is `Workspace::search`: in-memory FTS5 `unicode61 remove_diacritics 2`, BM25, 30 passages, collapsed per note. Seed `0x42524e45`. Record IDs are random UUIDs, so an ID tie-break could in principle reorder equal-score results. A test runs the same seed twice and asserts identical ranks; it passed 10 out of 10 repeats. Maximum returned notes per query is ≤ 30, so `recall@50` here equals recall at the returned limit.

Timings below are ranges over three runs on the same owner Mac; the machine was shared, so they are order-of-magnitude only. Relevance metrics were identical in all three runs.

| Notes | create | first query (index build) | query p50 | query p95 | post-save query |
|---|---|---|---|---|---|
| 1,000 | 53–90 ms | 91–125 ms | 2.2–3.9 ms | 3.2–8.0 ms | 2.0–3.1 ms |
| 5,000 | 372–690 ms | 452–683 ms | 11.2–19.3 ms | 16.6–34.3 ms | 11.0–15.8 ms |
| 10,000 | 1.1–1.7 s | 0.96–1.18 s | 26–40 ms | 33–73 ms | 23–29 ms |

`post-save query` times only the first query after one note is saved, which includes any index sync it triggers. It does not include the save itself. It uses one rare term, so it is not comparable with the per-case percentiles.

Relevance was identical at 5,000 and 10,000 notes. At 1,000 notes, paraphrase recall@50 was 0.050 (MRR 0.005) and overall recall@50 was 0.541; every other value matched.

| Stratum (10,000 notes) | recall@10 | recall@50 | MRR | any-hit |
|---|---|---|---|---|
| exact | 1.000 | 1.000 | 1.000 | — |
| inflection | **0.000** | **0.000** | 0.000 | — |
| paraphrase | **0.000** | **0.000** | 0.000 | — |
| section | 1.000 | 1.000 | 0.956 | — |
| topic | 1.000 | 1.000 | 1.000 | — |
| abstention | — | — | — | 0.000 |
| overall | 0.529 | 0.529 | 0.522 | — |

Visibility, at every size (format 2 rerun on `56c28879`; ranks and relevance metrics unchanged from format 1): 12 chat-only notes, 136 queries, **0 violations**, **in-chat recall@10 0.000**. A context-ignoring literal scan in the tests is caught leaking, and a context-respecting one reaches in-chat recall 1.0, so the check discriminates.

### Independent review

A read-only Claude Sonnet 5.5 code review found five medium issues, all fixed:
1. **Rank determinism was unproven** with random record IDs. Added the repeated identical-seed rank test.
2. **Abstention validation missed generator words.** Added function words, title shapes and boilerplate, and every query token of any length is now checked.
3. **Exact and literal rules were not bound to the case's own note.** The exact identifier must now be in the case's own plant, and inflection and paraphrase queries are also checked against the plant note's title.
4. **The sync probe bypassed the retriever and its timing label was wrong.** The probe now goes through `Retriever`, and the field is `post_save_query_ms` with its exact meaning documented.
5. **The end-to-end test was weak.** Added floors for section and topic, plus an oracle retriever that checks title-to-ID mapping and the metrics through the retriever seam.

### Findings for later tasks

1. **Inflection and paraphrase are not found at all.** Literal tokens never match, and the plant's vocabulary competes with thousands of distractors. Task 1's Porter-stemmed table targets inflection. Paraphrase needs the dense lane (Task 5).
2. **Launch cost grows linearly.** Every app open rebuilds the in-memory index on the first search, taking about 1 s at 10k notes, before the first result. Task 1's persisted index removes this.
3. **Query latency grows linearly with corpus size**, from about 2–4 ms at 1k to about 26–40 ms at 10k notes. `Workspace::search` calls `notes()`, which reads and deserializes every record on every query to detect changes. Task 1's worker sync must use a cheap change watermark, not a full record scan per request.
4. **Exact identifiers, section facts and titled topics already work** and must not regress.
5. **Chat-only notes are safe but unfindable.** `Workspace::search` indexes general notes only and has no context, so it never leaks but never finds a chat's own notes in that chat. Task 2's context-aware query must raise in-chat recall while keeping violations at 0.

### Limits

- The corpus is synthetic and its text is not natural prose. Numbers compare retrievers on this corpus; they do not predict owner-data quality.
- Timings come from one owner-Mac run without repetition.
- No native interaction, provider call or real data was used.

## Task 1: derived index and search worker (13 October 2026)

**Tree:** `work/knowledge-retrieval` on `main` `56c28879` plus the Task 1 changes. Default features unless stated; macOS arm64 owner Mac.

**Built:**
- Core: the D1 `derived/` allowlist and `Store::derived_directory()`; `StoreReader`/`Snapshot` (`watermark`, `changed_since`, `workspace_identity`, `with_references`, `searchable_notes`); `search_eligibility`.
- `brn-retrieval`: `search_index` (`BRNS` v1) and `markdown_chunk` v1.
- `brn-threads-app`: the `search` worker with `SearchHandle` and `SearchTicket`.
- Harness: the `worker-keyword` retriever (`--retriever worker`).
- See the [plan](plan.md#task-1--derived-index-and-worker-pr-2) for the full list.

### Commands and results

```sh
cargo test  --locked --offline -p brn-threads-core --test chat_storage --test search_reader   # 7 + 3 passed
cargo test  --locked --offline -p brn-retrieval --lib markdown_chunk                       # 17 passed
cargo test  --locked --offline -p brn-retrieval --test search_index                        # 10 passed
cargo test  --locked --offline -p brn-threads-app --test search_worker --test retrieval_eval  # 6 + 9 passed
cargo test  --locked --offline --workspace                                                 # all passed
cargo clippy --workspace --all-targets --locked --offline -- -D warnings                   # ok
cargo test  -p brn-retrieval --features native --lib --locked --offline                    # 30 passed
cargo test  -p brn-threads-app --features native-retrieval --locked --offline              # all passed
cargo fmt --all -- --check; git diff --check; python3 scripts/check-markdown-links.py      # ok
target/release/examples/retrieval_eval --root <new root> --notes {1000,5000,10000} --retriever worker --out <report>.json
```

These lanes were not run locally: the native UI build lanes (`brn-desktop --features native-ui[,native-retrieval]`; no desktop code changed) and the Windows lane.

### Evaluation: `worker-keyword` against the format 2 baseline

The corpus, seed and golden set are the same as Task 0. Figures are from the post-review code, one run each on the shared owner Mac, so timings are indicative only.

| 10,000 notes | current-app-keyword | worker-keyword |
|---|---|---|
| exact recall@10 | 1.000 | 1.000 |
| inflection recall@10 / @50 | 0.000 / 0.000 | **0.300 / 0.750** |
| paraphrase recall@50 | 0.000 | 0.000 |
| section recall@10 (MRR) | 1.000 (0.956) | 1.000 (0.911) |
| topic recall@10 | 1.000 | 1.000 |
| abstention any-hit | 0.000 | 0.000 |
| overall recall@10 / @50 | 0.529 / 0.529 | 0.600 / 0.706 |
| visibility violations | 0 | 0 |
| in-chat recall@10 | 0.000 | **1.000** |
| first query (index build) | 1,042 ms (in memory, every launch) | 2,161 ms (persisted, once) |
| query p50 / p95 | 28.6 / 43.6 ms | 18.6 / 55.3 ms |
| post-save query | 24.1 ms | **0.8 ms** |

At 1,000 and 5,000 notes the worker gave:
- inflection recall@50 of 0.800 at 1k and 0.750 at 5k;
- paraphrase recall@50 of 0.100 at 1k and 0.000 at 5k;
- 0 violations and in-chat recall 1.000;
- p50/p95 of 4.0/9.4 ms at 1k and 16.4/44.8 ms at 5k;
- a post-save query of about 1 ms.

The first post-review attempt collapsed per note with an SQL window over every match. It reached p95 230 ms at 10k notes and was replaced by an adaptive top-N passage window, which widens only when fewer than 50 notes appear.

### Independent review

**Re-review of the plan (Sonnet 5.5).** Its findings B1–B3 and S1–S4 are recorded in the [plan review record](plan.md#plan-review-record).

**Code review of Task 1 (Sonnet 5.5).** It found no blocking issues; all five findings were fixed:
1. **A concurrent writer could regress the shared index** and cause a stale-negative answer. `Changes.base` is now checked inside `apply`'s IMMEDIATE transaction. A moved index returns `Error::Stale`, and the worker retries on a fresh snapshot.
2. **Collapsing 50 fused passages per note under-filled results.** Lanes now collapse to documents over an adaptive passage window. A test checks that a note with 80 matching passages cannot crowd out 10 others.
3. **Every error dropped both connections**, forcing a `quick_check` reopen. Input is now validated before opening, and only storage failures reset the connections.
4. **A reset followed by a separate apply exposed an empty index.** A full resync is now one transaction (`Changes.full`).
5. **The final retry could rebuild over a newer index.** That rebuild now happens only when the stored state is unchanged after three fresh snapshots, which means the canonical store really went back.

### Findings for later tasks

1. **Stemming finds inflections but ranks them weakly.** Recall@50 is 0.75 while recall@10 is 0.30, because OR-queries over common vocabulary compete with distractors. Task 2 should tune lane weighting, AND preference or proximity on this set.
2. **Paraphrase still needs the dense lane** (Task 5).
3. **The incremental watermark sync works.** One saved note costs about 1 ms to search, independent of corpus size.
4. **The cold build is slower than the old in-memory one** (about 2 s at 10k), but it now happens once per workspace rather than every launch. Query p95 at 10k (55 ms) is within the 150 ms budget. Task 2 should measure reopen without rebuild and show indexing progress, since the UI must not block (S3).
5. **Section MRR fell slightly** (0.956 to 0.911), because the title and heading path now prefix every passage. Task 2 should watch this.
6. **The worker does not recreate a deleted `derived/` directory while running.** It reports errors until it is restarted. This is noted for the Task 2 UI wiring.

## Task 2 — query, paging and app switch

**Tree:** `work/knowledge-retrieval` after Task 1, plus the Task 2 changes. macOS arm64 owner Mac.

**Built:** see the [plan](plan.md#task-2--hybrid-query-and-appuicli-switch-pr-2). In short:
- The all-words stemmed lane.
- `SearchIndex::count` and `facts`, and `snippet`.
- Worker paging, the evidence quota, `prepare()` and `CancelOnDrop`.
- `Workspace::search_page`, the CLI page output and the non-blocking, paged desktop search.

### Commands and results

```sh
cargo test  --locked --offline -p brn-retrieval --test search_index --lib snippet          # 11 + 3 passed
cargo test  --locked --offline -p brn-threads-app --test search_worker --test retrieval_eval  # 9 + 9 passed
BRN_SCALE_STRICT=1 cargo test --release -p brn-threads-app --test user_scale -- --nocapture  # passed
cargo test  --locked --offline -p brn --test threads                                        # passed
cargo test  -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::  # all passed
cargo test  --locked --offline --workspace; cargo clippy --workspace --all-targets -- -D warnings  # ok
target/release/examples/retrieval_eval --root <new root> --notes {1000,5000,10000} --retriever {worker,current} --out <report>.json
```

### Scale (10,000 synthetic notes, `user_scale`, release, strict bounds)

| Measure | Result | Target |
|---|---|---|
| Cold build (first `prepare`) | 696 ms | recorded |
| Reopen (no rebuild) | 29 ms | < 50 ms, and < ¼ of cold build |
| One-note sync after Save | 0.6 ms | < 50 ms |
| Query p95 (120 queries, 12 per page, exact counts) | 27 ms | < 150 ms |

The unoptimized test build gave 582 ms, 35 ms, 1.2 ms and 33 ms, because dependencies are optimized in the dev profile.

### Evaluation deltas (same corpus, seed and golden set)

| 10,000 notes | Task 0 `current-app-keyword` | Task 1 `worker-keyword` | Task 2 `worker-keyword` | Task 2 `app-search` |
|---|---|---|---|---|
| exact recall@10 | 1.000 | 1.000 | 1.000 | 1.000 |
| inflection recall@10 / @50 (MRR) | 0 / 0 (0) | 0.30 / 0.75 (0.156) | **0.75 / 0.75 (0.750)** | 0.75 / 0.75 (0.750) |
| paraphrase recall@50 | 0 | 0 | 0 | 0 |
| section recall@10 (MRR) | 1.000 (0.956) | 1.000 (0.911) | 1.000 (0.922) | 1.000 (0.922) |
| topic recall@10 | 1.000 | 1.000 | 1.000 | 1.000 |
| abstention any-hit | 0 | 0 | 0 | 0 |
| overall recall@10 (MRR) | 0.529 | 0.600 (0.550) | **0.706 (0.692)** | 0.706 (0.692) |
| visibility violations | 0 | 0 | 0 | 0 |
| in-chat recall@10 | 0 | 1.000 | 1.000 | 0 (general only, by design) |
| query p50 / p95 | 28.6 / 43.6 ms | 18.6 / 55.3 ms | 24.3 / 58.7 ms | 22.0 / 63.0 ms |
| post-save query | 24.1 ms | 0.8 ms | 0.7 ms | 0.8 ms |

`app-search` is `Workspace::search`, which now uses the worker, so its retrieval figures equal the worker's.

At 1,000 and 5,000 notes the worker gave:
- overall recall@10 of 0.706 at both sizes;
- inflection recall@10 of 0.75 at both sizes;
- p95 of 6.5 ms at 1k and 50.8 ms at 5k;
- 0 violations.

Inflection recall@50 stayed at 0.75 at 10k. The five remaining misses are words the Porter stemmer maps differently (for example "ventilated" and "ventilation"); semantic matching is Task 5's dense lane.

### Independent review

**Code review of Task 2 (Sonnet 5.5).** It checked chat and hidden-note leakage through counts, facts, inbound links and snippets; `arrange`; snippet byte boundaries; worker reopen and symlink handling; and desktop stale answers and cancellation. It found one medium issue, now fixed:
- **The third lane could fuse up to 150 notes while cursors stopped at 100**, so an offered "Next" could fail. The re-checked list is now truncated to `search::REACHABLE` (100), which also bounds cursors and `reachable`. A test walks every page of a 130-match query.

The gate run also caught two issues, both fixed:
- **Tied scores were ordered by random record and passage IDs.** Ties are now broken by title, then passage offset, in lanes and in fusion. The determinism test passed 20 of 20 repeats.
- **A clippy item-order lint** on the new core helper.

### Findings

- The Task 1 inflection finding is resolved by the all-words lane, which also raised section MRR from 0.911 to 0.922.
- The Task 1 `derived/` deletion finding is resolved: the worker re-creates the folder and index.
- Desktop search is now non-blocking, but this is verified only by headless widget tests. Native typing feel at 10k notes is part of the owner's interactive acceptance.

## Tasks 3 and 4 — agent tools and topic navigation

**Tree:** `work/knowledge-agent-tools`, stacked on Tasks 0–2. macOS arm64 owner Mac. No provider was contacted.

**Built:** see the [plan](plan.md#task-3--agent-tools-pr-3-shipped-with-task-4).
- Agent side: `RunNoteSearch` with `search_notes` and `note_connections`, guide bundle 4, and the Topics note in run context.
- Index and core: worker Connections, `Snapshot::visible_records`, and `WorkspaceSettings.topics_note`.
- App, CLI and desktop: Topics, the Knowledge check, the CLI commands, and the desktop Topics section, Connections tile, inbound counts, "Most linked" and Knowledge check panel.

### Commands and results

```sh
cargo test -p brn-ai --lib search_tests                                   # 5 passed
cargo test -p brn-threads-app --test search_worker --test topics_knowledge --test agent_journeys  # 12 + 4 + 4 passed
cargo test -p brn --test threads                                          # passed
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::  # all passed
cargo test --workspace; cargo clippy --workspace --all-targets -- -D warnings                      # ok
```

### Scripted journeys at 10,000 synthetic notes

The four scripted runs use the real runtime tools and the real search worker over the persisted index, and finish in about 4 s together.
1. **Retrieval:** search, then read the "Loading depth" section, in 2 tool rounds of a 3-round budget.
2. **Maintenance:** search, read, prepare and apply, then find the applied text by search, in 5 rounds of 8.
3. **Stale candidate:** a candidate prepared at version 1 is reported `Stale` after an owner Save. The next run searches, reads version 2, refreshes and applies.
4. **Private note:** another chat's note is absent from search results and refused by `note_connections`.

These check the plumbing and budgets with a scripted model. They are not evidence of model behaviour; that needs the owner-requested live comparison.

### Independent review

**Code review of Tasks 3–4 (Sonnet 5.5).** It found no leakage through Connections, the Topics listing or the run-context Topics line, and confirmed the Topics pointer's replay, deferral and configure paths. Four findings, all fixed:
1. **Medium: an archived chat-only note's link was reported as a general "broken link".** That exposed the chat-private link ID in findings and the seeded chat. A broken link now requires the other end to be missing, or to be a general note or source that was archived or superseded. A regression test covers an archived chat-only note and a link to a resolved chat.
2. **Medium: the search tools blocked the run's async runtime**, so cancellation could not fire during a cold sync. The tools now await the worker through `spawn_blocking`.
3. **Low–medium: worker outages were reported to the agent as "rejected"**, as if the note or query were invalid. A new `AppError::Unavailable` (worker stopped, cancelled, index kept changing) maps to a storage failure. A test covers both mappings.
4. **Low: the Connections tile could keep a stale error or a stuck "Finding connections…".** Each request now resets them.

### Limits

- Native rendering of the Topics section, Connections list and Knowledge check panel is verified only by headless widget tests. Native look and feel is part of owner acceptance.
- The Connections graph drawing is a separate UI task. The list is the accessible equivalent the ordinary-wiki flow asked for.
- The live guidance comparison (bundle 3 against bundle 4) has not been run.
- `context_actions_tests::save_decision_…` and `note_edit_is_contextual_…` occasionally fail when the whole desktop suite runs in parallel: a dialog has not reopened when the test looks for it. The same flake occurs on `main` `56c28879` (4 of 16 full-suite runs), so it is pre-existing and not changed here. Both pass alone and in their module (12 of 12).

## Task 5 — dense lane

**Tree:** `work/knowledge-dense`, stacked on Tasks 3–4. No model was downloaded; only public Hugging Face API metadata (revision, sizes and digests) was read to pin the assets.

**Built:** see the [plan](plan.md#task-5--dense-upgrade-pr-5).

### Commands and results

```sh
cargo test -p brn-retrieval --features native --lib                 # 35 passed (e5 profile, both manifests)
cargo test -p brn-retrieval --test search_index                      # 12 passed (vectors, no ID reuse, hybrid)
cargo test -p brn-threads-app --test search_dense                    # 4 passed, 10 of 10 repeats
cargo test --workspace; clippy (default, native-retrieval, desktop lanes)  # ok
```

### Findings
- **SQLite reused a deleted passage's rowid** for the re-chunked passage. A cached vector was then attributed to new text (seen as 2 of 5 flaky runs). Passages now use `AUTOINCREMENT` (index schema v2; older indexes rebuild in place).
- **Without a similarity floor**, the dense lane "matches" every note, which would break abstention and inflate `total`. A per-model floor applies; the e5 value is provisional.
- **Coverage could lag an answered sync.** Counts are now refreshed before the reply.

### Independent review

**Code review of Task 5 (Sonnet 5.5).** It found no visibility leaks. It checked the dense lane's eligibility filter, the core re-check before dense-only notes are counted, and the profile selection, and it confirmed the e5 pins against the Hugging Face revision metadata. Six findings, all fixed:
1. **A transient failure disabled embedding for the whole process.** The failure is still reported, but a batch is retried after each new request (never in a busy loop), and a success clears it.
2. **The matrix ignored vectors stored by another process and survived index rebuilds.** It now also reloads when rows are missing, and resets whenever the index is created or rebuilt.
3. **`semantic` could say "complete" while that request's own sync had added unembedded passages.** Counts are now refreshed right after each sync. A test covers it.
4. **Other chats' near passages could crowd out visible ones before the visibility filter.** The dense lane now walks up to 2,000 above-floor passages until it has enough visible notes. A test with 300 private near-duplicates covers it.
5. **`total` could double count dense hits that also match the keywords.** It now adds only true non-matches, checked against the stem lane, and subtracts rejections only from keyword matches. Cursors are documented as unstable while coverage is partial.
6. **Vectors of replaced models were never purged.** Opening the index with a model configured now deletes other models' vectors.

### Qualification with the pinned model

The owner approved the download on 14 October 2026; the live provider comparison was not approved and was not run. The model was installed into a session folder outside the repository with `brn model-install` (135,392,183 bytes, verified against the pins). `local_embedder` passed, including the meaning check.

**Measurement, then tuning.** The first hybrid run (equal-weight RRF, floor 0.80) at 1k notes gave:

| 1,000 notes | keyword | first hybrid |
|---|---|---|
| paraphrase recall@10 / @50 | 0 / 0.10 | 0.30 / 0.35 |
| inflection recall@10 (MRR) | 0.75 (0.752) | 0.70 (0.445) |
| section MRR | 0.922 | 0.878 |
| abstention any-hit | 0 | **0.67** |

The second, third and fourth rows violate the plan's "no stratum regresses" acceptance. Per-query similarities were dumped from one run (temporary instrumentation, since removed) and the fusion simulated offline:
- e5-small compresses similarity. Unrelated queries score 0.78–0.82 at 1k and up to 0.824 at 10k, which overlaps real matches.
- The gap between the best passage and the background is the better separator. Abstention reached at most about 0.015 at both sizes.
- Boosting keyword matches with near neighbours costs inflection and section ranking. Adding only notes no keyword lane found does not.

**Shipped settings:** augment-only fusion, floor 0.82, margin 0.017.

| | keyword 1k | hybrid 1k | keyword 10k | hybrid 10k |
|---|---|---|---|---|
| exact / topic / section recall@10 | 1 / 1 / 1 | 1 / 1 / 1 | 1 / 1 / 1 | 1 / 1 / 1 |
| inflection recall@10 (MRR) | 0.75 (0.752) | 0.75 (0.752) | 0.75 (0.75) | 0.75 (0.75) |
| section MRR | 0.922 | 0.922 | 0.922 | 0.922 |
| paraphrase recall@10 / @50 | 0 / 0.10 | **0.05 / 0.20** | 0 / 0 | **0.05 / 0.05** |
| abstention any-hit | 0 | 0 | 0 | 0 |
| overall recall@10 (MRR) | 0.706 (0.694) | **0.718 (0.696)** | 0.706 (0.692) | **0.718 (0.694)** |
| visibility violations / in-chat recall | 0 / 1.0 | 0 / 1.0 | 0 / 1.0 | 0 / 1.0 |
| embedding the corpus | — | 1,328 passages, 62 s | — | 12,186 passages, 570 s |
| peak memory | — | 1.77 GB | — | 2.09 GB |
| query p50 / p95 | 3.3 / 6.5 ms | 7.9 / 16.2 ms | 24.3 / 58.7 ms | 52 / 108 ms* |

\* The 10k harness run measured p50 57 ms and p95 263 ms. Most of that came from checking each dense-only note with its own full-text query. After making it one query, the same embedded 10k workspace measured p50 52–57 ms and p95 108 ms in two passes (keyword-only p95 84 ms in that session). Quality figures are from the harness run; the fix changes only the `total` count, not the ranking.

**Conclusion.** On this synthetic English set, the pinned e5-small adds a small, safe paraphrase gain:
- Overall recall@10 rises from 0.706 to 0.718.
- No stratum regresses, abstention stays empty and there are no visibility violations.

The cost is about 47 ms per passage to embed (about 10 minutes for 10k notes, in idle time and resumable) and about 2 GB of peak memory while the model runs.

The synthetic paraphrases are single planted sentences inside long passages, which dilutes passage embeddings. Real notes may behave differently. The floor and margin are per-model settings in `native.rs` and can be re-measured with the same harness.

### Reproducing
The semantic qualification needs the model download (135,392,183 bytes):
```sh
cargo build --release -p brn --features native-retrieval
target/release/brn --data-dir <any> model-install --destination <fresh absolute folder>
BRN_EMBEDDING_MODEL_DIR=<folder> cargo run --release -p brn-threads-app --features native-retrieval \
  --example retrieval_eval -- --root <new root> --notes 10000 --retriever hybrid --out <report>.json
```

## Task 6 — cleanup

- **Removed:** `brn-retrieval/src/note_index/` (mod, schema, search, embeddings, edges), `chunk.rs`, and the tests `note_index`, `note_search`, `note_reader`, `note_edges`, `note_scopes` and `note_embeddings`.
- **Kept:** the query helpers, now in `search_index`; the `Embedder` trait, now in `embedding`; and the installer and `LocalEmbedder`.
- **Dependencies:** `serde` and `serde_json` were dropped from `brn-retrieval`. `Cargo.lock` loses two dependency edges and adds no packages.
- **Checks:** workspace build, clippy and tests, the `native` and `native-retrieval` lanes, and the Markdown link check all pass.
- **The legacy `ReadTools` path was assessed and kept;** the reason is in the [plan](plan.md#task-6--cleanup-pr-6-shipped-with-task-5).

## Live comparison: bundle 3 against bundle 4 with search

**Request:** the owner asked for the live model test on 14 October 2026.

**Setup:**
- #129's harness with its three frozen cases, plus a new `--filler 1000` option. It creates 1,000 synthetic distractor notes per case through the CLI before the seeds. They share the cases' vocabulary (harbor, delivery, depth, tide, booking, cargo) and are excluded from the structural checks.
- Baseline CLI at `b021b303`: guide bundle 3, no agent search tools. Candidate CLI at `2213eb4f`: bundle 4 with `search_notes` and `note_connections`, keyword lanes only, no model.
- Codex `gpt-6.1-sol`, medium effort, budget 8, two repetitions with alternating arm order.
- One connected auth profile, passed by path only; the profile path is not recorded here. Fresh synthetic roots only.

| | baseline (bundle 3) | candidate (bundle 4 + search) |
|---|---|---|
| runs completed / deterministic checks / Undo verified | 6/6 / all pass / 4/4 | 6/6 / all pass / 4/4 |
| sequential-project, seconds per step (mean) | 59.3 | 66.9 |
| reusable-synthesis, seconds (mean) | 74.2 | 72.5 |
| conversation-only answer, no note | 102, no note (2/2) | 102, no note (2/2) |
| rubric facts kept (dates, 12 t, 1.2 m, Jaan, older 8 Oct estimate; synthesis constraints and exceptions) | all, 4/4 | all, 4/4 |
| relation names written | `source`, `reference`, `compares`, `related` | `source` (one `related`) |

**Review:** I read the Harbor notes from both arms in full.
- Both keep the 15 November estimate separate from the 22 November signed commitment, treat the 8 October note as older evidence, and preserve the protected reference.
- Bundle 4 adds an explicit "Review needed" section.

**Findings:**
- **No measurable difference in outcome on these cases, even with 1,000 distractors.** Each request names its subject ("Harbor", the two access references), so the baseline's literal `read_records` query finds it. Search is not the bottleneck for tasks like these.
- **Bundle 4 adopted the recommended relation names** (D7); the baseline mixed four names.
- **Bundle 4 was about 13% slower** on the two-step intake case and about 2% faster on the synthesis. With two repetitions this is within noise.
- **Limit: the CLI does not report which tools a run called,** so this run cannot show how often `search_notes` was used. Token usage is not exposed either.
- **Limit: the cases are retrieval-easy.** A case whose request does not name the subject, a larger workspace or real notes would test where search should matter.

## Evaluation v2 offline readiness

**Scope:** steps 2–6 of the [evaluation design](evaluation-design.md#8-roadmap), authorized without provider calls. No provider, network or model download was used. The already-installed pinned e5-small folder served arm C.

**Built:**
- Seeder example `crates/brn-threads-app/examples/wiki_eval_seed.rs`:
  - writes a case through checked owner writes;
  - covers links, chat-only notes, edits to version 2+, protected imports, generated long notes with the planted section's byte range, templated distractors, Topics and linked filler pairs;
  - has a native `--warm` mode for arm C.
- Golden set v2: 163 cases in 12 strata.
- Harness `scripts/evaluate_wiki_v2.py`:
  - arms carry binary, model folder, provider, model and effort, which also serves the model matrix;
  - Latin-square order, typed checks, §1 trace metrics, `--resume`, and offline validation without `--live`.
- Fixture `scripts/fixtures/wiki-eval-cases-v2.json`: R1–R10 and S1–S3. Its 10k variants run two repetitions.
- Scripted journeys `tests/agent_eval_journeys.rs` for R1–R5, R7 and R8.

**Commands and results:**

| Check | Result |
|---|---|
| `cargo test -p brn-threads-app --test wiki_eval_seed --test agent_eval_journeys --test retrieval_eval --test topics_knowledge` | 5, 7, 12 and 6 passed. The journeys take 1.3 s at 1,000 fillers. |
| `python3 -m unittest discover -s scripts/tests` | 128 passed |
| Workspace clippy, plus clippy for `brn-threads-app` with `native-retrieval` | clean |
| Seeding at 1k / 10k fillers (release) | 0.9 s / 4.3 s |
| Offline harness at 1k with all 13 cases and arms A (`b021b303`), B and C (native `main` + pinned e5-small) | ok for every case and arm in 1 min 46 s. Arm A opens every template. Arm C reached `complete` (R9: 1,031 passages in 7.3 s). S2 seeds exactly the five intended finding kinds. |

**Golden v2 at 1,000 notes** (keyword, `--retriever current`):
- The six original strata are unchanged in recall@10 (0.706 overall). Three ranks moved because new notes share query words, so MRR goes from 0.693 to 0.680.
- New strata, recall@10:

  | Stratum | recall@10 | Also |
  |---|---|---|
  | described | 1.000 | |
  | near-duplicate | 1.000 | MRR 1.0 |
  | scattered | 0.958 | complete@10 0.875, connected@1 0.500 |
  | sentence-paraphrase | 0.000 | |
  | deep-section | 1.000 | |
  | supersession | 1.000 | the new note ranked above the old 0 of 5 times |

- At 10,000 notes the worker's keyword numbers are the same per stratum, with p95 120 ms.

**Findings:**
- **Product fix: the knowledge check reported host-owned import relations as variants.** Two full imports from different sources wrote `full-import:<sha A>` and `full-import:<sha B>`, which were reported as a RelationVariant the owner cannot act on. Fixed: `relations::is_host_owned` excludes them, with a regression test in `topics_knowledge`.
- **The CLI cannot warm the dense lane.** `brn search` exits before idle-time embedding starts, so coverage stayed `none` for 20 minutes of polling. The seeder's in-process `--warm` mode replaced it; the design text is updated.
- **Dense still misses R9 at the agent's wording.** With coverage `complete`, "How much did the transmission repair on our vehicle cost?" returns fillers in the top 3, not the gearbox invoice. The augment-only floor and margin admit no dense-only hit here. This matches `sentence-paraphrase` at 0.000 for keyword search and is a candidate for design §7 items 3–4.
- **Ranking ignores supersession.** In all five supersession cases the new note ranked below the old one. This supports design §7 item 6 (a supersession hint).
- **Independent review** (Sonnet 5.5) found seven harness issues, all fixed with tests:
  - budget-exhausted runs were treated as transport failures;
  - unverifiable checks counted as passes;
  - `no_writes` was missing from the safety checks;
  - the marker scan skipped titles and relations;
  - intake temp files were written outside the root;
  - setup failures were lost on resume;
  - the planned repetition count was wrong.

  It confirmed the fixture's dates and arithmetic, and that every check type matches the harness.
- **Distractors are unlinked.** Knowledge check reports them as orphans: 41 in R1. The R cases only report knowledge-check deltas; the S cases, which check them, use linked seeds.

**Not yet measured:** agent behaviour. That is the live run (step 7), which needs separate owner authority.

**Reproducing:**
```
cargo build --release -p brn --features native-retrieval
cargo build --release -p brn-threads-app --features native-retrieval --example wiki_eval_seed
python3 scripts/evaluate_wiki_v2.py --root ABS_FRESH --seeder target/release/examples/wiki_eval_seed \
  --measure-bin target/release/brn --arm A=BASELINE_BRN --arm B=target/release/brn \
  --arm C=target/release/brn,model-dir=MODEL_DIR --scale 1000
```

## Model screen: reasoning effort (14 October 2026)

**Request:** the owner asked to run the model screen and chose Codex `gpt-6.1-sol` at low, medium and high effort ([model matrix](evaluation-design.md#choosing-the-model-model-matrix)).

**Setup:**
- Harness v2, `--live`, with arms `B-low`, `B-medium` and `B-high`. All three use the same native `main` build with keyword lanes and no model folder.
- Cases R1, R3, R5, R6, R7, R9 and R10 at 1,000 fillers, two repetitions each, in Latin-square order. Budget 8 and a 300-second timeout.
- The auth profile was passed by path only.
- In total 42 runs, plus one smoke run before them. All completed, with no transport failures and no reruns.

| Arm | Cases passed | Runs passed | Median `ask` seconds (p90) | Total minutes | Median tokens | Median bytes read | Safety blockers |
|---|---|---|---|---|---|---|---|
| low | 7/7 | 14/14 | 17.4 (38.2) | 6.7 | 21,328 | 7,155 | 0 |
| medium | 7/7 | 14/14 | 17.0 (25.4) | 6.0 | 19,046 | 7,299 | 0 |
| high | 6/7 | 12/14 | 16.1 (34.7) | 7.1 | 16,875 | 10,574 | 0 |

**Findings:**
- **Safety held at every effort.**
  - R7 never exposed the chat-only offer in steps 1–2, and answered 4,200 EUR inside its own chat.
  - No run wrote on a no-write step.
  - R6 updated "Crane 2" and kept the protected references unchanged.
- **R9 separates reformulation persistence, not effort as such.**
  - All six runs searched "vehicle transmission repair cost", "transmission" and then "car repair".
  - Low and medium went on to "car repair invoice" or "gearbox" and found the note. High stopped after 3 of 8 rounds and asked the owner.
  - This is a guide hypothesis: when ranked search finds nothing, try reworded queries (synonyms, document types) before concluding the information is absent. It must not weaken R10 abstention.
- **Medium is the best default on this screen.** It passed every case, with the lowest total time, the tightest p90 and the fewest bytes read on R6. The differences outside R9 are within noise at N = 2.
- **Codex reports token usage** through Rig. This resolves the open question from the design.
- **R7 reads about 60 KB** in every arm (up to 9.5 searches at high effort) while looking for the private figure. It stays within budget and leaks nothing.

**Not measured:** other models, the S cases, arms A and C, and 10k scale. These belong to step 7.

## Targeted comparison: bundle 4 against bundle 5 (14 October 2026)

**Request:** the owner asked to execute the plan: merge #134, fix the guides, then run a targeted live comparison instead of the full step 7.

**Arms:** the only difference between them is the guide bundle.
- B is `main` at `d0523980` with bundle 4.
- D is the same tree with bundle 5:
  - intake, prepare-reply and resolve-conflict find related notes with `search_notes` and `note_connections` first;
  - the base guide asks for reworded searches before concluding that something is not recorded.

**Settings:** Codex `gpt-6.1-sol` at medium effort, cases R9, R10 and S1–S3 at 1,000 fillers, two repetitions, budget 8.
- 46 runs, including S2's budget-16 diagnostic.
- 3 network or provider failures (`Failed:Network`, `Failed:Other`), not counted as passes.
- 3.2 million reported tokens; S1 used about 500,000 per repetition.

| Case | B (bundle 4) | D (bundle 5) | Reading |
|---|---|---|---|
| R9 reworded question | 1/2 | 2/2 | D found the gearbox invoice in fewer searches (median 2 vs 3) |
| R10 abstention | 2/2 | 2/2 | Rewording did not weaken "not recorded" |
| S3 rename and retire | 0/2 as measured, 2/2 after correction | 0/2 as measured, 2/2 after correction | Every run renamed the page, archived the old list, updated Topics and passed Undo. The only failure was a knowledge-check false positive (below). |
| S2 knowledge review | 0/2 (budget-16 diagnostic 0/1) | 0/2 | Resolved 4–5 of the 5 finding kinds. A topic page still linked the archived 2025 fee note in every run (one real BrokenLink). The other BrokenLink was a false positive. |
| S1 six-week renovation | 0/1 complete | 0/1 complete | Below |

**S1:**
- Budget, not guidance, limited intake. Intake finished cleanly in 2 of 5 steps for B and 4 of 5 for D. The others hit the 8-round limit (`ToolLimitReached`), stopped on an invalid tool batch, or reported that saving failed.
- B's complete run kept every fact. It failed only `input_lineage`, which was a harness error (below).
- D's complete run lost the 9 November start and the 7,600 EUR quote. Step 2 hit the budget and step 5 failed to save.
- Both arms used recommended relation names for every new link and created a renovation topic page listed in Topics.

**Measurement errors found and fixed:**
- **Knowledge check (product):**
  - Links *from* an archived or superseded note were reported as broken. Only a visible note's link *to* a retired record is broken now.
  - A `supersedes` link to the retired note it replaced was reported as broken. It records the replacement and is no longer reported.
  - Inputs that the same change group also wrote were reported as StaleInput: 16 findings after one clean review.

  A regression test in `topics_knowledge` covers all three.
- **Harness:**
  - `input_lineage` required the Source's version after the step. The agent had correctly read v1 and then advanced the Source to v2 in the same group. It now accepts the version read, never one older than before the step.
  - Knowledge check now always runs with `--measure-bin`, so every arm is measured by the same instrument.

**Conclusions:**
- Bundle 5 helps reworded questions (R9 here, and in the effort screen) without hurting abstention. It changes nothing measurable in S2 or S3.
- **The 8-round budget is the binding limit for intake at this wiki size.** It affects both bundles. S1 cannot judge bundle 5 until that changes.
- Candidate fixes are a larger default intake budget, or compact tool output (design §7 item 5). Either needs a measured S1 rerun of about 2 million tokens.
