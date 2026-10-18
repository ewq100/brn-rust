# Source1b resolved-source receipt — 17 October 2026

Task `x2-source1b-all-document-audit-20261017`, bound branch `work/x2-intake-20261017`, source base `cd2da52ce85313ae45c0d590db64cc102a96655f`. This is a **prospective standalone all-family graph**, not canonical root adoption or a successful build. [binding.json](binding.json), [command/result](metadata-result.json), [raw stderr](metadata.stderr), [lossless raw stdout gzip](metadata.stdout.json.gz) and [package ledger](package-ledger.json) preserve the actual associations. The raw uncompressed metadata also remains in task scratch.

Exactly ONE authorized Cargo1.98.1 metadata invocation exited0 in3.5322835409315303s. No build/check/test, script/example execution or extractor invocation occurred. Cargo may query rustc/target identity; that is not compilation. No second Cargo command, custom transport or cache patch was used. Source acquisition populated the normal shared registry cache; no cache-independence claim. The scratch lock was seeded from Git65715ee X1, then legitimately changed by this new resolution. Its new hash is **not** the original-lock hash.

| Identity | Actual value |
|---|---|
| Scratch manifest | `9c85e21bedb8cd9dd9627d4d29427865e8df1bb1884777eb998b12ebb91a1e14` |
| X1 seed lock | `b5d6f8483304c913a264ac4b188d1429995322b513e8d8c2ea4de91f919b547f` |
| Resolved scratch lock | `dcaf563ce90289161bb4596976ae958439230a81bd1ce3e099442aa1c0292d78` |
| Root production lock, unchanged | `65d2aeee2a8ec2000e7825266a5f70a10725fd32c617d72ddbdf9361a1eb2a1c` |
| Raw metadata | `19496034b037fa4626b055b235a2fdeb11c00f39f5945973242fe9f12b84b170` |

Conservative normal/build traversal starts at metadata.resolve.root and follows non-dev dependency edges. On the requested aarch64-apple-darwin filtered metadata it reaches364 nodes: the local scratch root and363 registry packages. Cargo metadata retains some weak optional dependency edges (for example image→ravif while image avif is OFF); this is an audited upper bound, not a claimed shipping compile closure. A static feature-eligible traversal of the SAME saved metadata gives320 nodes/319 registry packages; [feature-edge-qualification.json](feature-edge-qualification.json) records exclusions and method. Emitted normal/build units remain a future qualification. Every registry archive SHA equals its lock checksum; every extracted archive file matches the corresponding cached source file. Zero missing archives/sources or byte mismatches. Ledger rows preserve source, features, dependency edges, archive path/hash, whole archive-file tree digest and X1/root membership. `new-source-files.json` additionally lists per-file hashes for61 new reachable registry identities versus X1. `.cargo-ok` is Cargo bookkeeping, not an upstream archive file. This is source provenance and a metadata feature union, not emitted host/target units or execution evidence.

Scratch full lock:399 entries versus337 in X1,63 additions and1 removal (the replaced local experiment root). No old registry identity/checksum is replaced. The closure includes61 new registry identities.132 registry identities in that audited metadata upper bound are absent from the current root lock (131 in the feature-eligible subset). The ledger's whole scratch-vs-root added134/removed700 set comparison is NOT a proposal to remove700 root packages; the workspaces serve different purposes. A future root resolver must map **every** selected new checksum to this ledger or source1a, or acquire/audit that exact newly selected source before compilation. B1 remains closed as a plan requirement; this scratch ledger does not claim it has already qualified an unresolved root graph.

| Exact selected dependency | Scratch fact and root consequence |
|---|---|
| xberg/native/lopdf |1.3.10/1.3.10/0.45.0 unchanged; top3 stop condition not triggered. |
| mail-parser |0.11.9 with full_encoding/encoding_rs via Xberg; current helper exact0.11.8 needs separately allocated shared-pin decision, CONVERTER provenance and host matcher update plus MIME/all-family tests. |
| Tokio |1.53.2, union fs/process/rt/rt-multi-thread/io-util/sync/time/macros; current root1.53.1 does not satisfy this Xberg lower bound. No root upgrade occurred. Runtime proposal in [runtime.md](runtime.md). |
| calamine |0.36.1 default/dates/chrono. Scratch picture OFF; current BRN helper picture ON would unify differently in a root graph and requires source/operation requalification. |
| ZIP/CFB |zip8.6.0 and9.0.5 coexist; cfb0.14.0 and0.15.0 coexist. Do not substitute one version's security behavior for the other. |
| HTML |html-to-markdown-rs3.18.1 already in the X1 PDF closure; HTML-only requires no package beyond that closure and no Tokio. |
| image |0.25.10 bmp/gif/jpeg/png/pnm/rayon/tiff/webp. These compiled codecs do not broaden BRN's PNG/JPEG asset admission. |
| email/Office ancillary |outlook-pst1.2.0, typst0.15.1, bib/org/DBF/math parsers compile through aggregate features; no automatic admission of those formats or temporary-file routes. |

Xberg's actual feature union is `email,excel,html,image-encode,notebook,office,pdf,pdf-native,tokio-runtime`. No Xberg defaults/OCR/pdfium/ORT/layout/model/remote/server/API feature is enabled. Dependency defaults and codecs still need their own ledger; this statement is not 'client/minimal/no native code'. Native PDF and its graph contain Rust and bundled source-level codec/C/assembly build possibilities, not authorization to acquire a runtime binary or model.

## Build-script and side-effect source audit

[build-scripts.json](build-scripts.json) identifies45 metadata-listed build scripts and hashes;41 reuse the exact X1 scripts.38 are in the static feature-eligible subset;7 are metadata-only optional descendants (listed in feature-edge-qualification.json). The4 newly selected scripts are all feature-eligible. Four newly selected scripts were read, **not executed**:

| Script | Source operation / qualification |
|---|---|
| nix0.31.3/build.rs |cfg_aliases and check-cfg emission; no fetch/exec operation. |
| portable-atomic1.15.0/build.rs +version.rs |local rustc/version/target probes and cfg selection; Command in version.rs15, no package/model downloader. |
| rustix1.1.5/build.rs |backend/cfg probes, local compiler test with task build output files; aarch64 Mac uses libc. Not an extraction-time process permission. |
| typst-utils0.15.1/build.rs1–22 |version env output and optional local `git rev-parse HEAD`; no fetch. Git child is build-time, not a BRN helper runtime route. |

The existing X1 script audit remains source-bound for unchanged scripts, including bundled BLAKE3 native compilation; all source/archive equality was rechecked here. No script has run under this grant. Future offline network-denied build evidence must distinguish compiler/build tools from prohibited native/model acquisition. Paths for model provisioning, PST tempfile/open_store, URI/file IO, OCR, structured extraction and batch workers exist in published code; [runtime.md](runtime.md) and [resources.md](resources.md) describe why the proposed byte-only operation excludes them. Presence is not execution, and static reachability is not a sandbox PASS.

The cache arrival of cfb0.15/mail0.11.9/Tokio1.53.2/zip9.0.5/outlook-pst after cd2 is caused by this command. The earlier 'missing' statements remain historically true. No claim that old source1a had audited these packages.
