# Final integration and verification plan

**Latest owner publication authority, 18 October 2026:** once the full all-document candidate and final verification are ready, [push/open the PR, monitor review and CI, merge after required gates, and verify merged-main CI](owner-pr-merge-ci-20261018.md). This supersedes earlier publication-hold statements below; no routine reauthorization is needed. It does not authorize early component publication or make any current gate pass. No task PR or CI watch exists yet.

**Source-only preparation, revision 3 (17 to 18 October 2026).** This is a plan and inventory. Nothing here is built, merged, pushed, verified, reviewed into acceptance, owner-accepted or released, and **Xberg all-document is not done.** Revision 2 answers the CHANGES_REQUIRED round 1 report ([verbatim](opus1/opus-review-round1.md); [point-by-point response](opus1/response.md)). It is a correction, not an approval; a new independent review decides. Revision 3 applies the two narrow round 2 precisions M1 (owner-only authority for narrowing) and M2 (case C evidence) and five advisories ([round 2 report, verbatim](opus2/opus-review-round2.md); [response](opus2/response.md)); everything else is unchanged.

The owner execution amendment (`~/.codex/brn-build-orchestration/2026-10-16/owner-kiro-final-integration-20261017.md`) names this work: final integration and verification after the Xberg **all-document** candidate is ready, with Kiro Sonnet 5.5 doing integration and Kiro Opus 5.5 doing independent review. It authorizes the work, not a green claim, a review bypass, an early start or a blanket publication grant.

Data: [inventory.json](inventory.json), [binding.json](binding.json). Merge simulation: [merge-simulation.md](merge-simulation.md). NF-1 test specification: [NF1-test-spec.md](NF1-test-spec.md). Status reconciliation draft: [status-draft.md](status-draft.md). Handoff: [handoff.md](handoff.md).

## 0. Names, so a component is not mistaken for the program

| Name | Meaning | What it is not |
|---|---|---|
| **Technical component integration (TCI-1)** | Optional, earlier: INT-W writing line + capability line, NF-1 fix, compile discovery. No Xberg code. Needs an exact heavy grant. | Not final integration, not a PR candidate by default, no "Xberg done" |
| **TCI-2** | TCI-1 plus the first standalone schema-1 HTML helper protocol capability (X2 `html1` WIP when it has a reviewed pin) | Not an all-document candidate: prepared HTML, host, PDF, Office, spreadsheet, MIME and extra families are untouched |
| **Final integration (FI)** | Merge of the reviewed pinned all-document Xberg candidate with both lines and `origin/main`, full gate map, whole-candidate review, required CI | Cannot start until the FI precondition (section 2) holds |

TCI work, if ever allocated, keeps status honest: the all-document objective stays open in every record it touches. Whether a TCI is published anywhere is a separate decision that this plan does not make. The owner sequenced final integration after Xberg, so no TCI is published to `main` ahead of that sequence unless a cited owner authority changes it; no TCI grant, build or publication exists now.

## 1. Binding and reconciled authority

| Field | Value |
|---|---|
| Checkout | `/Users/evokessler/.t3/worktrees/brn-rust/t3-ca8b0a32` (T3 allocated; the same checkout is reused for revision 2) |
| Initial state | branch `t3/prepare-final-integration-plan` at `9b47b285d23bb5f89db02ba398244827032fab6b`, tree `5556c9143388816cb3e8b2e88b48ec125a290ac0`, clean |
| Work branch | `work/final-integration-20261017`, created at exact `73ea9ca777e00d348966e37e8402f3eb68ecb20b` (tree `74d6dfb216585787e3d913ee45117868a2d2ceb9`, crates `bdc0a22a29e1aec741a78916ae5d0a7748531864`); revision 1 commit `d4574398516000f63d45e38f168b2a8294ca8951` (tree `d08b12cc77dc440d3a2d012953f42bc7386a9a1f`) |
| Untouched | X2 checkout `t3-9527c407`, INT-W checkout `t3-239c29dc`, X2 HTML worker checkout `t3-ad771af6` (read only), parent root (`main` `9b47b285`, untracked `.vscode/`, `experiments/`), draft PR #149, all other refs and remotes |

**Refs observed (Git only, no network):**

- Cached `origin/main` is `57c710ccec2b007c4d8a25c9f1d1b5c1c539907c`: two documentation-only commits over `8f768943`, same crates tree. Local `main` is 45 commits behind it.
- `work/x2-intake-20261017` is at `781c34dc466d6a6a210cdae4cf75e1096521c70d` (docs only over `73ea9ca7`). A separate uncommitted X2 HTML candidate lives in `t3-ad771af6` on `work/x2-html-20261017`; it has no pin and is read here only for the lock facts in section 5 and NF-4.

**Current dated authority, newest first:**

1. 18 Oct, principal: carried-failure gate policy (section 6.2; record `~/.codex/brn-build-orchestration/2026-10-16/final-integration-kiro-opus1-corrections-20261018.md`); the six-package TOML family disposition (`~/.codex/brn-build-orchestration/2026-10-16/x2-html-toml-family-disposition-20261018.json`).
2. 17 Oct, owner: Kiro/Sonnet/Opus execution split; Xberg is for all documents.
3. 17 Oct, owner: D-S, D-M, L approved (L: synthetic/public fixtures only). Real owner data, downloads, GUI, publication and main merge were never granted by those.
4. 16 Oct, owner: O1 CW-first; T/PC deferred (draft #149, do not merge).
5. Repository rules at `57c710cc` (newer than this base's [AGENTS.md](../../../../AGENTS.md)): check status branch/merge claims against Git; strategic-programming review; "When integration is authorized, compare the integrated tree with the verified candidate and inspect applicable main checks."

Publication authority is read at the moment a PR is proposed (section 7), not assumed from the earlier "PR and merge after work" request.

### 1.1 Stale statements to reconcile, not repeat

| Where | Says | Current fact |
|---|---|---|
| `agent-capabilities/README.md`, `modular-foundation/README.md`, `build-sequence/README.md` | "Proposed, not started" | C0, CL (no-op), CW1, CW2, B1a, B1b, B3, B4, INT-W, D1A to D4 and INT-D source are implemented and locally accepted on two unmerged lines. Nothing is on `main`. |
| `docs/status.md` | CW2 "pending", D3 "blocked by D-S/D-M", INT-D "not started"; H1 mid-file | Dated history; replace the top with a snapshot ([draft](status-draft.md)). |
| `modular-foundation/plan.md` "Current slice" | CW2 eligible next | CW2 through INT-W accepted locally. |
| INT-W `program-status.md` | D-S/D-M unresolved; D lane separate | D-S/D-M approved; the D lane is the other parent. |
| X2 plan "No compilation or extraction has occurred" | true at `73ea9ca7` | True only until the Xberg handoffs. |

## 2. Final-integration precondition (all must hold, each by a cited record)

The first schema-1 HTML helper component is an **engineering component, not Xberg done.** FI starts only when the principal records that all of the following hold. None is satisfied by a lane default or by "not part of this integration".

| ID | Requirement | Source of truth |
|---|---|---|
| FP-1 | One reviewed pinned Xberg all-document candidate (or an ordered, individually reviewed set of pins) covers **all current eight DocumentKinds**: PDF, DOCX, Markdown, Text, EML, HTML/HTM, XLSX, PPTX. Markdown and Text keep the declared Verbatim identity mode: byte-equal, zero provider calls, no helper requirement. The six native kinds use Xberg with typed projections into schemas 1 and 2. | `x2/owner-scope-all-documents.md`, `x2/formats.md`, `x2/plan.md` |
| FP-2 | Host, fidelity and lifecycle: every row of `x2/acceptance.md` has real evidence on the candidate pin: baseline/before, exact source/graph, feature-unified runtime (helper-local Tokio, all admitted kinds requalified under CPU10/NOFILE64/unchanged sandbox), exact helper restriction, PDF inventory/order and receipts/images, DOCX, PPTX, XLSX, EML/MIME, HTML, Markdown/Text, schema 1/2 binding, adverse resources, lifecycle, Luna, checked host, graph/package/platform. | `x2/acceptance.md` |
| FP-3 | The PDF resource STOP (native open, Brotli, default lopdf count, image decode, geometry, fonts) is resolved by a cited principal disposition: an enforceable profile or a source-proven pre-operation input profile. The disposition **states whether it narrows current PDF acceptance** (inputs the current route accepts that the profile would refuse); if it does, the authority rule below applies. | `x2/plan.md` resource STOP gate |
| FP-4 | **No old backend:** all three Poppler PDF routes (text, images, `pdftoppm` renderer) are retired with canary evidence (obsolete tools untouched, both PDF modes succeed, missing or old helper refuses with zero provider and save effects), and each replaced non-PDF reader is retired only after its family passes. Keeping Poppler or any old backend or fallback is an **owner-only** cited decision, because it changes the owner's all-document objective; it is not a plan default and a principal disposition cannot make it. | `x2/acceptance.md` row "No old backend" |
| FP-5 | **Every extra family** in `x2/formats.md` (legacy Office/OpenDocument/RTF/EPUB, other spreadsheets, MSG/PST, iWork, Hangul, notebook, XML/structured text, MDX, SVG/raster, WordPerfect, HEIC, archives, SQLite) has an explicit cited owner or principal disposition: admitted with its gates passed, or deferred or refused with a reason. Silence is not a disposition. | `x2/formats.md` |
| FP-6 | Reviewed pinned **family completion**: independent Opus review of the actual diff per completed family and for the whole candidate, root-graph checksum ledger audited before compilation, immutable pin, heavy slot released. | X2 plan, section 7 |
| FP-7 | Status keeps the all-document objective open until FP-1 to FP-6 hold. A partial result is reported as partial. | `x2/plan.md`: "No component-level success declares all-document adoption." |

**Authority rule (applies to FP-2 and FP-3 outcomes).** Any outcome that narrows what a **current** kind supports today (refusing inputs the current route accepts, lower fidelity, a changed profile acceptance, or a dropped kind) needs a cited **owner** decision. The principal may choose behavior-preserving technical solutions and make FP-5 extra-family admit, defer or refuse decisions with reasons; the principal cannot change the owner's all-document objective. No owner question is raised now: no narrowed proposal exists, and this only fixes who decides if one appears.

The earlier revision listed PDF resource, Excel/Tokio, Office/MIME fidelity, rich HTML and packaging as "not part of this integration". That is withdrawn: they are FP items, not deferred work.

## 3. Candidate graph and inventory

### 3.1 Topology

```text
8f768943 (main at CLI round 3)  ── 57c710cc origin/main (+2 docs commits)
   └ C0 c8b3026f → CL no-op → CW1 521454f2/3e09b979 → CW2 12b0d329/b617d784   (crates 53bcfa29…)
        ├─ writing line  B1a 60f604f5 → B1b 601e95d3 → B3 93b92d7b → B4 3515f96a/e8e98304
        │                → INT-W 0efd73f1 (src), HEAD d2d84e66           crates 9097bafb…   31 commits
        └─ capability line D1A 4db5b357 → D1B a94b9c99 → D2 d252a505/ca2bd94b → D3a 9b8acf66
                           → D3b 90f50da0 → D4 cc8338be → INT-D b9390407 → ordinary prepare 6b18324d
                           → refresh f590632e → history harness 39a67e4f → window result 65684dcb
                           → X2 docs 73ea9ca7 (this branch base)          crates bdc0a22a…   77 commits
        └─ Xberg all-document candidate: pending; only an uncommitted first-HTML WIP exists
```

The two lines are accepted locally and independently and have never been combined. Everything below about their union is static analysis of Git objects.

### 3.2 Divergence from the common base `b617d784` (CW2)

| Line | Source/manifests | Crates touched | Docs/experiments |
|---|---|---|---|
| Writing (INT-W) | 51 files, +12,964/−5,424 | brn-desktop 24, brn-writing-ui 18 (new), brn-threads-app 5; root `Cargo.toml`/`Cargo.lock`, `ci.yml`, `verify-desktop-shell.sh` | 150 files |
| Capability (X2) | 113 files, +25,891/−310 | brn-ai 40, brn-threads-core 25, brn-desktop 18, brn 14, brn-threads-app 13; `Cargo.lock`, two crate manifests (brn, brn-ai), `agent/skill-examples/**` | 1,355 files |

The writing line changes no file in brn-ai, brn-threads-core or brn. Only brn-desktop, brn-threads-app and `Cargo.lock` are changed by both.

### 3.3 Overlap and simulated merge

Thirteen source paths are changed by both lines: `Cargo.lock`, `brn-threads-app/src/lib.rs`, and in `brn-desktop/src/`: `threads_chat.rs`, `threads_state.rs`, `threads_tiles.rs`, `threads_ui.rs`, `threads_writing.rs`, and six test files. Most capability hunks are one-line additions. The large hunks are INT-W's: `threads_writing.rs` (+4/−2,864) and `threads_ui.rs` (+1,387/−717).

`git merge-tree --write-tree` of INT-W `d2d84e66` with X2 `73ea9ca7`: **zero textual source conflicts**; docs conflicts in `docs/status.md` and `docs/work/active/modular-foundation/plan.md`. With `origin/main` `57c710cc`: X2 conflicts only in `status.md`; INT-W in `status.md` and `docs/architecture/dependencies.md`. Textual cleanliness is not compile cleanliness: one semantic break is confirmed (NF-1), and the first compile may find more.

### 3.4 Accepted items versus integration work

**Accepted (do not re-implement; re-verify on the combined graph):** CW1, CW2, B1a, B1b, B3, B4, INT-W (two privacy fixes); D1A, D1B, D2, D3a, D3b, D4, INT-D, ordinary-prepare and refresh corrections, history harness and its closed 4/4 window. SHAs and dispositions are in `inventory.json`. X2 so far is documentation, source audit and proposals; no Xberg code is in any accepted tree.

**Needed integration work:**

| ID | What | Owner |
|---|---|---|
| NF-1 | `start_request` uses `reviewed.attach(prepared)` but `reviewed` is a local of `start_run`. Exact fix and tests: below and in [NF1-test-spec.md](NF1-test-spec.md). | Integration lead; Opus reviews |
| NF-2 | Docs conflicts: `status.md`, `modular-foundation/plan.md`, `dependencies.md`. | Integration lead |
| NF-3 | Replace stale claims with a current snapshot ([draft](status-draft.md)). | Integration lead |
| NF-4 | Lock reconciliation from the **exact Xberg handoff** (section 5). | Integration lead |
| NF-5 | Docs/CI edits only where a merged change requires them. | Integration lead |
| NF-6 | Link-check conditional re-run (section 3.5). | X2 worker owns the docs; integration lead re-runs |

Not proposed: new features, refactors beyond NF-1, new mutation campaigns, a repository-wide test audit.

**NF-1 fix, with the ordering preserved.** In the simulated tree `start_run` is `begin_run_start` → compute `queued` → `take_mcp_review` → build `ThreadRunStart` → `start_request`; `retry_rewrite` is `rewrite_run_start` → `begin_run_start` → `start_request`. The X2 order that must survive on every start is:

1. `begin_run_start`: busy refusal first. A busy refusal **does not** consume a pending review (it returns before any take); then `refresh()`.
2. `take_mcp_review(&thread, &prompt, queued)` **before** `prepare_thread_run`. Preparation creates the Run record; the existing `reviewed_snapshot_handoff_barrier` test shows that a refusal after prepare leaves a Failed Run with fence +1. Predicate and message are unchanged.
3. `prepare_thread_run`.
4. `reviewed.attach(prepared)` **after** prepare, with its existing refusal handling.

Move step 2 into `start_request` after its `thread` binding, with a new `queued: bool`. The value is the queue-text match (`queued.is_some()` in `start_run`), **not** `request.queue_claim.is_some()`: a matched queue item can have `dispatch == None`, and the existing predicate refuses on a match alone. `retry_rewrite` passes `false`. The compile error is a static finding, not an executed red; the tests in the spec are real call-path cases and the existing MCP controls (saved policy, context stamp snapshot, one prepare, queue lease, refusal, handoff barriers) stay unchanged. No production edit is made by this plan.

### 3.5 Which old evidence still applies, and NF-6

- **Same source closure as the capability line's receipts:** brn-threads-core, brn-ai, brn, brn-intake, brn-threads-intake, brn-retrieval (the writing line changes none of them). Those receipts (INT-D scripted AI 479/0/1, capability 455/0/1 + 3/0, CLI/app `int_d`, history window) describe those source bytes. They are not evidence about the final binary: the lock, workspace feature unification and the Xberg moves change it.
- **Needs fresh execution:** brn-desktop (all native lanes), brn-threads-app tests, brn-writing-ui, combined CLI/desktop journeys, `Cargo.lock` and feature graph, CI workflow, docs links.
- **Counts are anchors, not targets:** INT-W default 1,116/1/6, INT-D workspace 1,234/1/6, desktop 288/0/1 vs 315/0/1. For unchanged crates, the same per-package command should reproduce the capability line's count; a difference needs an explanation.
- **Live evidence** (original window, closed 4/4 history window) is historical: CLI path, `gpt-6.1-sol` medium, source `39a67e4f`. No new window is requested; closed windows stay closed.
- **NF-6 (docs link check):** on the old X2 base `scripts/check-markdown-links.py` measured 359 files, 1,703 links, **82 failures** (42 in `x2/source-audit/api.md`, 40 in `resources.md`). That count is immutable history and is not a target. The X2 worker's uncommitted HTML candidate measured 82 to 0 with the same unchanged checker (357 files, 1,611 links, 0 failures); that is a WIP receipt, not a pin and not a whole-root result. The X2 worker owns those docs. When a pin arrives, re-run the checker on the integrated tree; send any failure back to that lane; do not edit them here in duplicate, rewrite old ledgers (the cd2 ledger hash binding stays even though later B3 docs changed `resources.md` at `137726e1`), or claim root-wide green from the X2 count.

## 4. Gate map

One command at a time in the single heavy slot, Rust 1.98.1, `--locked --offline`, fresh explicit `TMPDIR`, `CARGO_INCREMENTAL=0`, target `/private/tmp/brn-final-integration-20261017-target`. Run `python3 scripts/development-preflight.py --native` once before the first heavy command, and set `BRN_VERIFY_OUTPUT_DIR` (existing absolute disposable directory) so the maintained scripts write `record.json` evidence. Record per command: tree, features, argv, exit, counts, ignored names, log hash. Zero-test or self-skipped runs do not count. Required hosted contexts (from `ci.yml`; GitHub rules are read at publication): `Core and CLI (ubuntu-24.04)`, `Core and CLI (macos-15)`, `Native UI build and state (macos-15)`, `Native retrieval and combined build (macos-15)`.

| Gate | Commands / witnesses | Anchors and notes |
|---|---|---|
| G0 static | `git diff --check`; no conflict markers; `python3 scripts/check-markdown-links.py` (0 failures, see NF-6); `python3 -m unittest discover -s scripts/tests`; `bash -n scripts/*.sh`; lock identity audit (section 5) | tooling 156; INT-D links 258 files |
| G1 graph | `cargo metadata --locked --offline` leaves `Cargo.lock` byte-identical. **F22:** `cargo tree --workspace --edges normal` and `normal,features` (default features) show no GPUI; native graphs contain GPUI by design. `cargo tree -p brn-writing-ui --features native-ui --edges normal,dev` has no `brn-desktop`. **Feature liveness per CI graph:** `cargo tree -e features -i serde_json` for default workspace, helper (`-p brn-intake --features helper`), native-ui, native-retrieval and capability-spike; record where `preserve_order` and each added image decoder are live. | Section 5 states what is already known |
| G2 default | `cargo fmt --all -- --check`; helper build, clippy, test; `check-provider-retirement.py`, `check-intake-retirement.py`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --no-fail-fast` for the counted run (maintained `verify-storage.sh` and `verify-end-to-end.sh` stop at the first failing crate, so with the carried PDF failure they cannot finish locally (TCI; re-derive at FI per section 6.2 item 5); run them for the record, then `verify-end-to-end.sh --fixtures-only` as CI does); capability `cargo test -p brn-ai --features capability-spike --all-targets` and clippy | INT-D workspace 1,234/1/6; capability 455/0/1 + 3/0 |
| G3 scripted AI | `cargo test -p brn-ai --features scripted-runs --lib --tests -- --test-threads=1` | 479/0/1. The four SDK loopback tests of the open 474/4/1 record live here; a recurrence is a new failure (section 6) |
| G4 native UI | `cargo build/clippy/test -p brn-desktop --features native-ui`; clippy and `--bin brn-desktop threads_ui::` with `native-ui,native-test-support`; `scripts/verify-desktop-shell.sh` (INT-W's version includes `brn-writing-ui` build, lint, 44 + 5 host tests); headless startup twice | INT-W desktop 288/0/1. NF-1 and the widget-confirmation tests appear here |
| G5 retrieval and combined | `cargo test -p brn-retrieval --features native --lib --test model_download`; `-p brn-threads-app --features native-retrieval`; `cargo build -p brn --features native-retrieval`; desktop `native-ui,native-retrieval` build, clippy, test | 35, 247/0/1, 55. Cached ORT archive only; no model assets, no download |
| G6 offline journeys | `TMPDIR=<fresh abs dir> bash scripts/verify-end-to-end.sh --fixtures-only`; combined binary `--headless-check startup` fresh and reopen | 108 steps / 1,266 checks |
| G7 existing witnesses (nonzero executed) | **Writing/F-suite:** core `writing_setup`, `cw2_visibility`, `edit_tokens`; app `cw2_review`, `writing_context`, `writing_setup`, `shared_run`, `import_coverage`; desktop CW2/B3/B1b/INT-W canaries; `brn-writing-ui` host journeys incl. cmd-shift-r. **Capability:** AI `threads_budget_tests`, `threads_cache_tests`, `threads_summary_tests`, `threads_continuation_tests`, `threads_skills_tests`, `threads_mcp_*`, `threads_int_d_tests::long_project_summary_selected_skill_optional_sdk_stop_cold_continue_exact_review_replay_undo`; app `int_d`, `skills`, `mcp`; core `skill_policy`, `mcp_policy`, `mcp_format`; CLI `int_d`, `skill_packages`, `mcp`, `agent_upgrade`, `agent_runs`. **Combined F21 channels (name each in the receipt):** the RewriteNote run through D1A/D1B/D2 plus skills and MCP; AI `threads_budget_tests::reserved_roles_and_private_other_chat_never_leak_through_history_query_or_exact_passages`; app `cw2_review::int_w_f21_private_context_never_enters_p2_or_general_delivery`; the export eligibility tests next to `export_metadata` (for example `source_links_without_import_coverage_are_exported`) | I02 map: [int-d/acceptance.md](../agent-capabilities/int-d/acceptance.md) |
| G8 schema, backup, restore, upgrade | Core tests that carry older stores to schema 9 backup-first (`edit_tokens`, `writing_setup`, `chat_storage`, `project_format`), backup/data-folder tests (`backup_directory`, `data_folder`); CLI `agent_upgrade`; app `int_d` recovery proof; export/backup tests for scoped setup and installed-skill exclusion | Chain: 6 (CLI contract 3), 7 (writing setup), 8 (skills), 9 (MCP). The `writing_setup` upgrade cases cover a schema 6 store; whether a schema 7 setup is carried through 8 and 9 was not found, so confirm it in the first run and report a gap. Old8/new9 binary compatibility rests on the earlier `90f` executable |
| G9 combined-only (technical and FI) | **NF-1:** the cases in [NF1-test-spec.md](NF1-test-spec.md). **Intake through the combined app + desktop paths (FI, and TCI-2 where the candidate touches them):** see the journey map below. **Serialization:** existing typed candidate/request/settings hashes, replay, backup/restore and export are byte-equal across the `serde_json` move in every graph where the move is live (host version move) and, separately, in the helper graph where the feature set changes (helper feature delta) | A hash or replay change is a stop |
| G10 live-bound evidence | Re-read, do not rerun: I01/I02 deterministic evidence; original window (2 starts, no candidates, closed) and history window (4/4 completed, 4 exact unapplied Prepared candidates, closed) | No Stop/Continue, owner Apply or superiority claim; no new paid activation |

**G9 intake journey map** (canonical journeys in `threads-target.md#acceptance-journeys`; each runs through the real `brn-threads-app` Workspace and the real `brn-desktop` conversion worker/UI, which this merge changes; locate each test by name at the first run and report a missing desktop case as a gap):

| Journey | App-level witnesses | Desktop/combined witnesses |
|---|---|---|
| Full note | `product::full_import_retains_assets_source_identity_and_rejects_changed_retry_input`; `import_coverage` (full receipt, B3 resolution); intake `qualification::ordinary_pdf_and_docx_preserve_checked_content_and_assets_without_original_payload` | desktop conversion completion to checked save; INT-W B3 attach and reimport cases; AI `conversion::tests` fixed-Luna mock |
| Partial / unsupported | intake `qualification::deliberately_omitted_figure_and_unchecked_full_note_are_partial`; `import_coverage` Partial/Unchecked; Xberg: unsupported-kind and over-bound refusals | desktop import-coverage display (B3) shows the gap; no wholesale "complete" |
| Crash / cancel during intake | intake `qualification::malformed_budget_cancel_and_timeout_leave_no_transient_copy`, `cancellation_after_pdf_copy_removes_raw_input_before_return`, `source_file_read_is_bounded_and_cancelled_before_conversion`; app `run_recovery`, `killed_run_candidate` | desktop cancel before parse, in extraction, after helper result, before save (acceptance row Lifecycle); pipe/handoff barriers, no sleeps |
| No hidden source archive | intake `ordinary_pdf_and_docx_..._without_original_payload`; export/backup tests (`product::export_refuses_occupied_or_incomplete_outputs_without_partial_publication`, `export_metadata`); Xberg: schema-1 root echo stays transient (not in `ConversionResult`, Note, backup, logs) | restored backup and offline read after the external original is removed |

Dedicated Luna and all-document contracts: Markdown and Text byte-exact with zero provider calls; the fixed `Chatgpt`/`gpt-6-luna`/`High` tool-free request exercised by the existing private `conversion::tests` mock (`cargo test -p brn-ai --features capability-spike --lib conversion::tests`, nonzero count). The first Xberg slice leaves prepared HTML byte-identical and Poppler in place. FP-2 requires the remaining rows.

## 5. Lock reconciliation: expected identity set, not a frozen number

No blind union, no wholesale update, no dedup. The expected set is computed at integration time from the **exact Xberg handoff lock**, the common base, and both parents; the figures below are the provisional facts of the current X2 HTML WIP lock (`Cargo.lock` SHA-256 `8b7a8ac078e5334119c4a497df362d111b644c164a01fc6f15a9645ce73db8dc`, graph gate receipt `7358546a2ed85ea204e12796ac67a39e686fd81f6d627082270742a32de166cf`, metadata run output `aa1a8b35405ccf7c21dc86ca7bde41ac68e697b3d7a29a0263f411eaa4528367`) and a principal-approved disposition, with no pin and no source approval. The all-document candidate will have further bounded reviewed changes; its handoff replaces these numbers.

**Existing-package moves in that WIP (six, not two).** The earlier "serde_json and toml" statement and the round 1 "minimum version" inference are superseded by the actual resolved lock:

| Package | Before | After |
|---|---|---|
| serde_json | 1.0.151 | 1.0.154 |
| toml | 1.1.6+spec-1.1.0 | 1.1.8+spec-1.1.0 |
| serde_spanned (1.x) | 1.1.1 | 1.1.2 |
| toml_datetime (1.x) | 1.1.1+spec-1.1.0 | 1.1.2+spec-1.1.0 |
| toml_parser | 1.1.3+spec-1.1.0 | 1.1.5+spec-1.1.0 |
| toml_writer | 1.1.2+spec-1.1.0 | 1.1.3+spec-1.1.0 |

The four companions are mandatory minimums of exact `toml` 1.1.8, approved by the principal (disposition file above) as a closure, with **52 added identities** each equal to the source1b archive/checksum ledger, **6 removed** old identities, and **zero** same-version checksum or source drift (counts 965 → 1,011). Coexisting versions stay: `toml 0.8.23`, `serde_spanned 0.6.9`, `toml_datetime 0.6.11`. A move **not declared by the audited handoff and approved** is a STOP. The six HTML-slice moves do not forbid a later exactly approved change that the all-document candidate needs (for example `tokio` 1.53.2 for Excel, `mail-parser` 0.11.9 for email, or side-by-side `zip`/`quick-xml` versions); those are still at the old versions in the WIP, and the actual final graph is not known. The disposition of the four companions is unchanged.

**Block-level three-way facts (read from Git blobs and the WIP lock, no Cargo):**

- INT-W vs CW2: one added block (`brn-writing-ui 0.1.0`) and one changed block (`brn-desktop`: two added dependency lines, `brn-writing-ui` and `rig`).
- X2 vs CW2: 13 added, 8 changed. WIP vs X2 base: 52 added, 6 removed, 11 changed same-identity blocks (dependency-line edits; for example the `phf 0.13.1` block changes while `phf 0.14.0` is added).
- **No block is touched by both INT-W and either X2 delta.** `brn-writing-ui` lists plain `serde_json`, valid while one `serde_json` version exists before and after.
- Provisional merged count: 966 (INT-W ∪ X2 base, verified statically) + 52 − 6 = **1,012**.

**Procedure.** (1) Resolve any textual `Cargo.lock` conflict by applying the declared block deltas (INT-W's two blocks; the handoff's added/removed/changed blocks) to the common base, never by `cargo update`. (2) Verify the identity set against the expected set: added = accepted ledger identities, removed = declared old identities, every other (name, version, checksum) unchanged. (3) The single permitted `cargo metadata --locked --offline` failure fallback is one `cargo metadata --offline` run whose diff must be **dependency-string edits only**; a new package, a version change or a checksum change is a STOP and cannot be masked as an edge rewrite. (4) Report the diff.

**`preserve_order` is not helper-only.** The earlier "likely live only in `-p brn-intake --features helper`" is withdrawn. Facts, with their sources: the registry manifest `rig-core-0.44.0/Cargo.toml` requests `serde_json` features `raw_value`, `preserve_order`, `float_roundtrip`; `rig-core` is a normal dependency of `brn-ai` and `rig` a dev dependency; the WIP's default-workspace metadata (association above) resolves `serde_json` features `alloc, default, float_roundtrip, indexmap, preserve_order, raw_value, std`; and the base lock's `serde_json 1.0.151` block already lists `indexmap`. So `preserve_order` is already live in the default workspace graph before Xberg. What Xberg newly changes is the `serde_json` patch version everywhere, and in an isolated `-p brn-intake --features helper` build the feature set (also gif/bmp/webp decoders). No workspace member enables `brn-intake/helper`, but that does not make the feature helper-only. The WIP's single metadata run used default features, so helper unions there are derived statically. G1's per-lane `cargo tree -e features -i serde_json` establishes liveness per CI graph when a heavy grant exists; host-side evidence is the version/hash compatibility check, helper-side evidence is the feature-delta check, and neither is inferred from the other.

## 6. Carried failures, gate policy and deferred work

### 6.1 Records (preserve; never relabel)

| Item | What is recorded | Not | Would change it |
|---|---|---|---|
| **PDFUnavailable**: `brn-threads-intake` `qualification::prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` (`tests/qualification.rs:470`) | Present in every local default run since C0 (1,116/1/6 INT-W; 1,234/1/6 INT-D; 1,224/5/6 D4). Local Poppler 26.10 lacks native `pdftoppm`. Hosted macOS CI installs Poppler with Homebrew; in main run 38908335265 three of four required contexts passed, including `Core and CLI (macos-15)`, which runs this test | Not fixed by wrapper, skip or install; not Xberg-related until a PDF cutover exists | Principal-provisioned genuine `pdftoppm` and one affected run; hosted pass at the exact head; or a proved Xberg PDF route (new evidence) |
| **SDK 474/4/1**: four `threads_mcp_http_tests` loopback setup failures (`Transport`) | Later runs 477/0/1, 478/0/1 (diagnostic), 479/0/1. Opus2 found no correction-caused path. No failing class captured; the diagnostic now prints stage, reqwest flags and OS error on recurrence | Not flaky, not baseline, not closed | A recurrence with the diagnostic output |
| **Widget 314/1/1**: `the_owners_stop_of_a_killed_reply_adds_its_review_but_no_notice` | Isolated 1/0, diagnostic 1/0, later full 315/0/1. State not printed | Not the hosted failure below; not covered by the reduced-motion fixture | A recurrence under the widget-confirmation diagnostic method |
| **Hosted main failure**: `an_edit_already_open_elsewhere_offers_compare_and_discard`, run 38908335265 (305/1/1) | Test-level fixture correction `aea4830f` (reduced motion; 800/800 local iterations), already in both lines; sibling B cause unconfirmed | Not a product-defect finding; not hosted-verified | Passing hosted native lane at the exact head |

### 6.2 Gate policy (principal disposition, 18 October; record `~/.codex/brn-build-orchestration/2026-10-16/final-integration-kiro-opus1-corrections-20261018.md`)

This is gate policy for this plan, not a push, PR or merge grant.

1. **Historical** SDK 474/4/1 and widget 314/1/1 stay disclosed. They are neither an automatic pass nor a blanket merge blocker when they do not recur.
2. **A new failure at the combined source** (any gate, including a recurrence of either test) is a new failure at its own source: capture its raw output and diagnostic class, preserve it, diagnose it, and get an explicit principal disposition **before the PR is ready**. A later passing run is not a diagnosis.
3. **A required hosted context that fails and then passes on rerun at the same head** needs `python3 scripts/ci-summary.py compare-logs` of the saved logs plus a principal disposition; the exact-head required contexts must pass. Use `ci-summary.py run` for the attempt-exact summary.
4. With publication authority checked at that stage (section 7), a PR **may be prepared and opened ready** with the historical qualifications disclosed. **Merge** requires the four required contexts passing at the exact head and an explicit disposition for any recurrence.
5. **PDF local carry** (per the INT-W and INT-D dispositions) stands only because hosted `Core and CLI (macos-15)` runs that test with Poppler; it requires an actual hosted pass at the target head and no silent test skip. This item and G2's "cannot finish locally" apply as written to TCI. At FI, FP-4 retires Poppler, so the `PDFUnavailable` test, the Homebrew Poppler CI step and `check-intake-retirement.py` change in the Xberg lane: re-derive the actual PDF qualification test and CI/retirement checks from the handoff; there is no automatic local PDF-test exception.
6. **N1 native observation is separate.** Owner PR or merge intent is not a native-verified claim; merge may precede N1 only under an existing owner PR/merge request, with F20 stated as not run.

### 6.3 Other deferred or scope-qualified items (disclose; disposer shown)

| Item | Disposer |
|---|---|
| Native-human F20, IME, picker, menu, quit, first-input ordering | Owner session N1 |
| I01 beyond deterministic: Stop/Continue live, live owner Apply/replay/Undo, unknown cause of three original prepare failures, no superiority claim | Principal; any new window needs a bounded L allocation |
| Mutation: closed budgets, survivors, NO_VERDICT stay as recorded | None; no new campaign without explicit allocation |
| Hosted Windows and Linux native-retrieval failures (non-required, visible on main); no hosted run has yet executed capability-line code, so the required Ubuntu Core lane will be its first | Owner (personal macOS first) |
| D3a crash and hostile-filesystem residuals; D4 SDK outer timeout, NACK source-only proof, per-frame stamp scan; installed skill bytes excluded from backups | As already dispositioned |
| Comment visibility for shared Notes (C0 question), PC/T, draft #149 | Owner |
| Owner acceptance, release, real user data | Owner |

The Xberg resource, runtime, fidelity, family and packaging items are **not** in this table: they are FP items (section 2).

## 7. Native, owner and live dependencies; review and publication

No paid activation, download, real owner data or GUI occurs under this plan.

**N1**, the one manual task, is one owner session on the target Mac after the final binary exists and passes G0 to G6, offline, on a fresh synthetic directory outside Git: the INT-W 16-step outline (`docs/work/active/modular-foundation/proposal/final/06-native-qualification.md`, INT-W version) for steps that need no provider (1, 3, 4, 5, 13, 14, 15, 16; step 15 launches `writing_host`, ⌘/, ⌘S, ⌘⇧R, editor right-click Comment, GUI quit), plus three D-lane additions: Settings → Skills shows an installed package inactive; the chat skill selector is visible; Settings → MCP is disabled and makes no connection. Provider-dependent steps (6 to 12) stay pending. Until N1 runs, F20 is not run and nothing is called native-verified.

**Review and publication order**

1. Make the **last** `origin/main` merge before the freeze, or re-review the delta. The frozen tree is what gets reviewed.
2. **Independent Opus 5.5 whole-candidate review**, fresh and read-only, on the frozen candidate. Supply base-to-pin diffs per parent, the combined diff, `inventory.json`, receipts and these focus items: NF-1 and start-path equivalence for chat, queue and writing; combined F21 channels; lock identity set and `preserve_order` effects; retired routes; docs truth against Git; strategic-programming questions (what became easier, justified complexity, retired paths, debt with a revisit trigger). Corrections only for relevant findings; a material change gets a new review. No automatic loop.
3. When publication is authorized: read branch protection and rulesets (`gh api repos/ewq100/brn-rust/branches/main/protection`, `gh api repos/ewq100/brn-rust/rulesets`), compare required contexts with `ci.yml`, check open PRs, confirm #149 untouched.
4. Push a new branch (never `main`, never force). **The pushed head tree must equal the frozen, reviewed tree.** PR with summary, tested, blocked and the historical qualifications; link it to the thread; apply section 6.2; `gh pr merge --merge` only when it allows.
5. After merge: fetch, compare the merged tree with the verified candidate, inspect main checks, reconcile status with the merge SHA, record implemented, verified, native-observed, owner-accepted and merged separately.

Nothing here authorizes a PR, push or merge.

## 8. Next allocations

**Optional TCI-1 (technical, not final).** Only after the heavy slot is free and with an exact principal grant: merge INT-W into a scratch descendant, fix NF-1, run the cheapest compile discovery (`cargo check -p brn-desktop --features native-ui --all-targets`, then the affected lanes of G4). It uses no Xberg code, is named TCI-1 everywhere, and is not offered for publication. No grant or build is requested now.

**FI (after FP-1 to FP-7).** Preconditions: section 2 satisfied and recorded by the principal; heavy slot released; an exact grant that **explicitly includes network** for the first `git fetch origin`.

Owner: Sonnet 5.5 integration worker in this checkout and branch. Writable: merges; NF-1 in `crates/brn-desktop/src/threads_ui.rs` plus its tests per the spec; conflict resolution in `docs/status.md`, `docs/work/active/modular-foundation/plan.md`, `docs/architecture/dependencies.md`; status/README reconciliation; `docs/work/active/final-integration/**`; any compiler-required hunk reported before editing. Read-only: everything else, including the Xberg crates and the X2 docs. One serial heavy slot; no live, GUI, download, mutation, PR, push or `main` merge.

```sh
cd /Users/evokessler/.t3/worktrees/brn-rust/t3-ca8b0a32
git status --short && git switch work/final-integration-20261017
git fetch origin                                    # needs the explicit network grant
git merge --no-ff -m "merge: Xberg all-document candidate <X>" <X>
git merge --no-ff -m "merge: INT-W writing line d2d84e66" d2d84e665e64de14a15f4704eb0868c553c2885a
# expected conflicts: docs/status.md, docs/work/active/modular-foundation/plan.md (keep both sides, newest first)
# NF-1 fix commit and tests; lock identity audit; G0 to G9 serially; commit evidence
git merge --no-ff -m "merge: origin/main <SHA>" origin/main   # LAST merge, then freeze (or review the delta)
# expected conflicts: docs/status.md, docs/architecture/dependencies.md
```

Stop conditions: an unmapped lock identity or checksum; any existing-package move not declared by the audited handoff and approved; a hash or replay change; an unexplained failure outside section 6.1 (NF items are expected and have their own witnesses); a recurrence handled per section 6.2.

## 9. Limits

Static Git, source and text inspection. Executed: Git reads, `git merge-tree --write-tree` (revision 1; unreferenced objects in the shared object store), text and hash reads, Python reads of lock blobs and of the X2 WIP's lock and metadata files (read only, no Cargo), `scripts/check-markdown-links.py` and `git diff --check`. No Cargo, build, test, lint, format, Store, CLI, auth, network, provider, GUI, live or mutation command; no heavy slot. The X2 HTML WIP has no pin; nothing from it is imported. Nothing was compiled; the lock identity facts are parses of text.
