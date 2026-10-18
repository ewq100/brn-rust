# PPTX order4 verified candidate — 18 October 2026

The bounded upstream candidate follows presentation declaration order, retains hidden slides, populates existing hidden-page metadata, and binds notes/comments through each actual slide's relationships. Identical final tests produce **before 2 passed / 19 failed**, **final 21 passed / 0 failed**. The unchanged namespace/S3 target passes **25/25**. Independent actual review and principal acceptance are pending; BRN adoption and full Office readiness are not claimed.

[Full allocation and exact association amendment](requirements.txt), [authored source facts](source-inventory.json), [commands and every selected outcome](check-index.json), [compiled source/artifacts](source-pin.json), and [patch provenance](../../../../../../../patches/xberg/1.3.10/0004-source-manifest.json) retain the review boundary. External evidence is under `/private/tmp/brn-x2-pptx-order4-20261018/evidence`; original namespace and S3 sources/artifacts remain untouched.

## Implemented boundaries

| Requirement | Actual change and proof |
|---|---|
| Declared presentation order | Direct PresentationML `sldIdLst/sldId` sequence; exact qualified relationship ID, package relationship namespace, exact slide type and internal target. Existing container target resolver supplies package-relative paths. Quay is slide9 then slide2. Nonnumeric Omega/Alpha controls reverse both relationship-file and ZIP order; unreferenced slides are excluded. |
| Hidden state without content loss | Same existing slide DOM reads truly unqualified `show`. Absent/true/1 becomes visible; false/0 hidden. Private state reaches existing `PageInfo.hidden` by declared ordinal. All six authored boolean/foreign-attribute pages retain exact content. Invalid lexical state is source-implemented as unknown plus warning, but that invalid-lexeme branch has no dedicated runtime assertion here. |
| Source association | Authorized amendment changes only `extract_all_notes` and `extract_comments`, plus shared private container/parser query plumbing and exact comments. Real notesSlide/comments relationships resolve relative to the actual slide. Existing text parsers, author map, revision delta and anchor representation are unchanged. Quay's first note is recovered; reversed/nonnumeric comments have complete exact JSON assertions. A missing first slide leaves the second page at ordinal 2 and its comment at zero-based index 1. |
| Optional/invalid metadata | No relationship means normal absence; orphan notes/comments are excluded. Foreign element/type-substring/external/escape/missing/malformed/ambiguous controls cannot supply metadata. Declared loss receives a specific speaker-notes/comments warning. A real empty comment part remains `Some(empty)`. Existing malformed slide-rels parsing can also skip the affected slide with its existing warning; no new completeness claim. |
| Empty, missing, invalid sequence | Present empty/no-list/foreign-child lists yield zero pages and the existing omitted page-collection envelope. Present malformed declarations, missing/unresolved IDs, duplicate references/IDs, foreign namespace/type/attributes and external targets fail with the precise presentation-order class. Missing slide parts preserve ordinal gaps and warnings. |
| Narrow recovery | Only actual absence of the presentation part permits numeric recovery. Present recovery relationships use exact namespace/type/internal-target checks; malformed/external recovery does not silently scan files. With both declaration and relationships absent, existing numeric filename recovery remains. Valid recovery relationships retain numeric order. |
| Admission/count limits | Existing member reads, ZIP validator, entry count, slide limit and all original limits are unchanged. The two-page hidden/visible control refuses at max_pages=1 and succeeds at 2. This does not establish complete XML/preallocation/actual-inflation/work bounds. |
| Existing content/projection | Quay asserts every public node's ordered content/page, all six addressed table cells/spans, full source wording and three picture placeholders on their proper pages. Whole page objects equal old source bodies after only source-authored order/ordinal and missing-note correction. Harbor's whole public value changes only by two `hidden:false` additions. Images/OCR remain off: placeholders are public Paragraph content, not decoded image or BRN occurrence/locator proof. |

The roxmltree string-name attribute lookup accepts any namespace. The actual foreign `f:show` failure led to the narrow private unqualified-attribute query, also used for the new OPC queries. No global namespace/parser framework, XML rewriting, second reader, new schema or backend flag was introduced.

## Verification and custody

Only six upstream production files differ from accepted namespace Xberg `00ba3b3183719643121824b48ce288a492c2c111`. Compiled final Xberg is `2aefc5301155bfddb19f48266e08acbb2407efd6`, tree `49445f5aee77a179ddfca7e8731d623324f655ab`; exact before/final harness is `2904a8775787eef216451261c732fd82a28d7787`. Both roots are genuine Git checkouts with separate fixed targets and 0700 private temporary directories. Rust 1.98.1, locked/offline, jobs 1; no source edit occurred during Cargo.

Actual compiler records show the same **239 registry identity/feature unions** as accepted namespace verification. Xberg is a normal dependency (`cfg(test)=false`) with emitted `html/notebook/office`; the harness directly requests Office with defaults off. Harness lock and upstream manifests/lock are unchanged; root lock remains `9d1240de6b039bdd9b34892c4d014fa73f7390b3fe3ddf91604d86d4f048f229`. No metadata reconciliation, new dependency, feature, build script or package source was added. Previously audited build sources executed under that same graph; raw compiler/build-script records remain retained.

All **30 accepted namespace DOCX/consumer JSON values** and **24 DOCX input vectors** compare completely equal, including corrected features and prefix results as the current baseline. All eight original S3 and 17 namespace controls are byte-identical. The shared PPTX OMML function is byte-identical. Strict all-target **harness** Clippy with `-D warnings`, direct owned Rustfmt, JSON/doc links/whitespace and independent cached archive → 0001 → 0003 → 0004 reproduction pass. Full 765 package files reproduce; nine accepted archive-external fixture/test adjuncts are unchanged separately. Dependency-wide lint, whole upstream cfg(test), native PDF and root BRN runtime checks were not run.

Receipts retain every command, literal root, precommand Git/tree and dirty snapshot, selected source bytes, environment, full compiler messages, logs and before/after artifact lists. Content-addressed objects preserve old/new executables, Xberg rlib/rmeta, dep-info and activated build artifacts before overwrite. The final source pin selects the actual before/final test executables, namespace executable, Xberg libraries and `.d` records; no future commit identity is fabricated.

## Preserved attempts

| Attempt | Actual disposition |
|---|---|
| 01 | E0308 in new harness: accidental PageConfig in force_ocr_pages. No behavior execution. |
| 02 | 0/14; mixed intended behavior failures and mistaken level-1 public heading expectations. Preserved without relabelling all failures as intended red. |
| 03 | Correct public heading projection: 2/12. Exact Quay notes and reversed comments motivated the separately approved amendment. |
| 05–06 | Expanded association controls: 2/16; production baseline unchanged. |
| 07 | E0599 at two new resolver calls: used Option method on maintained Result API. No behavior execution. |
| 08 | Recovery controls and complete Quay projection: 2/18; real external recovery acceptance preserved before its correction. |
| 09 | 18/2: real foreign-show production failure plus test-only empty-result unwrap mistake. Both separately diagnosed before correction. |
| 10 | 2/19 on original production with unqualified OPC attribute negatives. |
| 11 | 21/0 before the final missing-ordinal association strengthening. |
| 12 → 13 | Exact identical final driver: before 2/19 → final 21/0; no ignored/filtered tests. |
| 14–15 | Unchanged namespace/S3 25/0; strict harness Clippy succeeds. |

An uncompiled formatting attempt is preserved; original 120-column package formatting was restored before minimal export. Static checker mistakes are retained separately: initial reproduction accidentally included nine archive-external adjuncts, and an artifact receipt script shadowed a byte variable. Corrected static checks pass; neither is a behavioral result. Historical O1, A1, S3 and namespace verdicts/captures/registries are unchanged.

## Remaining gates

This measures and fixes presentation order/hidden metadata and the two demonstrated slide-owned notes/comment associations. Broader note/comment identity, authors-part relationships, annotations/locators, nested/merged table geometry, hidden shapes, inherited layout, rendered pixels/OCR/chart fidelity and full semantic source inventories remain open. Existing O1/S5 registry entries remain frozen; no broad O2 or all-eight parity closure is inferred.

Full XML/DTD read-set, preallocation/heap/actual expansion and repeated-work qualification, complete source provenance, restricted Office runtime, license custody, root adoption and reader retirement remain open. The provider-interrupted PDF task stays frozen/unaccepted and was not inspected. Carried PDFUnavailable, SDK/widget/platform/INT-W/native-human/owner/program/main/live gates remain unchanged. No helper/host/BRN production, manifests/root lock, old patches/oracles or registry changed; no provider/Auth/network/GUI/mutants/PR/push/merge ran.
