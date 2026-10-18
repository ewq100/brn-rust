# Response to Opus round 1 (CHANGES_REQUIRED)

> **Superseded in two places by round 2 ([response](../opus2/response.md)):** the case C evidence below (the spec now judges it on the synchronous `start_run` outcome only; the Run record has no attached field) and the round 1 spec/merge-simulation wording that a writing prompt never equals a pending composer prompt (the fixed rewrite string can equal one; an exact match may attach as in X2). Kept otherwise as written.

18 October 2026. Source-only. The report is copied verbatim in [opus-review-round1.md](opus-review-round1.md) (SHA-256 `65422c3e80159d9fc70b857fa54bedd58c64c22758c3516606ef8a04728d2211`, byte-identical to the file in `~/.codex/brn-build-orchestration/2026-10-16/`; the terminal JSON's `latestTerminalSummary` equals it except one trailing newline). Revision 1 was commit `d4574398516000f63d45e38f168b2a8294ca8951` (tree `d08b12cc77dc440d3a2d012953f42bc7386a9a1f`). This response is not an approval; each item says what changed, on what source fact, and what is still unverified. A new independent review decides.

## B1 (P1): the Xberg precondition

**Accepted; the revision 1 text was wrong.** Revision 1 let a first-HTML helper pin stand in for the program and classed Xberg resource, runtime, fidelity and packaging work as "not part of this integration". That contradicts the owner amendment ("all-document candidate"), `owner-scope-all-documents.md`, and `x2/plan.md`: "No component-level success declares all-document adoption."

Changes ([plan](../plan.md), sections 0, 2, 6.3, 8; [inventory](../inventory.json) `names`, `final_precondition`):

1. **FI precondition FP-1 to FP-7** defined from `x2/acceptance.md`, `x2/formats.md` and `x2/plan.md`: all eight current kinds (PDF, DOCX, Markdown, Text, EML, HTML/HTM, XLSX, PPTX) with Markdown/Text on the declared Verbatim mode; every acceptance row on the pin; PDF resource STOP resolved by a cited disposition; **no old backend** (all three Poppler routes retired with canary evidence, or a cited disposition that changes the objective); every extra family in `formats.md` with a cited owner or principal disposition; reviewed pinned family completion; status keeps the objective open.
2. **Naming.** The first schema-1 HTML helper component is an engineering component and is not "Xberg done". Any optional earlier INT-W × capability exercise is renamed **TCI-1** (technical component integration), with **TCI-2** for TCI-1 plus the HTML helper capability once it has a reviewed pin. Neither is final integration, a default PR candidate, or a grant; no build is requested now.
3. **§4.3 row moved.** The row "Xberg PDF resource STOP, Excel/Tokio, Office/MIME fidelity, rich HTML, packaging and notices | Xberg lane; not part of this integration" is deleted from the deferred table. Those items are FP-2/FP-3/FP-4/FP-6. Plan section 6.3 states they are not deferred work.
4. **G9 maps the canonical intake journeys** (full, partial/unsupported, crash/cancel, no hidden source archive) through the combined app and desktop paths with named tests (for example `product::full_import_retains_assets_source_identity_and_rejects_changed_retry_input`, `import_coverage`, intake `qualification::deliberately_omitted_figure_and_unchecked_full_note_are_partial`, `malformed_budget_cancel_and_timeout_leave_no_transient_copy`, `cancellation_after_pdf_copy_removes_raw_input_before_return`). Names were checked against the INT-W and X2 trees by Git grep. The desktop conversion worker lifecycle cases were not located by name beyond the existing steering/intake test files, so the map says to locate them at the first run and report a missing desktop case as a gap.

Unverified: whether the desktop has a named test for each lifecycle point; that is a first-run finding, not claimed.

## B2 (P2): NF-1 ordering and witness

**Accepted and expanded** ([NF1-test-spec.md](../NF1-test-spec.md); plan section 3.4; [merge-simulation.md](../merge-simulation.md#nf-1)).

- **Order preserved** (checked against `73ea9ca7` `start_run` and the simulated merged `start_request`): `begin_run_start` busy check, a busy refusal returns before any take and does not consume a review; then `refresh`; then `take_mcp_review` before `prepare_thread_run`; then `attach` after prepare. Preparation creates the Run record, so a post-prepare refusal leaves a Failed Run with fence +1 (the existing `reviewed_snapshot_handoff_barrier` assertions).
- **`queued` is the queue-text match** (`queued.is_some()` in `start_run`), not `request.queue_claim.is_some()`; `retry_rewrite` passes `false`.
- **Real call-path cases** specified: A `retry_rewrite` with a pending review (worker none; records and receipts equal, so no Run; review cleared; saved RewriteNote request and unsent feedback preserved; wrapped "Rewrite request saved; AI has not started. … Retry rewrite uses these same saved inputs." notice); A2 a second retry progresses to the provider-selection stage; B1 mismatched `start_run`; B2 queued with `dispatch` both `Some` and `None`; C matching positive; D busy does not consume.
- **Matching positive is feasible** with an existing offline fixture: `starting_a_replacement_run_clears_the_old_paused_notice` runs `start_run` with provider Codex and a nonexistent `auth_file` and reaches `worker.is_some()`. The spec (as corrected in round 2) asserts the synchronous outcome only and keeps the existing direct positive as the attachment evidence, because the Run record has no attached field.
- **Existing MCP controls remain** (saved policy, context stamp snapshot, one prepare, queue lease, refusal, the two handoff-barrier tests, the transient/once/scope test). They call `take_mcp_review` and `attach` directly and do not reach `start_run` or `start_request`; the spec says so.
- No production fix is made. The compile error is a static finding, not an executed red.

Unverified: that `warn` composes `notice` exactly as the quoted wrapper; the spec asserts containment first and the full string once the implementer confirms it. Whether a worker can be placed for case D with an existing constructor is stated conditionally.

## B3 (P2): expected lock set and `preserve_order`

**Accepted, and the latest fact supersedes the review's early inference** ([plan section 5](../plan.md#5-lock-reconciliation-expected-identity-set-not-a-frozen-number); [inventory](../inventory.json) `lock_facts`).

- The review's "serde_json + toml with source1b replacements" and its "toml 1.1.6→1.1.8" label are replaced by the real lock strings and the **six** existing moves in the X2 HTML WIP lock: `serde_json` 1.0.151→1.0.154; `toml` 1.1.6+spec-1.1.0→1.1.8+spec-1.1.0; `serde_spanned` 1.1.1→1.1.2; `toml_datetime` 1.1.1+spec-1.1.0→1.1.2+spec-1.1.0; `toml_parser` 1.1.3+spec-1.1.0→1.1.5+spec-1.1.0; `toml_writer` 1.1.2+spec-1.1.0→1.1.3+spec-1.1.0. I verified from the WIP `Cargo.lock` (SHA-256 `8b7a8ac0…`, matching the principal disposition) and its metadata: 965→1,011, 52 added, 6 removed, zero same-version drift; `toml 0.8.23`, `serde_spanned 0.6.9`, `toml_datetime 0.6.11` coexist. The principal's approval of the four companions as a closure is cited, not reproduced as a source approval; the WIP has no pin.
- **No frozen 52.** The expected set is computed at integration time from the exact handoff lock, the common base and both parents. The plan states the three-way block facts I parsed from Git blobs and the WIP lock (INT-W touches two blocks; neither X2 delta nor the WIP touches them; `brn-writing-ui` lists plain `serde_json`, valid while one version exists; provisional merged count 966 + 52 − 6 = 1,012).
- **Conflict handling**: apply declared block deltas to the common base, no `cargo update`, no dedup. The single `cargo metadata --offline` fallback must produce a dependency-string-only diff; a new package, a version change or a checksum change is a STOP and cannot be masked as an edge rewrite.
- **`preserve_order` "likely helper-only" is withdrawn.** It is not established and the evidence points the other way: `rig-core-0.44.0/Cargo.toml` requests `serde_json` features `raw_value`, `preserve_order`, `float_roundtrip`; `rig-core` is a normal dependency of `brn-ai` and `rig` a dev dependency; the WIP's default-workspace metadata resolves `serde_json` features `[alloc, default, float_roundtrip, indexmap, preserve_order, raw_value, std]`; the base lock's `serde_json 1.0.151` block already lists `indexmap`. I also verified the review's narrower fact: no workspace member enables `brn-intake/helper` (all four dependents list `brn-intake` with no features). That fact does not imply helper-only. The WIP's own note says helper-feature unions are derived statically because its metadata run used default features; the isolated helper build newly gains `preserve_order` from Xberg and gif/bmp/webp decoders.
- **Distinction kept**: host version/hash compatibility (the `serde_json` patch move, live in every graph) versus helper feature delta (new in the helper graph). G1 plans `cargo tree -e features -i serde_json` per CI graph for a future heavy grant; neither side's evidence is inferred from the other.

Unverified: any emitted feature set, any compile, the final Xberg lock, whether further bounded changes appear in the all-document handoff.

## B4 (P2): carried failures

**Accepted; replaced by the principal gate disposition** (plan section 6.2; inventory `carried_failures_gate_policy`). Revision 1's "no failure outside the named carried items" together with the deferred table made a recurrence pass automatically. Now: historical SDK 474/4/1 and widget 314/1/1 stay disclosed, neither an automatic pass nor a blanket merge block absent recurrence; a new failure at the combined source needs its own raw diagnosis and an explicit principal disposition before PR-ready; a required-context recurrence, including pass-on-rerun, needs `ci-summary.py compare-logs` plus a disposition, and the exact-head required contexts must pass; with publication authority checked at that stage a PR may be prepared and opened ready with the historical qualifications disclosed, and merge requires the required contexts passing and a disposition for any recurrence; the PDF local carry requires an actual hosted target-head pass with no silent skip; N1 stays separate and owner PR/merge intent is not a native-verified claim. This is gate policy only. It does not grant a push, PR or merge.

## Advisories

| # | Advisory | Disposition |
|---|---|---|
| 1 | G1 "zero GPUI" | Accepted: scoped to default-feature `--workspace` graphs; native graphs contain GPUI by design (plan G1). |
| 2 | Maintained tooling | Accepted: `scripts/development-preflight.py --native` before the first heavy run; `BRN_VERIFY_OUTPUT_DIR` for `record.json`; `scripts/ci-summary.py run` / `compare-logs`. I read the script headers, the `APPLICABLE` context list (identical to the four required contexts), and the `BRN_VERIFY_OUTPUT_DIR` handling; I did not run them. |
| 3 | Order: origin/main merge | Accepted: the last `origin/main` merge precedes the freeze, or the delta is re-reviewed; the pushed head tree must equal the frozen reviewed tree (plan sections 7 and 8). |
| 4 | Network | Accepted: the first `git fetch origin` needs an explicit network grant in the FI allocation; GitHub rules are read at publication. |
| 5 | NF-6 | Accepted: conditional. The 82 is immutable history, not a target. The X2 worker's uncommitted candidate reports 82→0 with the same unchanged checker (357 files, 1,611 links); that is a WIP receipt, not a pin and not a root-wide result. The X2 worker owns those docs; I do not edit them and do not rewrite the cd2-bound ledger hashes. Re-run on the pin. |
| 6 | Optional INT-W × D step | Accepted and renamed **TCI-1**; needs an exact grant and a free heavy slot; not requested now. |
| 7 | G7 F21 channels | Accepted: the RewriteNote run through D1A/D1B/D2 plus skills and MCP; `threads_budget_tests::reserved_roles_and_private_other_chat_never_leak_through_history_query_or_exact_passages`; `cw2_review::int_w_f21_private_context_never_enters_p2_or_general_delivery`; the `export_metadata` eligibility tests (for example `source_links_without_import_coverage_are_exported`). The test names were checked by Git grep. |
| 8 | N1 vs merge | Accepted: plan section 6.2 item 6. |

## Also done

- [status-draft.md](../status-draft.md): the NF-3 replacement snapshot, not applied (`docs/status.md` is read only).
- Revision 1 sections rewritten as described: the Xberg row, G9, NF-4, NF-6, section 4.2/4.3 policy and the section 7/8 order. The plan is regenerated whole, so use `git diff d4574398 HEAD -- docs/work/active/final-integration/plan.md` for the exact delta.

## Limits of this response

Static Git, text and hash reads and read-only Python parses of text/JSON. No Cargo, build, test, lint, formatter, Store, CLI, auth, provider, network, fetch, GUI, mutation or live command; no heavy slot; nothing imported from the X2 HTML WIP; no new worktree and no delegation. Not read in full: `threads-target.md` and `threads-behavior.md` beyond the journey table, full-note section and amendments; long evidence logs; the uncommitted X2 source files.
