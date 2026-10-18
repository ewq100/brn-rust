# Standard-default verification

The [approved plan](implementation-plan.md) keeps two converter modes and independent AI conversion Off by default. [Task1](task1/handoff.md), [Task2](task2/handoff.md) and [Task3](task3/handoff.md) preserve their original source pins, failures and limits. Task4 implements distribution and integrated local gates; its final receipts will distinguish implementation, automated verification, native-human observation, owner acceptance and merge.

Build with `bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root /absolute/task-target`. Stock uses tools/xberg-intake-helper/Cargo.lock with no patch. Adapted uses its committed adapted.Cargo.lock and a generated scoped workspace under ignored target/xberg-adapted; every one of765 package files is checked after applying only0001/0003/0004/0005. Separate literal targets and build-identity files prevent source aliasing. No runtime source generation or download is needed.

Export the two printed artifacts as BRN_XBERG_HELPER and BRN_XBERG_HELPER_BRN for Mac tests. These are trusted development overrides only; production discovers both named siblings. Test both standalone manifests with `cargo test --all-targets --locked --offline --jobs 1` and strict Clippy using separate stock/adapted CARGO_TARGET_DIR values. Default workspace, native state/widgets, capability, scripted CLI and retrieval checks remain separate lanes. The scripted CLI selector is nonzero document_route_tests under the nondefault scripted-document-tests feature; it is synthetic in-process transport, never live/subprocess On success.

| Kind | Stock / optional direct output | Qualification |
|---|---|---|
| PDF | Normal pinned public API / same API | Representative non-encrypted fixture; no bespoke correction, fidelity/resource closure or encrypted-support claim. Encrypted support explicitly deferred. |
| DOCX | Stock / accepted span, namespace and typed comment-reference bundle | Representative API and mode-difference controls; broader annotation, layout and source fidelity remain qualified. |
| PPTX | Stock / accepted declared-order, hidden-state and slide-owned notes/comment association bundle | Representative API/order controls; broader rendering/O2 fidelity remains qualified. |
| XLSX | Normal pinned public API / same API | Normal stock limitations retained; unaccepted stopped XLSX work excluded. |
| EML | Normal pinned public API / same API | Normal stock body/attachment limitations retained; no MIME repair/fallback. |
| HTML | Normal pinned public API / same API | Historical restricted textual component is not silently applied to stock. |
| Text | Normal pinned output / finite valid UTF8 identity projection | Nine exact source vectors across Text/Markdown; no line-ending normalization for equality. |
| Markdown | Normal pinned output / finite valid UTF8 identity projection | Same source vectors; retained asset relocation is checked separately. |

AI Off actual selected-helper tests cover all eight/both modes without credentials; Task2 synthetic On tests cover all eight/both, while Task3 CLI-route On is separately in-process. No result is labelled complete custom fidelity. Non-Mac restricted execution is explicitly unavailable; Mac execution tests cannot self-skip on missing artifacts. Pure DTO/settings checks remain portable, with source-defined non-Mac unavailable controls.

Packaged qualification uses deployment15.0, three signed arm64 Mach-O binaries and system-only linked libraries, relocated ZIP extraction, headless startup/reopen/foreign-data refusal and both helpers on all eight inputs. It does not open GPUI or establish native-human/owner acceptance. Existing legacy PDFUnavailable, unexplained older SDK/widget failures, platform/compatibility/INT-W/main/live and owner/program gates remain separately qualified. Parent owns whole-feature independent review and integration; no publication authority is exercised here.
