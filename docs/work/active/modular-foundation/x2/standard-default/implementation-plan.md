# Standard Xberg default and optional BRN mode Implementation Plan

> For agentic workers: use the existing owner-supplied T3/Codex execution method with serialized builds and fresh delegated review rounds. This repository/runtime workflow overrides unavailable superpowers execution skills and the generic skill's persistent-reviewer mechanism. The principal owns this plan and revisions.

**Goal:** implement two persisted converter choices and an independent AI switch, defaulting to stock Xberg with AI Off, through existing imports and Settings.

**Architecture:** keep one checked BRN import/save service. Build the same small new document-helper program against stock Xberg and against a separately materialized, verified patched Xberg in an isolated standalone workspace. Reuse the existing restricted process boundary; do not patch the stock graph. Both helpers return one BRN-owned transient document envelope. The application chooses the helper from the captured setting, then optionally invokes the existing fixed-model, tool-free conversion transport. No legacy-reader fallback.

**Tech Stack:** Rust1.98.1, Xberg1.3.10 defaults disabled with pdf/office/excel/email/html, maintained Tokio current-thread runtime, existing serde/image/protocol and checked SQLite configuration. Stock and adapted tool workspaces have independently locked graphs. Root production SDK/model pins remain unchanged.

**Spec:** [owner-confirmed feature brief](feature-brief.md). [Latest all-document scope](../owner-scope-all-documents.md) supersedes old all-fidelity-before-default gates. The owner answered every product question: Xberg default, BRN-adapted second, AI independent Off default, Settings only, eight kinds.

## Global constraints

- No new library parser/decoder patches, PDF correction resumption, raw-cell/MIME fidelity project, OCR/model download, external source retrieval or new paid-provider activation.
- Retain source/protection/version/replay/Undo boundaries. Comparison does not alter existing notes merely by changing settings.
- Stock is exact published1.3.10, archive SHA f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d. Use published Cargo.toml, not workspace-inherited Cargo.toml.orig.
- Initial optional upstream bundle contains ONLY accepted Xberg patches0001,0003,0004,0005, combined package source ce3bce28f1e6e7393efe27cfad61f8159208d286. The small existing UTF8 identity adjustment is also retained for Text/Markdown in the BRN projection. Omit unaccepted0007 and frozen PDF0002/0006. No adapted native-PDF fork is needed. PDF/XLSX/EML/HTML have no new fidelity adjustment in this first bundle; old restricted-HTML behavior remains historical, not silently applied to stock. All eight kinds remain usable with either setting, without a claim that every format improves.
- Common SDK options retain normal text/preprocessing behavior. Only kind/MIME, output Markdown, finite host-security settings, disabled external/model/OCR services, serial execution and explicit runtime are common overrides. Do not inherit custom HTML escaping/autolink changes in stock mode.
- Both modes run under existing empty-environment, deny-default helper restriction and owned process/deadline/pipe lifecycle. Those limits are not a hard heap/RSS guarantee; report actual constraints without reopening bespoke resource engineering.
- Eight kinds only: PDF, DOCX, PPTX, XLSX, EML, HTML, Text, Markdown. No password UI/implementation; no source-controlled routes. Normal library failure is a safe failure, not permission for a hidden reader retry.
- One heavy slot, jobs1, offline/locked after audited source reconciliation, fixed source/target roots and fresh0700 TMPDIR outside Git. No compiled-input edits during Cargo. Preserve failed attempts and source/artifact associations.
- Existing optional XLSX work is checkpoint8acffb9e, unaccepted. Initial product brief is eb32c673. Preserve these and all older evidence; do not clean/reset unrelated checkout state.

## Review focus

1. A path/bundle override selects the wrong executable: reject mismatched response identity before save.
2. Stock accidentally inherits a global patch or HTML custom config: compare normal stock public results and inspect exact emitted graph/source hashes.
3. AI Off still authenticates, or Text/Markdown bypass AI On: assert zero auth with Off and an actual mock request for every kind with On.
4. Reused import keys silently replay a different converter/AI setting: bind the captured options and keep historical committed replays explicit.
5. Restored settings or mid-import changes mislabel output: persist preferences, snapshot the job, reject stale authority normally and preserve actual provenance.

---

## Task1: reproducible stock/adapted document helper and common envelope

**Files**
- Create tools/xberg-intake-helper/{Cargo.toml,Cargo.lock,src/main.rs,src/engine.rs,src/project.rs,build.rs,build-identity.txt,adapted.Cargo.lock}.
- Create scripts/{prepare-xberg-adapted.py,build-document-helpers.sh}.
- Create crates/brn-intake/src/document.rs and crates/brn-intake/src/helper_runtime.rs.
- Modify crates/brn-intake/src/{lib.rs,bin/brn-intake-helper.rs} to export light DTOs and share existing restriction/protocol runtime; no legacy extraction behavior change.
- Create tools/xberg-intake-helper/tests/{eight_kinds.rs,identity.rs}; use existing public fixtures under crates/brn-intake/tests/fixtures and crates/brn-threads-intake/tests/fixtures.
- Create scripts/tests/test_prepare_xberg_adapted.py.

**Interfaces** (all BRN-owned; SDK types stay in the standalone tool)
```rust
// brn_intake::document
pub struct DocumentRequest {
    pub schema: u32, // 1
    pub backend: String, // exactly "xberg" or "xberg_brn"
    pub kind: String, // exact eight host tags
    pub limits: IntakeLimits,
    pub bytes: Vec<u8>,
}
pub struct DocumentExtraction {
    pub schema: u32,
    pub backend: String,
    pub engine_identity: String,
    pub source_sha256: [u8; 32],
    pub markdown: String,
    pub assets: Vec<ImageAsset>,
    pub gaps: Vec<String>,
}
impl DocumentExtraction {
    pub fn validate_for(&self, request: &DocumentRequest) -> Result<(), String>;
}
pub fn expected_engine_identity(backend: &str) -> Result<String, String>;
// Shared binary source, not a second sandbox implementation:
pub fn main_with(handler: fn(bool, &[u8]) -> Result<Vec<u8>, String>);
// Shared helper_runtime.rs owns restrictions, self-tests, arguments and bounded
// stdin/stdout. Each program decodes its own typed DTO inside this callback.
// Old main imports #[path = "../helper_runtime.rs"] mod helper_runtime;
// the standalone main imports the same file by its relative repository path.
// Do not put this shared module in src/bin: Cargo would discover another binary.
```

- [ ] Add pure DTO validation tests for wrong schema/backend/kind/hash, nonfinite quota requests, oversized text/asset/gap collections and wrong bundle identity. No test executes a provider.
- [ ] Add stock helper fixture tests using eight real byte inputs: harbor-study.pdf, harbor.docx, quay.pptx, forecast.xlsx, single.eml, textual.html plus authored UTF8/Markdown strings. With the chosen exact SDK config, compare returned Markdown to normal public Xberg output without custom textual rewriting. Existing source facts describe known gaps, not requirements to repair stock output.
- [ ] Add generator tests for bad archive hash, changed patch, occupied/mismatched generated source, path traversal archive member, stock source unchanged and full765-file adapted reproduction. Verify the four patch hashes from their existing manifests; no provider/network fixture.
- [ ] Implement a normal standalone tool workspace with its own [workspace] root, depending on brn-intake by path WITHOUT helper, Xberg1.3.10 selected features, serde_json, libc and required maintained Tokio runtime features. Do not enable legacy helper: its mail0.11.8 conflicts with Xberg email's0.11.9 range. Keep the root workspace unchanged initially.
- [ ] Extract existing restrictions and bounded protocol lifecycle into helper_runtime.rs without new filesystem/network permissions. Preserve self-test-restrictions and process-tree controls and non-macOS unavailable behavior. Old handler decodes HelperRequest and calls the old extractor exactly as before; new handler decodes DocumentRequest and rejects the unused --prepare flag. Do not duplicate the sandbox profile or enable legacy helper dependencies in the new tool.
- [ ] The new helper validates input before extraction, maps exact MIME tags and calls normal xberg::extract. Use a current-thread Tokio runtime with finite blocking threads and one Rayon thread, await and drop it. No custom HTML config. SDK warnings become honest gaps; do not claim Complete. Empty/unreadable results have explicit safe outcomes, not fabricated text.
- [ ] Project only actual returned values into DocumentExtraction. Keep direct Markdown unchanged except checked relocation when returned assets require retained names; retain raw stock Markdown in test evidence and enumerate each relocation. Do not invent source cell addresses, MIME part IDs or image data. Existing bounded PNG/asset validation applies to retained images; unavailable external references are reported, never fetched.
- [ ] In the BRN backend only, retain the existing explicit Text/Markdown UTF8 identity adjustment after successful extraction: valid source text is returned exactly, including CRLF/BOM/whitespace, with the existing finite size/NUL admission. This is decoding original text, not another parser or a legacy-converter fallback. Stock keeps normal Xberg text derivation. Add exact nine-vector comparison cases for both modes. No other custom family adjustment is introduced.
- [ ] Materialize a SHA-verified stock archive under ignored target/xberg-adapted/source; apply only accepted0001/0003/0004/0005, verify full source manifest, and make a generated standalone adapted workspace. It compiles the SAME tool source and same brn-intake light dependency, with [patch.crates-io] scoped ONLY to that workspace. Stock workspace has no patch. Existing root legacy code is neither engine nor fallback.
- [ ] Use build-identity.txt derived from the archive and ordered patch hashes, not enclosing Git HEAD. The committed stock workspace contains the stock identity; the generated adapted workspace contains its exact adapted identity. Shared build.rs reads that workspace-local file via CARGO_MANIFEST_DIR and embeds its validated fields. This works even for direct manual builds; a missing file is a build error, not a stock default. The program rejects a request for a different backend. The builder also sets maintained XBERG_BUILD_ID explicitly. Source hash/graph proof remains distinct from a self-reported string.
- [ ] Commit a lock for each standalone graph after ONE audited offline reconciliation each, seeded from approved source1b/remaining-format locks. List every identity/feature/checksum movement before build; unknown/unread sources stop for principal allocation. No root dependency upgrade or registry source edit.
- [ ] Build serial stock and adapted helpers with distinct fixed targets; copy to declared artifact paths brn-xberg-helper and brn-xberg-helper-brn. Missing adapted artifact cannot substitute stock silently.
- [ ] Run the same eight-kind public tests, genuine stock-vs-adapted namespace/PPTX differing-output controls and actual restriction/process-tree witnesses. Preserve unsuccessful setup/compiler/runtime attempts; a new API's absence is not a manufactured behavioral red.
- [ ] Strict tool all-target Clippy and static generator tests; preserve source/lock/binary/.d and exact patch reproduction. Commit Task1 separately.

**Commands**
```sh
python3 -m unittest discover -s scripts/tests -p 'test_prepare_xberg_adapted.py'
bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root /private/tmp/brn-x2-modes-target
cargo +1.98.1 test --offline --locked --jobs 1 --manifest-path tools/xberg-intake-helper/Cargo.toml --all-targets
```
Generated adapted workspace runs the same tool tests with its committed lock and explicit manifest; the build script reports its exact path. Root old helper tests remain relevant to the runtime extraction only; they do not prove new converter behavior.

## Task2: settings and source-bound conversion/AI routing

**Files**
- Modify crates/brn-threads-core/src/{types.rs,store.rs,upgrade.rs} and current schema compatibility tests.
- Modify crates/brn-threads-app/src/{settings.rs,intake.rs,keyed.rs,lib.rs}; create tests/document_conversion_settings.rs.
- Create crates/brn-threads-intake/src/documents.rs; modify src/lib.rs for export, using the existing process.rs lifecycle.
- Modify crates/brn-ai/src/{conversion.rs,threads.rs,threads_sources.rs,threads_sources_tests.rs}; create document_refine.rs and focused tests. Existing source-report observation/anti-forgery paths are reused, not replaced.
- Create crates/brn-threads-intake/tests/document_modes.rs.
- Mechanical default-field initializers across existing tests/examples only, preserving all old assertions.

**Interfaces**
```rust
// Core canonical preferences; serde default is stock/false.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentConverter { #[default] Xberg, XbergBrn }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConversionSettings {
    pub converter: DocumentConverter,
    pub ai: bool,
}
// WorkspaceSettings adds #[serde(default)] pub document_conversion: ...;
impl Workspace {
    pub fn choose_document_conversion(
        &mut self, settings: DocumentConversionSettings
    ) -> AppResult<ApplyOutcome>;
}
// Existing checked SettingsUpdate gains one typed preference delta.
// Store::choose_document_conversion takes &OperationId,
// &DocumentConversionSettings and &HostAuthority and returns Result<ApplyOutcome>,
// using the same receiver/transaction pattern as existing choose methods.
// Host intake owns executable selection; no model/path choice in DTOs.
pub struct DocumentHelpers { pub stock: PathBuf, pub brn: PathBuf }
impl Converter {
    pub fn convert_document(
        &self, request: ConversionRequest<'_>, helpers: &DocumentHelpers,
        backend: &str, cancel: &AtomicBool
    ) -> Result<ConversionResult, ConversionError>;
}
// New job; old ConversionJob/convert stay explicit historical APIs so their
// existing safety tests are preserved, never selected as a fallback.
pub struct DocumentConversionJob {
    pub converter: Converter,
    pub helpers: DocumentHelpers,
    pub settings: DocumentConversionSettings,
    pub kind: DocumentKind,
    pub intent: ImportIntent,
    pub bytes: Vec<u8>,
    pub source_reference: String,
    pub auth_file: Option<PathBuf>,
    pub credentials_dir: Option<PathBuf>,
}
pub async fn convert_document_job(
    job: DocumentConversionJob, cancel: CancellationToken,
    conversion_cancel: Arc<AtomicBool>,
    emit: Arc<dyn Fn(ConversionProgress) + Send + Sync>
) -> Result<ConversionResult, ConversionFailure>;
// Engine extraction always precedes the optional model phase.
pub async fn refine_document(
    converted: ConversionResult,
    auth_file: Option<&Path>, credentials_dir: Option<&Path>,
    cancel: CancellationToken
) -> Result<ConversionResult, ConversionFailure>;
// Core ImportReport gains #[serde(default, skip_serializing_if = "Option::is_none")]
// pub conversion: Option<DocumentConversionProvenance>. Both Source.report
// and Note.import.report already use this type; no duplicate Coverage field.
// Old reports remain None, never relabelled with a new engine.
pub struct DocumentConversionProvenance {
    pub settings: DocumentConversionSettings,
    pub engine_identity: String,
}
// Shared host builder, exported from brn-threads-app::intake/lib.rs.
// Rejects an actual converter label inconsistent with captured settings and
// expected_engine_identity; preserves existing status/issues.
pub fn import_report_with_conversion(
    converted: &ConversionResult, settings: &DocumentConversionSettings
) -> AppResult<ImportReport>;
// TransientSource already has report: Option<ImportReport>; useful intake
// carries the host-built report there, with no separate provenance field.
// New app methods use the same checked import logic with an explicit variant.
pub fn import_full_scoped_with_conversion(
    &mut self, key: &str, title: &str, converted: &ConversionResult,
    scope: Option<&str>, settings: &DocumentConversionSettings
) -> AppResult<ApplyOutcome>;
pub fn full_import_replay_with_conversion(
    &mut self, key: &str, title: &str, source_reference: &str,
    source_bytes: &[u8], scope: Option<&str>,
    settings: &DocumentConversionSettings
) -> AppResult<Option<ApplyOutcome>>;
```

- [ ] Prove absent preference deserializes as stock/AI Off; store/reopen/backup/restore preserve independent values and generic Configure does not erase them. Follow existing checked configuration/replay/fencing semantics.
- [ ] Bump schema9 to10 because old binaries would ignore an AI-off preference and retain always-AI behavior. Reuse backup-first protocol-only upgrade, add9 to supported old versions, preserve canonical rows, old executable refusal and upgrade-only-unpublished restore behavior. Do not add a second migration framework.
- [ ] Implement choose_document_conversion through checked Store SettingsUpdate and Workspace wrapper. Add serde defaults and only compiler-required initializer fields; no unrelated formatting.
- [ ] Add convert_document using the existing bounded process::run, absolute selected helper, empty environment, deadline, job PGID and output validation. Validate exact requested backend and expected bundle identity, source hash, assets, output limits and post-completion cancellation. No raw source retained after ConversionResult.
- [ ] Capture both preferences once in DocumentConversionJob. Always convert with the captured helper. AI Off returns direct output before Auth or credential requirement. AI On calls the same dedicated gpt-6-luna/high, tool-free authenticated transport for either engine and every admitted kind, including Text/Markdown. The new route has no legacy text bypass; old ConversionJob/convert and prepared APIs remain explicitly historical, not selected by the settings path.
- [ ] Reuse the existing Auth/OwnedClient/collect_stream path; a narrow transport factoring may preserve legacy candidate_for_prepared tests. Do not add new account selection, fallback, retries, general model settings or tools.
- [ ] Use a small source-bound JSON refinement response: schema1, source_sha256, markdown, uncertainty. The model is asked for faithful readable formatting, not summary or invented corrections. Check exact source hash, finite prompt/output budgets, natural terminal, no unrecognized fields, valid retained asset links and post-result cancellation. Preserve original assets/source references/intent/mode and mark coverage Unchecked or genuine reported omissions; never infer Complete from a model response. Existing strict prepared conversion remains historical/explicit API, not a hidden alternate engine.
- [ ] Test all eight kinds x both engines with AI Off and a failing auth seam proving zero requests. With synthetic provider transport test both engines and all kinds AI On, actual selected input delivery, safe error/no fallback, invalid hash/output, stream interruption and zero canonical save on failure.
- [ ] Persist DocumentConversionProvenance in a serde-default optional ImportReport.conversion field shared by Source.report and Note.import.report. Build expected engine identity from fixed stock/archive and accepted ordered-bundle constants in brn_intake::document; do not infer it from filenames or trust helper self-report alone. ConversionResult.converter carries the validated actual identity and AI label; import_report_with_conversion rejects a mismatch and snapshots actual identity beside captured settings. Admit only known fixed identities/enum/boolean, with no source bytes or auth information. Existing import_report and historical literal constructors use None; old stored reports stay unlabelled. Do not add a second provenance field to ImportCoverage.
- [ ] Useful intake uses import_report_with_conversion before destructuring ConversionResult. Its existing TransientSource.report carries that exact host report into threads.rs::begin, whose durable Source write already clones supplied.report. Preserve canonical locator, Source id, source fingerprint and scope; changing converter does not create a new Source. A later supplied conversion updates the latest Source report normally; existing revision history retains prior reports and full-note per-import reports retain their own variant. Do not invent one aggregate converter label for useful notes combining different sources: their aggregate report has conversion=None and exact source reports supply provenance.
- [ ] Retain threads_sources.rs::materialize_selected_sources whole-report equality guard before effects, now including conversion. A model-authored Source write may preserve its existing report but cannot add/change/clear conversion, create a duplicate host Source, or forge a historical unlabelled Source report. Extend threads_sources_tests.rs with actual prepare-loop negative variants and unchanged-report positive control. In source report adaptation/retained-useful-report code preserve source-specific provenance where copying a single report; leave combined-source reports unlabelled rather than selecting one arbitrarily. No new model authority or Source identity rule.
- [ ] Add useful-intake host handoff tests for all four settings combinations: exact captured identity reaches persisted Source.report before model dispatch, survives Store reopen/backup/restore, and remains the captured variant if settings change mid-job. Assert historical None stays None, same locator/fingerprint stays the same Source, and model attempts to add/change/clear conversion cause no canonical effects. No live provider is needed.
- [ ] Implement full_import_replay_with_conversion in keyed.rs: same key plus matching bytes/path/scope/title/options/expected engine identity replays without dispatch. Historical None provenance or different mode/AI/bundle must refuse ImmutableRequest before conversion/auth, not return None and later spend a model call. Keep the old historical replay method explicit.
- [ ] Include Source-only already-imported receipts and prepared-but-unapplied operations in replay fixtures. Validate the stored Source report or stored prepared request's provenance before returning to conversion when no new Note is in the receipt. A stored different or historical-unlabelled variant refuses before dispatch; matching variants retain the existing replay/resume behavior. An operation with no stored prepared request has not yet committed conversion provenance. Preserve exact candidate/replay rules; do not infer a new Note from a Source-only receipt.
- [ ] Implement import_full_scoped_with_conversion by sharing the existing checked group builder. Existing same-source/hash dedup additionally matches the Note's exact conversion provenance on this new path. A new key + different converter/AI/bundle produces a separate protected imported note, so the owner can compare results; it does not overwrite the old note. A new key + same exact variant retains existing already-imported behavior. Keep canonical Source identity and source byte hash stable, with separate Note import reports; no new Source per converter or relation parser/framework is needed.
- [ ] Commit Task2 after targeted core/app/intake/AI tests and strict affected lint. No paid workflow.

**Tests to implement**
```rust
#[test] fn preferences_default_stock_without_ai_and_survive_restore();
#[test] fn selected_helper_identity_mismatch_has_no_save_effect();
#[test] fn same_import_key_different_conversion_options_refuses_before_dispatch();
#[test] fn new_import_key_different_variant_retains_both_protected_notes();
#[test] fn useful_source_provenance_survives_reopen_without_changing_source_identity();
#[test] fn model_cannot_add_change_or_clear_source_conversion_provenance();
#[test] fn source_only_and_unapplied_replay_check_captured_variant_before_dispatch();
#[tokio::test] async fn ai_off_never_opens_auth_for_either_engine();
#[tokio::test] async fn ai_on_receives_selected_engine_output_for_every_kind();
```
Use existing maintained test runtime style if Tokio test macros are not in the approved feature graph. Missing test macros are not a reason to add a dependency.

## Task3: Settings controls, CLI and existing import surfaces

**Files**
- Create crates/brn-desktop/src/threads_documents.rs and tests in existing native test module.
- Modify threads_ui.rs registration/SettingsSection/import handoff and threads_tiles.rs Settings child only.
- Modify crates/brn/src/{main.rs,spec.rs,contract.rs} and exact help/README command entries.
- Add crates/brn/tests/document_modes.rs and app shared-route tests.
- Update docs/development/setup.md and architecture/status from selected to actual verified behavior only after tests.

- [ ] Native Settings adds Documents with the exact two labels and independent AI toggle. Reuse current shared settings service, no separate persistence or compare page. Display stored state on reopen and refresh.
- [ ] Capture preferences with the source request and select normal/adapted helper from trusted bundled paths. Neither new path discovers or requires PdfTools/Poppler; construct the reused host Converter with pdf_tools=None and use only convert_document. Old explicit compatibility APIs/tests may keep their tools, never as default/fallback. Never build a helper path from source content. Stop/cancel remains existing worker behavior.
- [ ] CLI adds one checked setting command document-conversion with --converter xberg|xberg-brn and --ai off|on; settings inspection exposes both. Native and CLI call choose_document_conversion. No new per-import UI or third mode.
- [ ] Remove conversion credential preflight when captured AI is Off. Useful-information's separate agent stage still has its existing credential requirement; conversion Off does not disable chat/semantic intake. AI On missing credentials preserves the normal safe unavailable error before a provider request.
- [ ] Route desktop/CLI imports through DocumentConversionJob/convert_document_job and the mode-aware shared import/replay methods. Read settings before keyed replay so a mode mismatch refuses before any auth or engine work. Both settings apply next import only; setting change by another process does not relabel the in-flight result. Show stored mode/AI provenance through the existing import-detail/report surface without changing the converted Markdown or adding a comparison screen.
- [ ] At CLI main.rs useful-intake TransientSource construction and native threads_ui.rs::useful_intake, pass captured settings alongside the converted result, call import_report_with_conversion, then assign its report to TransientSource.report. Do not reread settings at handoff. The native worker completion retains the conversion job's snapshot until useful_intake receives it. Render source report provenance in the existing source/import detail; preserve historical None and do not derive identity from current Settings. Add synthetic CLI/native handoff assertions as well as Task2 source anti-forgery tests.
- [ ] Test rendered controls -> shared saved values -> reopen, and actual CLI setting lifecycle. Process-level intake for all eight representative formats with Off must work without auth under both helpers. Synthetic provider process case proves On for both; missing/wrong helper and mode cannot fall back.
- [ ] Update exhaustive help/owner/outcome/spec tables and README together; run original agent_contract assertions. Existing protection/Project/private/source-import/Undo cases remain, changing only explicitly superseded output oracles with source-derived stock expectations.
- [ ] Commit Task3 and preserve prior failed suite results rather than relabel them.

## Task4: helper distribution and final integrated verification

**Files**
- Modify scripts/{make-macos-app.sh,package-macos-preview.py,verify-storage.sh,verify-end-to-end.sh} and their existing tests.
- Modify .github/workflows/ci.yml for explicit new tool builds/checks; preserve all required lanes.
- Update helper build setup and all eight format status in standard-default/verification.md.
- Existing final-integration records remain separate and are reconciled at actual integration.

- [ ] Bundle both real helpers, sign and hash them, relocate a synthetic app and execute both after removing source/build directories from discovery. No optional mode falsely offered without a usable bundled helper. CI uses the checked builder, separate lock/source identities and exact artifact paths.
- [ ] Generated sources live only under ignored target; verify clean-tree regeneration from cached SHA-checked archive plus recorded patches. No runtime download or source patching on the owner's machine.
- [ ] Record dependency/source/license custody for both bundles using existing audits and actual graph; resolve missing attribution before distribution. No release/push under this implementation task.
- [ ] Run helper-first focused suites, core/app/CLI/default workspace no-fail-fast, capability and applicable native/retrieval/combined lanes under existing verification instructions. Preserve carried PDFUnavailable and unexplained old SDK/widget qualifiers independently; do not skip, rerun-to-green or weaken tests to claim all-green.
- [ ] Run strict affected/helper/native lints, Rustfmt, Markdown links and whitespace; report actual counts and platform qualification. No live provider or native-human claim from mocks/widgets.
- [ ] Freeze one material pin/full diff and separate evidence handoff with source/lock/artifact/.d associations. Release all build/process ownership promptly.
- [ ] Principal requests a NEW actual-diff independent review of the full confirmed brief and whole implementation. Fix supported defects, rerun affected checks, preserve all failures.
- [ ] Complete actual INT-W/final integration and refreshed required CI before the already-authorized PR/merge; register and watch the PR, then verify merged-main CI. No component PR or premature readiness claim.

## Execution boundaries and self-review

Tasks1-4 are one coherent product change; do not create new feature projects for each file. The principal owns shared settings/schema/protocol/build interfaces and uses one implementation child with the existing execution method. Each task's explicit tests pin the five review-focus cases. A source-derived packaging obstruction is reported before manifest/version/authority expansion, not converted into another owner format-exclusion interview. Additional product decisions are not required by this plan.

The separate stock/adapted standalone builds deliberately avoid a workspace-wide patch or falsely labelling one patched engine as stock. The plan reuses maintained conversion and shared process/service mechanisms; custom parser and fidelity work is stopped. Future optional improvements need demonstrated owner benefit and a separate bounded allocation.
