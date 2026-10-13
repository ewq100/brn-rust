# brn-retrieval

Derived search components for the BRN application, built for the [knowledge and retrieval architecture](../../docs/work/active/knowledge-retrieval/spec.md). SQLite in `brn-threads-core` stays authoritative: everything here can be deleted and rebuilt.

- **`search_index`**: the persisted workspace index `derived/search.sqlite` (brand `BRNS`, schema v2).
  - Exact, English-stemmed and all-words keyword lanes over `markdown_chunk` v1 passages, fused by RRF and collapsed per document.
  - A dense lane over per-model vectors, a links mirror, exact counts, and link facts scoped to the caller's context.
  - Opens and rebuilds go through an advisory lock shared by every BRN process. Writes are guarded against concurrent writers with `Changes.base`.
  - Passage IDs are never reused. A foreign file is refused unchanged.
  - The app's search worker (`brn-threads-app::search`) keeps it current and re-checks every result with core visibility.
- **`markdown_chunk`**: heading- and paragraph-aware chunks with heading paths and exact byte ranges.
- **`snippet`**: bounded matching context with highlight ranges.
- **`embedding::Embedder`**: the model interface, with prefixes and a per-model similarity floor.
- **`native`** (feature): the load-only `LocalEmbedder` and the pinned, explicit installer described below.

The vault-era `NoteIndex` (metadata, UUID and edge-proof caches, and its reader) was retired in Task 6 of the plan once nothing used it.

## Language scope

Retrieval is tuned and qualified for English only. Estonian is on hold by owner decision (13 October 2026). Keep tokenizer and diacritic policy, stemming, the embedding model and evaluation sets as derived-index configuration. Adding a language should then mean adding an analyzer or model and rebuilding the index, never migrating authoritative data. Today's tokenizer is `unicode61 remove_diacritics 2`. That fits English but folds distinct Estonian letters (õ/ä/ö/ü), so revisit it before qualifying Estonian.

## Dependencies and features

No workspace dependencies; defaults are empty. `native` enables FastEmbed/ONNX and the explicit bounded model installer with pinned reqwest, sha1 and macOS libc. `brn-threads-app/native-retrieval` exposes that optional graph; desktop combines it with `native-ui`. Builds alone do not establish model or corpus quality.

## Pinned, explicit native installer

[`download_model`](src/native/download.rs) installs only the five
`multilingual-e5-small` assets (decision D3) at revision
`761b726dd34fb83930e26aab4e9ac3899aa1fa78` of `Xenova/multilingual-e5-small`,
the ONNX conversion of the MIT-licensed `intfloat/multilingual-e5-small`:
`onnx/model_quantized.onnx` (installed as `model.onnx`), `tokenizer.json`,
`config.json`, `special_tokens_map.json` and `tokenizer_config.json`. Exact
per-file sizes total **135,392,183 bytes**. The previously pinned MiniLM bundle
(revision `2c4055b1…` of `Xenova/paraphrase-multilingual-MiniLM-L12-v2`,
135,392,488 bytes) is no longer installed, but an existing install is still
recognised and verified.
ONNX and tokenizer use pinned LFS-content SHA-256; the other files use Git blob SHA-1,
including the `blob <length>\0` prefix. No response-provided digest or floating
revision is trusted.

Downloads have bounded connect/request timeouts, enforce size while streaming
and check cancellation between chunks. Assets are verified and synced in an
exclusive private sibling stage, then installed as one complete directory with
macOS `renameatx_np(RENAME_EXCL)`. Other platforms report typed exclusive-install
unavailability. Verified pinned directories can be reused without fetching;
other occupied targets are unchanged. Failure/cancellation cleans only
identity-checked files in the install's own stage. No implicit retry.

The host supplies fresh consent and owns the blocking job (`brn model-install
--destination …` in native builds). `LocalEmbedder::open` remains **load-only**.
A recognised pinned bundle requires all five exact assets before initialization
and hashes the entire bundle into a distinct profile identity. e5-small uses mean
pooling, 384 dimensions, 512 tokens, Static quantization, `query: ` and
`passage: ` prefixes and a provisional 0.80 similarity floor; MiniLM keeps 128
tokens, no prefixes and its identity.
Other explicit directories retain their exact legacy behavior and identity.
Synthetic profile checks do not establish ONNX compatibility or quality; that
is the owner-run qualification in the plan. Native tests use
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
