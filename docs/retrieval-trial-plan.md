# Retrieval adapter trial implementation plan

Goal: qualify local Rust retrieval with synthetic documents and shared source/version/passage evidence. This is roadmap 03, independent of the still-blocked editor acceptance gate. User authorized autonomous implementation, delegation, verification and trial push; no production framework decision or private corpus ingestion is implied.

## Design

One standalone `experiments/retrieval-trial` crate, own workspace/lock, Rust 1.98.1. Use keyword ranking, real FastEmbed CPU embeddings and LanceDB exact cosine vector scans, with reciprocal-rank fusion for hybrid search. All profiles use one request and evidence contract and identical source/version/approval/current filters before ranking. Retain immutable fixture versions and validate quote byte offsets, source hash and identifiers. Never substitute fake vectors or keyword fallback for failed semantic search.

A small synthetic fixture corpus includes current/superseded contradictions, draft/withdrawn items, overlapping identifiers, Unicode and exact identifiers. Freeze comparison queries and expected sources before inference. Persist the disposable index plus model metadata; reopening must reject incompatible/missing state clearly. Explicit model download/build commands are separate from ordinary offline checks. No user documents, provider account, service, editor code or originals are touched.

Initial dependency candidates from Astra's primary-source inspection: LanceDB 0.39.0 / Arrow58 / FastEmbed7.1.0 CPU / AllMiniLML6V2 (384 dimensions). Pin versions that the actual package registry supplies and document justified compatibility changes. Skip Swiftide because fixture ingestion does not warrant orchestration. No ANN index for this tiny corpus.

## Task 1 — shared contract and adapters

Owner: Sol, exclusive ownership of experiments/retrieval-trial. Parent owns scripts and project docs. Write failing contract tests first; implement typed profile/filter/evidence and immutable fixtures, conservative validations, deterministic keyword search and fusion. Add optional native dependencies and CLI for explicit index creation and query/benchmark/reopen. Real FastEmbed inference and LanceDB storage/search must be exercised for semantic qualification. Keep core tests usable without model downloads. Test invalid query/limit, conflicting/empty filters, Unicode provenance, version isolation, fusion deduplication and corrupt/missing index behavior. Run tests/build/fmt/Clippy; report exact evidence and any blocker. No commits by worker.

## Task 2 — integration evidence and review

Owner: parent, with independent Astra review. Run native dependency build and actual synthetic inference/index/query/reopen; measure cold/warm time, fixed-query hit quality, model/index disk size and identify licenses/download requirements. Run credential-free regression checks, secret/diff review, targeted fixes and re-review. Record remaining native packaging/performance/generalization limits honestly. Commit and push trial/retrieval-adapters; compare remote SHA to HEAD.

## Review focus

- Filters must apply before top-k, consistently across profiles.
- Stale/corrupt provenance must error instead of silently presenting wrong evidence.
- Offline or missing model/index failures must not trigger unrequested fallback.
- State creation must refuse overwriting unrelated paths or partially publishing an index.
- Synthetic results must not be represented as user-corpus quality or production readiness.

## Progress

- [x] Roadmap and clean checkpoint inspected; Mac still locked.
- [x] Architecture advice gathered from Astra; synthetic-only scope chosen.
- [x] Contract, tests and real adapters implemented.
- [x] Native evaluation and verification recorded.
- [x] Independent Astra review and scoped re-review.
- [x] Commit, push and remote verification: implementation `95b5bb5f6d5e6fcca49f8fe192b2ec2b5741b08d`; documentation delivery follow-up recorded in evidence.
