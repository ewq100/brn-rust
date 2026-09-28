# Retrieval adapter trial evidence

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

Date: 2026-09-27. Branch: `trial/retrieval-adapters`. Base: `d422e192120dad778b181aba8d9367361689a229`. Worktree: `/private/tmp/brn-editor-trial` (reused isolated worktree; earlier editor branch remains at its pushed checkpoint).

## Scope and environment

The user authorized the next trial with implementation, delegation, verification and push. Native editor inspection was attempted first; computer use still reported the Mac locked. Roadmap 03 is independent of that interaction gate, so this trial uses newly authored synthetic documents and does not read/import the user's vault. No merge, release, account setting change or paid model service is included.

Host: Apple Silicon arm64, macOS 15.3.1, pinned Rust 1.98.1 and Command Line Tools. Initial free disk: 29 GiB. Protobuf compiler was absent; `brew install protobuf` installed protobuf 36.2 and abseil 20260817.0. `protoc --version` returned `libprotoc 36.2`.

Astra advised the contract and dependency route. Sol owns the trial implementation. Luna reviewed dependency provenance and the verification script. Parent owns integration and final verification; source review does not substitute for measured runtime results.

## Dependency findings

Cargo registry confirmed FastEmbed 7.1.0 (Apache-2.0, Rust >=1.88) and LanceDB 0.39.0 (Apache-2.0, Rust >=1.91). Arrow resolved to 58.4.0. The lockfile is authoritative for transitive versions. No Swiftide orchestration or approximate vector index is needed for the tiny corpus.

An actual native build of LanceDB 0.39.0 with `default-features=false` failed: `src/job.rs:102,112` uses `Error::Http` unconditionally, while `src/error.rs:113` gates that variant on `remote`. The trial enables `remote` as a compilation workaround; it uses only local database paths, with no remote service configured. This increases dependency/packaging surface and should be revisited before production rather than hidden by a vendor patch.

FastEmbed uses `ort` / `ort-sys` 2.0.0-rc.13. The build downloaded the Apple Silicon ONNX Runtime 1.28.0 archive from `cdn.pyke.io`; the archive includes CoreML support, though trial inference selects default CPU behavior. Cargo's `--offline` does **not** disable network access from dependency build scripts: the first ORT download failed in the network sandbox and succeeded when explicitly allowed. The later cached build is the offline check.

The AllMiniLML6V2 mapping in downloaded FastEmbed source is `Qdrant/all-MiniLM-L6-v2-onnx`, file `model.onnx`. Model files download from Hugging Face; inference on fixture/query text is local. Reopening is implemented through FastEmbed's local byte-loading API, not Hub initialization. The model card identifies Apache-2.0; ONNX Runtime is MIT and ort is MIT OR Apache-2.0. Redistribution requires retaining applicable licenses and notices, including transitive native notices; no installer or complete distribution-license audit is supplied here.

Primary references:

- [LanceDB 0.39.0 source](https://docs.rs/crate/lancedb/0.39.0/source/)
- [FastEmbed Rust source](https://github.com/Anush008/fastembed-rs)
- [Model card and files](https://huggingface.co/Qdrant/all-MiniLM-L6-v2-onnx)
- [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE)
- [ORT release notes](https://github.com/pykeio/ort/releases)

## Existing regression baseline

`PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh` passed: starter/provider builds, formatting, Clippy, 16 provider tests and nine smoke cases. No live provider calls were made.

`PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-editor-trial.sh` passed: 11 model tests, native build, formatting, Clippy and five CLI smoke cases. The existing transitive `block 0.1.6` future-incompatibility advisory remains. Native editor interaction and user acceptance are still unverified because of the locked Mac.

## Retrieval verification

The final verification commands were:

```sh
CARGO_BUILD_JOBS=4 bash scripts/verify-retrieval-trial.sh --native
experiments/retrieval-trial/target/debug/brn-retrieval-trial build --state /private/tmp/brn-retrieval-final-state-20260927
bash scripts/verify-retrieval-state.sh /private/tmp/brn-retrieval-final-state-20260927
/usr/bin/time -p experiments/retrieval-trial/target/debug/brn-retrieval-trial reopen --state /private/tmp/brn-retrieval-final-state-20260927
```

All exited 0. The first command passed locked cached builds, formatting, Clippy (`-D warnings`), eight contract tests and three native state tests, plus help/check and invalid/missing command smoke checks. Unit tests use no model or credentials. Tests were written before implementation, but dependency access blocked the initial failing execution; no fully observed red-green cycle is claimed.

Explicit `build` required network permission to fetch model files, indexed **seven versioned passages** (four current/approved), and reported **13,290 ms** including model fetch/init and indexing. The final state path was newly created. Afterward the runtime script ran in the default network-restricted environment: it evaluated all profiles, checked empty/historical/draft/withdrawn filters across all three, and reopened in a separate process using local model bytes. It then checked seven failures on a disposable copy: missing state, existing output refusal, missing model file, changed model file, changed documents, incompatible complete-format manifest, and changed database file. Specific error diagnostics were checked; no fallback occurred. The original completed state stayed intact.

Fixed expected-source rank (lower is better):

| Query | Keyword | Semantic | Hybrid |
|---|---:|---:|---:|
| launch window | 1 | 1 | 1 |
| café | 1 | 1 | 1 |
| BRN-482 | 2 | 2 | 2 |
| when does the release begin | 3 | 2 | 3 |

Each profile achieved hit@3 4/4 and hit@1 2/4. Only four default-eligible passages exist: this is a wiring/filter/provenance comparison and does **not** demonstrate semantic superiority or reliable real-corpus relevance. Semantic search returns nearest eligible passages even for an unrelated query; there is no relevance rejection threshold. Keyword ranking is the fraction of whitespace query terms appearing as case-insensitive substrings, not BM25. Hybrid uses RRF with constant 60 and up to 100 candidates from each adapter, sufficient for this fixture only. Whole tiny documents are one passage (`p0`); production chunking and tokenizer truncation on long documents are not qualified.

Measured in the final evaluation process, query times exclude model loading/hash checks: keyword rounded to 0 ms; semantic 6–13 ms; hybrid 5–6 ms. Separate-process reopen reported an 8 ms query, while `/usr/bin/time -p` measured **2.18 seconds** for the complete reopen process, including file verification and model initialization. These are individual debug-build observations, not latency percentiles or a scale benchmark.

Model artifact payload totals **91,102,048 bytes**; LanceDB's four files total **13,792 bytes**. `du -sk` reports 88,980 KiB for the local model copy, 99,052 KiB for the download cache, and 28 KiB for the database. The cache also contains symlinked snapshot files; summing followed symlink sizes double-counts payloads. Keeping both download cache and independent verified model copy is deliberate trial overhead.

Observed Hugging Face snapshot: `8f518e882455312b086101e60691f5e6e2f05c3c3`. Model ONNX SHA-256: `bbd7b466f6d58e646fdc2bd5fd67b2f5e93c0b687011bd4548c420f7bd46f0c5`. Every model file hash, document snapshot hash and database file hash is recorded in state `manifest.json`. New builds resolve the upstream model and capture its hashes; they are not guaranteed to reproduce this snapshot forever. Reopen verifies that captured state, not a trust/signature boundary against a malicious actor who rewrites both files and manifest.

## Review, limitations and delivery

Astra source review found no Critical or Important implementation defects. Follow-up coverage suggestions were implemented: a true interior UTF-8 boundary test, a complete wrong-format manifest, uniquely created temporary test directories, all-profile constrained queries, and actual-state corruption qualification. Astra scoped re-review approved those changes and the runtime script. Parent independently executed the recorded checks.

The linked unoptimized executable is **647,290,736 bytes**. Apple's linker warned that `__eh_frame` exceeds 16 MiB and exception-handling performance could be affected. The executable runs, but release size, stripping, signed/notarized distribution, native dependency redistribution and sustained desktop performance have not been verified. The upstream feature workaround and transitive dependency size are architecture-checkpoint inputs.

Each public `native::search` currently reloads/verifies model and state, even for keyword profile; the internal evaluation reuses loaded resources. Offline core `check` does not need a model. This deliberately simple lifecycle should be replaced by an owned reusable engine only during a later integration phase. The completed state is immutable; crash-resilient publication/fsync guarantees, concurrent mutation, index updates/deletions and large-corpus search are not supplied by this trial. Failed construction leaves an incomplete directory; retry requires a new path and never overwrites it automatically.

No user corpus, provider credentials or model service were used. No UI acceptance, architecture approval, merge, installer or release is implied. Git delivery will be verified against the private remote and reported with the final commit.


## Verified delivery

Implementation commit `95b5bb5f6d5e6fcca49f8fe192b2ec2b5741b08d` was pushed to private `origin` on `trial/retrieval-adapters`. `git ls-remote origin refs/heads/trial/retrieval-adapters` returned the identical SHA; the worktree was clean. A secret-pattern scan covered all 16 changed/authored files with no findings; all 578 lockfile dependency sources were crates.io. `git diff --check` passed. This documentation-only follow-up records completed delivery; its final remote-verified commit is reported in the task response. Nothing was merged or released.
