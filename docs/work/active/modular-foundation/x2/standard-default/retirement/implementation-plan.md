# Retire the superseded conversion pipeline

18 October 2026. Lead-authored follow-through to the owner's question about unused legacy PDF tests. This amends the earlier transition-only retention in the [feature plan](../implementation-plan.md); the [two-mode feature brief](../feature-brief.md) is unchanged. Source base `8d72901e25114784e046cb2c89ba1c8c2e9eeb68`, clean work/x2-office-prep-20261018. Read the complete [caller facts](source-facts.md) and [99-file source map](source-map.json) before implementation. No removal has been performed by this document.

Goal: remove the old readers, prepared-page AI protocol and their obsolete tests/build steps together. Preserve the two current Xberg helpers, optional AI refinement and all active safety/data compatibility. Repository-bounded caller inspection is the compatibility basis; these private publish=false workspace APIs have no assigned external API stability requirement. Historical artifacts and Git retain the old implementation. Do not invent compatibility shims or a third converter to retain dead code.

## Boundaries

- Keep the exact accepted four Xberg patches, stock source/configuration, optional UTF8 projection, helper runtime/sandbox, all current Settings/provenance/scope/Save/review/replay/Undo and schema9→10 compatibility. No new parser, converter, dependency version, schema or paid activation.
- Do not change Skill/MCP/Stop behavior or remove their tests. The active skill-lock failure is independent. Historical failures/receipts remain immutable evidence, not retrospectively passing runs.
- One serialized heavy slot, Rust1.98.1, fixed literal source/targets, fresh0700 external TMPDIR. No compiled-input edits during Cargo. Preserve old binaries/.d/source/locks before overwrite. No global cache cleanup.
- Shared safety tests are migrated to the current path before obsolete harnesses retire. A failed old PDF test disappears only with its old implementation/callers, never through ignore/skip or assertion weakening.

## A: retain shared transport and light host contracts

Files: brn-ai/src/conversion.rs, conversion_tests.rs, document_refine.rs/tests and examples/qualify_conversion.rs; brn-threads-intake/src/{lib.rs,documents.rs,process.rs,transient.rs} and its qualification/document_modes tests. CLI/native/test constructors and CLI contract exhaustive arms change only for removed legacy fields/variants.

- [ ] Leave the module name conversion for active MODEL, ConversionProgress, ConversionFailure, conversion_response and require_natural_finish, avoiding a naming-only public caller migration. Remove old ConversionJob/convert/prepared candidate/plan APIs, old prepared prompt/parser and their imports. Preserve the current transport operation ordering, auth/model/effort/deadline/natural finish/cancellation. Remove only now-unreachable error variants/exhaustive arms. Current document_refine remains the only conversion pipeline.
- [ ] Remove the auto-discovered qualify_conversion example and old ConversionPlan-specific tests. Preserve its old source in Git/evidence; current commands/docs must not advertise it. Before deletion, map old no-tools/auth/terminal/cancel assertions to existing document_refine tests; migrate a genuinely uncovered active property into that suite rather than preserving the old page protocol to test it.
- [ ] Simplify the current host types to the fields actually used:

```rust
pub struct Converter { pub temp_root: PathBuf, pub limits: IntakeLimits }
pub struct ConversionRequest<'a> {
    pub kind: DocumentKind,
    pub intent: ImportIntent,
    pub bytes: &'a [u8],
    pub source_reference: &'a str,
}
```

Remove old Converter::convert/prepare/finish_candidate, PdfTools/pdf_tools.rs, docx_markdown.rs, old preview/geometry/crop prepared.rs, inventory/check_inventory types/functions, old-only PDF error variants. Keep current DocumentHelpers/convert_document, result/assets/coverage, input reader, limits and generic errors. Make exact compiler-required constructor field deletions in current hosts/tests; do not change their captured choice or destination semantics.
- [ ] Keep process::run's selected-executable/env_clear/pipe/output/deadline/PGID/reaping/storage checks. Remove old run_renderer/Renderer/Fontconfig/Homebrew profiles and only callers exclusive to them. Remove the obsolete restriction selector argument once current run always uses the existing false branch; preserve that branch's behavior exactly. Keep shared transient cleanup.
- [ ] Split qualification.rs before retiring it: migrate bounded file-read/cancellation and expired/symlink transient cleanup to a current safety target (intake_safety.rs), exercising the current helper route for cleanup where needed. Retain process cancellation/tree tests on current run/helper. Remove only old readers/page-preview tests. Keep shared fixture bytes referenced by current routes, patch controls or source inventories.

## B: remove legacy helper and dependency edges

Files: brn-intake/src/lib.rs, helper.rs/helper/**, prepared.rs/prepared/**, bin/brn-intake-helper.rs, Cargo.toml, old helper-specific tests; brn-threads-intake/Cargo.toml; root and both standalone locks. Existing new tools and shared helper_runtime stay.

- [ ] Remove old HelperRequest/Extraction/SourceNode/ImageOccurrence/IntakeUsage/SizeCounter and converter constants after callers in A retire. Preserve IntakeLimits/used caps, ImageAsset codec, digest/hex/asset_file_name and complete bounded decode_png. Transfer generic PNG rejection/size/hash witnesses from protocol.rs to a light document/asset test before old protocol snapshots retire.
- [ ] Remove the old helper binary, feature and its reader modules plus old-only maintained/docx_inventory/pptx/office_differential/old restricted protocol tests. Preserve current helper_runtime exactly; new main still rejects unused --prepare. Current tools' identity/restriction/process-tree tests cover the actual shipped binaries. No accepted optional upstream patch or stock library change.
- [ ] Remove only proven unused direct edges: the 13 helper-only brn-intake edges and old comparator tl; prepared-only light schemars/unicode-normalization; old PDF-only threads-intake png/quick-xml. Actual retained references override a deletion guess. Do not remove a transitive package merely because another package shares its name.
- [ ] Reconcile each affected lock OFFLINE once, retaining all surviving name/version/source/checksum identities. No new/upgrade/downgrade packages. Record direct-edge and unreachable-package removals before building. If solver changes a surviving identity, stop with exact delta rather than adopt it. Both standalone brn-intake lock nodes change with the light manifest.
- [ ] Archive the exact owned generated target/xberg-adapted generation to the new task's external custody, preserving its manifest/hash receipt, then regenerate at its same declared ignored path with the updated committed adapted lock. Do not mutate a registry cache or broaden the generator's occupied-output policy. Four patches/full765-file source reproduction remain identical.

## C: current tooling, examples and retirement guard

Files: CI workflows and setup/build/verification scripts; check-intake-retirement.py plus focused tests; intake/desktop/setup/verification READMEs; evaluate-wiki-guidance.py/evaluate_wiki_v2.py and their offline tooling tests.

- [ ] Remove old helper builds/env and Poppler setup from current CI and verification scripts. Keep all required lane identities, real new helper build/lint/tests, scripted-document-tests, root/native/retrieval checks and artifact availability failures. Build dependency fetching remains allowed only in normal CI setup, not runtime or local unpinned downloads. Fetch the committed STOCK helper lock before generation now that root no longer fetches Xberg; generate adapted source, then fetch adapted lock. The documentation generator-test lane also needs the stock archive from this exact fetch. Subsequent builds remain locked/offline.
- [ ] Keep maintained wiki evaluation tools, including their active research-helper settings and safety/metrics. Extend Cli with optional keyword-only stock_helper/brn_helper paths; default them to each tested executable's new siblings, and populate BRN_XBERG_HELPER/_BRN explicitly. The old helper argument is optional and used only as BRN_INTAKE_HELPER for explicitly supplied external historical binaries; never discover or require a newly built old helper. Add --stock-helper/--brn-helper options and preserve explicit --helper as documented historical-binary support. Do not alter live activation, provider selection, retry budgets or execute evaluation runs. Offline fake tests verify environment/artifact choice and no provider starts.
- [ ] Extend the existing retirement checker to reject live legacy modules/binary/feature/dependency/build invocations without banning old strings inside historical evidence, accepted upstream patches or explicit historical eval support. Add negative synthetic source-tree tests so the checker proves the removal boundary instead of claiming prior checks already covered it.
- [ ] Update CURRENT instructions to new build/import commands. Keep historical reports/verdicts/source hashes and reused input bytes. If a current Markdown link points at a deleted old source, replace only that source link with its archived or pinned-Git reference; retain original findings/result text and record link-only maintenance. No bulk rewrite of historical evidence.

## D: prove current behavior and review the complete diff

- [ ] Before code deletion, preserve original Task4 failed skill/PDF/help evidence and source/artifacts. Verify new helper baselines already passed; meaningful removal proof is the post-change no-callers/no-dependencies checker plus actual current routes, not a contrived missing-symbol compiler error.
- [ ] Run light PNG/DTO/image/refinement safety, all8×2 actual no-auth helper/CLI paths, gated synthetic On for both modes, captured scope/provenance/mismatched-key/no-dispatch tests. Current protected save/version/replay/Undo/schema9 upgrade/backup/restore tests remain unchanged. No deleted legacy scenario is counted as a newly passing test.
- [ ] Run the default build, strict workspace/tool/scripted lint, and ONE full stable workspace --no-fail-fast run after this broad removal. Keep raw results even if the active skill failure recurs; no broad retry-to-green. Compare selected feature fingerprints and account for removed targets/counts explicitly.
- [ ] Rebuild both helpers and native-ui artifact against the pruned light graph; update notices/source custody; run actual signed/relocated startup/reopen/foreign refusal and both-mode all-format checks. Old package/source pins remain historical. Recheck native Documents/scope/intake controls and appropriate changed native/combined lints/builds; do not repeat unchanged broad lanes without a concrete dependency risk or failure.
- [ ] Run tooling, retirement negative controls, Rustfmt/links/whitespace and offline maintained journeys. Keep non-Mac/current UI/live/resource/fidelity qualifications distinct. No renderer installation or encrypted-PDF work.
- [ ] Freeze one final material pin and whole feature diff (base612144c5 plus this retirement), preserve source/lock/artifact/.d receipts, release all processes/paths, then request NEW independent whole-feature actual-diff review including deletions. Final INT-W/shared integration, required PR/main CI and owner authorization still govern publication/merge. No component PR or self-acceptance.

Principal self-review: product choices are unchanged; all deletion families have observed callers and shared exceptions; current data/authority tests stay; every affected build/eval/example consumer has a disposition. The independent actual-diff review is the meaningful code/deletion review. This bounded migration does not assign a new custom fidelity or skill feature.
