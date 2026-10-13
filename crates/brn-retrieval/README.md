# brn-retrieval

Derived search components for the Threads application. The [shared app](../brn-threads-app/src/lib.rs) opens an in-memory `NoteIndex`, refreshes it from current non-archived/non-superseded notes and performs FTS5 keyword queries. SQLite in `brn-threads-core` remains authoritative; opening a workspace can rebuild retrieval.

`search_index` is the persisted workspace index (`derived/search.sqlite`, schema `BRNS` v1) from the [knowledge and retrieval plan](../../docs/work/active/knowledge-retrieval/spec.md). It has exact and English-stemmed FTS5 lanes over `markdown_chunk` v1 passages, a links mirror, and a sync watermark. Opens and rebuilds go through an advisory lock, and writes are guarded against concurrent writers with `Changes.base`. The app's search worker (`brn-threads-app::search`) keeps it current and re-checks results with core visibility. The app's `Workspace::search` still uses the in-memory `NoteIndex` until Task 2 switches it.

The crate also retains a reusable persistent index reader/writer, metadata/edge cache and optional local embeddings. Their richer scope/model contracts below describe component behavior, not a claim that current Threads search uses semantic ranking or the former Markdown-vault orchestration.

## Language scope

Retrieval is tuned and qualified for English only. Estonian is on hold by owner decision (13 October 2026). Keep tokenizer and diacritic policy, stemming, the embedding model and evaluation sets as derived-index configuration. Adding a language should then mean adding an analyzer or model and rebuilding the index, never migrating authoritative data. Today's tokenizer is `unicode61 remove_diacritics 2`. That fits English but folds distinct Estonian letters (õ/ä/ö/ü), so revisit it before qualifying Estonian.

## Dependencies and features

No workspace dependencies; defaults are empty. `native` enables FastEmbed/ONNX and the explicit bounded model installer with pinned reqwest, sha1 and macOS libc. `brn-threads-app/native-retrieval` exposes that optional graph; desktop combines it with `native-ui`. Builds alone do not establish model or corpus quality.

## Reader and model snapshot contract

`NoteIndexReader::open` uses read-only SQLite flags, query-only mode and a
busy timeout, validating BRNI application ID, schema version/objects and
integrity. It never calls the writer's create/rebuild path and never repairs a
foreign or damaged file. `NoteSearch` keyword/vector queries and note listing
share connection helpers between reader and writer.

BRNI schema V3 caches optional Markdown UUIDs, source/history flags and metadata
issues supplied by the caller. The branded writer rebuilds old/damaged derived
schemas; the reader refuses them without repair. UUID duplication is allowed
here and resolved by fresh caller evidence inspection. `KnowledgeScope`
defaults to Current: eligible current knowledge only. Source includes original
sources in any state; History includes historical knowledge and sources; All
includes every eligible classified note. Rows with metadata issues are excluded
from every scope. Scope filtering precedes keyword/vector ranking and limits.
Metadata updates preserve unchanged passages and embeddings; no index row is
authoritative knowledge or proof of UUID uniqueness.

V3 adds disposable typed note edges for explicit Markdown links and separately
labelled provenance inferences. Healthy V2 upgrades additively without replacing
passages or vectors; V2 readers refuse until the writer upgrades. Edge replacement
is atomic and requires distinct unique cached UUIDs, eligible endpoint metadata,
full saved hashes and exact nonempty Unicode byte/quote proofs reconstructed from
contiguous passages. Each coalesced edge admits at most 8192 proofs and 4 MiB of
quotes; proof ranges end within 1 MiB. Explicit-link proofs belong to the source,
provenance-inference proofs to the target. Duplicate pairs/origins or proofs are
refused without clearing the prior cache.

Writer/reader edge queries share one SQLite snapshot, deterministic source/target/
origin ordering, both-endpoint scope filtering before pagination, matching totals,
and limits of 1–200. Malformed stored schemas, endpoints or proofs make readers
refuse and branded writers rebuild. Note changes remove touching edges; cached
UUID alias admission also invalidates affected endpoints. Identical metadata and
mtime-only touches retain proofs. Callers supply fresh source observations;
these index records confer no durable knowledge or identity authority.

The writer checkpoints its newly created BRNI header before readers attach,
so a live/new index cannot look like a foreign unbranded file on a subsequent
open. Existing foreign-file refusal rules are unchanged.

`semantic_for_model` checks identity **and** dimension in the same short read
transaction as vector/hit lookup; `ModelMismatch` never becomes keyword
fallback. Writer stale-passage/model insertion checks remain intact. Close
readers before the writer checkpoints at shutdown; drain/detach readers before
rebuilding/replacing the index file or switching a tool/model snapshot.

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
Other explicit or saved directories retain their exact legacy behavior and identity.
The 128-token limit needs real bilingual/long-passage quality qualification;
synthetic profile checks do not establish ONNX compatibility. Native tests use
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
