# Vault Markdown format

This is the current V1 file contract, consolidated from the implementation and
[workflow contract](../../crates/brn-workflow/README.md#typed-proposal-review-foundation).
It documents existing bytes and authority; it introduces no format migration.
The [ownership model](overview.md#frozen-target),
[invariants](invariants.md#frozen-target-guarantees) and
[semantic intelligence and deterministic authority](overview.md#semantic-intelligence-and-deterministic-authority)
govern interpretation and changes. Operational proposals, review comments,
sessions and recovery records belong to [WorkStore](../../crates/brn-store/README.md),
not additional note fields.

## Files and byte preservation

Notes are ordinary UTF-8 Markdown files with a case-insensitive `.md` suffix.
Current note reads and writes are bounded to 1 MiB (1,048,576 bytes), including
frontmatter. Vault-relative paths use `/`, remain contained, exclude hidden
components and do not traverse symlinks. An optional leading UTF-8 BOM, original
line endings and unrelated frontmatter/body bytes remain exact. A note needs no
managed metadata merely to be readable as ordinary current knowledge.

Images and other meaningful assets belong in ordinary files alongside Markdown,
as required by the [product vision](../product/BRN_PRODUCT_VISION.md).
Ordinary Markdown links/images may refer to them. The current text-only Inbox
conversion formats below do not implement binary extraction, asset placement or
a new managed asset field; those capabilities follow the [roadmap](../roadmap.md).
Asset links are not automatically note-to-note relationships.

## Managed frontmatter

BRN interprets a narrow set of ordinary unindented root fields, rather than a
general YAML schema. A supported leading header opens with an exact `---` line
(after an optional BOM) and closes with an exact `---` or `...` line. LF and CRLF
are supported. Managed keys are case-sensitive and require whitespace after
their colon. Scalar identity/kind/state values may be unquoted or simply
single/double quoted, with optional trailing whitespace/comment. JSON values
below remain JSON on one physical line, not a quoted YAML string or block scalar.

Duplicate managed keys, recognizable unsupported key layouts, malformed managed
values and incomplete managed headers are refused rather than interpreted as
absent. Unrelated fields remain opaque; unmanaged Markdown thematic breaks are
not automatically frontmatter. See the shared
[field reader](../../crates/brn-store/src/note_identity.rs).

| Field | Current value and meaning |
| --- | --- |
| `brn_id` | Optional nonnil canonical hyphenated UUID; input hex may be upper/lower case, BRN writes lowercase. Stable logical identity, independent of path and content hash. Duplicates remain ambiguous; no automatic reassignment occurs. |
| `brn_kind` | Optional scalar `knowledge` or `source`; absence means knowledge. Source wording is evidence, separate from curated knowledge. |
| `brn_state` | Optional scalar `current` or `history`; absence means current. History is retained meaningful evidence rather than current knowledge. |
| `brn_inbox_source` | Optional strict single-line JSON object recording original-copy provenance, described below. |
| `brn_provenance` | Optional strict single-line JSON array of exact vault citations, described below; absence means no citations. |

There is no overall `brn_version` field in this format. The Source conversion
`format` string explicitly identifies its V1 body encoding. Citation objects have
no separate version field. Hashes remain JSON arrays of exactly 32 integer bytes
in the range 0–255; timestamps remain integer milliseconds since the Unix epoch.
These forms are also used by existing checked records. Do not convert hashes to
hex, rename/reorder generated Source fields or normalize notes as an incidental
documentation or serialization cleanup.

## History and retrieval scopes

`brn_state: history` is the canonical in-note History marker. The existing
case-insensitive top-level `archive/` convention is an additional read-only
History alias: `archive/a.md` and `Archive/a.md` count as History even with
`brn_state: current`. A nested `projects/archive/a.md` does not acquire that alias.
Archive paths permit explicit evidence reads, but cannot be editor or proposal
write destinations. Frontmatter History at another visible path retains the
existing path-based editing authority; a History label alone is not a write fence.

For metadata-valid notes, Current excludes both Source and History; Source
includes Source notes even when historical; History includes historical notes
even when Source; All includes these valid classes together. Classification is
recorded metadata/path policy, not an assessment that wording is correct.
Source conversion, a citation, an explicit link or a History/current label never
establishes semantic truth or resolves a contradiction by itself.

Unknown `brn_kind` or `brn_state` values fail closed. Malformed managed identity,
classification or `brn_provenance` produces visible
`RefreshReport.unreadable` issues and exclusion from every scoped query,
including All. Unsupported values are not silently promoted to Current.
`EvidenceNote` can still read explicit original text with malformed metadata,
including archives, subject to the same size, UTF-8, file/path and unresolved
Save/application fences. Identity inventory separately reports duplicates and
incomplete inspection; unreadable or oversized evidence cannot certify UUID
uniqueness/absence. The
[classification reader](../../crates/brn-store/src/note_metadata.rs) and
[saved metadata derivation](../../crates/brn-workflow/src/library.rs) implement
these rules. `brn_inbox_source` additionally has its own strict provenance reader
and bound Source validation; do not infer that successful indexing validates an
original-copy conversion proof.

## Inbox Source provenance and body

Generated Sources have an LF header in this exact field order: `brn_id`,
`brn_kind: source`, `brn_state: current`, `brn_inbox_source`, then `---` and the
complete converted body. Imported Markdown frontmatter stays inside that body;
it never replaces the new managed header. The JSON object has these fields in
the generated order:

| JSON field | Current encoding |
| --- | --- |
| `item_id` | Nonnil original Inbox item UUID. |
| `kind` | `text`, `markdown`, `email` or `teams`; these are deliberate text copies. |
| `title` | Nonempty title, at most 512 UTF-8 bytes, without control characters. |
| `original_name` | Original label as a string with the same bounds, or `null`; not a machine-specific filesystem path. |
| `received_at_ms` | Nonnegative integer milliseconds, at most `i64::MAX`. |
| `original_byte_len` | Exact original byte count, at most 1 MiB. |
| `original_sha256` | SHA-256 of the original bytes, as a 32-byte integer array. |
| `format` | `verbatim_markdown_v1` for Markdown; `literal_text_v1` for other current kinds. |

Objects reject unknown fields. `verbatim_markdown_v1` preserves all original
Markdown bytes as the body. `literal_text_v1` encloses exact original text in a
`text` code fence: choose backticks or tildes with the shorter longest run in the
input (backticks on a tie), at least three delimiters and one more than that run;
write the opening fence plus `text` and LF, exact text, an LF if the original does
not end in LF, then the closing fence and LF. Added wrapper bytes are not original
bytes. Conversion and complete Source-wrapper size checks refuse overflow without
truncation. Bound Source proposals protect exact generated header/body proof
through review and application. Conversion alone does not establish semantic
completeness, approved knowledge or original-removal authority. See
[Source binding](../../crates/brn-store/src/work/inbox_source.rs) and
[conversion](../../crates/brn-workflow/src/inbox_processing.rs).

## Vault citations

Each `brn_provenance` object contains `note_id`, `sha256`, `start_byte`,
`end_byte`, `quote`, in that generated order, with no unknown fields. `note_id`
is the nonnil stable source UUID; `sha256` hashes the source's full saved UTF-8
bytes, including its frontmatter/BOM/line endings. The byte range is zero-based
and half-open in those full bytes, with valid UTF-8 boundaries and an exact
nonempty `quote`. `end_byte - start_byte` equals the quote's UTF-8 byte length.
Each quote is at most 16 KiB, ranges end at or before 1 MiB, arrays contain at
most 32 citations and exact duplicate objects are refused.

Fresh resolution distinguishes Matched, Changed, Absent, Ambiguous and Incomplete
while retaining saved quotes; paths/hashes do not substitute for UUID identity.
New/changed references require exact saved evidence validation through proposal
approval. Unchanged historical references may remain readable when their source
has changed. Citations prove retained wording/version relationships, not that a
source claim is true. See
[citation representation](../../crates/brn-store/src/note_provenance.rs) and
[workflow inspection](../../crates/brn-workflow/src/knowledge/provenance.rs).

## Links and supersession

Ordinary CommonMark inline/reference links remain ordinary Markdown. Stable
managed note targets use `brn://note/UUID`, for example
`[Source](brn://note/11111111-1111-4111-8111-111111111111)`.
Contained relative Markdown paths also resolve; anchors/query components do not
become identity. Fresh inventory reports missing, changed, ambiguous, incomplete,
external and non-note targets instead of guessing. Explicit relationships derive
from saved links; inferred provenance edges remain derived candidates. Both
rebuild offline in the disposable index, without AI calls or Markdown writes.

Approved Inbox knowledge supersession creates new Current knowledge and changes
its predecessor at the existing path to `brn_state: history`, preserving other
bytes. The new note ends with this exact suffix (two leading LF bytes and one
trailing LF):

```text
\n\nPrevious version: [History](brn://note/PREDECESSOR_UUID)\n
```

The escapes above denote bytes; `PREDECESSOR_UUID` is replaced by the predecessor's
canonical UUID. The footer must parse as an actual Markdown link outside code/HTML;
it is not a separate managed field or an automatic archive-directory move. See
[supersession preparation](../../crates/brn-workflow/src/inbox_actions/knowledge.rs).

## Synthetic byte examples

This complete generated Markdown Source has LF line endings and a final LF after
`Evidence.`. Its original body is exactly ten bytes, `Evidence.\n`:

```markdown
---
brn_id: 11111111-1111-4111-8111-111111111111
brn_kind: source
brn_state: current
brn_inbox_source: {"item_id":"33333333-3333-4333-8333-333333333333","kind":"markdown","title":"Synthetic source","original_name":null,"received_at_ms":0,"original_byte_len":10,"original_sha256":[225,198,27,22,98,223,205,186,130,48,116,208,163,173,8,121,44,2,232,135,53,200,96,178,41,238,28,27,10,249,62,87],"format":"verbatim_markdown_v1"}
---
Evidence.
```

The following knowledge note's citation quotes bytes 429–438 (end exclusive) of
that full Source, excluding its final LF. Its hash covers the entire 439-byte
Source above. These are synthetic file shapes, not an approval receipt or a
claim of semantic validity:

```markdown
---
brn_id: 22222222-2222-4222-8222-222222222222
brn_kind: knowledge
brn_state: current
brn_provenance: [{"note_id":"11111111-1111-4111-8111-111111111111","sha256":[181,137,221,200,178,141,235,238,142,7,78,60,167,68,231,27,238,60,158,80,77,118,190,3,26,10,168,203,213,2,118,185],"start_byte":429,"end_byte":438,"quote":"Evidence."}]
---
Recorded wording: Evidence.

[Source](brn://note/11111111-1111-4111-8111-111111111111)
```

Moving either note does not change its UUID. Editing Source bytes changes its
hash and therefore citation observation; duplicating its UUID makes resolution
ambiguous. Removing `index.sqlite` does not remove the saved fields, links or
quotes. Formatting JSON, changing a timestamp/hash encoding, or altering a Source
header changes authoritative bytes and can invalidate exact retained bindings;
future format evolution must preserve existing readable evidence and compatibility.
