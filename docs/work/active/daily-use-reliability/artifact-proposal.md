# Constrained calculation, chart and report proposal

Design only, observed 2026-10-23 against main
`f30f37e0dc50fdcf91e97a77bc7cda9acb678d78`. No production worker, new
dependency, tool registration or activation is implemented by this document.

## Focused decision

Recommend approving implementation of **one fixed-operation local worker** that
calculates over selected typed data, returns validated PNG and numerical results,
and proposes a report Note through existing review/Apply/Undo. It accepts no
executable code, shell, database access, network destinations or arbitrary paths.
Approval of this design would not establish qualification, authorize merge/release,
or permit activation on an unqualified platform. Until that separate decision,
implementation and activation remain deferred; intake and workflow work continue.

The agent selects relevant evidence, operations, chart and prose. The worker owns
exact arithmetic and drawing. The host owns scope, provenance, validation and
immutable candidate construction. Existing core/services own acceptance and Undo.

## Evidence and alternatives

The [M5 receipt](../vacation-campaign/evidence/artifact-capability-proof.json) and
archived [proof README](../vacation-campaign/evidence/artifact-capability/README.md.gz)
show Python 3.12.14/ReportLab 4.4.9 producing expected 3.00/2.50/3.00 values,
reproducible SVG and Markdown. PNG was not attempted. Its separate sleeping-child
termination witness does not qualify actual chart cancellation, hostile input,
filesystem/network restrictions, memory limits or production runtime packaging.

| Option | Assessment |
| --- | --- |
| Existing agent and ordinary Notes | Already suitable for interpretation and reports; no reliable exact calculator or chart asset creation. Keep for prose. |
| Skills/examples alone | Can improve use of existing tools but cannot grant execution or create missing numerical/raster processing. |
| Promote Python/ReportLab proof | Reuses arithmetic/chart primitives, but requires owned interpreter distribution, raster backend and dependency qualification. The Codex-installed proof runtime is not a BRN shipping runtime. |
| Small Rust helper with maintained libraries | Recommended: matches existing sibling-helper packaging and pipe-only restrictions, avoids an interpreter and system-font reads. Requires two new dependency families and real qualification. |
| Custom arithmetic/chart engine | Unnecessary maintenance and correctness burden; do not implement an expression language or plotting engine. |

Use `rust_decimal` checked arithmetic with explicit rounding and Plotters 0.3.7
with default features disabled, `bitmap_backend`, `line_series` and `ab_glyph`.
Render to an RGB buffer, then encode using the existing `png = 0.18.1`. Plotters
documents [in-memory bitmap output](https://docs.rs/plotters/0.3.7/plotters/prelude/struct.BitMapBackend.html)
and [font registration from embedded bytes](https://docs.rs/plotters/0.3.7/plotters/style/fn.register_font.html).
The [Decimal API](https://docs.rs/rust_decimal/latest/rust_decimal/struct.Decimal.html)
provides checked operations and selectable rounding. These are API research,
not a compiled or security-qualified dependency selection.

Current root Cargo.lock already includes png 0.18.1, image 0.25.10, tiny-skia
0.11.4 and resvg, but not Plotters or rust_decimal. Before implementation locks
are accepted, select an exact compatible rust_decimal version, pin both new
dependencies, inspect licenses/transitive features and compile the minimal APIs.
Embed a licensed font with its license/hash; never load system or caller fonts.
No runtime download, PATH interpreter or fallback backend is allowed.

## Version-one contract

All limits below are proposed qualification targets, not measured guarantees.
The complete request must satisfy every limit; row/column maxima do not promise
that every maximal combination fits the byte budget.

| Boundary | Contract |
| --- | --- |
| Executable | One named sibling `brn-artifact-helper`, host-selected absolute path and verified protocol/build identity. One request per process; no caller argv. |
| Input | Versioned JSON, unknown fields rejected; 256 KiB wire cap, at most 1,000 rows × 16 columns, 64 ordered calculation steps and 160 UTF-8 bytes per label. No expression strings, recursive evaluation, file paths or URLs. |
| Numbers | Decimal strings ≤64 bytes, input scale ≤12. Checked add/subtract/multiply/divide, sum/mean/min/max, row ratio and percentage change. Operands name supplied cells or prior results only. Reject overflow, zero denominators and missing cells. Require explicit output scale 0–12 and one enumerated rounding mode; report both. |
| Units | Preserve explicit unit labels; no implicit conversion. Add/subtract/aggregate require matching units; ratio and percentage change require a declared compatible pair. Unsupported compound-unit reasoning is refused. |
| Chart | Optional single bar or line chart; ≤4 series and ≤100 displayed points total. No silent sampling, logarithmic axes, caller styling scripts or external images. Exact numerical table accompanies the plot; float conversion is drawing-only and cannot change reported arithmetic. |
| Raster | Default 960×540, maximum 1,280×720. One nonanimated RGB PNG ≤1 MiB; no SVG/HTML. Host fully decodes it and verifies dimensions, byte cap and digest independently of worker declarations. |
| Output | Versioned numerical results, rounding/unit metadata and optional PNG; complete JSON/base64 response ≤2 MiB. Host/model report prose ≤64 KiB. No output filenames or direct publication. |
| Resources | One active worker per Run; 5-second wall deadline, 2-second CPU limit target, 256 MiB address-space target where enforceable, finite descriptors, no core dumps. A platform remains unavailable if its required containment cannot be qualified. |
| Cancellation | Host checks cancellation/fences before spawn, during work and before preparation; kills the whole process group, joins pipes and reaps on cancellation, timeout, overflow or failure. Late output cannot become a candidate. |

Every calculation step declares its own scale/mode, stores the rounded decimal
result and passes that stored result to later steps. There is no hidden
higher-precision value used by downstream steps. Nonterminating division is
allowed only when the implementation can establish the correctly rounded result
at that step's declared scale. `checked_div` alone does not establish this:
library-internal rounding can affect a later tie. Qualification must compare
division and chained results against independently specified exact-rational
expectations, including near ties; reject unsupported arithmetic rather than
silently accepting an unqualified approximation. For example, scale-2 nearest-even
`1 / 3` stores `0.33`, and subsequent scale-2 multiplication by `3` yields `0.99`;
scale-2 nearest-even `1.005` yields `1.00`, while `1.015` yields `1.02`.

Ordinary unsupported characters or labels that cannot fit the fixed chart must
produce a visible refusal or table-only result with an explicit warning; never
silently drop a label or use unreadable replacement glyphs. Embedded-font Unicode
coverage and chart readability need fixtures and visual review.

## Reuse, containment and packaging

[Intake process ownership](../../../../crates/brn-threads-intake/src/process.rs)
already supplies absolute executable selection, empty environment, bounded pipes,
process groups, deadlines and kill/reap. Its API is private and coupled to intake
errors/scratch. Extract only the small process mechanism shared by these two real
consumers after activation approval; retain existing intake behavior/tests. Do not
build a configurable command runner or expose executable selection to model input.

The existing [helper runtime](../../../../crates/brn-intake/src/helper_runtime.rs)
and [Linux restrictions](../../../../crates/brn-intake/src/helper_runtime_linux.rs)
are source precedents, not proof for the new dependency graph. Restrict before
reading untrusted input. Close ambient descriptors, inherit no credentials and
deny filesystem/network access. Computation and raster output stay in memory;
the host may supply an empty private working directory, without granting worker
write access. No canonical SQLite handle/path crosses the boundary.

Linux x86_64 currently has fail-closed seccomp and a 1 GiB RLIMIT_AS for intake.
macOS has deny-default sandboxing and CPU/descriptor/file-size limits, but no
qualified hard heap/RSS cap. RLIMIT_AS is not an RSS guarantee. The proposed
smaller worker budget requires actual platform measurements and enforcement
tests; a watchdog alone is not hard memory containment. Do not weaken intake
restrictions to make a chart library work. Unsupported OS/architecture or failed
restriction setup means unavailable, with no interpreter fallback.

The current Mac profile permits bounded system-runtime reads, `/dev/null` access
and `process-fork`; it is not literally a prohibition on all filesystem access or
child creation. Explicit ambient-descriptor closure is a Linux precedent here.
The new worker must separately qualify descriptor isolation and process denial
on Mac, document necessary runtime exceptions, and retain process-group cleanup.
Source reuse does not establish those stronger requirements.

Build/package/sign the named helper beside desktop and CLI through existing
packaging ownership. Qualify installed discovery with a cleared development
environment. Include dependency/font licenses and exact build identity. No
dynamic plugins, shell, SQLite dependency or runtime package installation belongs
in the worker. A new library version requires repeated relevant qualification.

## Checked candidate integration

[Transient source assets](../../../../crates/brn-ai/src/threads_sources.rs) demonstrate
host-held bytes and opaque links. Reuse that pattern with a narrowly typed generated
result owner, not a fabricated external Source or conversion receipt. The host
binds recipe version, selected exact source/revision identities, original input
digest, helper identity, units, rounding and output digest. Preserve privacy and
historical/current-read distinctions; a model cannot retag derived values as an
observed external fact. Do not retain unrelated raw documents or worker logs.

The first implementation should expose one fixed tool returning typed calculation
results and an opaque chart handle. Existing `prepare_changes` can then select
that handle into a report candidate. Host materialization supplies ordinary Note
and Asset writes together, with source/current-version observations and stable
operation identity. General maintenance must not silently publish this new output:
version one always presents the exact report/chart for owner review.

[Review](../../../../crates/brn-threads-app/src/review.rs) already owns exact apply,
replacement identities and attention cleanup. [Core](../../../../crates/brn-threads-core/src/store.rs)
owns atomic preparation/application, replay, dirty/version/protection guards and
Undo. Use these unchanged authorities; no second artifact store, write bypass or
new acceptance screen. A failed worker creates no accepted artifact. An already
prepared immutable candidate survives reopen through existing candidate storage;
replay resolves the same receipt rather than generating a second chart. Later
human edits must block stale Apply/Undo as they do for other Notes.

Format contracts differ by boundary: generic core assets require nonempty MIME
and bytes; [application export](../../../../crates/brn-threads-app/src/export.rs)
recognizes PNG/JPEG/WebP; transient agent source validation recognizes PNG/JPEG/GIF;
the document helper currently retains validated PNG. Choosing PNG satisfies the
relevant route without claiming that these allowlists are identical or expanding
the asset contract.

## Required evidence before activation

1. Independent arithmetic expectations including rounding ties, chained rounded
   results, nonterminating division and near-tie exact-rational cases, overflow, zero
   divisors, missing values, incompatible units, oversized/unknown fields and
   invalid references. Reuse the M5 3.00/2.50/3.00 case without treating it as the
   whole calculator qualification.
2. Real worker calculation/raster cancellation, timeout, output overflow and crash
   tests. Confirm reaping, joined pipes, no surviving descendants and no prepared
   or committed result after cancellation. The old sleep-child proof is insufficient.
3. Filesystem, network, inherited-descriptor, process-creation and resource-limit
   witnesses on each enabled platform, with deliberate negative controls.
4. Full PNG decode, independent dimensions/hash, numerical table agreement, long
   labels/Unicode, bounded font work and readable bar/line output. Reproducibility
   claims name the exact binary/font/platform; cross-platform byte identity is
   not assumed.
5. Extend existing daily workflow/scripted journey tests for preview → Apply →
   reopen → Undo, duplicate replay, stale source, protection, dirty typing,
   concurrent edits, interruption and recovery. Distinguish deterministic models
   from a later small separately authorized live usefulness sample.
6. Installed sibling packaging checks without development PATH assumptions,
   independent source review and applicable headless integration checks.
7. Later actual Mac rendering, review focus/accessibility and owner acceptance.
   No Mac access is authorized for this cloud batch; CLI success cannot clear it.

Current outcome: implementation-ready boundary recommendation with dependency,
memory enforcement, rendering and platform qualification still open. No new
worker test, live model result, native observation or owner acceptance is claimed.
