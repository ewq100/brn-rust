# Independent final four-package TLS candidate review

**APPROVED — no actionable source findings. Qualification remains pending.** Reviewed the complete actual candidate `234956cb171beb025acb1dc17b41be98992bddf0` in `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/tls-certificate-trust-20261022`. Verified clean tracked/untracked status and merge base `3827d6a13953533c7322fd7157d975a32dee5cb2`. The entire task diff is one file, `Cargo.lock`, 12 insertions/12 deletions; `git diff --check` passed. Prior approved `003edd71518300f545fbde2d71bf850470f7ff0e` remains an ancestor. Reviewer did not author source or resolution scripts.

## Complete lock verification

Independently parsed the base lock from Git and actual candidate TOML, constructed the expected full base structure with only the four selected version/checksum replacements and qualified roots references, and compared every field/package/dependency. Exact equality passed. There are 943 packages before/after; only four qualified `webpki-roots 1.0.9` references change to 1.0.10 (hyper-rustls, reqwest 0.12, ureq, compatibility wrapper). No other package/source/checksum/dependency, manifest, helper lock or feature changed.

Candidate lock SHA256 is `f6d7419899f81f4e28eaae198475e69067ac34d174c5293a0f9d13e012fbc01a`, matching `final-candidate-scope.json` and exact saved `Cargo.lock.final`. Base lock SHA is `fc1ca2cf08b51f357dccd277b2fdb5491454c7bb87bf96e1026322d6b5f85d89`.

| Package | Final version | Verified registry/archive SHA256 |
|---|---|---|
| rustls | 0.23.46 | `48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf` |
| rustls-webpki | 0.103.16 | `4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281` |
| webpki-roots | 1.0.10 | `1c1c8e20c8a38fc7d65abfe74eb7280e4aa667e8eec8bc84996d2874810748d9` |
| webpki-root-certs | 1.0.10 | `58ec80e73fd8baf57bf0d9615554c75348bda990d6d5bbeab964dd068319fecb` |

Read and hashed all four already saved archives under `/tmp/brn-tls-resolution/packages`: actual archive bytes match the candidate and recorded primary registry metadata. Read their packaged manifests in memory: all declare Rust 1.71, below repository 1.98.1. No archive was downloaded or extracted into a filesystem by this reviewer.

Compared `Cargo.lock.rustls-resolver-original` against actual candidate independently: the sole difference is tempfile 3.27.0's getrandom edge restored from 0.3.4 to its unchanged base 0.4.3. Its previously inspected published constraint `>=0.3.0, <0.5` permits that restoration. `Cargo.lock.before-rustls` exactly equals prior 003's lock. The recorded one Rustls update exited zero, reaped/left its group absent; this is resolution evidence, not a test pass. The candidate preserves the prior graph rather than adopting unrelated resolver churn.

## Requirements, compatibility and responsibility

Rechecked all cached Rustls consumer ranges (including gpui-pre-reqwest, both reqwests, hyper-rustls, Quinn, platform-verifier, tokio-rustls, tungstenite and ureq); each admits 0.23.46. The actual new Rustls archive still accepts rustls-webpki `^0.103.14` and the existing selected crypto/pki dependencies. Prior three-package reverse ranges likewise admit their patch versions. No new authority, verification bypass, fallback trust root, runtime setting, provider/model action, abstraction or duplicated upstream test was introduced.

Re-read exact DB records for RUSTSEC-2026-0346/0347/0348/0349: rustls 0.23.46 meets all four patched thresholds. The other three packages meet the selected 0381/0382/0383 ranges established by the prior complete review. The unchanged roots 0.26.11 wrapper now points at patched parent 1.0.10. This closes the **seven selected version matches across four packages**, not a count of seven independent exploited vulnerabilities: the three emSign records share CVE-2026-111943, while the four Rustls records are distinct GHSA issues.

Explicitly reuse `/tmp/brn-tls-advisory-assessment.md`, `/tmp/brn-tls-candidate-review.md` and `/tmp/brn-tls-expanded-assessment.md` for unchanged source/platform/configuration evidence. Mac platform certificate trust does not remove Rustls session/handshake logic. The Brotli/ECH/custom Debug conditions were not established in BRN's selected configuration; hybrid preference/endpoint behavior and actual exploitation were not observed. Narrow compatible patch adoption is still justified. No full dependency-clearance or native exposure claim follows from lock membership.

## Remaining gates and limits

Freeze this identity for the lead's planned locked/offline deterministic workspace/provider fixtures, affected strict lint, CLI/ordinary native-ui build and optional native-retrieval synthetic test/build gate, followed by normal protected platform CI. Existing 003 passes remain source-bound historical results. The earlier optional-native linker failure is not a passing qualification; correct/verify the cached ORT environment for the final gate without model acquisition, and report the actual result.

The separately prepared audit continuation needs its own adapter review and exact-input/DB provenance proof. Preserve the valid main213 finding-bearing baseline, the two failed harness receipts and all warnings. No final audit outcome is inferred here.

Only this report was written. Reviewer ran no build/test/audit/CLI workload, dependency fetch/install, source mutation, native/account action or helper. Approval concerns the actual complete source candidate, not execution, native acceptance, protected integration or release.
