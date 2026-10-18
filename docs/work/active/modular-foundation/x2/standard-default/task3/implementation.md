# Task3: native and CLI document choices

The [approved brief](../feature-brief.md) and [complete plan](../implementation-plan.md#task3-settings-controls-cli-and-existing-import-surfaces) are implemented for Task3. Material `74feabeaee4ea9f1c8e551e4655d5c40c752a7d7`; actual source/build associations are in [source pin](source-pin.json), with every attempted Cargo command in [check index](check-index.json). This is local verification, not whole-feature acceptance.

Native Settings → Documents offers Xberg (default) / Xberg + BRN layer and independent AI conversion Off (default) / On. Controls use the shared checked preference service and refresh canonical state, including Deferred while Working. CLI adds checked `document-conversion --converter xberg|xberg-brn --ai off|on`; spec/owner/outcome/help/README tables and explicit default settings output agree. Changing preferences does not convert files or alter existing notes.

Both hosts snapshot canonical conversion settings and use DocumentConversionJob/convert_document_job for all eight kinds, including Text/Markdown. Both create Converter with pdf_tools=None; legacy helper/Poppler discovery is removed from production routes. DocumentHelpers::discover uses only the two absolute trusted development overrides or executable siblings. Selected-artifact validation remains in the existing bounded process boundary. No fallback, engine/runtime/parser, root dependency, auth or protected-write policy change was made.

Full/chat/project routes use mode-aware shared save and keyed replay. Replay checks captured settings/bundle before Auth or helper dispatch. Useful handoff validates import_report_with_conversion before consuming the result; CLI also validates it before creating the useful chat. Native Converted carries the captured settings through completion. Existing report details/attachment summary delegate to one small helper displaying stored converter/AI data; historical and mixed None remain unlabelled.

## Bounded scope correction

Source inspection found existing useful-intake completion selecting the Project from current navigation. Four real public-native/helper tests reproduced A→B, Project→general, general→Project and deleted-original-target fallback. The first driver assumed General Source.chat was Some; its unwrap failure is retained as a fixture error. After correcting only that projection,05e is0/4 genuine scope failures. The exact navigation helper/assertion tail is byte-identical in final05f, which passes all8 new native controls.

With explicit approval, convert_file captures the existing canonical Project at request start, Converted carries it, and useful_intake passes it to unchanged source_intake_chat. An unavailable captured Project refuses with its existing exact error; navigation cannot select another destination. No chat/writing lifecycle or general authority mechanism changed.

## Approved synthetic transport seam

`brn_ai::document_refine::convert_document_job_with_http(job: DocumentConversionJob, cancel: CancellationToken, conversion_cancel: Arc<AtomicBool>, emit: Arc<dyn Fn(ConversionProgress) + Send + Sync>, http: DocumentHttpClient) -> Result<ConversionResult, ConversionFailure>` is async and gated by existing scripted-runs. It forwards to the exact private convert_job with fixed TIMEOUT. Three minimal maintained transport/mock type re-exports share that gate. There is no timeout/account/model/budget override.

One nondefault CLI feature alias, scripted-document-tests=["brn-ai/scripted-runs"], gates the injection with cfg(test). Normal CLI/native calls retain transport=None and expose no CLI/environment endpoint switch. The actual in-process CLI-route On test covers both engines, exact selected-helper input delivery, checked protected saves/provenance and no dispatch on mismatched replay. It is not a subprocess/live-provider success proof. Task4 must add this explicit test lane to maintained verification/CI.

## Checks and preserved failures

Final selected targets total191 passed/0 failed/1 existing ignored, without summing repeated runs: CLI90 (including1 synthetic On), app30, core2, native69. Fourteen new tests pass:8 native,5 process document-mode and1 synthetic in-process route. The process corpus exercises8 kinds×2 engines with Off and no credentials. Actual native workers execute both helpers without credentials. Existing CLI contract, agent streams/cancellation, protection/Undo, Project/private intake and native queue/handoff/coverage assertions remain.

Strict all-target Clippy passes ordinary affected packages, the explicit scripted alias and native test-support. The ordinary native-ui executable is also built without test-support. Direct owned Rustfmt, JSON, Markdown links and Git whitespace checks are recorded separately. No existing behavior assertion was weakened. Two old native report literals gain only conversion=None; existing completed-result fixtures use a valid fixed stock/Off label, captured defaults and general None where appropriate. The old credential-refusal test explicitly selects On, then Off for its no-provider positive, preserving its safety assertions.

Initial missing README row/wrong expected CLI class, native target/visibility/default-field errors, new control ElementId error, edit-script setup failures, first scope-driver unwrap and repeated pre-correction run remain recorded. Only05e's four corrected assertions are claimed as the scope behavioral before; compiler/setup errors are not behavior reds.

## Remaining gates

Task4 distribution/relocation, maintained CI lanes, full independent actual-diff review, native-human/owner acceptance and integration are pending. One existing cross-process native fixture needs BRN_DWR_SHARED_* references and remains ignored. Task2's legacy prepared-PDF Unavailable remains carried38/1/3; new normal PDF import success does not close it. Encrypted PDF stays explicitly deferred without new refusal implementation/verification. No frozen provider-screened PDF work, unaccepted XLSX work, old patches/oracles, root manifests/lock or Task1 build/runtime/engine was changed.

No full fidelity/resource/restricted-runtime/license/all-eight-retirement, SDK/widget/platform/INT-W/native-human/owner/program/main/live gate is claimed closed. No paid/live/provider activation, GUI observation, delegation, Task4 implementation, publication or merge was performed.
