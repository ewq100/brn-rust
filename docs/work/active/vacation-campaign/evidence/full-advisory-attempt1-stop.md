# Current-lock advisory scan — stopped before audit

Status: **incomplete; no lockfile audit was run**, so this packet contains no advisory findings or clean result.

I downloaded the official RustSec `cargo-audit` 0.22.2 Apple Silicon release archive into the fresh task-owned directory `/private/tmp/brn-advisory-audit-20261022`. Its 5,954,803 bytes match the official release SHA-256 `ec7ca4263769593df4d909be85b94a6b79efa2897be5d2bb8ebd516e823175af`. The archive-member preflight then stopped before extraction: my allowlist expected the directory entry with a trailing slash, while Python's tar reader reports the same directory path without it. The observed members were that single directory and the five regular files `LICENSE-APACHE`, `CHANGELOG.md`, `README.md`, `cargo-audit`, and `LICENSE-MIT`. No link or special member was reported in the inventory. The check rejected the member set, and the executable was not extracted or launched.

The raw task packet is `/private/tmp/brn-advisory-audit-20261022/packet/`; its `scan-receipt.json` records the conservative stop. The selected process-cleanup helper hash matched the approved value. No advisory database clone/fetch or audit command occurred. The existing shared advisory DB remains at the observed initial HEAD `7eebec69c352c7191b1f13eb95dd510eeca5d1de` (2026-10-09); it was not modified.

Pre-run lock observations (source hashes; no copies were made before the stop):

- Main-213 root snapshot `/tmp/brn-tls-resolution/Cargo.lock.before`: 249,732 bytes, SHA-256 `fc1ca2cf08b51f357dccd277b2fdb5491454c7bb87bf96e1026322d6b5f85d89`.
- Export worker checkout `003edd71518300f545fbde2d71bf850470f7ff0e`, `Cargo.lock`: 249,738 bytes, SHA-256 `2dcc9ad99f05129f077ebc063b0f65fe69bda993ef2bd4824da4d9a6be2b189f`.
- Lead checkout `250348fcd3919c4982784e30ec49a94c1bcf36ce`, stock helper lock: 97,318 bytes, SHA-256 `4d375d6c65cc56a573203fa838f0d42d49e9d6764487bf56adb5b05006e32f36`.
- Same lead checkout, adapted helper lock: 97,175 bytes, SHA-256 `9e3fa02dfa4533233ffdba0338829d01a1ebe767ccdb47d01da4d581a040f508`.

These are only pre-run observations; they are not audited results or post-run immutability checks. No source, lockfile, global installation/configuration, or shared cache was changed. The runner stopped at the authorization boundary after its exact-member check failed; a corrected run requires a newly selected invocation.
