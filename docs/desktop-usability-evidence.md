# Native usability trial evidence

2026-09-28 · `trial/end-to-end-flow` · baseline `7380b63`.

Scope: [focused usability plan](desktop-usability-plan.md). This continues the [successful synthetic native flow](end-to-end-flow-evidence.md#native-acceptance-after-unlock-2026-09-28), without merging or releasing it.

## Baseline and review

The requested checkout was clean on the requested commit and branch. Roadmap, status and end-to-end evidence were read before design. Astra reviewed the architecture; Sol owns implementation and corrections; Luna performs narrow checks. The controller handles native inspection and final integration. Baseline `cargo test --workspace --locked --offline` passed on macOS Apple Silicon with the pinned Rust toolchain.

## Verification record

Final credential-free verification passed:

- `bash scripts/verify-end-to-end.sh`: format, locked/offline workspace build, Clippy with warnings denied, all 65 workspace tests and import/reimport/build/search/change/rebuild smoke checks. Repeated on the final source after the native layout correction.
- `cargo +1.98.1 test --manifest-path experiments/codex-app-server/Cargo.toml --locked --offline`: 16 existing provider-harness tests passed; no live call in this command.
- With `CARGO_TARGET_DIR=/private/tmp/brn-editor-trial/target`, `CARGO_BUILD_JOBS=4` and the pinned toolchain, native `brn-desktop` build, test and all-target Clippy with `--features native-retrieval --locked --offline` passed. Three native unit tests and five CLI tests passed. The known compact-unwind linker warning and upstream future-incompatibility notice remain.
- `bash scripts/test-make-macos-app.sh`, shell syntax checks and `git diff --check` passed. The launcher test checks valid plist, literal argument preservation for spaces/Unicode/quotes/backticks/command-substitution text, invalid input paths and a dependency disappearing before launch. Its log and injection sentinel are confined to scratch.

Astra requested three corrections: compact saved-question buttons, full diagnostics inside scrolling content instead of an unbounded header, and no live-progress label after a terminal failure. These were implemented and scoped re-review passed. Native inspection then found a shrinking question editor; its 110px height now remains fixed inside the scrolling body. Final native tests/Clippy and Astra scoped review passed after that correction. No remaining Critical, Important or Minor review findings.

## Native observations

Supported Computer Use exercised the generated app on macOS Apple Silicon, initially at 1100×800 content size and then at its 800×600 minimum. The disposable source was a 216-byte Markdown fixture with spaces, Unicode and a deliberately long filename.

1. **Choose file…** displayed the native picker. Selecting the Markdown file showed its path while Sources remained zero. Opening and cancelling a second picker preserved that selection. **Import and approve** then produced one approved source.
2. Searching before building displayed `no active index; build it first`. Building showed immediate action status, elapsed time and embedding progress. Conflicting work controls were visibly disabled while navigation and Cancel remained available.
3. At the minimum size, long filenames and metadata wrapped, the page scrolled, navigation/status/Cancel stayed visible, and all question/search controls and evidence remained reachable. The editor initially shrank, was corrected, rebuilt and visually rechecked at its full 110px height.
4. Hybrid search returned one compact passage button. Opening it displayed the full quote with source/revision and bytes `0..216`. A live bounded question through existing Codex sign-in completed with Tuesday and BLUE-HERON, citation `[1]`; Workspace displayed the saved answer and a visible route to its conversation/evidence.
5. Closing the window exited the process. Relaunching the same app retained the session and answer. A multiline semantic follow-up completed in that same session with Niko as the botanist and citation `[1]`. Activity compacted its multiline question label while preserving the complete question in details.
6. Choosing a CSV succeeded as file selection, but explicit import displayed `select a .md or .txt file`; the approved source count remained one. The installed GPUI picker has no extension filter, so validation remains authoritative at import.
7. A longer synthetic answer ran while Activity was visible; elapsed status and Cancel remained available there. Cancel returned to ready and saved a third, Interrupted turn with partial text. Two completed and one interrupted turns remained visible.

One attempt sent keyboard commands before the native dialog state was observed and left selection pending. Closing/reopening and waiting for the actual dialog before entering a path resolved it; the proper interaction sequence succeeded. No reproducible application defect was established by that automation timing attempt. Native inference interruption, missing-model/provider UI variants and screen-reader qualification were not newly exercised. Existing active-provider-close acceptance is retained from the baseline; shutdown implementation is unchanged.

## Repeatable local launch

`scripts/make-macos-app.sh` creates an unsigned local app with a copied BRN executable and safely preserved explicit dependency/workspace paths. Startup failures open `~/Library/Logs/BRN Usability Trial/startup.log`; the script does not download or bundle Codex or model assets. It refuses an existing output app. Build a new output after source changes.

A durable local instance was created at `~/Applications/BRN Usability Trial.app`. After closing the disposable app, its synthetic workspace and verified model files were copied to `~/Library/Application Support/BRN Usability Trial/{data,model}`. The durable launcher uses those paths and the existing `/Applications/ChatGPT.app/Contents/Resources/codex`. Its Settings page confirmed the persistent paths; Activity reopened the two completed and one interrupted synthetic turns. No new credential material was read or copied. These machine-local artifacts are outside Git and are not a distributable release.

## Boundaries

All new native checks use disposable synthetic documents in an explicit trial workspace. Original documents, the earlier native acceptance workspace, and provider credentials are preserved. The launcher remains local and unsigned; model assets and Codex remain external. Clean-machine packaging, signing, accessibility qualification, natural credential expiry and representative-corpus ranking are not covered.
