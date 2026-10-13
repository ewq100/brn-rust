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
