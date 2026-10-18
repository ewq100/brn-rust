# B4 Opus1 bounded correction response

Bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`, branch `work/b4-20261017`, OpenAI/Codex gpt-6.1-sol. Correction started clean at principal CI/docs HEAD `50fa7ae3237bbde8dd008501f9eeead681f56943`; reviewed B4 source is `c95b17a8642353641b92e2801fc242e263290bff`, original accepted B3 base remains `c5b257fb643794e0802dd1cb4d3fee5ff3ee0edf`. No branch/worktree recreation, reset, rewrite, delegation or D1B/D2 integration.

[Full original Opus1 report](opus-review-round1.md) is copied verbatim, SHA256 `036f524616b9bb15741440e7d2b5f17840069d05794cc15430d25162f61aac95`. [Actual command/source/artifact receipts](opus1-checks.json) preserve every new build/test/lint failure and result; earlier [c95 checks](checks.json) retain their own source identity and counts.

## Findings and dispositions

| Finding | Bounded correction / actual evidence |
|---|---|
| MEDIUM: missing host action wiring; earlier claim false | Only `WritingHost::render` gains `key_context("WritingHost")` and four action listeners. Save, CommentSelection, Rewrite and AskAboutSelection forward to the same existing `dispatch(WritingRequest, ..)` callback, with AskSelection using its existing explicit unavailable notice. No runtime/provider/preparer/authority/library change. The new keyboard fixture sends real focused-editor `cmd-/`, `cmd-s`, `cmd-shift-enter`, checks exact saved passage anchor, actual saved buffer and real shared scripted run/candidate. |
| LOW: test harness absent from all-target lint | Only `test = true` added to the existing required-native-ui example entry. Ordinary native writing tests actually execute library44 + host5. Strict all-target native Clippy's separate example `profile.test=true` artifact/dep-info includes `journey.rs`; normal binary artifact does not. Conditional removal of only redundant principal CI/script example test lines followed that observed coverage. |
| Apply refresh gap | Original real-service journey now checks `note_editor.value() == REVISED` immediately after Apply and before Undo. Every previous assertion/event remains exact. |
| First-edit proof precision | The existing typing fixture checks after parking. Code places `note_changed` before derived reads/notify, but this fixture does not prove native event ordering before render. No stronger claim. |
| GUI quit cleanup gap | Fixture drop/reopen/TempDir cleanup remains tested. Actual macOS GUI quit cleanup/destructor behavior remains unqualified; no GUI launched. |
| Preferences | No change to the bounded latest-run callback glue or redundant fresh candidate check; no material redesign. |

The second new native fixture focuses/selects the real shared editor and dispatches the exact `CommentSelection`/`AskAboutSelection` actions through `Window::dispatch_action`, the path used after native menu selection. It asserts comment anchor/open state and an explicit unavailable notice with no run/candidate/comment writes. It never calls the host action method directly. **This proves focused-editor action bubbling and handlers, not an actual native menu-item click or menu construction.** GPUI Component 0.7.3 `native_menu/macos.rs::show` returns if `ns_view_ptr(window)` is absent; the headless test platform supplies no NSView. A real menu click would require the prohibited GUI lane or a separately allocated toolkit test seam. The unchanged shared menu source emits these exact actions (`writing.rs`); no desktop or toolkit bytes were edited.

## Genuine before and retained fixture defect

Before editing host production, the test-only patch was recorded against exact 50fa production (same crates tree as c95) and applied in synthetic Git at fixed `/private/tmp/brn-b4-opus1-before-20261017-source`, target `/private/tmp/brn-b4-opus1-before-20261017-target`. No remote/credentials/history were copied. Candidate target remains `/private/tmp/brn-b4-20261017-target` for its same literal bound source root; no cross-root fingerprints or artifact reuse.

First fixture compilation failed because `comments_open` is a WritingView method, not a WritingPanel method. This is retained as a fixture defect, not behavioral red. The corrected fixture uses existing `comments_expanded` for the fixed Stacked fixture. The exact full test filter includes `journey::`; the initial unexecuted filter was also corrected. On unchanged reviewed host, the repaired fixture compiled and executed one test, then failed at `cmd-/ from the editor must open the saved passage comment anchor` (0 passed /1 failed /4 filtered). Candidate passes the same unchanged repaired fixture in the ordinary native suite. Both patches, source maps, log hashes and distinct actual binaries are retained in scratch and receipts. No fabricated missing-example/API red or mutation claim.

## Actual correction-window checks

Rust1.98.1, locked/offline, CARGO_INCREMENTAL=0, fresh synthetic TMPDIR per command. Initial checkout/process inventory was clean/empty of heavy work. The current task target is reused only for the same literal candidate source root. Native helper/Poppler environment identities remain in command receipts; missing native pdftoppm is not wrapped or hidden.

- Ordinary native writing library/example tests: **44 +5 passed**, 0 failed/ignored; includes real-service comment/Rewrite/Review/Apply/Undo, immediate editor Apply refresh, shortcuts/actions, dirty/access-loss and cold reopen fixtures.
- Strict all-target native writing and desktop Clippy: passed with `-D warnings`; actual example test-harness association retained.
- Native-only example build **without test support**: passed; new binary and dep-info retained, not launched.
- Full native desktop regression: **287 passed /0 failed /1 existing ignored**; original A/B/CW2/F15/B3/queue/typing/Undo assertions untouched.
- Default workspace normal and normal+feature trees: zero GPUI. Strict default all-target workspace Clippy, workspace fmt, bash syntax and whitespace checks passed.
- Local Markdown links are checked after final evidence edits. Maintained native commands were executed individually, not as a claimed full script run.

13 command receipts before the final link audit include the initial fixture compiler failure and genuine behavioral before failure; no rerun-to-green or deleted failure. The first direct formatter attempt used a nonexistent Homebrew path and did not run; the actual rustup formatter resolved through PATH and pinned 1.98.1 succeeded. Existing `block 0.1.6` future-incompatibility advisory remains in raw native logs, with strict crate lint passing.

The native writing test execution preceded only the conditional removal of the redundant CI/script lines. Its Rust/manifest/lock inputs exactly match final source; every later gate records the final material map. There is no fabricated future SHA association. Final material: 332 inputs, fingerprint `3d784297cafb1ad752f73b0ae5e32538974dfce0e8c66f2e0bd20759a3bc9397`; exactly five material paths differ from 50fa, remaining327 exact. Cargo.lock is byte-identical, SHA256 `0798be049da224edbfb4ce951a80fcd20b066fba6f9632c164b111d5e30de290`; no package/edge changes.

## Ownership, acceptance and limits

This correction owns only host.rs render action wiring, journey.rs assertions/fixtures, Cargo.toml test flag, the exact redundant script/CI line removal and B4 evidence. All other B4 sources, existing shared library/desktop/app/core/AI/CLI production, manifests/dependencies/lock and pre-B4 tests remain unchanged. The original journey prefix is byte-exact after removing only the new immediate Apply assertion. No unchanged guard mutation/stress campaign was run; B3 budget is closed.

F18 same-panel/real Workspace/shared scripted service remains demonstrated; action wiring is now executed locally. F22 defaults remain GPUI-free. Original c95 full default suite **1115 passed /1 carried PDF Unavailable /6 ignored** is retained as earlier-source evidence, not rerun or relabelled at this pin. No unchanged combined/retrieval/E2E/full default suite repetition is needed for four host listeners and one example test flag. INT-W, hosted, native-human/IME/picker/menu-click/GUI-quit, live provider/converter, full F21, PDF tooling, owner/program and principal acceptance remain open. Fresh principal-owned Opus2 follows the immutable corrected source pin; this worker does not self-accept or integrate.
