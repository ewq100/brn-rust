# Whole-feature review round1 response

Both findings in the [full independent report](whole-review-round1.txt) are addressed on base `4c45619f80130d54744b917d0f33ae1b4a96c424`. This is a correction candidate for parent-owned fresh round2 review, not self-acceptance.

F1/P1: `Converter::convert_document` now explicitly returns Unavailable on non-macOS after the existing pure limits/cancel/input/source/backend/absolute-path validation, before cloning request bytes, creating a transient directory, or calling process::run. The five-line production addition does not change Mac dispatch, helper selection, protocol, process policy, runtime restrictions or public APIs. The unsupported-platform test no longer points at libtest: on non-Mac Unix it supplies a bounded executable marker, on other unsupported platforms a non-executable fixture. Both backends must return Unavailable, leave scratch absent and never create the dispatch marker. A portable positive test preserves existing validation precedence and no-scratch behavior. The existing three Mac mode/identity/budget tests and all8×2 checked helper expectations remain unchanged.

F2/P2: only `#[cfg(unix)]` was added to `oversized_current_prompt_refuses_before_credentials_or_transport`, matching its transport helper. Its prompt budget, missing-auth path, zero-request and failure assertions remain unchanged. No production refinement or transport code changed.

Actual Mac checks: intake all-targets10 passed/0 failed/1 ignored, including all8×2 actual selected helpers; focused current refiner6 passed/0 failed, including prompt-before-credentials/transport and AI-Off/On controls. Strict intake all-target and scripted AI all-target Clippy, owned Rustfmt and Git whitespace pass. Commands06–08, exact inputs/diffs, original binaries/libraries/dep-info and results are retained under `/private/tmp/brn-x2-retirement-distribution-20261018/evidence`. Their actual source is HEAD4c45619f plus captured WIP hashes, never a future commit.

These were source-derived Linux/Windows findings. No unsafe recursive Linux before-run, Linux behavioral red, Windows compilation or cross-platform execution is claimed. Required Ubuntu/Windows CI qualifications remain pending; no target/toolchain/dependency download or platform emulation was used. The fullAI496/0/2 and historical workspace results retain their original source qualification; neither broad suite was rerun for this platform-only correction.

The successful pre-correction builds, notices and signed package remain immutable under the same external root, commands01–05, source70f2440f. They are not final post-correction distribution proof. Native/CLI artifacts and a fresh final package will be built at the actual correction pin before the remaining relocation/native gates. Runtime, root/tool manifests and locks, accepted patches, core/schema/Auth/UI/skills/INT-D authority and all other production source remain unchanged. No advisory was present in round1. Native-human/owner/hosted CI/integration/publication and carried qualifications remain open.

Exact corrected source hashes:

- `crates/brn-threads-intake/src/documents.rs`: `6c79b83ff92928110c351db5f15f5549e357f6f947f15572f18c0a72a5f4f00c`
- `crates/brn-threads-intake/tests/document_modes.rs`: `0917d52ea42454fb0e93a92f3a01f3e35ab69557f5b6ef90e79602a0106aa83e`
- `crates/brn-ai/src/document_refine_tests.rs`: `48ea6c1c3694200cbe2e009aa2369a3765ee76b7a16361f37e5c8ffaed7df9aa`
