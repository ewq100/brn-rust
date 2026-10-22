# TLS lockfile patch validation route

**Source identity:** product source `3827d6a13953533c7322fd7157d975a32dee5cb2`; lead checkout HEAD observed `8d3699ecc38ded1f87f3d584a3e7bf2447566a1b`, docs/evidence-only concurrent changes. No source, lockfile or test was changed and no command below was run.

For a root `Cargo.lock`-only TLS resolution change, the shortest meaningful existing offline sequence is:

```sh
cargo test -p brn-ai --locked --offline
cargo test -p brn-ai --features capability-spike --all-targets --locked --offline
cargo build -p brn --locked --offline
cargo build -p brn-desktop --features native-ui --locked --offline
```

`brn-ai` README already identifies the ordinary library suite and the optional `capability-spike` suite/Clippy. The second command is also the existing CI provider-fixture lane: bounded mock/scripted capabilities, no account or live provider. `threads_mcp_http_tests::actual_sdk_http_discovery_rig_adaptation_and_dispatch_revocation` is a useful existing integration witness for the real RMCP/reqwest client path; adjacent tests cover oversize/invalid HTTP behavior and cancellation. Its server binds loopback and the endpoint is `http://…`, so it does **not** exercise TLS certificate parsing or trust-anchor validation. `provider_formats_tests` uses synthetic transports; it is not a live TLS test either. No existing test establishes direct reproduction/removal of the emSign certificate-path issue. A lock-only patch does not justify adding a bespoke test for upstream certificate behavior.

The CLI build confirms the `brn` consumer resolves the changed workspace lock. The production `native-ui` desktop build is the exact ordinary consumer command used by `.github/workflows/ci.yml`; it verifies the real desktop feature graph without `native-test-support`. `scripts/verify-desktop-shell.sh` adds writing-UI/native test-support build, lint and tests, but these do not improve TLS validation over the production desktop compile for a lock-only change. The broader `scripts/verify-storage.sh` builds/tests the full workspace and then performs headless startup checks, but repeats much more than needed for this dependency-only scope. The same CI matrix also includes the `native-retrieval` lane; that optional feature adds `brn-retrieval`'s own reqwest/rustls route and should be included only if the reviewed affected graph/lock resolution shows the patch changes that feature's packages.

The two Xberg helper lockfiles are standalone and, in the inspected lock package records, have no `rustls`, `rustls-webpki`, `webpki-roots`, or `webpki-root-certs` entries. No helper build/test is specifically needed for a root-lock-only TLS patch; unrelated helper tests remain part of their normal independent gates.

These commands validate test/build compatibility and the affected consumer graphs, not the advisory's exploit condition, runtime TLS use, OS certificate verifier, endpoint behavior, network exposure, GUI interaction, or owner acceptance. The current helper/CI gate intentionally makes no live provider calls. Re-run the selected advisory scan against a current recorded RustSec database separately; a successful Rust build is not advisory clearance.
