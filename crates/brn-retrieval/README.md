# brn-retrieval

Search over the vault's notes: a disposable `index.sqlite` with FTS5 keyword search, local embeddings and reciprocal-rank fusion ([`note_index`](src/note_index/mod.rs)), plus the local embedding model ([`native`](src/native.rs), feature `native`).

## Dependencies and features

No workspace dependencies. Default features are empty. `native` enables FastEmbed/ONNX for the local embedding model and an explicit blocking installer using pinned reqwest 0.12.28, sha1 0.10.6 and macOS libc 0.2.189. Consumed by `brn-workflow`.

## Reader and model snapshot contract

`NoteIndexReader::open` uses read-only SQLite flags, query-only mode and a
busy timeout, validating BRNI application ID, schema version/objects and
integrity. It never calls the writer's create/rebuild path and never repairs a
foreign or damaged file. `NoteSearch` keyword/vector queries and note listing
share connection helpers between reader and writer.

The writer checkpoints its newly created BRNI header before readers attach,
so a live/new index cannot look like a foreign unbranded file on a subsequent
open. Existing foreign-file refusal rules are unchanged.

`semantic_for_model` checks identity **and** dimension in the same short read
transaction as vector/hit lookup; `ModelMismatch` never becomes keyword
fallback. Writer stale-passage/model insertion checks remain intact. Close
readers before the writer checkpoints at shutdown; drain/detach readers before
rebuilding/replacing the index file or switching a tool/model snapshot.

## Pinned, explicit native installer

[`download_model`](src/native/download.rs) installs only the five MiniLM
assets at revision `751bff37182d3f1213fa05d7196b954e230abad9` of
`Xenova/all-MiniLM-L6-v2`: `onnx/model.onnx` (installed as `model.onnx`),
`tokenizer.json`, `config.json`, `special_tokens_map.json` and
`tokenizer_config.json`. Exact per-file sizes total **91,100,408 bytes**.
ONNX uses its pinned SHA-256; configuration files use pinned Git blob SHA-1,
including the `blob <length>\0` prefix. No response-provided digest or floating
revision is trusted.

Downloads have bounded connect/request timeouts, enforce size while streaming
and check cancellation between chunks. Assets are verified and synced in an
exclusive private sibling stage, then installed as one complete directory with
macOS `renameatx_np(RENAME_EXCL)`. Other platforms report typed exclusive-install
unavailability. Verified pinned directories can be reused without fetching;
other occupied targets are unchanged. Failure/cancellation cleans only
identity-checked files in the install's own stage. No implicit retry.

Workflow supplies fresh consent and owns the blocking job. `LocalEmbedder::open`
remains **load-only**, with its existing five-asset identity. Native tests use
private synthetic manifests/HTTP sources; no public alternate manifest or
production asset download is used in verification.

## Verification

Run from the repository root:

```sh
cargo test -p brn-retrieval --locked
cargo test -p brn-retrieval --features native --locked
cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline
```

The local model test runs only when `BRN_NATIVE_MODEL_DIR` points at the model files; a skip is not model verification. Never treat an index as authoritative storage.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
