# Exact proposed all-document allocation (not granted by this document)

Current writable scope is **x2/** and one checkpoint in modular-foundation/plan.md. All production/test/manifests/root-lock/CI/package/canonical docs remain principal custody. Source map is at65684dcb; new names below are proposals, not existing APIs. [Latest owner scope](owner-scope-all-documents.md) approves common document-backend integration; prior PDF-only restriction is superseded; principal exact allocations/heavy scheduling still precede edits/builds. Do not transfer INT-W ownership or change other worktrees.

## X2a: common boundary/source audit + first PDF implementation slice

| Proposed file/hunk | Responsibility and limit |
| --- | --- |
| `crates/brn-intake/Cargo.toml` helper feature and dependency row ONLY | Add dep:xberg to helper; exact optional line below. No root manifest/Rust pin/runtime feature. |
| root `Cargo.lock` (principal) | Controlled single combined resolution and full graph/hash diff before builds. Existing packages/checksums unchanged; xberg/native1.3.10/lopdf0.45.0. No dedup/upgrades or copying standalone lock. |
| new `crates/brn-intake/src/helper/documents.rs`; new `src/helper/pdf.rs` | Shared private Xberg invocation/config/budget adapter (existing HelperRequest→BRN DTO interfaces, no registry), first PDF byte reader/schema1 projection. Exact per-format mode; native API audit prerequisite. Later families reuse this same adapter. |
| new `crates/brn-intake/src/helper/prepared/pdf.rs` | PageText+PNG previews only; source points/geometry/rotation, transcription and per-page identity/usage; no new model evidence kind. |
| `src/helper.rs` module declaration + extract pdf dispatch; `src/helper/prepared.rs` module/pdf arm + measured usage finalization | Reuse unchanged other-format readers and common validation. First PDF slice is top-level only; X2d later preserves and extends the same bounded MIME context explicitly. No accidental recursion/feature admission. |
| `src/lib.rs` converter constant near20 ONLY | Add exact PDF_CONVERTER; no public DTO/types/schema change. Reuse existing PNG/limits. |
| `src/bin/brn-intake-helper.rs` failure-output allowlist hunk ONLY if needed | Existing restrictions/profile/resources/startup/env check READONLY. Do not interpolate Xberg errors. Existing public CLI arguments unchanged. |
| `tests/restricted_helper.rs` focused real PDF cases | Existing binary under env_clear and restriction-before-input, denied file/live localhost canaries and actual process tree cleanup; test both protocol modes. No broad sandbox allowance. |
| `tests/protocol.rs`; new `tests/pdf.rs`; `src/prepared/tests.rs` focused PDF assertions | source/limit/profile/converter/page/hash/PNG roundtrip, independent source line+image inventories, incomplete/over-limit/diagnostic/forged evidence; keep all prior assertions. |
| new `tests/fixtures/pdf/**` within brn-intake, or direct include_bytes of existing maintained PDFs | Checked synthetic malformed/encrypted/rotated/CropBox/Unicode/font/adverse fixtures plus independently authored inventories. Do not copy/generate owner documents or overwrite maintained PDFs. No fixture-generation dependencies unless separately allocated. |

```toml
# existing helper = [...] gains only "dep:xberg"
xberg = { version = "=1.3.10", optional = true, default-features = false, features = ["pdf"] }
```

Subsequent dependency grant should explicitly permit normal exact Rust package acquisition/source inspection (no build script execution during acquisition), followed by the principal root resolution and graph review. X1 full lock has the exact known checksums, but is not a current production dependency approval. If Cargo selects a newer native transitive or changes an existing package, stop before adoption/build and submit the minimal constraint/lock proposal; no unallocated unused direct native dependency.

The source-audit checkpoint must settle preallocation/expansion enforcement before claiming the reader safe for supported untrusted PDFs. No hard-RSS mechanism or smaller accepted input profile is yet verified. Production helper restriction edits, new resource quotas or a PDF pre-parser are explicitly NOT included; return exact source-backed proposal if necessary.

## X2b: host switch/retirement/whole-flow proof, after X2a

| Proposed file/hunk | Exact limit |
| --- | --- |
| `crates/brn-threads-intake/src/lib.rs` imports4/80–87, convert dispatch207–215, document242–347, pdf375–575 | Retire PdfTools field/module and old PDF function; route PDF via document helper request, choose exact PDF_CONVERTER, retain source/limits/usage and content-only result projection/gap classification. Other formats/coverage APIs unchanged. Keep public ConversionError compatibility categories unless an exact need is reviewed. |
| `src/prepared.rs` imports7–8, prepare23–58, prepare_pdf270–395, pdf_renderer418–440, parse_pages459–797 | Replace PDF exception with helper --prepare request, validate one source block/preview per complete numbered page; remove bbox/discovery code and only its tests. Keep finish_candidate/crop geometry/formatting/evidence logic. |
| `src/process.rs` run21–43 and only obsolete profiles45–70/111–129/228–311 + actual boolean callers | After call search, remove renderer/external-Process profile/fontconfig/Homebrew route and obsolete restrict bool; one existing supervised helper path. Preserve absolute path/env_clear/PGID/pipes/output/deadline/join/kill/reap/storage handling. Mechanism-specific tests become helper restriction witnesses, not deleted safety assertions. |
| `src/pdf_tools.rs` | Retire after callers removed; no discovery/override/fallback remains. |
| `crates/brn-threads-intake/Cargo.toml` quick-xml direct dependency ONLY | Remove only when last bbox caller gone. Keep helper Office quick-xml0.41; Xberg may add0.42, no dedup. |
| `crates/brn/src/main.rs::converter`926/929 ONLY | Remove discovery/local var and PdfTools field; no CLI API/Auth/conversion/save behavior. |
| `crates/brn-desktop/src/threads_ui.rs` import3 and converter3575–3580 ONLY | Remove PdfTools import/field; no worker/writing/queue/composer/geometry change. |
| `crates/brn-ai/examples/qualify_conversion.rs` import4/constructors65,204; `src/conversion_tests.rs` constructor427–433 ONLY + focused source-bound fixture assertions | Mechanical field removals; retain actual fixed Luna/high/account/tool-free/cancel/schema/output mock assertions. No conversion.rs production edit. Extend its existing private model_candidate/DynHttpClient mock test with actual helper PDF evidence; same fixed Chatgpt/Luna High, source-bound plan/crops and finish_candidate. Existing CLI support/stub.rs is Copilot chat-only: do NOT claim it can mock the fixed Luna subscription route. |
| `crates/brn-threads-intake/tests/qualification.rs` constructor5–22 + PDF cases40/232/309/432 | Genuine helper-route before/after, inventory/coverage/no raw payload; replace disk-copy cancellation trigger with deterministic existing process/pipe handoff barrier, actual during and post-completion cancellation, obsolete-tools canary. No sleeps as race proof. Keep other formats. |
| prepared/process in-file test sections | Replace only retired bbox/fontconfig-specific assertions with actual native PDF evidence; keep same stricter crop/receipt/liveness/out-of-bounds/process-tree assertions. |
| new `crates/brn-threads-app/tests/xberg_intake.rs` | Real checked Workspace receives actual conversion result; general/Project/private attach/import, scope/protection/replay/Undo/backup/restore/offline assets and no original bytes. App/core production READONLY. |
| new `crates/brn/tests/xberg_intake.rs` | Normal CLI helper refusal/no-Poppler/no-save before provider plus existing provider-free text/save/replay controls. Successful fixed-Luna mock proof stays in the existing AI transport test seam and checked host-service tests; a full normal CLI Luna success requires later separately activated live qualification or a precise test-only allocation, never production auth interception/new driver. |
| `crates/brn-desktop/src/threads_intake_tests.rs` conversion cancellation/completion cases ONLY | Existing synthetic state/worker queue can prove cancel-after-result-before-save; preserve unexplained unrelated run-stop test and old failures. No GUI. |

## X2c/X2d: exact current-family adaptation proposal

- `helper/documents.rs` shared configuration/dispatch/projection entry only; `helper.rs::{extract_with_budget,email,MimeContext}` and `helper/prepared.rs::prepare` current format arms. Existing `{html,pptx,xlsx}.rs` private prepared modules are adapted to project verified Xberg rich results instead of invoking a second full reader. Existing `helper.rs` DOCX/EML functions retire only as their exact contract replacements pass.
- `crates/brn-intake/Cargo.toml`: add only per-slice explicit `office`, `excel`, `email`, `html` to the exact optional Xberg dependency after source/graph gates. office already closes over html/notebook; no aggregate formats/full/default. Each feature change/root lock delta needs principal allocation. Keep existing pinned readers until source-proven compatibility needs and retirement are reviewed.
- `brn-intake/tests/{maintained,pptx}.rs`, `src/helper/prepared.rs` tests, `src/prepared/tests.rs` and new `tests/xberg_documents.rs`: whole current format inventory, typed receipt/value/MIME/image negatives, resource and exact executable tests, no assertion weakening. Existing hostile DOCX/MIME fixtures remain authoritative; additional all-family fixture paths `tests/fixtures/xberg-documents/**` need bounded synthetic provenance.
- `brn-threads-intake/src/lib.rs::document/convert` exact kind dispatch and converter identity; `src/prepared.rs::prepare` same helper mode/format/source/usage checks. Add schema1 projections for HTML/XLSX and enable existing PPTX schema1 intentionally; no public ConversionRequest/DTO rewrite. `docx_markdown.rs` only retires if the replacement already provides the same readable-content normalization and its safety assertions are ported.
- Tests `brn-threads-intake/tests/qualification.rs`, existing private `brn-ai/src/conversion_tests.rs`, new app/CLI xberg_intake tests cover all eight kinds with the limitations above. No app/core/runtime production hunk proposed. Mechanical constructors remain the exact X2b list.
- Any source-demonstrated missing rich field gets a separate **named, always-used metadata compatibility projection**, ideally reusing an existing bounded maintained reader for that field only. No fallback after extraction failure, guessed reconstruction from Markdown, whole old-reader pipeline or permanent duplicate catalogue. Current APIs are unavailable, so no compatibility gap is yet a proven Xberg defect; source inspection narrows the exact hunk before edit.
- `email` dependency lower bound^0.11.9 does not satisfy BRN’s existing=0.11.8; `excel` requires Tokio^1.53.2 above locked1.53.1. These edges are manifest facts, not an executed resolver result. Recommend separate principal exact version/feature allocation plus full affected MIME/provider/runtime checks if necessary; no silent upgrade. Do not use excel-wasm to evade qualification without source/platform evidence.

Additional families are mapped in formats.md. No new DocumentKind/CLI extension/MIME admission or new API/schema hunk is allocated merely by an Xberg feature name. Submit the exact mapping/DTO/test coupling after package registry inspection; this preserves the broader adoption objective without assuming every compiled parser is user-qualified.

## Packaging/shared documentation allocation for final candidate

- `.github/workflows/ci.yml:68–79`: retire brew Poppler and obsolete exports ONLY, preserve helper/full/native/retrieval gates; no check suppression. Actual non-Mac helper refusal stays, test/build capability claims platform-qualified.
- `scripts/check-intake-retirement.py`: bounded explicit production no-Poppler/obsolete override check, complementing real canary tests; not a new policy framework.
- `scripts/package-macos-preview.py:77`: external_pdf_tools becomes empty after proof; include bounded licence/notice inventory in the existing build metadata if needed, retain artifact/hash/arch/minOS/signing/refusal/publication guarantees.
- `scripts/make-macos-app.sh`: exact resource-copy/presence hunks for new `licenses/xberg-pdf/**` notices only; helper bundling stays matched. New notice files must reproduce acquired package licences/DejaVu/vendor-fontdb/PDFOxide attribution and selected-transitive obligations (incl option-ext), not broad legal clearance.
- `scripts/tests/test_macos_packaging.py::qualify_app`: real bundled helper both PDF modes plus current EML/startup/reopen/foreign folder assertions and notice presence; no GUI.
- `docs/development/{setup,verification,preview-release}.md`, `docs/architecture/{dependencies,overview}.md`, `crates/{brn-intake,brn-threads-intake,brn-desktop}/README.md`: precise helper/PDF backend/build/limits/notice/platform facts; canonical status/plan acceptance summaries remain principal-owned until allocated.
- `scripts/verify-{storage,end-to-end,desktop-shell}.sh`, core/app production, auth.rs, dependency pins outside the exact approved delta, runtime guides and database/schema: READONLY; their maintained commands are reused rather than rewritten.

This is the complete proposed minimum coupling found at current source. Compiler-required additional callers or a concrete resource/host defect require a source-backed exact hunk request before edit. No blanket crate ownership, new public API, format expansion, paid call, mutation campaign or shipping authority is implied.
