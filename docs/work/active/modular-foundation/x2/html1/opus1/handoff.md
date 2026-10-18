# HTML correction candidate handoff

18 October 2026. Corrected source pin: `695f750af5103550239830a541b3673e107c624c`, branch `work/x2-html-20261017`, literal checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-ad771af6`. Review baseline: `bbf107e370a5c5dc35a7588e97c477d5639401a6`; full implementation base: `781c34dc466d6a6a210cdae4cf75e1096521c70d`. Cached `origin/main` was `57c710ccec2b007c4d8a25c9f1d1b5c1c539907c`; no fetch or remote operation occurred.

This is the completed reviewable candidate for the same B1–B4/A1/A3/A5/A6 correction grant, resumed using Codex after actual Kiro monthly-limit failure. Both handed-off source hashes matched on arrival. Useful WIP and all old evidence were preserved. No further Kiro call, delegation or independent review was started.

Read the [complete review](opus-review-round1.md), [finding response](response.md) and [check index](check-index.json). The evidence-only handoff commit does not alter the source pin's crates tree. Parent owns the next fresh independent review using available Codex and subsequent local component acceptance; neither is claimed here.

## Implemented and verified

- Hrefs use a monotonic source-order cursor, removing links × output scanning. Words and hrefs each consume at most one normalized output length. Backslash normalization is symmetric for comparison; actual returned Markdown stays unchanged.
- Duplicate body refuses before backend; unused count metadata is test-only and the tautology is removed. All other grammar/resource limits are unchanged.
- A deterministic corpus checks 219 admitted parent/child pairs, roots/voids/entities/attributes/raw text/parents, CRLF, list/document whitespace and depth-32 voids against known source trees, astral-tl, maintained tokenizer corroboration and real Xberg. Exact literal text/code, link multiplicity/order, tagless input and executor wake tests are included.
- Historical B4 evidence statements are corrected with dated qualifications. The old raw records and hashes remain historical; no deleted old build was recreated.

| Frozen-source check | Actual result |
| --- | --- |
| Helper build first | exit 0 |
| Focused HTML/admission tests | 23 passed, 0 failed |
| Helper all targets | 91 passed, 0 failed, 0 ignored (50/0/17/12/6/6 across lib/bin/maintained/pptx/protocol/restricted) |
| Helper all-target strict Clippy | exit 0, `-D warnings` |
| Workspace formatting check | exit 0 |
| Existing threads-intake all targets | exit 101: 35 passed, 1 carried PDF `Unavailable`, 3 ignored |
| Existing rich prepared HTML | dedicated ordinal/table/figure tests passed in helper lane; host fixture sequence passed HTML before failing at PDF |
| Markdown links and diff whitespace | pass; exact logs/index retained |

The genuine compiled before witness produced 0 passed / 2 failed: B1 consumed 1,607,542,747 window bytes for 1,092,781 output bytes; B2 reached actual backend conversion then refused valid literal backslashes. The identical final witnesses pass. The window counter measures matched prefixes/remaining failure suffixes, not CPU comparisons or whole suffix lengths offered to `find`.

The preserved source snapshot's candidate helper build had completed before provider handoff. Its test build was interrupted without a verdict. The resumed build then found E0434 in a test panic diagnostic; it is retained and fixed. A subsequent 22-test intermediate pass is separate from the final 23-test source. No compilation error is counted as a behavioral red.

## Restricted runtime measurements and limits

The preserved new final helper, SHA-256 `f88eda50815ad2d11adf54d4972b3142bc62c682a3458d19bb511501bb1cc78d`, ran with an empty environment and unchanged sandbox/CPU10/NOFILE64/FSIZE restrictions:

| Fixture | Actual result |
| --- | --- |
| 2,179,794 input bytes; 300,000 steady words then 2,000 unique links | exit 0; 2,157,781 Markdown bytes; 2.51 s wall, 1.02 s user, 0.03 s system; max RSS 70,991,872 bytes |
| Existing 400-byte friendly fixture | exit 0; 243 Markdown bytes; 0.01 s wall; user/system round to 0.00; max RSS 20,480,000 bytes |

Exact original hashes/bytes, source text, all repeated words and all 2,000 ordered label/target pairs were validated from retained output. The original new before helper measured 4.81 s wall / 3.32 s user / 0.02 s system on the same large request; its surviving raw timing/output are retained, with no retroactive environment/PGID claim. Measurements do not establish complexity, hard RSS/thread caps, or absence of syscall attempts.

The ad hoc runtime validator first mishandled digest-array and base64 DTO encodings, then used expectations from a different friendly fixture. Its scripts, failure note and available raw tracebacks are retained. The helper succeeded on each fixture once; completed conversions were not repeated. Correct validation reread their actual bytes and authored fixture facts. Exact runtime PGIDs were not retained before the validator failed; `communicate` had waited for each operation and the final precise process inventory is separate. Existing deterministic group-cancellation and restriction-canary tests also passed.

## Retention and release

New artifacts and `.d` files remain outside build targets under `/private/tmp/brn-x2-html-r1/artifacts`, including before/candidate helper and test executables, actual source snapshots, instrumentation diff, native blake3 emitted archive/output, and complete base-to-pin and review-to-pin binary diffs. [The index](check-index.json) gives every path, byte size and hash. Test-built helper `alltargets-brn-intake-helper` is separately identified from the dev-built measured helper. Nothing was cleaned up.

The literal before root `/private/tmp/brn-x2-html-r1/before-src` uses only `/private/tmp/brn-x2-html-r1/target-before`; the allocated checkout uses only `/private/tmp/brn-x2-html-r1/target`. Source roots were never aliased through a target. Raw logs are retained compressed in this folder, with raw and stored hashes. Started/finished command receipts record input hashes and unchanged-source checks; those hashes equal the later source pin, without claiming commands started at a future SHA.

[Release receipt](release.json) records the final empty heavy-process inventory and completed command sessions. Sole heavy and every allocated path are released to principal on this terminal handoff. No worker commands, Cargo, helper/test process or writer are left running. No review/next family will be started by this worker.

## Remaining gates

This is implemented and locally verified with the explicit PDF qualification above. Fresh independent review and principal acceptance are pending. Native/owner observation, hosted/platform CI and integration are not claimed. No PR, push, merge, live/Auth/provider operation, download, mutant or other-format source change occurred.

The historical broad workspace/native/retrieval checks remain qualified to `bbf107e3`, including their failed attempts. They are not new-pin passes. Full rich-HTML user-facing cutover, PDF resource STOP, Office/XLSX/MIME, helper-local runtime adoption and the owner's all-document objective remain pending under their separate allocations.
