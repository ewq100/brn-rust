# Light document boundary

This library defines the bounded `DocumentRequest` / `DocumentExtraction` envelope,
finite intake limits, expected engine identities, content-addressed asset names,
base64 image codec, complete PNG validation, bounded JPEG raster decoding and
rendered retained-image checks.
It links no document reader. The standalone [document helpers](../../tools/xberg-intake-helper/Cargo.toml)
use pinned stock Xberg or the separately generated four-patch BRN bundle.

Build both with `bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root /absolute/task/targets`.
The builder consumes the cached, SHA-verified archive; normal CI fetches committed
stock/adapted locks before offline builds. Install both `brn-xberg-helper` and
`brn-xberg-helper-brn` beside the host. There is no third converter or fallback.

`src/helper_runtime.rs` is shared binary source. It owns empty-environment
checking, protocol bounds, process self-tests and resource limits. On macOS it installs
a deny-default sandbox; on Linux x86_64 it installs a fail-closed seccomp-BPF policy
for pipe-only helper work. Other operating systems and Linux architectures refuse
conversion. The new binary removes only macOS's startup-inserted CoreFoundation
marker before restriction setup. This is not a hard heap/RSS guarantee or full
document-fidelity claim.

`cargo test --locked --offline -p brn-intake` checks light DTO, retained-image and
PNG/hash/codec safety. The standalone tools' own tests qualify normal API output,
all eight representative kinds, identities, restrictions and process-tree behavior.
Historical typed-reader protocols/tests remain in Git; their results are not
relabelled by retirement. Reused public/synthetic fixture bytes remain here.

## Retained figures

Both stock and BRN helpers explicitly enable the SDK retained-image option;
stock refers to the unpatched SDK, not an untouched SDK configuration. All other
non-security/service options keep their published defaults. The helpers request
native image bytes without OCR, classification or
remote fetching. Projection retains static PNG and JPEG only. GIF, WebP, SVG,
missing bytes and unresolved image links remain explicit coverage gaps. Reference
images without titles are made inline at the image occurrence; shared definitions
and ordinary links are unchanged. Titled or noncanonical image links remain gaps.

PNG uses the existing complete checksum/chunk decoder; JPEG uses pinned
`zune-jpeg` 0.5.15 in strict mode (already in the Xberg graph). Header dimensions
are checked before raster allocation, with a finite scan limit and decoder output
budget. JPEG decoding is **not an entropy-integrity certificate**: the maintained
decoder can recover incomplete scans when a terminal marker is present. A regression
witness records this limitation. It does not change exact retained bytes/hashes or
make recovered pixels proof of source fidelity. A probe of maintenance-only
`jpeg-decoder` 0.3.2 had the same behavior and was not adopted.

Source-document SHA-256 and retained-image SHA-256 remain hashes of their exact
respective bytes; no new transcoding occurs. Raster dimensions come from decoded
bytes, since Office drawing dimensions are presentation extents. Encoded image
bytes (including duplicate SDK occurrences) are bounded before projection copies;
unique assets, pixels, text, gaps and serialized output retain existing quotas.
SDK archive/input limits precede extraction; its per-page image cap is PDF-specific.
These are not a universal preallocation cap inside the SDK. Existing helper CPU,
wall-time, pipe and process limits remain in force, including Linux's 1 GiB virtual
address-space limit; no aggregate RSS or macOS hard-memory guarantee is claimed.

The checked helper identity ends in `/figures-v1` for this service profile. A
current host rejects older helper responses. Historical saved imports keep their
original provenance unchanged; requesting a fresh import under this profile
creates a distinct protected conversion variant for the same source rather than
silently reusing the earlier figureless Note. Current-profile replay still deduplicates.
