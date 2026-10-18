**Verdict: CHANGES_REQUIRED** for the source-only final integration plan at `ae63fb7d` (tree `5f877d63`). There are two narrow, docs-only must-fixes; everything else in B1–B4 and the advisories now holds against source. This is not source approval, verification or a merge.

## Candidate identity

- **Branch and tree:** `work/final-integration-20261017`, clean. HEAD `ae63fb7d5110327c8aecad5684e65e0573267fe4`, tree `5f877d636499f613af3cdd74c5cf23a2e75016c1`.
- **Change since the prior plan:** `d4574398..ae63fb7d` touches 11 files, all under `docs/work/active/final-integration/`. No code changed.
- **Merge simulation:** `deb060ee` exists as an unreferenced tree. I read it and did not apply or compile it.

## Checked against source

**B1, now FP-1 to FP-7: holds, with one authority gap (M1).**
- FP-1, FP-2, FP-4 and FP-5 match the X2 sources at `781c34dc`:
  - the eight-kind owner scope in `owner-scope-all-documents.md`;
  - the `acceptance.md` row 24 "No old backend" (all three Poppler routes, including `pdftoppm`);
  - the extra-family table in `formats.md` (lines 47–67);
  - `plan.md:14` ("No component-level success declares all-document adoption") and the resource STOP at lines 46–52.
- The first HTML helper is named a component. TCI-1 and TCI-2 are technical steps, not the final PR, and no grant or build is requested.
- The withdrawn Xberg "deferred" row is really gone; the 6.3 table says so explicitly.

**B2, NF-1: the diagnosis and ordering are correct; one witness case is unsound (M2).**
- In `deb060ee`, `start_run` calls `begin_run_start` (busy refusal before any take, then `refresh`), then the queue-text match, then `take_mcp_review(..., queued.is_some())`, then `start_request`.
- `start_request` uses `reviewed.attach(prepared)` with no binding for `reviewed`. That is the static E0425.
- `take_mcp_review` (`threads_mcp.rs:361`) refuses on `queued` alone, so a queue-text match is the right argument, not `queue_claim`.
- `retry_rewrite` → `begin_run_start` → `start_request`, and the busy-path wrapper notice matches the spec.
- The existing direct MCP tests never reach `start_run` or `start_request`. The spec says so.

**B3, lock: holds.** I re-parsed the lock blobs read-only:
- Package counts 952 / 953 / 965 / 1,011, with the WIP lock still at SHA `8b7a8ac0`.
- INT-W touches only `brn-desktop` and `brn-writing-ui`. Neither X2 delta nor the WIP touches either block.
- The WIP changes 11 blocks, none of them INT-W's. The WIP has a single `serde_json 1.0.154`.
- The six moves match the disposition JSON. The coexisting old versions are kept.
- The expected set is computed from the handoff, and 1,012 is labelled provisional.
- `preserve_order` is described correctly: already live via `rig-core` in the default workspace graph, kept separate from the helper feature delta, with per-lane `cargo tree` planned for later.

**B4, carried failures: holds.** Section 6.2 matches the principal gate policy in `final-integration-kiro-opus1-corrections-20261018.md`. The CI job names, the `ci-summary.py compare-logs` command and `BRN_VERIFY_OUTPUT_DIR` exist as the plan states.

**Advisories 1–8 from round 1:** all applied (section 1, G1, the section 4 header, sections 3.5, 6.2 and 7–8).

## Must-fix

**M1 (from B1): FP-4 lets the principal change the owner's objective.**
- **Where:** plan FP-4, "If the principal or owner instead decides to keep Poppler, that is a cited disposition that changes the objective". Mirrored in the `inventory.json` FP-4 entry.
- **Why it's wrong:** the owner scope makes Xberg the shared backend. It says "preserve all currently supported behavior" and "do not quietly drop the format, lower quality or retain an undisclosed fallback", and anything that can't be met is an "explicit consequential decision". Under AGENTS.md that choice belongs to the owner.
- **Fix:**
  1. Keeping Poppler requires a cited **owner** decision.
  2. Add one rule: any FP-2/FP-3 outcome that narrows behavior a current kind supports today needs a cited owner decision. That covers a pre-operation PDF profile that refuses inputs the current route accepts, lower fidelity, or a dropped kind.
  3. The FP-3 profile disposition must state whether it narrows current PDF acceptance.
  4. Principal dispositions remain enough for outcomes that preserve behavior, and for FP-5 extra-family admit, defer or refuse decisions with reasons (`formats.md:67`).

**M2 (from B2): case C in `NF1-test-spec.md` needs evidence that isn't timing-dependent.**
- **The problem:** case C requires "exactly one new Run, not Failed" and "assert on the new Run's MCP-attached state".
- **Why neither works:**
  - `spawn_prepared` starts a background thread. Under the nonexistent-`auth_file` fixture that run can legitimately fail and record Failed at any time after spawn, so "not Failed" depends on timing.
  - A successful attach (`attach_external_input`) only sets the in-memory `scope.external`. It writes no record and leaves the fence alone, so the Run fields never show the attachment.
- **Fix:** use the synchronous outcome of `start_run`, which `start_request`'s control flow makes conclusive:
  - a take refusal returns before prepare, with no Run;
  - an attach refusal fails before spawn, with `worker == None` and the "MCP input authorization refused" notice.
- **So case C should assert, immediately after `start_run` returns:**
  - `worker.is_some()`;
  - neither refusal message in the notice;
  - `mcp_view.pending` is `None`;
  - exactly one new Run record, whose id equals the worker's run id;
  - no assertion on the terminal state.
- Keep the existing direct take/prepare/attach positive as the evidence for attachment content.

## Advisories (not blocking)

1. **NF-1 spec wording.** "Its generated prompt can never equal a pending composer prompt" is wrong. INT-W's `rewrite_run_start` uses a fixed string, identical to the one X2's `retry_rewrite` passed. An exact-match review would attach, as it did in X2, so behavior is preserved; say that instead. Also state that `false` differs from X2 only if a queue item's text equals that exact string. In case B2, if a matched item with `dispatch == None` can't be built through existing paths, record that limit.
2. **Cite the policy file.** Name `final-integration-kiro-opus1-corrections-20261018.md` in plan sections 1 and 6.2. Today only `binding.json` names it.
3. **Lock STOP rule wording.** Reword "Any other existing-package move is a STOP" to "not declared by the audited handoff and approved". Known future candidates: `tokio 1.53.1` (Excel needs at least 1.53.2), `mail-parser 0.11.8` (email needs ^0.11.9), and distinct `zip`/`quick-xml` versions. All are still at the old versions in the WIP.
4. **PDF carry at FI.** FP-4 retires Poppler, so the `PDFUnavailable` test, the Homebrew Poppler step in CI and `check-intake-retirement.py` all change in the Xberg lane. Section 6.2 item 5 and G2's "cannot finish locally" apply as written only to TCI; re-derive both at FI.
5. **TCI publication.** The owner sequenced final integration after Xberg. If a TCI is ever proposed for `main`, record whose authority covers that sequencing change.

## What can proceed

- **Now (source-only):** the M1 and M2 docs edits in this branch. X2's work continues in its own checkout. No build, merge or grant.
- **Optional, before Xberg:** TCI-1, only with an exact grant once the heavy slot is free. It is not final and not a default PR candidate.
- **After the all-document X2:** final integration per section 8, once these hold:
  1. FP-1 to FP-7 recorded, with owner decisions for any narrowing (M1);
  2. a reviewed, pinned handoff;
  3. the checksum-ledger audit;
  4. NF-6 re-run on the pin;
  5. heavy slot released;
  6. an exact grant that includes network for the first fetch.

## Limits on qualification, owner and publication

- **Historical failures:** SDK 474/4/1, widget 314/1/1, PDFUnavailable and the hosted native failure stay disclosed. None is an automatic pass. A recurrence needs a diagnosis and a principal disposition before the PR is ready.
- **PR versus merge:** a ready PR is allowed only once publication authority is checked at that stage. Merge needs the four required contexts passing at the exact head.
- **Separate gates:** N1/F20 native observation, owner acceptance, release, real owner data and any new live window.
- **Draft #149** stays do-not-merge.
- **No grants here:** none of this grants a push, PR, merge, fetch or heavy run.

**What I ran and didn't read:** read-only Git reads of the candidate, `73ea9ca7`, `781c34dc`, `d2d84e66` and `deb060ee`; the brief, owner, correction and disposition records; and one read-only Python parse of the lock blobs plus the WIP `Cargo.lock`. No Cargo, build, test, lint, link check, network or GUI, and no edits or child agents. I didn't re-read `threads-target.md`, `threads-behavior.md` or `invariants.md` in full; round 1 covered them and the corrections don't depend on them. I didn't open the uncommitted X2 source files.
