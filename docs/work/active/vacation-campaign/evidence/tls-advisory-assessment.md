# Independent TLS advisory assessment — 2026-10-22

**Disposition: select a bounded root-lock-only repair to rustls-webpki 0.103.16, webpki-roots 1.0.10 and webpki-root-certs 1.0.10.** No requirement for a full audit-tool installation before making that selection. This is an assessment of the selected advisories and source paths, not exploit reproduction, candidate approval, a complete dependency clearance or native/runtime qualification.

## Identity and evidence

Inspected lead checkout `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`. It was clean at `8d3699ecc38ded1f87f3d584a3e7bf2447566a1b` initially. During read-only assessment the lead advanced to `b01423439a4d79ecefa5e08eb38c4be113e6702a`, with campaign/overview edits and TLS evidence untracked. These are lead-owned; none were changed by this reviewer. All committed differences from product `3827d6a13953533c7322fd7157d975a32dee5cb2` remain under `docs/`. The inspected root lock SHA256 is `fc1ca2cf08b51f357dccd277b2fdb5491454c7bb87bf96e1026322d6b5f85d89` (943 packages).

Independently parsed the lock and cached published manifests/source, read the official RustSec advisories and upstream patch pages. Verified the three saved record hashes against `/tmp/brn-tls-advisories/source-receipt.json`, which binds RustSec DB commit `59ef7abf8ce56ea80fe7eba97ba3835449009fa4`. Release metadata in the lead's `release-metadata.json` records published, non-yanked target versions with MSRV 1.71 and registry checksums; that fetched receipt is supporting lead evidence, not a reviewer dependency download. The repository uses Rust 1.98.1.

| Locked package | Selected patch | Official evidence |
|---|---|---|
| rustls-webpki 0.103.15 | 0.103.16 | [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382.html) |
| webpki-roots 1.0.9 | 1.0.10 | [RUSTSEC-2026-0381](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) |
| webpki-root-certs 1.0.9 | 1.0.10 | [RUSTSEC-2026-0383](https://rustsec.org/advisories/RUSTSEC-2026-0383.html) |

All three records alias **CVE-2026-111943**: report one overlapping distrust incident with three affected packages, not three independent underlying flaws. Upstream's [WebPKI backport](https://github.com/rustls/webpki/pull/546) rejects selected distrusted public keys throughout certificate paths. The [root-store patch](https://github.com/rustls/webpki-roots/pull/133) removes emSign ECC Root CA - G3 from both representations. Updating only roots is weaker than adopting the matching path-verification repair. Existing rustls 0.23.45 does not itself resolve the separately locked WebPKI finding.

## Actual paths and platform limits

`crates/brn-ai/Cargo.toml:22–26` explicitly enables Rig 0.44 reqwest/RMCP and Rustls. Both CLI and desktop depend on brn-ai without requiring native-retrieval. Cached rig-reqwest 0.44's `rustls` feature enables reqwest 0.13.5 `rustls`; its shared client uses ordinary `Client::builder().build()`. RMCP's HTTP client uses the same reqwest version; feature unification supplies Rustls. Actual BRN consumers include shared clients in auth/conversion/threads and `threads_mcp_http.rs:79`'s configured builder. The selected anonymous MCP endpoint policy is literal loopback HTTP, so that particular service's dependency path is not proof that it performs TLS.

Reqwest 0.13.5 `src/async_impl/client.rs:751–783` uses rustls-platform-verifier 0.7.1 for normal verified clients. Its cached platform implementation is decisive:

- **macOS:** `src/verification/mod.rs:18–22` selects `apple`; `apple.rs` uses Security.framework trust evaluation (`evaluate_with_error`) for server certificate paths. Rustls still links WebPKI for other operations, including signature verification, but the new path-distrust check must not be claimed to replace Apple's trust policy. This review did not inspect this machine's trust data or test an affected chain.
- **Linux:** `verification/others.rs` loads native/system roots and delegates chain verification to `WebPkiServerVerifier`. Thus system-root selection does not imply avoidance of rustls-webpki's affected path logic. Acceptance still depends on configured trust anchors and the presented chain.
- **Optional native retrieval:** `crates/brn-retrieval/Cargo.toml:18` selects reqwest 0.12.28 `rustls-tls`; its cached feature maps to `rustls-tls-webpki-roots`. The model downloader's ordinary client (`native/download.rs:172`) therefore uses bundled roots and WebPKI, including on Mac when this feature/runtime action is selected. This is separate from the provider client's platform verifier. No model download was performed.
- **webpki-root-certs nuance:** the resolved rustls-platform-verifier dependency on this package is **cfg(target_arch = "wasm32")**. Its native Mac/Linux lock presence is not evidence of an active native trust store. Updating its resolved lock record is still a small coherent selected-advisory closure, not a claim of native exposure. The lock is broader than any one active target/feature graph.

The two standalone helper locks each have 409 packages and no records named rustls, rustls-webpki, webpki-roots or webpki-root-certs. No change to their locks is justified by these three advisories. Their other dependencies remain outside this narrow assessment.

## Compatibility and smallest responsible change

Cached reverse-dependency manifests impose compatible ranges: rustls 0.23.45 uses `rustls-webpki ^0.103.14`; platform-verifier uses `^0.103`; root consumers use `1`/`1.0.0`; the unaffected webpki-roots 0.26.11 wrapper depends on parent `1`. No exact old-version pin requiring a product manifest change was found. Keep the 0.26.11 wrapper and update its separately resolved 1.x references. Preserve rustls, Reqwest/Rig/RMCP, features, platform policy and all unrelated packages. Resolve/check the candidate rather than assuming compatible ranges prove compilation. Target crate archives are not presently cached according to lead evidence, so obtaining them is a separate lead-owned dependency operation; no reviewer fetch occurred.

Removing this trust is intended behavior and may reject endpoints whose chains previously passed. That is the security repair, not a reason to add fallback verification, custom certificates or runtime switches. No new BRN TLS abstraction or duplicate upstream certificate test is required for a lock-only patch.

## One concrete next gate

**Gate: freeze and qualify one minimal lock-only candidate.** Require an independently inspected complete diff containing exactly these three version/checksum updates and necessary lock dependency-reference replacements; validate registry checksums against the recorded release evidence, confirm no manifest/helper-lock/schema/provider-policy changes, then run the pinned locked/offline existing brn-ai deterministic tests (including capability fixtures), workspace checks, ordinary native-ui build, and the existing native-retrieval compilation/model-download synthetic tests with no model acquisition or live requests. Preserve normal required Linux/Mac/Windows CI gates so native verifier and optional-feature paths are compiled on their supported targets. Record exact source/lock and actual results; stop and reassess if the resolver expands scope or any gate fails. This packet is the gate, not an instruction for this reviewer to execute it.

A current full cargo-audit pass would add breadth across all three locks and can be a separately selected hygiene task. It would not establish actual verifier reachability, endpoint exploitability or runtime safety. The existing Oct-9 DB predates these records and cannot clear them. The [official cargo-audit documentation](https://raw.githubusercontent.com/rustsec/rustsec/main/cargo-audit/README.md) supports a selected existing lock via `--file` from outside the project and describes automatic fix as a separate modifying feature. Installing a tool/database now is unnecessary to prove these exact current version matches or to unblock their narrow repair. Do not label the resulting candidate “audit clean” or “no vulnerabilities” without an actual appropriately scoped current audit.

Only this report was written. No install, dependency fetch, build, test, runtime/account/native action, global configuration or product/shared-record mutation was performed.
