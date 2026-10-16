# Combined C0 implementation plan

**Goal:** establish a reviewable baseline for modular foundation plus agent-capabilities Slice 0, without production edits.
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
- [ ] Lead resolves remaining C0 exit gates individually; no automatic CW/D start here.

CL deletion candidates: **empty**. Examined and rejected: `brn-threads-intake/src/pdf_tools.rs::PdfTools::discover` is a used production Poppler path and X is unapproved; `brn-ai/src/threads.rs::begin`'s `.take(40)` history assembly is still active and belongs to D1A replacement, not removal; `ThreadsUi::open_note`'s linked-thread fallback (`threads_ui.rs:1307–1324`) and duplicated host start need CW2/B1b replacement, not CL deletion. No cleanup is authorized by this empty list. Any future concrete destructive list needs explicit owner approval before deletion. See [start messages](c0/start-messages.md).

Baseline PDF/hosted-widget failures are disclosures under master §6, **not C0 exit prerequisites**. Principal owns their carry-forward/disposition. After C0 review, an empty CL can be recorded as no-op without claiming four green lanes; any later change/green-baseline claim needs explicit failure disposition. CW1 may proceed after C0 with failures retained. B1a waits before moving `threads_writing_tests.rs` for the principal's bounded follow-on diagnostic disposition. [Correction record](c0/review-corrections.md) tracks Opus round 1 and the diagnostic evidence; no review approval or later-slice start is implied.
