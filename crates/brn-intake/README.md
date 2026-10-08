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
DOCX members and 16 MiB actual expanded package bytes; 512 aggregate MIME parts,
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
requiring original inspection. No PPTX conversion is included.

Pinned mail-parser 0.11.8 decodes headers and actual text/HTML alternatives. HTML
is preserved as inert quoted source, and html5ever tokenization identifies CID
image references. CID targets resolve only in their nearest actual MIME related
container; missing/ambiguous targets never borrow a sibling target. Unsupported
attachments remain exact unprocessed children. PNG uses maintained complete
CRC/Adler/pixel/terminal-chunk decoding; animated PNG is unprocessed. JPEG uses the
maintained image decoder. Other formats are retained with an explicit gap. The
legacy-record PNG entry retains its 1 MiB/4,096-dimension/4,194,304-pixel budget.

`cargo test --locked -p brn-intake --all-targets --features helper` covers retained
public P1 single/plural mail, repeated exact images, unsupported XLSX, scope and
ambiguity, malformed attachments, four hostile packages, prefix/visual omissions,
aggregate member budgets, damaged/truncated/trailing PNG, real native restricted
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
