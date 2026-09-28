# Retrieval adapter trial

This standalone Rust crate uses only synthetic passages. It compares deterministic keyword ranking, FastEmbed 7.1.0 CPU embeddings with AllMiniLML6V2, LanceDB 0.39.0 exact cosine search, and reciprocal-rank hybrid fusion. The same `Request` and `Evidence` types cover all profiles. Source ID, version ID, passage ID, byte offsets, quote and SHA-256 provenance are checked against an immutable fixture snapshot. Default filters select current approved passages; explicit filters can inspect historical, draft or withdrawn fixture versions.

Rust 1.98.1, `protoc`, and a native C++ toolchain are needed for the native feature. The explicit build step downloads a public model (about 90 MB) and an ONNX Runtime binary through FastEmbed's dependencies. Network access is needed only to obtain dependencies and create a new state. Reopen/evaluate use the hashed local model files, not the hub. Keep the state under a disposable directory outside source control.

```sh
PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH cargo +1.98.1 test --manifest-path experiments/retrieval-trial/Cargo.toml --locked --offline
PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH cargo +1.98.1 run --manifest-path experiments/retrieval-trial/Cargo.toml --locked --offline -- check
PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH cargo +1.98.1 run --manifest-path experiments/retrieval-trial/Cargo.toml --features native --locked -- build --state /private/tmp/brn-retrieval-trial-state
PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH cargo +1.98.1 run --manifest-path experiments/retrieval-trial/Cargo.toml --features native --locked --offline -- evaluate --state /private/tmp/brn-retrieval-trial-state
PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH cargo +1.98.1 run --manifest-path experiments/retrieval-trial/Cargo.toml --features native --locked --offline -- reopen --state /private/tmp/brn-retrieval-trial-state
```

`build` requires a *new* directory. A failed build leaves an incomplete directory that cannot be queried or overwritten. State has a completion marker, model and document hashes, and a database file inventory. Reopen rejects mismatched files, missing rows and changed source/version identities. Indexes and model files are derived trial data, not authoritative user documents.

The fixed queries include exact identifiers, Unicode, a launch-version contradiction, and a paraphrase. Ranking hit@3 is printed as a measurement. The evaluator fails on filter/provenance invariant violations. Results on this tiny synthetic fixture do not estimate quality on the user's corpus or native app packaging performance.

The pinned `lancedb` package currently requires its `remote` Cargo feature to compile because a job error variant is conditionally compiled upstream while referenced unconditionally. This enables code paths at build time but the trial uses only local filesystem connection paths; no remote LanceDB service is contacted. CPU execution is selected for inference; ONNX Runtime's Apple Silicon prebuilt archive may include CoreML components.
