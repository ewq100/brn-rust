# B1a panel: final local correction and qualification handoff

The allocated panel move is implemented and independently Opus-approved at `ca44bd7987e817b0d1ad75d1ff3855ea4d03c64a` (tree `a487a68546b2e5f5b52db354219c5df60be3c8d9`). Its bounded LOW followups are now qualified locally with the limits below. **Principal acceptance and dependent B1b allocation remain separate; no publication, main merge or native-human claim.** The final local commit/tree is reported after creation, avoiding a self-referential receipt.

Bound checkout: `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`, branch `work/b1a-cw2-20261016`. Material panel baseline: `df5e48e38549ad19a70daeaf47f9b6b185ef6d0e`; current correction commands ran at precommit HEAD ca44bd79 plus the exact recorded dirty source. Neither D1A source/lock nor other worktrees were integrated or edited.

## Review dispositions and source identity

The [verbatim Opus report](panel-opus-review-round1.md) is 3438 bytes, SHA-256 `2410ec3f2df1a54dce47fef0a4692adafff10fe4473492e84a932de562f75e68`. [Round1 response](panel-round1-response.md) and its [static source receipt](panel-round1-source.json) retain the exact corrections:

- Useful explanations are restored on their actual moved functions/fields and retained host handlers; misleading stranded comments are removed or rebound. Handler bodies/order, IDs, authority and state values are unchanged.
- The original `section_header(label, count, trailing)` body moves once from desktop into public library components. Existing desktop re-export uses that function; native `section(text)` delegates with `None, None`. No styling redesign or second implementation remains.
- The work-bound assertion message/prose now specifies **no saved writing request** in the zero-work phase. Fixture, counter values, assertions and events are unchanged. Its later real-request positive control and the existing CW2 real-request/privacy/typing witness retain their actual coverage; request-bearing renders may read latest Run.

Seven-file correction fingerprint: `ec40184d6b756b7e460f455ed0e86f006bf54902908c0e13f0b1d7e58062d55a`. Full production manifest: `dfbc90a48df172b61d6e635714c62f6765bdc1ba1cdd6fdd16da1eede11446dd` (295 current files under crates plus root/toolchain/lock/principal verification glue). Every command snapshot matches those production bytes. Approved formatting/diff helpers, manifests, lockfile and principal CI/script bytes remain unchanged from ca44bd79. No compile fix or guard redesign was needed in this window.

## Actual second-window checks

[Machine receipt](panel-window2.json) retains all **21 command receipts**, exact commands/features, source identities, UTC/durations, exit codes/log hashes, **131 artifact hashes**, tool identities and the ownership map. Raw evidence: `/private/tmp/brn-b1a-panel-window2-20261016/`. Commands use Rust **1.98.1**, task target `/private/tmp/brn-b1a-20261016-target` from this same source root, `CARGO_INCREMENTAL=0`, fresh synthetic TMPDIR, locked/offline and `CARGO_NET_OFFLINE=true`. No shared C0/cross-root artifacts were adopted.

| Gate | Actual result on corrected source |
|---|---|
| `cargo fmt --all -- --check` | Passed. |
| Maintained intake-helper build | Passed; artifact SHA-256 `25b6e40b3e4aa29f90465c3234ac02b132b2b4d5ab4387dd1a5bcd597d68e445`. |
| Default workspace normal/feature graphs | Passed; zero GPUI in both. |
| Default workspace build and strict all-target Clippy | Passed; no GPUI compilation. |
| `cargo test --workspace --no-fail-fast --locked --offline` | **1109 passed / 1 failed / 6 existing ignored / 0 filtered**. Failure is the carried PDF Unavailable case below. This is an explicit coverage superset of the maintained default test command, not a claim that verify-storage.sh ran. |
| Exact `bash scripts/verify-end-to-end.sh --fixtures-only` | Passed retirement checks, offline Save/recovery/search/export/backup/startup/refusal and JSON CLI journey: **108 steps / 1266 checks**. Uses default build artifacts before native build replaces them. |
| Exact `bash scripts/verify-desktop-shell.sh` | All seven maintained commands passed: helper, native library build, strict all-target native/test-support library lint, **44 library tests**, desktop native/test-support build/lint, full desktop **270 passed / 0 failed / 1 existing ignored / 0 filtered**, 4.72s test execution. |
| Retrieval `native --lib --test model_download` | **35 passed / 0 failed / 0 ignored** after explicit existing local ORT prerequisite selection. No model assets/download. |
| App `native-retrieval` tests | **240 passed / 0 failed / 1 existing ignored** after same prerequisite correction. |
| CLI `native-retrieval` build | Passed after same prerequisite correction. |
| Desktop `native-ui,native-retrieval` build / strict all-target Clippy / tests | Passed; **55 state tests / 0 failed / 0 ignored**. Exact CI combined features, without native-test-support. Build/tests use the explicit local ORT correction; strict lint passed on its initial invocation and was not repeated. |
| Combined native headless fresh/reopen | Both invocations passed on one new disposable directory; `threads.sqlite3` exists, retired `brn.sqlite` absent. Binary SHA-256 `04a484e5606a0360f2de2c1e62d38594c043439590a54c2f55a8440ac011d2c5`. No GUI. |

The complete desktop invocation includes the original reduced-motion A/B journeys, CW2 mandatory disclosure/scope/cache/access-cycle/real simulate_input witness, the real-editor F15 Undo-after-Apply/refresh/attempted-Save witness and the narrowly claimed work-bound witness. Their original values/events/assertions remain intact. No redundant focused reruns, new tests, blanket renaming, skips, mutants or stress ran. The single native ignored fixture still requires the three `BRN_DWR_SHARED_*` inputs. Library44 and native desktop271 registrations retain the [first-window relocation accounting](panel-window1.md); native combined55 is a different feature lane, not missing widget results.

## Retained failures and environment limits

**Default test failure:** `brn-threads-intake/tests/qualification.rs::prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` panics with `Pdf preparation failed: Unavailable`; that harness is10 passed/1 failed. Native `pdftoppm` is absent. Existing native arm64 Poppler26.10.0 `pdftotext` and `pdfimages` identities are retained in the machine receipt (SHA-256 `47dc9f7013c900875f09faf1f8788269b6a422adb2e0dd5218590b7731f328a7` and `adc3b7f3e43a78bcb497d99e3c388ea010351febc6236ff2dd9dbf8a1fde85a7`). No tool wrapper, skip, substituted converter, repeated failing PDF test or green workspace claim. F22's GPUI-free graph/build/lint/test traversal is evidenced; full all-green workspace qualification remains blocked by this carried environment gap.

**Verification prerequisite omission:** five initial retrieval/app/CLI/combined build/test commands failed at linking, before executing tests. `CARGO_NET_OFFLINE=true` triggers pinned ort-sys's SKIP_DOWNLOAD path before automatic cached distribution resolution, and ORT_LIB_PATH was unset. Inspection found the existing matching arm64 ORT1.28.0 distribution cache from ort-sys2.0.0-rc.13's manifest. The five blocked commands were rerun only after explicitly selecting it with ORT_LIB_PATH; all passed. Initial failures/logs remain in the receipt. Extracted static-library SHA-256: `d68759bf6c644dd5c6cb4e735699fba89e431b49ca588f73c11ee1c48ba838b3`. No source/dependency/feature change, linking stub, network fetch or model download. This is a corrected local verification setup, not a newly introduced panel failure or a concealed rerun-to-green. Combined strict lint had passed and was not repeated.

Existing external block0.1.6 future-compatibility and initial linker deployment-target warnings are retained; no suppression or upgrade. The original ca44 first-window failures/results stay historical and are not relabelled final-SHA executions. Global CW2 qualification, widget-B original-cause uncertainty, missing shared native/CLI intake fixture, hosted CI, human native observation and owner acceptance remain open. B1b adapter retirement, B3/B4 and D1A integration are not part of this extraction.

## Complete ownership handoff

**Release BOTH the sole heavy slot and ALL B1a UI/shared source/test ownership to principal after the local evidence commit. No paths remain reserved by this worker.** The exact map is also in `panel-window2.json::ownership_handoff`.

- Entire `crates/brn-writing-ui/`: Cargo.toml and src/{lib,formatting,diff,native,writing,document,workbench,layout,markdown,components,theme}.rs. Approved helpers remain unchanged.
- Desktop panel/support: src/{threads_ui,threads_writing,threads_components,threads_theme,threads_layout,threads_state}.rs, plus replaced/deleted {threads_document,threads_workbench}.rs. Earlier deleted {threads_format,threads_diff}.rs belong to the approved helper move; no shim or duplicate route.
- Desktop caller/focus/layout forwarding: src/{threads_shell,threads_tiles,threads_chat,threads_context_actions,threads_projects}.rs; open_project_note allocation required no new production delta. All runtime/queue/authority remains host-owned.
- Desktop allocated tests: src/{threads_dialogs_tests,threads_document_tests,threads_focus_tests,threads_omakase_tests,threads_pages_tests,threads_polish_tests,threads_review_tests,threads_steering_tests,threads_tiled_tests,threads_workbench_tests,threads_writing_tests}.rs. Other existing suites are unmodified and no tests are reserved.
- Principal-supplied glue already committed in ca44: .github/workflows/ci.yml, scripts/verify-desktop-shell.sh, Cargo.lock and desktop/new-crate Cargo feature/dependency hunks. No changes to these in the LOW commit; root manifest's approved helper member remains intact.
- Documentation: this b1a directory and minimal affected crate README links. No status/plan/evidence outside b1a was rewritten.

The final empty-process receipt records zero precise heavy executables and zero own controllers/children before handoff; final SHA/tree/crates/status are reported separately after commit. Principal owns local acceptance, any material-delta review decision, lock reconciliation and dependent allocation. This worker stops; no next slice or further heavy commands follow release.
