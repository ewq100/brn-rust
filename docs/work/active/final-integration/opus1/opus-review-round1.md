**Verdict: CHANGES_REQUIRED** for the source-only final integration plan at `d4574398` (tree `d08b12cc`, parent `73ea9ca7`, five new docs only).

The fixes are narrow and docs-only. The binding, topology, divergence counts, lock union, NF-1 discovery and failure records mostly check out against Git. The main problem is how the plan treats the Xberg input: it doesn't meet the owner's all-document requirement. This is not source acceptance, product verification or a merge.

## What I confirmed in Git (read-only)

- **Refs:** INT-W `d2d84e66` and X2 `73ea9ca7` share merge base `b617d784` (31 and 77 commits beyond it). X2 `781c34dc` is one docs commit past `73ea9ca7` and changes no crates, lock or manifest. `aea4830f`, `8f768943` and `9b47b285` are all ancestors of CW2.
- **Lock, re-parsed from Git blobs:** 952 / 953 / 965 / 966 packages; the simulated lock equals the union of INT-W and X2; no checksum drift from `8f768943`; SHA `72f9ced9…` matches. The only new duplicate names are the three darling crates. This is still not a Cargo resolution or emitted-unit proof.
- **NF-1 is real.** In the simulated tree, `start_request` (line 1949) uses `reviewed` at line 1983, but `reviewed` is a local of `start_run` (line 1914). Only `start_run` (line 1939) and `retry_rewrite` (line 1860) call `start_request`.
- **The proposed direction preserves D4.** In X2, `retry_rewrite` → `start_run` → `take_mcp_review(...)` ran with the generated writing prompt, so a pending review was consumed and refused with "No new run was started". Moving the take into the shared start keeps that.
- **Static checks I re-ran that found no break:**
  - Every `Run` literal and `WorkspaceSettings` site in INT-W-touched code has the new fields or uses a constructor.
  - `ThreadRunStart` is unchanged, and `brn-writing-ui::rewrite_run_start` sets all 9 fields.
  - The X2 skill and MCP detail children sit in the chat-conversation render that INT-W kept.
  - INT-W's export-eligibility filter keeps X2's `export_metadata` fixture, whose Source has `chat: None`.
- **CI:** the job names and commands match the plan. The four required contexts match verification.md and tooling.md; GitHub rules are still unread.

## Blockers

**B1 (P1): The Xberg precondition lets a component stand in for the all-document program.**
- **Where:** plan §3 tail, §4.3 row "Xberg PDF resource STOP, Excel/Tokio runtime, Office/MIME fidelity, rich HTML, packaging and notices | Xberg lane; not part of this integration unless its handoff includes them", G9 "after Xberg: its HTML-slice tests", and §7 "Xberg handoff committed with an immutable pin".
- **What it conflicts with:**
  - Owner amendment: "after the Xberg **all-document** candidate is ready".
  - `owner-scope-all-documents.md`: all eight kinds plus explicit extra families.
  - X2 `plan.md:14`: "No component-level success declares all-document adoption."
  - `first-html-slice.md` and the allocation: "not … all-doc done".
- **Result as written:** the frozen first-HTML helper pin could be merged as "final integration", and every remaining family gets classed as deferred and non-blocking.
- **Correction:**
  1. Define the final precondition as an all-document candidate that meets `x2/acceptance.md` for every current kind. That covers checked host, no old backend (all three Poppler routes), graph/package/platform, Luna and lifecycle, plus explicit dispositions for extra families. Any residual family needs a cited owner/principal disposition, not a lane default.
  2. If the principal wants an earlier limited integration, give it an honest name, such as "technical component integration (INT-W + D [+ first HTML protocol capability])". It must not be the final program PR, and status must keep the all-document objective open.
  3. Move the §4.3 Xberg row from "deferred, non-blocking" to "final-program precondition".
  4. Make G9 re-execute the combined-graph-dependent acceptance rows: host save through `brn-threads-app` and intake through `brn-desktop`, both of which this merge changes. Map the canonical intake journeys (full import, partial/unsupported, crash during intake, no hidden source archive).

**B2 (P2): The NF-1 fix spec and witness leave key invariants implicit.**
- **The take must happen after `begin_run_start` and before `prepare_thread_run`.** Preparation creates the Run record: `reviewed_snapshot_handoff_barrier` shows a refusal after prepare leaves a Failed Run with fence +1.
- **Keep the existing ordering and predicate:** busy refusal leaves the review pending, then refresh, then take. `take_mcp_review`'s predicate and message stay unchanged, and `attach` stays after prepare.
- **The `queued` argument must be the queue-text match (`queued.is_some()`), not `request.queue_claim.is_some()`.** `dispatch` can be `None` for a matched queue item.
- **The current witness doesn't reach the moved code.** Existing MCP widget tests call `take_mcp_review` and `attach` directly; nothing exercises `start_run` or `start_request`. Proportionate cases:
  - (a) `retry_rewrite` with a pending review: refused before prepare, no new Run, the RewriteNote request and feedback preserved, review cleared, and the wrapped "Rewrite request saved; AI has not started. MCP input review…" notice. A second retry with no review then reaches prepare, so the refusal is not a permanent block.
  - (b) `start_run` with a mismatched or queued pending review: refused with no Run record.
  - (c) A matching chat start attaching through `start_request`, if the offline fixture allows it; otherwise state that limit.
- The compile error stays a static finding, not a behavioral red.

**B3 (P2): The post-Xberg lock and feature graph lacks an exact expected set and a check of where `preserve_order` is actually compiled in.**
- **Expected set:** (INT-W ∪ X) minus {`serde_json 1.0.151`, `toml 1.1.6+spec-1.1.0`} plus their exact source1b replacements, with every edge repointed. The plan's "toml 1.1.6→1.1.8" label should use the real lock version string; `toml 0.8.23` also coexists.
- **Conflict handling:** say how a `Cargo.lock` textual conflict is resolved (union plus audit, no `cargo update`). Treat the single allowed `cargo metadata --offline` rewrite as an edge-only diff.
- **`preserve_order` liveness:** no workspace member enables `brn-intake/helper`. So `preserve_order` is likely live only in `-p brn-intake --features helper` builds, not in workspace, native, retrieval or capability host graphs. G1 should record `cargo tree -e features -i serde_json` per CI lane. G9 should run hash/replay/backup compatibility in each graph where the move is actually live. Host-side evidence covers the version move only unless the feature check shows otherwise.

**B4 (P2): Carried failures get an automatic non-blocking classification no disposition supports.**
- **Where:** §4.2(2) "no failure outside the named carried items" together with §4.3 means a recurrence of SDK 4/1 or widget 1/1 in the combined run passes automatically.
- **What the record actually says:** the D4 disposition says these are not a *source-approval* blocker. It says nothing about the combined candidate or merge.
- **Correction:** a recurrence is a new failure at its own source. Capture the diagnostic class, preserve it, and get an explicit principal disposition before PR-ready.
- **PDFUnavailable:** the local carry is fine per the INT-W and INT-D dispositions, but only because required hosted `Core and CLI (macos-15)` runs that test with Poppler and must pass at the exact head.
- **Hosted retries:** a required context that fails and then passes on rerun at the same head needs a principal disposition backed by `ci-summary.py compare-logs`, not an automatic pass.
- **The one disposition needed:** "the PR may be opened ready, not merged, with these historical qualifications disclosed; merge needs the required contexts passing and a disposition for any recurrence."

## Advisory

1. **G1 wording:** "zero GPUI" applies to the default-feature `--workspace` graph only (`default-members = brn`). Native graphs include GPUI by design.
2. **Use the maintained tooling:**
   - `scripts/development-preflight.py --native` before the first heavy run.
   - `BRN_VERIFY_OUTPUT_DIR` for the `record.json` evidence from the verify scripts.
   - `scripts/ci-summary.py run` and `compare-logs` for hosted runs.
3. **§6 order:** the origin/main merge comes after the Opus review. Freeze after the last origin/main merge, or re-review the delta. The pushed head tree must equal the frozen, reviewed tree.
4. **Network:** §7's `git fetch origin` is the first network use. It needs explicit inclusion in the integration grant; GitHub rules are read at publication.
5. **NF-6:** the X2 worker owns the fix, and §7's writable list excludes the `x2/` docs. Make it conditional: re-run the checker on X, and send any remaining failures back to that lane. Keep the 82 count as history, not a target.
6. **Optional earlier step:** an INT-W × D compile-discovery run once the heavy slot is free. It needs an exact grant and should be labeled technical, not final.
7. **G7:** name the combined F21 channels explicitly: the RewriteNote run through D1A/D1B/D2 plus skills and MCP; `threads_budget_tests::reserved_roles_and_private_other_chat_never_leak…`; INT-W's `cw2_review` F21; and the export eligibility tests next to `export_metadata`.
8. **N1:** state that merge may come before N1 only under the existing owner PR/merge request, with no native-verified claim before N1.

## What can proceed, and when

- **Source-only now:** correct these five docs (B1–B4 plus advisories), draft the NF-3 status reconciliation, and write up the NF-1 test spec. No production edit and no build.
- **After the heavy slot is released, without Xberg:** only the optional technical INT-W × D discovery, under an exact grant.
- **After the all-document Xberg handoff, its independent review, the checksum-ledger audit and a heavy grant:** the §7 merges, NF-1, then G0–G9 serially, freeze, and a fresh Opus whole-candidate review.

## Separate owner and principal gates

- All-document completion, or a disposition for each residual family.
- Disposition of carried or recurring failures (B4).
- Publication and merge authority confirmed at that stage, with GitHub rules read.
- N1/F20 native observation.
- Any new live window under L.
- Owner acceptance, release, real owner data.
- Draft #149 stays do-not-merge.

Limits: static Git and text reads plus one in-memory Python lock parse. No edits, Cargo, build, test, network, `merge-tree` run, or application or compile of `deb060ee`. I didn't read the long evidence logs, and I didn't re-run the link checker on `781c34dc`.
