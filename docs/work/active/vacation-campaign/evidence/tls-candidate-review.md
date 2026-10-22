# Independent complete TLS candidate review

**APPROVED for the planned qualification; no actionable source findings.** Reviewed exact clean candidate `003edd71518300f545fbde2d71bf850470f7ff0e`, branch `codex/tls-certificate-trust-20261022`, in `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`. Verified merge base `3827d6a13953533c7322fd7157d975a32dee5cb2`. Complete change: only `Cargo.lock`, 10 insertions/10 deletions. No dirty or untracked source was present. This reviewer authored neither the candidate nor its resolution driver.

## Independent comparison and restoration

Parsed the base lock directly from Git and the actual working-tree candidate with Python TOML. Constructed an independent expected structure by changing only the three selected version/checksum pairs and qualified `webpki-roots 1.0.9` references. Full structural equality passed, including package order, package sources, all other checksums and dependencies, and top-level lock fields. Both locks contain 943 packages. Base SHA256: `fc1ca2cf08b51f357dccd277b2fdb5491454c7bb87bf96e1026322d6b5f85d89`; candidate: `2dcc9ad99f05129f077ebc063b0f65fe69bda993ef2bd4824da4d9a6be2b189f`. Complete diff and `git diff --check` inspected/passed.

The four qualified-reference replacements belong to hyper-rustls, reqwest 0.12, ureq, and the webpki-roots 0.26.11 compatibility wrapper. All other package records are identical to base.

Compared the preserved resolver output against the actual candidate separately: exactly one additional edge differs, tempfile 3.27.0's `getrandom 0.3.4` restored to its base `getrandom 0.4.3`. Cached tempfile's applicable manifest constraint is `>=0.3.0, <0.5`, so this restores a permitted original dependency without introducing a package or changing features. The final candidate equals the base graph plus the selected TLS changes. Locked compilation still must confirm acceptance.

Evidence naming matters: `/tmp/brn-tls-resolution/Cargo.lock.after` and `Cargo.lock.resolver-original` are identical **pre-restoration resolver outputs**, SHA256 `da8b87bb74faa8dd7d1cd2a9eeba977edb8d1829be1ad05b647c84f9e977f4ad`; neither is the final candidate. The final hash in `candidate-scope.json` matches actual source. `resolution.json` and three update logs record each exact update exiting zero; the later inline checker error was not a Cargo failure or a passing qualification. No repeated Cargo resolution is claimed. My first evidence-inspection script incorrectly assumed the saved `.after` was final and stopped at that assertion; subsequent direct comparison established the distinction above without any mutation.

## Security and compatibility

Each candidate checksum exactly matches the saved primary registry release metadata:

| Package | Final version | SHA256 checksum |
|---|---|---|
| rustls-webpki | 0.103.16 | `4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281` |
| webpki-roots | 1.0.10 | `1c1c8e20c8a38fc7d65abfe74eb7280e4aa667e8eec8bc84996d2874810748d9` |
| webpki-root-certs | 1.0.10 | `58ec80e73fd8baf57bf0d9615554c75348bda990d6d5bbeab964dd068319fecb` |

Re-read the exact saved RustSec records at DB commit `59ef7abf8ce56ea80fe7eba97ba3835449009fa4`: these versions satisfy respectively RUSTSEC-2026-0382's patched 0.103 range and 0381/0383's `>=1.0.10`. The records share CVE-2026-111943. The unchanged 0.26.11 wrapper now resolves its patched 1.x parent. Registry metadata reports non-yanked releases/MSRV 1.71, below repository Rust 1.98.1. This verifies metadata equality, not downloaded archive bytes; lead's planned public-archive checksum verification remains pending.

Rechecked actual cached reverse constraints: rustls requires WebPKI `^0.103.14`; platform verifier `^0.103`; roots/certs consumers require `1` or `1.0.0`; the compatibility wrapper requires parent `1`. All admit the selected patches. No manifest, feature, verifier selection, trust override, provider behavior, helper lock or runtime prompt changed. No broad dependency churn or new BRN abstraction/test is needed for this transitive patch.

Reuse the independently inspected, unchanged feature/platform evidence in `/tmp/brn-tls-advisory-assessment.md`: normal provider reqwest 0.13 uses Apple's Security trust path on Mac; Linux's platform verifier uses WebPKI with system roots; optional native retrieval reqwest 0.12 uses bundled WebPKI roots. webpki-root-certs is a wasm32-only platform-verifier dependency, so lock presence does not establish native Mac exposure. Exact affected-version and active source-path evidence justifies repair without establishing an exploitable endpoint or claiming all linked paths use the same verifier. The two unchanged standalone helper locks do not contain the selected TLS packages.

## Remaining gate and limits

Proceed with the lead-owned archive verification and one frozen locked/offline deterministic qualification packet, including existing provider fixtures, ordinary native-ui compilation, optional native-retrieval/model-download synthetic coverage, affected lint/build checks and normal protected platform CI. No live request, model acquisition, native launch, custom distrust implementation or full audit-tool installation is needed to validate this narrow change. Resolver or compilation failures require reassessment rather than widening the patch silently.

No builds, tests, dependency/archive fetches, CLI/runtime/account/native actions, source writes or helpers were performed by this reviewer. Only this report was written. Passing source review is not a build result, full cargo-audit clearance, native acceptance, merge or release.
