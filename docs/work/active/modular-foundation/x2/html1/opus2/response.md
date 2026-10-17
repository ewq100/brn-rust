# HTML actual review 2: single-finding response

18 October 2026. Same allocation/check-out, Codex GPT-6.1 Sol. The [full review](opus-review-round2.md) is copied byte-for-byte, including its historical source-traced/not-executed qualification. Its material baseline is `695f750af5103550239830a541b3673e107c624c`; entry evidence HEAD was clean `ccbcf422eaa97c73694001410e238844dc0eace8`. No Kiro call or delegation occurred.

## Blocking href-serialization finding

Both new witnesses were first compiled against **unchanged 695 production** in `/private/tmp/brn-x2-html-r2/before-src`, with only the identical test-only witness block added. Its separate target is `/private/tmp/brn-x2-html-r2/target-before`, never the earlier bbf before target. Helper build passed first. The subsequent real-Xberg tests produced **0 passed / 2 failed**, after successful admission, corroboration and exactly one backend call per case:

- Email returned `<ops@example.test>`; integrity falsely refused because `mailto:` was absent.
- Entity-shaped URL returned `[go](https://x.test/?v=&amp;copy;)`; integrity falsely refused the serializer's added ampersand layer.

The correction touches only `documents.rs` configuration, comparison and tests. The exposed maintained `ConversionOptions.autolinks` field is now explicitly false, so email and equivalent HTTP labels retain standard explicit destinations. For href comparison only, one linear pass removes exactly the literal `&amp;` layer added by the pinned destination serializer. It does **not** decode named/numeric URL entities: literal `&copy;`, `&#169;` and Unicode `©` remain different URL contents. Word/backslash comparison is unchanged. Returned Markdown is still exactly backend output; no output rewrite or trim is introduced.

This uses an exposed maintained option and a fixed literal inverse of the known serializer operation, with no entity table, extra API/dependency, HTML/Markdown parser or framework. The word and href cursors remain monotonic and independent. Normalization adds only constant linear passes over output; aggregate work remains `O(M + S + H)`. Integrity is still a truncation defense with the previously disclosed substring/normalization limitations, not a fidelity proof.

The two identical before witnesses now pass. Two additional tests exercise ten entity/query spellings, an equivalent HTTP label, mixed/repeated targets, literal body entities, consumed-window bounds, missing/reordered targets, and changed literal URL values. Existing backslash/literal negatives, exact fidelity assertions and the 219-pair corpus are unchanged. Admission grammar/resources, lock/manifests, host/prepared HTML and every other source file are unchanged.

## Nonblocking historical log clarification

Small dated metadata in the original report/check-index now states that its 37 `raw_log_sha256` fields identify original scratch bytes. Historical repository gzip copies shortened `/Users/evokessler/` to `~/` before compression; that prefix must be expanded after decompression to compare with the original raw digests. All 37 expanded repository copies were verified against those original digests. Every old raw hash and gzip byte was preserved. Opus1/opus2 raw-to-gzip storage remains separately byte-preserving.

## Current affected verification

- Helper build first: pass.
- New real-Xberg witnesses/supplements: **4 passed**.
- HTML/admission documents tests: **27 passed**.
- Helper all targets: **95 passed**, zero failed/ignored (54/0/17/12/6/6), including the existing rich prepared-HTML ordinal/table/figure regressions and unchanged restriction/cancellation canaries.
- Strict helper all-target Clippy: pass with `-D warnings`.
- Formatting: recorded in the check index.

Before/final source, artifact, `.d`, log and full-diff associations are retained in the check index and handoff. No source edit occurred during a Cargo command. There were no unplanned compile/test/lint retries. The expected two before behavioral failures remain failed evidence.

Prior 91-test and host/workspace/native/retrieval results stay qualified to 695/earlier pins, including carried PDF `Unavailable`; they were not rerun or relabeled for this tiny adapter correction. No new measurement, live/Auth/provider/GUI/download/mutant/PR/push/merge operation occurred. Parent owns final focused precision review and component acceptance. Rich HTML cutover, all-eight-kind adoption, resource/platform/native/owner gates and final integration remain pending.
