# brn-retrieval

Search over the vault's notes: a disposable `index.sqlite` with FTS5 keyword search, local embeddings and reciprocal-rank fusion ([`note_index`](src/note_index/mod.rs)), plus the local embedding model ([`native`](src/native.rs), feature `native`). The older generation-based `Index` in [lib.rs](src/lib.rs) keeps keyword search only until the cleanup step removes it.

## Dependencies and features

No workspace dependencies. Default features are empty. `native` enables FastEmbed/ONNX for the local embedding model. Consumed by `brn-workflow`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-retrieval --locked
cargo test -p brn-retrieval --features native --locked
```

The local model test runs only when `BRN_NATIVE_MODEL_DIR` points at the model files; a skip is not model verification. Never treat an index as authoritative storage.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
