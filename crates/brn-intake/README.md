# Light document boundary

This library defines the bounded `DocumentRequest` / `DocumentExtraction` envelope,
finite intake limits, expected engine identities, content-addressed asset names,
base64 image codec, complete PNG validation and rendered retained-image checks.
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
