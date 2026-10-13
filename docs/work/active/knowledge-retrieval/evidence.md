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
