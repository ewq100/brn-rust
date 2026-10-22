# BRN dependency-advisory preflight

**Source:** `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, HEAD `b7cdd4fde5c39962bd0b62b2ba0958ceb17ec0cf`, product source `3827d6a13953533c7322fd7157d975a32dee5cb2`. Root `Cargo.lock` is 943 packages; the stock and adapted document-helper locks are each 409 packages. Scoped source is unchanged against the selected product source. No build, dependency update, tool installation, audit command, CI change, runtime/provider action or source edit was made.

## Finding

The root lock has current RustSec advisory/version matches in TLS certificate verification:

- `rustls-webpki 0.103.15` (`Cargo.lock:7011`) matches RUSTSEC-2026-0382; the official advisory's patched line begins at `0.103.16`.
- `webpki-roots 1.0.9` (`Cargo.lock:8995`) matches RUSTSEC-2026-0381; the patched line begins at `1.0.10`.
- `webpki-root-certs 1.0.9` (`Cargo.lock:8977`) matches RUSTSEC-2026-0383; the patched line begins at `1.0.10`. The lock also includes `webpki-roots 0.26.11`; RustSec marks that compatibility wrapper itself unaffected, while documenting that its separately resolved 1.x parent must be `1.0.10+`.

The root lock has `rustls 0.23.45` (`:6937`), which is already at RUSTSEC-2026-0285's patched threshold; this does not clear the separate 2026-0382 finding for `rustls-webpki 0.103.15`. The three 2026-0381/0382/0383 records concern the distrusted emSign intermediates/root material, so they overlap in topic and should not be reported as three independent underlying flaws. The record text says affected paths may accept certificate chains through those intermediates; this preflight did not test any chain or endpoint.

This is more than an unqualified name/version coincidence: the root `brn-ai` manifest enables `rig`'s reqwest/RMCP transports and `rmcp`'s streamable-HTTP reqwest client, with Rustls features (`crates/brn-ai/Cargo.toml:22–26`). Lock dependency paths include `brn-ai → rmcp → reqwest 0.13.5 → tokio-rustls → rustls → rustls-webpki`, and paths through `hyper-rustls → webpki-roots 1.0.9` and `rustls-platform-verifier → webpki-root-certs 1.0.9`. The shipped CLI/desktop depend on `brn-ai`; `threads_mcp_client.rs` and `conversion.rs` contain the network-client consumers. This supports compile-graph reachability in the provider-capable root build. Actual TLS execution, enabled runtime path, platform verifier selection, endpoint trust chain and exploitable exposure were **not** observed or proven here.

The two standalone helper locks contain none of `rustls`, `rustls-webpki`, `webpki-roots`, or `webpki-root-certs` in the inspected package records. This is a bounded comparison of those named advisories, not a full advisory clearance for either helper lock or the root lock.

## Tooling and evidence

`cargo-audit` and `cargo-deny` are absent from normal `PATH` and `~/.cargo/bin`. No repository `deny.toml`/`audit.toml` or advisory-check step appeared in the targeted CI/scripts/development-policy scan; current CI performs locked/offline builds/tests and helper checks, not a RustSec gate. A local `$HOME/.cargo/advisory-db` clone exists and is clean at commit `7eebec69c352c7191b1f13eb95dd510eeca5d1de`, dated `2026-10-09T10:12:02+02:00`. It predates RUSTSEC-2026-0381 (2026-10-19) and RUSTSEC-2026-0382 (2026-10-20), so it is stale for this question. No fetch/update was performed.

Primary-source checks: [RustSec advisory DB README](https://github.com/RustSec/advisory-db) identifies `cargo-audit` as a Cargo.lock consumer; the [cargo-audit README](https://github.com/rustsec/rustsec/blob/main/cargo-audit/README.md) describes `--file` for auditing a selected lock without generating one and says to run from outside the project to avoid project Cargo configuration. The current records are [RUSTSEC-2026-0381](https://github.com/RustSec/advisory-db/blob/main/crates/webpki-roots/RUSTSEC-2026-0381.md), [RUSTSEC-2026-0382](https://github.com/RustSec/advisory-db/blob/main/crates/rustls-webpki/RUSTSEC-2026-0382.md), [RUSTSEC-2026-0383](https://github.com/RustSec/advisory-db/blob/main/crates/webpki-root-certs/RUSTSEC-2026-0383.md), and [RUSTSEC-2026-0285](https://github.com/RustSec/advisory-db/blob/main/crates/rustls/RUSTSEC-2026-0285.md). The official current advisories were inspected online; no local audit tool/database was used to reach these matches.

## Next proportionate check

After a separate explicit tool-use decision, perform one lockfile-only `cargo-audit` pass with a current recorded RustSec database over the root lock and both helper locks. Keep the binary/database task-local, run with `--file` from outside the checkout, capture the exact database commit and findings, and do not use `cargo audit fix`. Then triage only confirmed root matches and any helper findings before selecting a dependency-resolution task. This report does not recommend a broad `cargo-deny` policy or claim full reachability, binary, license, or supply-chain coverage.
