# Maintained intake boundary

The default library contains the schema-1 extraction protocol, per-intake quotas, consumed-scope evidence, bounded validation,
exact byte hashing, content-addressed asset filenames and retained-record PNG
validation. It does not link BetterOffice, MIME, HTML or JPEG parsers. Source and
asset byte fields serialize as base64; the request's byte field is a JSON byte array.
Optional request limits tighten the qualified hard profile. Every real helper
extraction retains the exact selected limits and measured input/expanded/decoded
bytes, package/MIME counts and depth, unique image pixels and canonical output
bytes. Unknown synthetic/historical usage is explicitly absent, never guessed.
When preserving a new Source, derive its Markdown with
`Extraction::materialize_for_source(note_id)` and create files using
`asset_file_name_for_source(asset, note_id)`. Nonnil UUID namespaces give each
Source distinct destinations while retaining image content identity and repeated
occurrences. Only qualified image destinations change; source text and occurrence
offsets follow those exact edits. The measured snapshot stays immutable and the
derived clone clears consumed usage. Citations use `original_source_range` to map
unchanged quotes back to snapshot offsets; ambiguous correspondences or quotes
crossing changed destinations fail closed. Always derive from the original snapshot.
The host enforces selected wall time (default 5 seconds, hard maximum 30 seconds).
MIME library decoding occurs under the hard input/process profile before its
returned tree can be checked against lower part/depth/decoded quotas; there is no
second MIME preflight parser.

Build the separate helper with `cargo build --locked -p brn-intake --features helper
--bin brn-intake-helper`. Shipping/install tooling must include this executable and
start it with an empty environment, bounded pipes, a new process group and an owned
wall-time/cancellation lifecycle. No alternate converter is available. Helper stdin
accepts one `HelperRequest`; stdout returns one `Extraction`, or `{"error":"..."}`
with a failing exit status. Call `Extraction::validate` before trusting its output.

On macOS the helper initializes its native deny-default sandbox after trusted
runtime startup and before stdin access. It grants only bounded system-runtime
reads, `/dev/null`, sysctl reads and process creation for the process-tree witness;
network, vault/home/credential reads and general writes remain denied. CPU is
limited to 10 seconds, file descriptors to 64 and regular output files to 64 MiB.
The host still owns wall time, inherited descriptors, process groups and shutdown.
Other platforms fail closed until a native restriction profile is qualified.
Signed/packaged distribution and supported macOS-version qualification remain
separate from these local tests; there is no claimed hard RSS ceiling.

Current job budgets are 16 MiB input; 64 MiB serialized output; 8 MiB Markdown and
8 MiB aggregate source text; 32 MiB aggregate retained payloads; 512 aggregate
Office members and 16 MiB actual expanded package bytes; 512 aggregate MIME parts,
32 MIME-container levels and 16 nested messages; 32 deduplicated image assets,
2,048 distinct occurrences and source nodes, 16 MiB aggregate image bytes, 32 Mi
pixels per image and 64 Mi pixels across unique assets. XML parsing uses upstream
16 MiB/200,000-event/128-depth limits. Overflow is a visible refusal or retained
unprocessed attachment, never silent truncation. Upstream truncation is disclosed.

DOCX semantics, relationships, selected stories and Markdown come from pinned
BetterOffice 0.3.0. The adapter walks only typed exported images, gives each a
unique rendering placeholder, and joins the exact upstream image identity/anchor
and resolved package part to that placeholder. Repeated images remain distinct
occurrences. Missing/ambiguous associations produce gaps; there is no XML mapper,
picture-order matching, prefix repair, title scanner or automatic fallback.
Picture titles remain explicitly unavailable; supplied upstream alt text survives.
Charts, SmartArt, shape/layout fidelity and unknown containers remain visible gaps
requiring original inspection. PPTX uses the same helper through pinned BetterOffice PPTX 0.3.0. The bounded
partial profile projects presentation-order authored text, nested groups, ordinary
table cells, presenter-note body and complete embedded PNG/JPEG pictures. Slide
and notes nodes retain exact package-part bytes; shape/cell descendants identify
part/shape/element-path or row/cell locators. Hidden content is labelled. Source
inventories must match typed model traversal/kind/ID, and image relationships must
resolve uniquely to the same exact internal image part; ambiguous joins refuse.
Consumed singleton relationship joins (root presentation, per-slide notes/layout
and the selected layout's master) must also be unique under the maintained
parser's actual selectors. Distinct relationship IDs do not justify selecting an
arbitrary first notes page. Valid slide/master lists and image collections remain
supported. Titles/descriptions come from the matching maintained source inventory, never a
free title scan. Repeated pictures remain distinct occurrences of deduplicated
asset bytes. Native charts/SmartArt, inherited master/layout content, unsupported
media/drawings/inlines and missing/external images retain located gaps. No chart
facts or visual/layout/crop/rotation equivalence are claimed, and originals remain
retained. The PPTX parser receives explicit aggregate XML/text/event/depth,
attribute/relationship/shape/paragraph/run/comment bounds in addition to shared
OPC admission and the process deadline; no large upstream defaults are used. Native retained inspection supports both qualified PNG/JPEG
assets. Existing model transport remains PNG-only: only explicitly selected PNG
assets are consumed visually, and omitted visual scope remains visible. JPEG
retention/inspection does not imply JPEG model consumption.

Pinned mail-parser 0.11.8 decodes headers and actual text/HTML alternatives. HTML
is preserved as inert quoted source, and html5ever tokenization identifies CID
image references. CID targets resolve only in their nearest actual MIME related
container; missing/ambiguous targets never borrow a sibling target. Decoded headers
are source claims: sender authenticity and thread relationships are not independently
verified, including when Authentication-Results reports success. Present identifiers
remain decoded values; In-Reply-To/References retain every parsed identifier across
all physical matching fields and within each scalar/list field, in source order
including repetitions. Unavailable fields stay unknown. Plain-text-only email does
not acquire HTML or remote-resource diagnostics. Actual HTML stays inert, and actual
remote/non-CID or missing/ambiguous CID references retain their specific gaps. Root
partial status is unchanged and does not imply an identifier is missing. Retained
schema-1 snapshots keep their old caveats and bytes; reading never relabels them.
Supported DOCX/PPTX attachment conversion is isolated, then atomically merged
into the email envelope. The local root maps onto the exact retained attachment;
every child parent/source/occurrence ID and global Markdown interval is remapped.
After successful typed parsing, the attachment node carries the validated package
MIME type even when the email declared application/octet-stream; exact original
EML bytes preserve that declaration.
Two identical attachments keep separate descendant and quote identities. Decoded
single-part attachments are also children, preserving raw email/headers separately.
A conversion or aggregate merge refusal retains the exact attachment unprocessed
without leaking partial child nodes/assets/ranges. Unsupported attachments remain
exact unprocessed children. PNG uses maintained complete
CRC/Adler/pixel/terminal-chunk decoding; animated PNG is unprocessed. JPEG uses the
maintained image decoder. Other formats are retained with an explicit gap. The
legacy-record PNG entry retains its 1 MiB/4,096-dimension/4,194,304-pixel budget.

`cargo test --locked -p brn-intake --all-targets --features helper` covers retained
public P1 single/plural mail, repeated exact images, unsupported XLSX, scope and
ambiguity, malformed attachments, four hostile packages, prefix/visual omissions,
aggregate member budgets, Harbor and unrelated producer PPTX (order, groups,
soft breaks/field display, cells, notes, hidden content, exact PNG/JPEG and located
gaps), identical attachment attribution/materialized quotations, atomic quota
refusal, duplicate shape/relationship identity, damaged/truncated/trailing PNG,
real native restricted
parsing, existing synthetic forbidden-file/live-loopback denial and cancellation
of a real helper/child/grandchild group. `cargo clippy --locked -p brn-intake
--all-targets --features helper -- -D warnings` checks the optional parser graph.
These checks establish neither native review usability nor Office corpus fidelity.

Source materialization gives each Source note UUID an independent asset filename
namespace, so shared images and repeated preservation of one snapshot do not
collide. It edits only qualified image destinations, preserves asset/original
bytes and occurrence identity, and maps quotations back to immutable node ranges.
Measured helper usage stays on the original snapshot; the derived representation
has no new measured usage. Ambiguous duplicate full-node text remains visible but
cannot be cited with guessed attachment ownership.
