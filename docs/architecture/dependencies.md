# Architecture checkpoint dependency record

> **Baseline reference for the Threads rebuild.** This document describes the pre-Threads implementation or historical evidence. The [canonical Threads target](threads-target.md) owns the replacement contract. Inspect this material for component reuse; do not resume its old tasks or infer current authorization from it. Update this local contract when its implementation is replaced.

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](https://github.com/ewq100/brn-rust/blob/af9239c741c7ab0983e62f0253e607b9607727e6/docs/status.md) for the present checkout and remaining gaps.

Inspected 2026-09-27 from the three committed trial Cargo.lock files and matching downloaded Cargo manifests. These are **observed trial versions**, not a complete distribution audit or approval of a production dependency graph. Lockfiles remain the exact transitive record. No library was added to the production workspace for this checkpoint.

| Role | Observed artifact/version | Declared license | Decision/status |
|---|---|---|---|
| Rust toolchain | 1.98.1, pinned in root | Toolchain upstream licensing | Keep tested toolchain for next chunk |
| UI/component facade | gpui-kit, gpui-base, gpui-component, gpui-kit-assets 0.6.6 | Apache-2.0 | Provisional until native editor acceptance |
| Underlying native UI | gpui-pre and platform crates 0.3.6 | Apache-2.0 | Lock alongside facade; do not assume facade version identifies native engine |
| Revision diff | similar 2.7.0 | Apache-2.0 | Reuse behind document-review seam |
| Provider executable | codex-cli 0.155.0-alpha.16.4, installed arm64 binary | Exact artifact redistribution not cleared; official source is Apache-2.0 with notices | Explicit installed path for local development; no copying embedded ChatGPT binary |
| Provider JSON | serde_json 1.0.151 | MIT OR Apache-2.0 | Trial transport dependency |
| Embedded vector DB | lancedb 0.39.0 | Apache-2.0 | Candidate backend behind retrieval boundary; compile workaround enables remote feature while runtime paths stay local |
| Arrow transport | arrow-array / arrow-schema 58.4.0 | Apache-2.0 AND MIT / Apache-2.0 | Keep inside adapter, not domain contract |
| Local embeddings | fastembed 7.1.0 | Apache-2.0 | Candidate local embedding worker |
| ONNX Rust binding | ort / ort-sys 2.0.0-rc.13 | MIT OR Apache-2.0 | Native runtime/packaging still to qualify |
| ONNX Runtime | 1.28.0 prebuilt arm64 | MIT plus third-party notices | Trial downloaded archive includes CoreML support; execution uses default CPU |
| Embedding model | Qdrant/all-MiniLM-L6-v2-onnx, observed snapshot `8f518e882455312b086101e60691f5e6e2f05c3c3` | Model card Apache-2.0 | Record immutable revision and per-file hashes for later reproducible distribution |
| Async runtime | tokio 1.53.1 in trial lockfiles | MIT | Candidate runtime for bounded background workers |
| Serialization/error/hash helpers | serde 1.0.229; thiserror 2.0.21; sha2 0.10.9; hex 0.4.3 | MIT OR Apache-2.0 | Keep data formats explicit and versioned |
| Authoritative data storage | SQLite proposed; Rust binding/version not yet qualified | Not inventoried for a shipped artifact | Select/pin and verify migration/recovery behavior in chunk06 before storing user data |

The editor lockfile also contains `gpui-pre-reqwest 0.12.15` (MIT OR Apache-2.0). Runtime native libraries, fonts/assets, model files and transitive notices require artifact-specific review before distribution. These metadata checks make no claim that the entire application has one license.

## Observed constraints informing the design

- The native editor builds with Command Line Tools on macOS 15.3.1 arm64, but remains uninspected while the Mac is locked. `block 0.1.6` emits a future-Rust incompatibility advisory. No framework acceptance is inferred from compilation.
- The subscription provider successfully streamed, handled a read-only tool call, continued/resumed conversations, interrupted a turn and proactively refreshed managed authentication. Natural expiry/revocation, clean-machine sidecar installation and signing remain unverified. See [sidecar distribution boundary](../../experiments/codex-app-server/PACKAGING.md).
- LanceDB's `remote` feature workaround is explained in [retrieval evidence](../work/completed/retrieval-trial/evidence.md). The debug binary is approximately 647 MB and the linker reports a large unwind section. Neither is a measured release-bundle size. A reusable loaded engine is needed before extrapolating the measured 5–13 ms query times: complete trial reopening took 2.18 seconds.
- Keyword ranking in the trial is simple substring matching. A production lexical ranking/index choice (for example SQLite FTS5) needs its own contract/quality tests; the trial does not establish BM25 quality.
- All retrieval quality observations use seven synthetic versions with four default-eligible passages. No real user corpus was imported. Embedding dimensions are 384, whole tiny documents form single passages, and no ANN index is trained.

## Build and release boundary

The proposed first desktop shell should depend only on the UI/core it needs; defer embedding/vector libraries until the search integration chunk. Keep provider and retrieval libraries out of domain types. This avoids treating a successful trial as permission to load every native dependency into the first app window.

Initial development continues with the installed sidecar and explicit model acquisition. A future distributable needs pinned artifact provenance, license/notice inventory, signed/notarized app and sidecar, model acquisition/update rules, and clean-machine/offline acceptance. No signing credentials, account settings or redistribution action is part of this checkpoint.


## Storage checkpoint additions (2026-09-28)

Chunk 06 adds `brn-store` independently of the desktop dependency graph. The root lockfile pins rusqlite 0.40.2 (MIT), libsqlite3-sys 0.38.2 (MIT), SHA-256 via sha2 0.10.9 (MIT OR Apache-2.0), UUID generation via uuid 1.26.1 (Apache-2.0 OR MIT), and test-only tempfile 3.27.0 (MIT OR Apache-2.0). These declarations were read from Cargo metadata and downloaded package manifests. Rusqlite uses `default-features = false` with `bundled`, compiling SQLite 3.53.2 from the pinned package (version read from its bundled header) instead of relying on macOS's system SQLite. [Rusqlite features](https://docs.rs/crate/rusqlite/0.40.2/features) document that build choice. No additional paid service or database server is introduced. The original checkpoint table above remains the historical inventory; [storage evidence](../work/completed/storage-recovery/evidence.md) records qualification and remaining limits.

## Integrated workflow additions (2026-09-28)

The first end-to-end flow now integrates the previously qualified UI, SQLite, provider and retrieval adapters into the root workspace. Native retrieval remains an explicit feature; the default workspace path uses keyword retrieval without loading ONNX/LanceDB. The root lockfile records the combined transitive graph, retaining FastEmbed 7.1.0, LanceDB 0.39.0, Arrow 58.4.0, ORT 2.0.0-rc.13, Tokio 1.53.1 and the UI/storage versions above. Development/test debug information is disabled to bound native build artifacts. No executable or model is bundled, and no distribution approval is inferred. See [integrated evidence](../work/completed/end-to-end-flow/evidence.md) for the observed runtime path and outstanding packaging limits.

## Selected P2 intake dependencies — 2026-10-07

The separate `brn-intake-helper` pins BetterOffice edit/parse/opc 0.3.0 and
mail-parser 0.11.8 (`full_encoding`) for maintained structured/Markdown/package
exports and MIME decoding. html5ever 0.27 tokenizes inert HTML CID references;
image 0.25.10 decodes PNG/JPEG. The lightweight protocol uses png 0.18.1 for
complete retained PNG validation. The larger export graph is an accepted P2
tradeoff; the helper owns no collaboration lifecycle. Store's direct ZIP/XML/PNG
implementation dependencies and bespoke DOCX interpreter are removed. See the
[adapter contract](../../crates/brn-intake/README.md) for quotas, native containment
and unqualified distribution/RSS limits. Historical dependency evidence above
remains dated evidence rather than a complete current shipping manifest.
