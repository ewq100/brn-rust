**Principal INT-W local disposition, 17 October:** source0efd73f1 has [comprehensive Opus approval](int-w/opus-review-round1.md) and [local deterministic acceptance](int-w/principal-disposition.md). Both confirmed privacy fixes and three actual seeded catches are reconciled. The full milestone remains qualified by PDF tooling, hosted candidate lanes and native/owner gates. Heavy and all paths returned; D-S/D-M still block D3/D4 and INT-D. No further eligible implementation is assigned. This supersedes dated checkpoints below.

**Current INT-W, 17 October:** compiled local qualification on `work/int-w-20261017` from accepted B4 `e8e98304` is ready for principal comprehensive review. [Plan/ownership](int-w/ownership.md), [F01–F22 map](int-w/f-cases.md), [actual checks](int-w/verification.md) and [section17 dispositions](int-w/program-status.md) supersede dated “next slice” checkpoints below. Two appended fixtures exposed two retained-Source delivery defects; principal allocated only canonical visibility in app export::publish and desktop note_rows. Corrected-source normal/native/combined checks pass except the carried PDF tooling failure. No schema/interface/manifest/lock/CI change; hosted/native-human/owner/full-program acceptance remains open. No D-lane integration/publication.

**Principal B4 local disposition, 17 October:** source3515f96a has [Opus3 approval](b4/opus-review-round3.md) and [local contract acceptance](b4/principal-disposition.md). B3/B4 dependencies permit INT-W on a local descendant branch, with full F21 and comprehensive combined review still required. Heavy and all B4 paths are returned. D-S/D-M await explicit owner decisions; no D3/D4 start. Hosted/native-human/owner/PDF gates remain open. This supersedes dated current-slice checkpoints below.

# Combined C0 implementation plan

**Principal disposition, 17 October:** B3 source93b92d7b has Opus3 approval and [local contract acceptance](b3/principal-disposition.md). B4 may prepare the real-Workspace development host source-only; D2 owns the sole heavy slot. INT-W, full F21, hosted/native-human/owner and carried PDF qualifications remain separate. This supersedes the dated current-slice checkpoint below.

**Current slice:** B1b is source-reviewed and accepted for local dependent development after clean final handoff `d9aab018`. [Principal disposition](b1b/principal-disposition.md) records exact normal checks, the bounded contextual-reader recheck and outstanding qualification gates. B3 is next in the writing lane; D1B proceeds separately. No combined-lane integration, hosted/native-human/owner acceptance or main merge is implied.

**C0 goal:** establish a reviewable baseline for modular foundation plus agent-capabilities Slice 0, without production edits.
**Architecture:** keep the checked core/app services and Rig runtime. C0 documents their seams and tests upstream reuse in an independent experiment.
**Stack:** repository Rust 1.98.1 and root lockfile; Rig 0.44.0, schema 6, CLI contract 3.
**Spec:** [build ordering](../build-sequence/README.md), [master plan](proposal/final/01-master-plan.md), [bootstrap brief](proposal/final/02-foundation-lead-prompt.md). These packs stay provenance; ordering and current owner instructions win over historical launch text.

## Baseline and authority

Baseline `8f768943964a77711b26d5235df146f8efe1dc6c`; checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-ceb6e16c`, branch `work/build-c0-20261016`. Integration lead is thread `2f893fce-357e-4754-9842-468b65c24d66`; this implementation worker is the sole C0 writer. [Evidence](evidence.md) owns verification, inventory and exact integration receipts. The [capabilities plan](../agent-capabilities/plan.md) consumes this baseline/ownership.

Stage 0 release condition **landed**: `81129f07` UI polish, `192cd765` reliability and `36f2c733` CLI are ancestors; #155/#156/#157 are on baseline. Existing worktrees and uncertain branch activity are preserved; existence alone does not allocate a writer or build slot. Parent `.vscode/`, `experiments/` and staged writing research remain untouched. Only one heavy build runs at a time.

## Ownership by path

Current write authority is only the C0 row. **Principal** below names integration lead thread `2f893fce-357e-4754-9842-468b65c24d66`; it holds custody of every other path until it explicitly allocates a writer for a named slice. Future slice reservations are not concurrent write grants. Each path has one primary owner: specific rows take precedence over the final catch-all; manifests remain principal-owned even inside a reserved crate. Other worktrees retain their own owners and are outside this checkout's allocation.

| Paths | Current primary owner | Reserved slice / handoff rule |
|---|---|---|
| `docs/work/active/modular-foundation/{plan.md,evidence.md,c0/**}`, `docs/work/active/agent-capabilities/{plan.md,evidence.md}`, `docs/status.md` minimal links/corrections, `experiments/r0-rig-reuse/**` | C0 worker | Local C0 corrections only |
| `crates/brn-threads-core/src/{writing.rs,store.rs,types.rs,lib.rs,context.rs,converge.rs,stage.rs,edit.rs,backup.rs,restore.rs,upgrade.rs}`, `crates/brn-threads-core/tests/**` | Principal | C/CW1 schema/setup/request v2, backup/restore/upgrade; C/CW2 binding, visibility, compatibility |
| `crates/brn-threads-app/src/{writing.rs,review.rs,lib.rs}`, `crates/brn-threads-app/tests/**` | Principal | C checked operations/shared active context; B consumes Workspace |
| `crates/brn-ai/src/{threads.rs,threads_reads.rs,threads_guides.rs,threads_trace.rs,threads_research.rs}`, `crates/brn-ai/src/*_tests.rs` | Principal | C/CW2 shared start before serial D1A/D1B/D2; D3/D4 gated |
| `crates/brn/src/{main.rs,spec.rs,contract.rs,help.rs}`, `crates/brn/tests/**` | Principal | CLI writing `ask`/`--edit-note` (`main.rs::ask`), exact `apply`/`dismiss`, edit/save/Undo routes and contract compatibility; C submits any CW/D adapter patch to principal |
| `Cargo.toml`, `Cargo.lock`, `crates/*/Cargo.toml`, `.github/**`, `docs/architecture/**`, `docs/development/**`, `scripts/**`, `release/**` | Principal | Shared dependencies/CI/canonical docs/verification and integration; no co-editing |
| `crates/brn-desktop/src/threads_ui.rs` | Principal | Shared start/active-context for CW2; B1a/B1b submit bounded extraction hunks for serialized handoff, never a second writer |
| `crates/brn-desktop/src/{threads_writing.rs,threads_writing_tests.rs,threads_workbench_tests.rs,threads_format.rs,threads_layout.rs,threads_document.rs,threads_workbench.rs}`, future `crates/brn-writing-ui/**` excluding its manifest | Principal | B reservation only after exact allocation and writing-widget failure disposition before moving implicated `threads_writing_tests.rs` |
| `crates/brn-intake/**`, `crates/brn-threads-intake/**`, `crates/brn-threads-app/src/intake.rs` excluding manifests | Principal | Retain production; A integration only after X approval and exact allocation |
| `crates/brn-retrieval/**` excluding manifest, `crates/brn-threads-app/src/search.rs` | Principal | C correction only for demonstrated defect; ranking/model work deferred |
| `agent/BRN.md`, `agent/skills/**` | Principal | C/D slice only after its gate and explicit allocation |
| **All other paths in this checkout**, including unlisted core/app/AI/desktop/CLI modules, fixtures and docs, and prior experiment records | **Principal; no writer without allocation** | Preserve existing work/provenance; catch-all grants no implementation authority |

The [route trace](c0/routes.md) supplies current exact symbols and gaps. CW2 extends `brn_ai::threads::prepare_run(ThreadRunRequest) -> AiResult<PreparedThreadRun>` then consumes it through `run_prepared`; it must not extract a competing start service. B consumes notes, not `ConversionResult`.

## Frozen section 7.2 contract

Contract revision **C0-20261016-1**, semantics owned by [master §7.2](proposal/final/01-master-plan.md#72-writing-context-and-versioned-request); no implementation or canonical contract change in C0.

- Writing context is `(working Note, host-owned scope)` using current `context::visible`: general, one project or private conversation. Setup is a separate small scoped record: optional brief and ordered reference Note IDs with requirements/evidence/style purposes. Owner-only checked edits, replay, Undo and backup/restore; edits never bump Note version or auto-share across contexts.
- Shared active-context resolution selects opening project/private scope, otherwise the Note's own conversation scope, otherwise general. In that scope use the last explicitly used writing conversation for that Note, or create one on first comment/rewrite. Display scope/conversation with Chat closed; no arbitrary linked-thread fallback.
- `WritingSnapshot` envelope v2 captures scope, brief and visible reference IDs/versions/purposes, never reference content; retain 128 KiB bound. Preserve v1 requests/candidates/receipts/replay/restart and original v1 input binding.
- Working Note, comments, linked messages, request marker and setup edits bind `Current`: stale needs new request. Ordinary reference edits bind exact `Historical` version and remain applicable with “changed since”. Exhaustive checks or “must be current” references bind `Current`: stale needs Refresh. Refresh captures current scope/versions, regenerates and requires new exact review; Retry reuses request identity and revalidates.
- CW2 must test-and-fix owner-review Apply visibility for **every input**, historical too, with the same predicate. Access loss refuses Apply; earlier approval and automatic retry confer nothing. Restore access then explicit exact review, or Refresh. Private evidence into a wider Note needs named disclosure and exact owner review, retaining private provenance.
- Persist existing `ReadLedger` entries (ID/version/covered retained bytes/complete) and host-verified quotations on candidates. Present separately: import coverage (inventory-complete/partial/unchecked), retained text read (full/partial X of Y bytes/not read), requirements **reported** checked with verified quotes. Exhaustive is complete over retained text only if every requirements reference version was fully read; never “all requirements verified”.
- Preserve explicit Save-and-rewrite; unsaved-snapshot requests remain outside scope. Comment visibility currently follows the Note, including shared Notes; whether that matches intended private-context feedback remains an open consequential product question for the lead before changing it.

## Scenario applicability and review focus

[Master §13](proposal/final/01-master-plan.md#13-shared-acceptance-suite) owns exact F01–F22 scenarios. This mapping is scope, not a claim that new behavior exists.

| IDs | Slice / evidence |
|---|---|
| F01–F03 import, replay, failure/Stop | Existing baseline regressions; INT repeats; A repeats only for approved cutover |
| F04 scoped persistence/closed Chat; F06 multi-feedback; F07 targeted rewrite; F09 dirty typing; F12 exact Apply/replay/Undo; F16 crash/restart | CW plus B, scripted deterministic; native subset remains open |
| F05 exact repeated/Unicode selection; F15 editor Undo | B on current editor; Z1 report remains negative |
| F08 exhaustive coverage; F10 edit/revoke/Refresh; F11 injection; F14 scoped tools; F21 shared Note in two scopes | CW tests and B display, deterministic; new setup/reference semantics pending |
| F13 index/embedding failure | Existing baseline; evaluation only if retrieval changes |
| F17 backup without originals; F19 unrelated behavior | C regression gates at integration |
| F18 real shared-service host independent of desktop | B4 after CW2, scripted real Workspace plus native launch |
| F20 complete native workflow/IME | Owner/native gate, unavailable and unauthorized in C0 |
| F22 default build excludes GPUI | Every integration: `cargo build/clippy/test --workspace` default must compile **no GPUI**. `brn-writing-ui` uses optional `gpui-kit` under `native-ui`, depends on `brn-threads-app`, never `brn-desktop`. No extra headless writing crate or host-operation trait. Explicit native lane covers new UI. |

Review focuses: cross-scope metadata leakage (F21), owner Apply after revocation (F10), v1 candidates across schema/request changes (F10/F16/F17), stale buffers after Apply/Undo (F09/F15), and false exhaustive claims (F08). Implementers add failing-first witnesses locally and land them passing in their owning slices.

## Sequence and gates

O1 **CW-first authorized by current owner**. After C0 and any separately approved CL: CW1 → CW2 → D1A → D1B → D2; B1a can overlap CW1 only after precise allocation and principal disposition of the implicated writing-widget file, B1b follows CW2. D3/D4 cannot start until D-S/D-M respectively. PC deferred (T unresolved); retain Poppler (X); no live-provider qualification (L). Native observation, real data, release, publication and merge remain outside this worker's authority.

- [x] Inspect actual baseline, preserved work, pack contracts and routes.
- [x] Install the linked active plans and baseline/ownership map.
- [x] Finish serial baseline commands and per-crate/feature/time inventory; record failures honestly.
- [x] Finish executable Slice 0 reuse proofs and decision evidence.
- [x] Commit integration ref without new required-test failures/skips; independent actual-diff review remains with lead (see evidence receipt).
- [x] Principal accepted C0 exit at reviewed candidate `adfab8793c98ff4e3ce9d143dd8894662f190cb8`; [independent approval](c0/opus-review-round2.md) and [CL no-op disposition](evidence.md#principal-exit-and-cl-disposition) recorded.

CL deletion candidates: **empty**. Examined and rejected: `brn-threads-intake/src/pdf_tools.rs::PdfTools::discover` is a used production Poppler path and X is unapproved; `brn-ai/src/threads.rs::begin`'s `.take(40)` history assembly is still active and belongs to D1A replacement, not removal; `ThreadsUi::open_note`'s linked-thread fallback (`threads_ui.rs:1307–1324`) and duplicated host start need CW2/B1b replacement, not CL deletion. No cleanup is authorized by this empty list. Any future concrete destructive list needs explicit owner approval before deletion. See [start messages](c0/start-messages.md).

Baseline PDF/hosted-widget failures are disclosures under master §6, **not C0 exit prerequisites**. Principal owns their carry-forward/disposition. After C0 review, an empty CL can be recorded as no-op without claiming four green lanes; any later change/green-baseline claim needs explicit failure disposition. CW1 may proceed after C0 with failures retained. B1a waits before moving `threads_writing_tests.rs` for the principal's bounded follow-on diagnostic disposition. [Correction record](c0/review-corrections.md) tracks Opus round 1 and the diagnostic evidence; no review approval or later-slice start is implied.

**Principal disposition, 16 October 2026:** C0 completed within its baseline disclosures. CL completed as a removal-only **no-op**, zero deleted production lines/tests/dependencies; no four-green or baseline-repair claim. CW1 is the next assigned production slice. B1a remains held before implicated UI files while a separate bounded diagnostic/repair establishes the widget-failure disposition. CW1 owns core/app writing; the UI prerequisite owns only explicitly allocated desktop confirmation/test paths, with one heavy build at a time.

## CW1 implementation allocation and steps

16 October 2026: sole CW1 worker at `/Users/evokessler/.t3/worktrees/brn-rust/t3-cf730cbc`, new branch `work/cw1-20261016`, accepted base `c8b3026f582c8770c24a984111f562becf529cad`. Principal grants core writing/setup/schema/upgrade/backup/restore/store/context/types/exports and relevant tests, app writing/config/replay/exports/tests, minimal AI/CLI compatibility, affected canonical summaries. Desktop allocation is only necessary `WritingSetup` label/order arms in `threads_ui.rs::record_label`, `threads_lists.rs`, `threads_projects.rs::kind_order`, `threads_writing.rs`; no widget/confirmation/test changes. Root manifests/lock/CI remain principal-owned. One serial heavy build slot; C0 target reused with Rust 1.98.1, locked/offline, no competing builds observed.

1. Add failing-before v2/schema witnesses, then a separate `WritingSetup` checked configuration record with canonical `(note, scope)` identity, exact scope filtering, versioned owner mutations/replay/Undo and one transactional schema-7 index upgrade after external verified backup.
2. Extend snapshot with omitted-by-default v2 metadata (scope, setup Current input/version, brief, visible reference IDs/exact versions/purposes); preserve original v1 encoding and bindings. Capture defaults atomically with the request, refuse overflow, expose unavailable references only in the scoped owner setup view. Preserve comment behavior.
3. Exercise F04/F10/F16/F17/F21 through shared services: separate contexts, no Note mutation, conflicts/replay/Undo/reopen, access loss, v1 persisted prepares/candidates/receipts and backup/refusal/interruption/restore. Check AI read/proposal guards, search/export and required compile glue.
4. Run focused tests, fmt/strict lint/default and affected native lanes with baseline failures retained; bounded installed cargo-mutants on new scope/version/replay/backup/limit guards. Record exact commands, counts, limitations, changed files/signatures and local commits. Return for principal-owned independent Opus review; CW2 and integration remain pending.

CW1 handoff: local implementation/corrections are committed through source/test pin `521454f2`, with independent source approvals at 690fb705 and 521454f2. Normal and focused checks, baseline failures, partial mutation classifications and all acceptance/provenance limits are in the existing evidence. The heavy slot is released; final receipt changes are documentation only. Principal owns acceptance/integration; no next-slice implementation begins here.


## CW2 local source checkpoint

Approved CW2 is implemented in the isolated lane-C branch on `3e09b979`; exact [ownership, contracts and evidence](cw2/handoff.md) remain reviewable. Failing-before visibility/disclosure receipts and corrected native 310/0/1 plus strict affected lint are retained separately. Committed-source verification/mutation and independent Opus review are pending. No acceptance, full F21/qualification, hosted CI or integration claim; other slices remain unassigned/held per existing ordering.


## CW2 review correction checkpoint

Independent Opus round1 on d4a51409 is CHANGES_REQUIRED; the verbatim report and numbered response are retained under cw2/. The actual general-scope read/search privacy defect (original finding5) is HIGH, proved by two distinct failing-before0/1/0 receipts against unchanged f282 production. Corrections limit selected-conversation access to host-bound workingNote, preserve safe v1/plain Undo while gating reapplication, harden evidence replacement flags, retain typed CLI invalid_value(provider/effort), eliminate repeated/programmatic selection writes, and batch checked display reviews with local/external change-token invalidation. Exact extra desktop hunks/callers are allocated and mapped in [handoff](cw2/handoff.md). Current affected strict lint and actual repository cargo fmt pass; corrected native310/0/1 includes access cycles/cache work-bound proof. Final source pin, broader normal/feature checks, remaining one-budget mutation and fresh independent review are pending. No full acceptance/F21/native-human/hostedCI/integration or B1a handoff.


## CW2 final cache correction / capped verification checkpoint

Frozen source12b0d329 (crates53bcfa293e56c04c629b5eb80fee306d955d3eae) corrects the remaining round2 typing/request-batch blocker. [Response](cw2/round2-response.md) retains failing-before real typing12-vs1 batches and32-vs2 canonical request verifications, bounded token/routing correction, exact ownership and final core/app442/0/2/native310/0/1/lint/format proof. All204 Rust byte hashes match the final pin; unchanged144848 feature proof is explicitly qualified. Principal-owned focused Opus round3 is running; source is frozen.

Final copied-pin guard rechecks actually catch5/5 selected mutants, distinct from prior f282/144848 partial campaigns and survivors. Per-ID verdicts/diff/log/source-copy hashes remain in cw2/mutation-outcomes.json. One original1800s cap:1604.808563916944s total,195.191436083056s unused; all heavy children reaped. No complete mutation/F21/native-human/hostedCI/source-acceptance/integration claim. Shared/heavy ownership remains CW2 pending explicit safe serial release; B1a remains parked. No next slice starts here.


## CW2 final source-approved handoff

Principal-owned focused [Opus round3](cw2/opus-review-round3.md) is APPROVED exact12b0d3296ce116e99d4dee5800f2f02edebec5ac, with no blocking defect. Entire crates tree53bcfa293e56c04c629b5eb80fee306d955d3eae remains unchanged. [Final receipt](cw2/final-receipt.json) and [handoff](cw2/handoff.md) retain prior resolved findings, source-hashed442/0/2 coreapp and310/0/1 native, strict lint/format,204-Rust-hash association, qualified unchanged144848 feature checks and actual mutation outcomes. Final5/5 rechecks are separate from earlier survivors/unexecuted scope; cumulative1604.808563916944/1800s, no full campaign/F21/hosted/native-human claim. The reviewer mutation-pending wording predates those final actual receipts.

Final documentation commit changes no source/tests. On its completion the worker explicitly releases the sole heavy slot AND all allocated production/test/shared UI files to principal; final precise process check is empty, no child remains and no more worker source/test/build work is planned. Exact file hashes/hunk limits remain in cw2/path-ownership.json and handoff. Principal owns local acceptance, integration, next slice and B1a allocation. No retained worker ownership, publication/main merge/release or implicit next-slice start.
