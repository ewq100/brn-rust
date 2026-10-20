# Independent evidence review: embedded PNG retention

Reviewed evidence: `/private/tmp/brn-image-retention-proof-20261020`, base `9c18bc5f5aab0944335c2d5614b3f3009d8401b1`, and `/tmp/brn-image-retention-preresearch.md`. This is a bounded experiment-evidence review, not a production-candidate approval.

## Verdict

The requested interpretation is supported. The pinned stock SDK returned the fixture's actual embedded PNG bytes with a test-local image-extraction setting. That alternative setting is **not qualified for BRN**: the existing checked projection refuses the result because SDK drawing dimensions differ from the PNG's pixel dimensions. Both recorded one-test attempts failed (0 passed, 1 failed, 7 filtered; lead reports exit 101). No successful retained-Asset, helper-subprocess, CLI, native or persistence result is established.

No actionable finding against that interpretation or the decision to defer production changes. The first test's exact image-count expectation is a witness limitation, not by itself an SDK-defect finding.

## Independently checked facts

- Inspected the synthetic DOCX ZIP read-only, without extracting files. Its source SHA-256 is `d753d9a98afe1771c0d0488871e322900ac97f74a5390feb9911fcadc9a0fc4c`. It has exactly one `word/media/` member, `word/media/image1.png`, 13,308 bytes, SHA-256 `86e6c9325de51d5c6b84dfd5aad8f5b4d09cfccc447a1626f8be3380e7700599`; its PNG IHDR is 940×340. These facts match the experiment constants and both raw-metadata log entries.
- The document has two drawing occurrences: the first contains `rId11` and extent 5,212,080×1,885,220 EMU; the second is an authored chart using `rIdFreshChart`. The local registry DOCX extractor (`xberg-1.3.10/src/extractors/docx.rs:1087-1184`) iterates drawings, reads bytes only for resolvable image relationships, and derives returned width/height from drawing extent at 96 DPI. The first therefore reports 547×197. A chart drawing need not supply an embedded raster image relationship, explaining the second empty entry without proving an erroneous duplicate PNG.
- Both raw logs and `captures/footnotes-image-retention-sdk-metadata.json` record one successful SDK document, no SDK errors, the matching PNG hash/13,308-byte payload, dimensions 547×197, no OCR result, and an additional zero-byte entry with null path/dimensions. The test's `images.len() == 1` assertion conflates one package PNG with the SDK's drawing-entry inventory. That count failure does not show that the real PNG was lost, duplicated or corrupt.
- Shared production `engine::config` still has `images: None`, OCR disabled, embedded-image OCR false, no captioning and QR detection false. The test changes only a local config instance to `extract_images: true` and explicitly `run_ocr_on_images: false`; it does not change shipped defaults. Existing published-default policy assertions remain intact.
- The projector copies SDK dimensions into the retained PNG asset (`tools/xberg-intake-helper/src/project.rs:72-79`), then invokes `result.validate_for(request)` before returning (`:124`). Validation decodes PNG dimensions and rejects a mismatch (`crates/brn-intake/src/document.rs:138-150`). The diagnostic capture reports `projection_error: document image dimensions/budget`. This fixture's 319,600 decoded pixels and 13,308 image bytes are well below the default quotas; the dimension mismatch supplies the concrete rejection reason. The guard is enforcing the existing contract correctly.

## Attempt and source custody

Read both full patches, both logs, `run.sh`, captures, source/test identity receipts, current engine/project/document validation and build identity, published-default fixture assertions, fixture documentation and relevant local pinned SDK source.

The first patch SHA-256 is `de378eeb14f6bc5908bb07e6c349b4cf357430c4214ce0fbefa5d7dcc58dcdf6`; its log identifies test-file SHA-256 `e5ba4a324e7f8c71ba6940234256fe19193d21a69f5c5368cdc19a5d667cc5e2`. The diagnostic patch SHA-256 is `cc2c43da60b4f36b8a1f4ae16417edb1c93feac4037ca3df3a880f26497ed019`; its log identifies test-file SHA-256 `3f8c07d6a00c9d61d731441a5d3444d1b3f389a8272f22a008739cfdea31c9f2`. Both logs name the same base commit and actual one-test execution.

Diffing the preserved patches confirms that the second attempt adds only early diagnostic projection/validation capture before the original count assertion. It does not remove or relax any expectation, replace dimensions, filter the raw entries or alter production logic. Technically it adds an extra diagnostic call rather than moving/removing the original later projector call. That later call and all original dimension/hash/link/asset assertions remain present and unreachable after the count failure. The checked projection internally reaches validation and fails; the diagnostic branch's success-only validation field is consequently not executed.

No successful asset-envelope capture was produced. The shared raw metadata capture path is reused by the two attempts, so its final file is the second run's capture; the first run's metadata remains separately preserved inline in `first-run.log`. The current tracked product configuration, projector, validator and standalone manifest/lock showed no diff during inspection. The review relies on the frozen patches/logs, not continued presence of the temporary test edit. No repository changes were made by this reviewer.

## Limits and safe disposition

Capability is demonstrated for one embedded PNG in one synthetic DOCX under the pinned stock SDK and test-local configuration. The in-process call does not run the restricted helper subprocess or host budgets, establish adapted-helper behavior, prove larger/adversarial inputs or every resource limit, demonstrate hard RSS limits, establish general Office/PDF image fidelity, persist an Asset, qualify backup/restore/export, or establish native usability/owner acceptance. Later byte/dimension/link-relocation/caption/asset-count/gap assertions were not reached; the experiment is failed qualification even though its raw-byte capability observation is useful.

Keep the existing published defaults and guards. Retain both failed attempts and the precise mismatch; defer retained-image policy/integration to separately selected work. Any future selected engineering work would need to resolve the layout-versus-pixel contract, occurrence/placeholder semantics, provenance/configuration identity and actual restricted-host qualification before adoption. This review does not choose a normalization or add an owner-approval request for that out-of-scope future product choice. It authorizes no source fix, rerun, new control or production default change.

This reviewer performed read-only source/log/ZIP inspection and wrote only this requested report. No builds, experiment repeats, network, provider/account actions, native interaction or nested work were performed.
