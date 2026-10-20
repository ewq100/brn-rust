# Bounded note formatting, read mode, and passage reveal audit

## Source and scope

- Repository: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`
- Branch / HEAD: `codex/agent-first-followthrough` / `e1de1396c054f765723781acf0350ae04f45fc84`.
- Product-source identity: the inspected checkout was HEAD `e1de1396c054f765723781acf0350ae04f45fc84`; the claim that its product tree matches reviewed worker `28474ab705dceb718dedbdb96a027f2c5c36c371` and current main `d2e0e298` was supplied for this audit and was not independently recomputed. `git status --short` showed changes in `docs/work/active/vacation-campaign/campaign.json` and `docs/work/overview.md`; I read the selected campaign task record for its scope and authority, did not read the overview, and changed neither file.
- Read-only inspection only. No tests, builds, app launch, account action, network, or Git mutation were performed.

## Result

No concrete stale-target, silently lost formatting edit, read-only mutation, or wrong passage reveal risk was established in this bounded static pass.

The UI dispatches formatting through `WritingRequest::Format` to `ThreadsUi::format_note` (`crates/brn-desktop/src/threads_ui.rs:4498-4500`). That callback rejects an invisible editor, Read mode, and active review before calling the formatter (`:3753-3764`). `format_selection` computes a byte-range edit against the current editor value, then replaces that range and restores the resulting selection (`crates/brn-writing-ui/src/document.rs:411-432`); the formatter validates bounds and UTF-8 boundaries and refuses unsupported or unsafe Markdown (`crates/brn-writing-ui/src/formatting.rs:22-40`). After replacement, `format_note` routes the changed text through `ThreadsUi::note_changed` (`threads_ui.rs:3765`), whose shared handler reports a concurrent edit conflict or preservation failure to the user (`:1362-1382`). Thus the formatting callback does not silently swallow the persistence error.

Read-mode switching calls the checked lifecycle path; it attempts to preserve the visible buffer before changing presentation, synchronizes the derived read text, and clears selection (`crates/brn-writing-ui/src/note.rs:214-235`). Formatting controls are omitted in Read mode (`crates/brn-writing-ui/src/document.rs:111-160`), while the host callback also protects against a direct/programmatic invocation (`threads_ui.rs:3753-3758`). These guards support the read-only behavior statically.

Passage reveal is wired from the comment control and outline through `WritingRequest::Reveal` (`crates/brn-writing-ui/src/writing.rs:1388-1400`, `document.rs:189-199`, and `crates/brn-desktop/src/threads_ui.rs:4494`). In Read mode, the host maps the source range through the rendered document and reveals that rendered range; if mapping fails, it switches to Write mode and selects the original source range (`threads_ui.rs:1335-1360`). Comment reveal controls are disabled for dirty text and active review (`writing.rs:1375-1399`); unlocated passages render as unavailable instead of being sent to reveal. This pass did not find a concrete route that supplies a range for a different source version.

## Existing meaningful coverage

- `crates/brn-desktop/src/threads_document_tests.rs:48-115` covers switching with a dirty buffer, preserving that recoverable buffer across note navigation, and deriving Read text from it.
- `threads_document_tests.rs:186-277` covers clearing a selection after source changes and preventing a stale comment shortcut from using it; `:117-183` covers exact Markdown-source selection/comment anchoring in Read mode.
- `threads_document_tests.rs:531-649` covers no formatting mutation in Read mode, recovery-blocked state, and active review, including a click on the blocked formatting control.
- `crates/brn-desktop/src/threads_review_tests.rs:1366-1377` exercises Show passage and asserts the selected quote; `:1402-1452` checks that a passage removed by an edit remains unlocated and is reported as such.
- Formatter unit tests in `crates/brn-writing-ui/src/formatting.rs` cover UTF-8/repeated text (`:303-311`), unsafe syntax and refusal (`:327-350`), repeated quoted passages (`:512-519`), and selection preservation around inline formatting (`:521-541`).

## Limits

This is source/caller inspection, not execution evidence. No test result or native visual observation is claimed. I did not inspect the full Markdown renderer implementation or broader comment remapping service; this report only assesses the named UI callback paths and directly relevant tests. The supplied source-equivalence identity was not independently recomputed.

## Lead disposition

Lead independently inspected the named formatting/read/reveal handlers and their caller guards, including `WritingPanel::note_editor_visible` → `note_input_active` plus `recovery_blocked`. A Git tree comparison of all `crates`, `tools`, root Cargo/toolchain and `agent` paths against frozen28474 passed with no differences. No concrete defect or additional mirror test selected. This is bounded static evidence only; no test, native, renderer-wide or owner-acceptance pass is implied.
