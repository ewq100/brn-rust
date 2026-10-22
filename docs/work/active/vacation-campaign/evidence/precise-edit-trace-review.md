# Independent complete-candidate review

Reviewed 2026-10-22, source inspection only. Reviewer owns this report only; no source, Git, shared task-record, tests/builds, account, network, native or nested-agent actions were performed.

## Candidate identity and complete change inventory

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`
- Branch: `codex/precise-edit-trace`
- Base / verified merge base: `2ecf634d7de93fbf7381317168109b6d199531e4`
- Base tree: `ad176b75b0ef839ec052f4bf21010f139ab9203d`
- Candidate HEAD: `777e346150ab718756002bb868d3bd6ff723ed3e`
- Candidate tree: `d4c1a780944674509cd9300f4929ad5c173779af`
- Working tree and untracked inventory clean at entry and final inspection; candidate file equals HEAD. Complete delta is one modified file, 112 additions / 0 deletions.

| Changed file | Base Git blob | Candidate Git blob | Base SHA-256 | Candidate SHA-256 |
|---|---|---|---|---|
| `crates/brn-ai/src/threads_trace.rs` | `44e2cc7c755736934c6dc495f5d18339d804b51c` | `889c6a5b360b7197e26930e6f26a19304745b7a1` | `ff3b900c5491c7ae720b1e4dd9683466da4f245919929bf32bf07645934ee5e4` | `6a1435bba00af9456477a242e2ac466810edc84b96e3e0608d864ad142b361b7` |

Prior activity/continuation candidates are frozen dependencies, outside this delta. No PR199 head modification is part of this review.

## Requirement and standards review

Authorities read: root AGENTS, current status/index, `brn-ai` README, architecture overview/invariants, verification/workflow, campaign plan/resource policy, actual lead `precise-edit-trace-evidence` task in vacation-hardening `campaign.json`, independent-review skill/resource policy and Rust review skill. Relevant implementation followed through collector dispatch/outcome, registered tool activity/outcome labeling, `threads_note_edit.rs`, prepared-output projection, checked preparation, core operation allocation and core prepare validation. External Rust guidance was not fetched because reviewer network use is excluded. Memory registry keyword lookup had no relevant hits and supplied no review facts.

## Finding

**P2 — Restrict recorded rejection reasons to the required host-static diagnostic boundary.** Location: `crates/brn-ai/src/threads_trace.rs:392-399`.

The new branch copies any string `reason` from an `Invalid` result, truncating it to 256 Unicode scalars. The four direct `threads_note_edit.rs::invalid(...)` calls are fixed host strings, but they are not the complete result surface: `prepare_edit` returns `prepare_checked` errors unchanged (`threads_note_edit.rs:174-181`), and `prepare_checked` forwards `Error::Invalid(reason)` and `Error::Missing(reason)` (`threads.rs:2042-2045`). A valid precise edit using a fresh request key while the same Note already has an active candidate reaches `store::prepare_on` and returns the formatted string `note {write.id} already has an active candidate` (`crates/brn-threads-core/src/store.rs:3049-3052`). This is not a host-static reason, contrary to the explicit reviewer packet requirement. Normalize this trace-only case to approved fixed wording (and retain the existing separate ID field), or otherwise project only approved static diagnostics with a fixed fallback for other reasons. Add a test covering the actual active-candidate rejection surface and unknown/dynamic reason fallback; the synthetic `static host rejection` assertion does not establish that boundary.

Observed impact is deliberately limited: the currently reachable formatted reason contains only a Note ID already allowed in the argument projection. I found no current raw passage, model explanation, request key, prepared request or credential leak through this path. This finding is an explicit contract mismatch, not evidence of such a leak or an established cause of the failed account edit. Lead disposition remains required; no correction was made by reviewer.

## Other conclusions and limitations

- Argument projection avoids the generic arbitrary-Value clone closure for this tool: ID accepts only strings up to 256 UTF-8 bytes; version/start/end accept only unsigned numeric primitives; passage fields contribute UTF-8 lengths only. Malformed objects/arrays project to null/zero. Summary and Full invoke the same projection.
- Successful real outputs come from `prepared_output`, whose `operation` is the host-generated UUID-backed `OperationId`; complete request/Note bodies are not retained by this result branch. The result projection does not itself reject malformed object/array operations, but I found no production path by which such an operation reaches it. This is a defensive-test opportunity, not an additional actionable production finding.
- The Invalid reason bound counts Unicode scalars without splitting UTF-8. Both levels share the same result function. Existing call sequence/count/bytes/timing handling is unchanged; the change adds only small bounded retained metadata in real calls. No new execution owner, public API, dependency, tool schema, authority, runtime prompt, retry, apply or logging mechanism was introduced.
- New tests assert exact projections, Unicode byte counts, malformed argument containers, exclusion of other supplied fields, prepared operation-only output and 256-scalar reason length. They do not test 256/257-byte ID boundaries, malformed primitive numeric cases, collector dispatch/outcome wiring, or all actual host Invalid sources. The direct projection checks are meaningful but do not establish the integration path by themselves.
- Compile configuration was inspected: `threads_trace.rs` is included unconditionally by production `threads.rs`; `threads` is publicly compiled in ordinary `lib.rs`; these unit tests use `cfg(test)` and do not require `capability-spike` or `scripted-runs`. Manifest and repository pin are Rust 1.98.1 / edition 2024. No compile result is inferred from inspection. Lead-owned format, runtime tests, strict lint and CLI build were described as running; their results were not available to or independently executed by this reviewer.
- The first real account attempt's two Invalid results, absent candidate and unchanged synthetic target/sentinel are context supplied by the lead and task record. The old trace cannot prove exact rejection cause; this review does not claim a cause or a successful follow-up account/native/owner-acceptance result.

Review outcome: one bounded P2 requirement finding; no other actionable findings from the complete actual delta. Tests/build qualification, finding disposition, integration and resulting-main evidence remain lead-owned separate gates.

## Corrected complete candidate: c4f0e2e

Re-reviewed 2026-10-22 after the lead accepted the P2 and committed a trace-only correction. The original review above remains the record of the intermediate candidate; this section supersedes its unresolved disposition and reason-bound implementation description.

- Base / verified merge base remains `2ecf634d7de93fbf7381317168109b6d199531e4`, tree `ad176b75b0ef839ec052f4bf21010f139ab9203d`.
- Final candidate HEAD: `c4f0e2e663cb04654c2bc49f9a540499fdd139b9`.
- Final candidate tree: `0a72e02e10211c44f1d9fae1d029c0d25a5b8728`.
- Same checkout and branch; working tree/untracked inventory clean, actual file equals HEAD. Complete base-to-final inventory remains only `crates/brn-ai/src/threads_trace.rs`.
- Final file Git blob: `8ca15df8a03da0c5731fbdecd39fb316442fe196`.
- Final file SHA-256: `f3c6a0d3e5d8b5f43a2cf291208ee5254e5b40c310b81425e88424de7e314a82`.
- Unchanged base file hashes are recorded in the initial inventory table. No other changed files exist in the complete candidate.

**Finding disposition: valid/fixed.** `threads_trace.rs:393-406` now permits only exact equality with the four actual direct static messages in `threads_note_edit.rs`; every other/missing/nonstring reason maps to the fixed `Other checked preparation refusal.` diagnostic. The active-candidate formatted private-ID case is explicitly among the negative test cases. Unknown data is borrowed only for type/equality checks and never serialized into the projected reason. Each permitted literal and the fallback are below 256 Unicode scalars; replacing arbitrary-text truncation with this finite static vocabulary preserves the bound while closing the finding.

The complete original argument/success projection was carried forward unchanged. I re-read the actual correction, final projection/tests, exact direct handler messages, candidate inventory and clean identity. New tests enumerate all four permitted literals and exercise active-candidate formatted text, arbitrary dynamic text, absent reason and object reason. Tests preserve exact result projection assertions and the original Summary/Full/UTF-8/malformed argument checks. No editing, authority, schema, tool, prompt, retry, dependency, dispatch or application behavior changed.

**Final review outcome: no unresolved actionable findings.** The previously recorded defensive operation-type and integration-test limitations remain observations, not newly discovered production defects. No tests or builds were run by this reviewer. The lead reports intermediate `777e3461` runtime tests `494 passed / 0 failed / 1 ignored`, lint and CLI build passed, with final formatting/whitespace passed; these are lead-reported evidence for those exact identities, not reviewer-executed results or qualification of the final candidate. Exact-`c4f0e2e` runtime tests, lint and CLI build remain pending at this review checkpoint. Integration, resulting-main evidence and any later account/native/owner-acceptance claims remain separate lead-owned gates.
