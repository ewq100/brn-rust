# Independent helper ZIP advisory assessment

**Recommendation: retain these three unchanged helper matches as a separately tracked dependency-maintenance concern, with source-bound non-reachability/containment evidence. Do not expand root TLS214 or force ZIP 9 into the existing Calamine range. No presently supported lock-only patched update exists for the affected helper edge.** A dependency modernization requires its own selected helper compatibility/identity task; this review found no current call to the affected filesystem-extraction APIs.

## Exact inputs

Inspected lead source at `3d12de74c6453aa4ceec6a428c0b1958668907f9`, product main213 `3827d6a13953533c7322fd7157d975a32dee5cb2`. Current lead dirty/untracked files concern campaign/evidence/overview; they were preserved. Both helper manifests/locks and relevant product source remain unchanged by TLS candidate `234956cb171beb025acb1dc17b41be98992bddf0`.

- Stock lock: `tools/xberg-intake-helper/Cargo.lock`, SHA256 `4d375d6c65cc56a573203fa838f0d42d49e9d6764487bf56adb5b05006e32f36`, 409 packages.
- Adapted lock: `tools/xberg-intake-helper/adapted.Cargo.lock`, SHA256 `9e3fa02dfa4533233ffdba0338829d01a1ebe767ccdb47d01da4d581a040f508`, 409 packages.
- Actual parsed reports in `/private/tmp/brn-advisory-audit-20261022/packet-continuation-source-proof-corrected/commands/audit-{stock-helper-lock,adapted-helper-lock}.parsed.json` both SHA256 `07b41cea2a182e92c80428d015f4b727da8ed5a8ae4a31ae97dac72e1b2a609b`: each reports three vulnerability matches for ZIP 8.6.0 and one paste unmaintained notice, with no ignore/target/severity filter. The completed receipt records exit 1 for each helper and exit 0/zero vulnerability matches for the separate final root lock.

Read pinned DB commit `59ef7abf8ce56ea80fe7eba97ba3835449009fa4` records and official pages. [RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html) concerns privileged permission bits applied by extraction; [0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html) concerns externally resolving created symlinks; [0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html) concerns applying permissions through symlinks. All declare patched `>=9.0.0`, with distinct GHSA aliases. In particular, 0368's prose describes a pre-release reproduction; the actual audit match for 8.6.0 follows the published version range, not a reproduction in BRN.

## Actual graph and APIs

Both locks resolve **two different ZIP versions**:

`helper → xberg 1.3.10 → calamine 0.36.1 → zip 8.6.0` (affected version match)

`helper → xberg 1.3.10 → zip 9.0.5` (already within these patched ranges)

The affected ZIP 8.6.0 is not Xberg's direct DOCX/PPTX ZIP implementation. Cached published manifests confirm Xberg's direct ZIP `9.0.2` requirement and Calamine's ZIP `8.6` requirement. Read and SHA-verified already cached archives against locked checksums: Calamine `5fa68281b1a76b54a62156474adb06bb380a67e07dd60656e3217152b42183f3`, Xberg `f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d`, ZIP 8.6.0 `2d04a6b5381502aa6087c94c669499eb1602eb9c5e8198e534de571f7154809b`. Targeted inspected cached source/manifests exactly match their archive members; no dependency was downloaded or extracted to disk.

The helper enables Xberg `pdf,office,excel,email,html`, disables defaults, and accepts only eight explicit document kinds (`DocumentRequest::mime_type`, `crates/brn-intake/src/document.rs:75`). Generic ZIP input is not an admitted kind and Xberg's `archives` feature is not selected. `tools/xberg-intake-helper/src/engine.rs:66` supplies `ExtractInput::from_bytes` and disables local-file/URI input, OCR/cache and recursive archives.

For XLSX, Xberg `src/extraction/excel/open.rs:596–597` constructs `calamine::Xlsx::new(Cursor<...>)`. Calamine's `src/xlsx/mod.rs:2544` constructs `ZipArchive::new(reader)` and `:2761` obtains `zip.by_name(...)` into an XML reader; indexed media/VBA reads likewise consume entry bytes. Inspected the whole Calamine source for `ZipStreamReader`, `.extract(`, `set_permissions`, `unix_mode`, `File::create` and symlink calls: none occur. Calamine also has indexed/named ODS/XLSB readers, not filesystem unpacking. `std::fs::File` occurrences are input-reading entry points; the helper selects byte input.

In ZIP 8.6.0 itself, dangerous effects reside in `ZipArchive::extract_internal` (`src/read.rs:416–447`) and streaming `extract` (`src/read/stream.rs:56–109`), creating paths/symlinks or applying modes. Ordinary indexed/named entry reads do not invoke that materialization. No BRN or affected Calamine consumer calls these APIs. Thus a version finding is established, but no current helper route to these three described filesystem effects is established.

## Containment and limits

Both helpers install restrictions before reading untrusted input (`crates/brn-intake/src/helper_runtime.rs:12`). On Mac the deny-default sandbox grants system-runtime reads and `/dev/null`, not source/workspace/scratch writes, chmod or symlink authority. Linux x86_64 closes descriptors above stderr, applies no-new-privileges and fail-closed pipe-only seccomp without pathname operations; other OS/architectures refuse conversion. Host `crates/brn-threads-intake/src/process.rs` clears environment and bounds pipes, time, cancellation and owned process-group lifetime. Both engines share this wrapper. These are independent mitigating boundaries; they neither change the advisory status nor constitute a new runtime exploit/sandbox qualification. No privileged helper launch, malicious ZIP test, filesystem escape or actual exposure was observed here.

The adapted generator copies the committed adapted lock and applies four hash-checked DOCX/PPTX source patches; it does not patch Calamine/ZIP. Therefore the same affected reader edge persists in both modes. Broadly disabling Excel or changing stock semantics would be disproportionate to the established unused-API finding.

## Compatible repair and next gate

Calamine `^8.6` excludes every declared patched ZIP version. Pinned Xberg `=1.3.10` requires Calamine `^0.36.1`, which excludes 0.37.0. Cached registry metadata lists no newer 0.36 patch; its next line is 0.37.0 with ZIP `^9.0`. Official [Calamine 0.37.0 release notes](https://github.com/tafia/calamine/releases/tag/v0.37.0) explicitly explain that ZIP's public `ZipError` necessitated the version bump, and also list quick-xml/parser behavior changes. This is not a permissible one-record lock rewrite. A direct helper dependency on ZIP 9 would not remove Calamine's separate 8.6 dependency; a global override that misrepresents 8.6 or silently forks a “stock” engine is not a sound repair.

Leave TLS214 root-lock-only and preserve the helper reports/paste notice without new audit ignores or a full-clearance claim. If helper modernization is selected, first establish a supported published Xberg/Calamine combination (or an explicitly owned, honestly identified patch boundary), inspect the entire changed source/graph and identity contract, then qualify both engine builds, eight-kind synthetic output/retained-byte expectations, restriction/cancel/process-tree/no-fallback checks and audit both exact final locks. Expected Calamine/quick-xml behavior changes need review, not automatic snapshot replacement. No speculative duplicate extraction test or new containment framework is required solely for an API the current helper never calls.

Revisit sooner if affected filesystem extraction is introduced, helper restrictions/privilege policy change, a practical current-reader exploit appears, or a compatible upstream fix becomes available. Only this report was written; no audit, build, scan, fixture, model, account/native or product/shared-fact mutation was performed by this reviewer.
