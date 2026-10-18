# Threads transient document intake

`Converter::convert_document` runs the host-selected stock or BRN Xberg helper
through the shared bounded process lifecycle. The host snapshots converter/AI
settings, chooses an absolute executable from `DocumentHelpers`, validates exact
backend/bundle/source hash/output/assets and never falls back. The two trusted
optional overrides are `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN`; otherwise
both are siblings of the host executable. All eight kinds use this path.

The converter returns derived Markdown, retained assets, source hash/reference,
intent and honest gaps. Originals stay external. Useful-information selection
and protected-note creation belong to the checked host; conversion grants no
write authority. Stock output is normal pinned Xberg output, not a promise of
complete fidelity. Optional AI refinement follows extraction in brn-ai; Off
requires no conversion credentials. Historical reports/provenance remain data.

The host clears environment, bounds pipes/scratch/output and wall time, monitors
cancellation and kills/reaps the owned process group before joining pipes. The
helper installs its own restriction: deny-default sandboxing on macOS and a
fail-closed seccomp-BPF pipe-only policy on Linux x86_64. Other operating systems
and Linux architectures return unavailable. No Poppler or host renderer exists;
no hard heap/RSS ceiling is claimed.

Use an absolute private temporary root outside Git and knowledge data. Run
directories are private and removed on success/failure/cancellation. Creation
cleans only owned ordinary directories older than 24 hours, without following
symlinks. Bounded input reads and cleanup have current `intake_safety` witnesses;
process cancellation/tree and transient boundary unit tests remain active.

Build both helpers first, export their declared absolute paths, then run
`cargo test --locked --offline -p brn-threads-intake`. `document_modes` exercises
all eight kinds under both engines, identity/limit/cancel failures and no fallback.
Retired prepared-reader inventories and failed PDF results remain historical;
fixture inputs reused by current tests are retained.
