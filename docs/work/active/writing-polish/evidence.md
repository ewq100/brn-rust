# Writing polish evidence

**14 October 2026. Baseline:** `2213eb4f5c67e2b2f53287ee60f8cf2bbbef46ce`, fetched `origin/main`; isolated `codex/writing-polish` checkout. No schema, dependency, lockfile, provider or runtime-guide changes. The original retrieval checkout and research worktree are preserved.

## Implemented

Document and passage feedback occupy the Note tile; full chat remains a separate resizable tile. The Write preset gives the document more space. Feedback has quieter cards, readable labels and passage order. Read renders headings, emphasis, lists, quotations, tables and retained figures from the same Markdown buffer; Outline keeps navigation compact. Write has exact-selection Bold, Italic, Heading, List and Quote controls. Unsupported complex inline syntax/code is conservatively left unchanged.

Rendered selections use the toolkit's exact source mapping, validate parser/source equality and are cleared when the source changes. Read/Write preserves the checked human buffer and existing navigation drafts. Save and comment commits the human edit before opening the existing composer; it does not post automatically. Review offers a formatted Read result and moves focus to the actually displayed candidate/diff view. Apply, protected authority, replay, captured feedback/context and Undo retain the existing services.

This is formatted reading plus Markdown editing with formatting controls. It does not claim editable WYSIWYG, pixel-aligned margin cards, perfect semantic anchors or durable comment drafts beyond the existing window-local behavior.

## Initial candidate verification (`640911442f`)

Pinned Rust 1.98.1 and GPUI Kit 0.7.3; compatible shared target cache `/Users/evokessler/repos/brn-rust/target`.

- `cargo fmt --all -- --check`: passed.
- `CARGO_TARGET_DIR=/Users/evokessler/repos/brn-rust/target scripts/verify-desktop-shell.sh`: passed on final source. Builds the maintained intake helper and native UI, runs strict native Clippy with all targets, then **123 native tests** (123 passed, 0 failed/ignored). Local log: `/Users/evokessler/.codex/writing-polish-data/native-verification-final.log`.
- Fifteen pure formatting tests cover UTF-8/Unicode, repeated occurrences, whitespace outside emphasis, block/caret mapping, toggling, CRLF/blank lines, invalid bounds and ordered/nested fenced code refusal.
- Six native document tests cover same-buffer Read/Write navigation, exact original-source selection, source-change invalidation, explicit Save/composer continuation, stale Save preservation and blocked formatting. Existing native review/Apply/addressed/Undo journey now verifies formatted-result focus and unchanged candidate identity.
- `python3 scripts/check-markdown-links.py`: 63 files, 275 local links, 0 failures. `git diff --check`: passed.

Preserved failed evidence: the initial compile needed explicit color conversion removal and cloned focus handles. Strict lint caught a reversed-range test literal and was corrected without suppression. An older dirty-comment test expected a disabled control; it was updated to exercise explicit Save and comment while retaining the raw-shortcut/menu refusal checks. The toolkit does not expose the disabled accessibility flag on every button, so the new recovery test clicks the control and verifies no mutation. All resulting gates passed.

## Independent review

GPT-6.1 Sol performed read-only whole-diff review; Luna performed plan review and bounded native test work. The lead verified and resolved findings:

1. Toolkit append updates can retain a rendered selection: clear the actual TextView selection before replacing source, with an append regression.
2. Review of another note could show the open note's comments: display its margin only when identities match.
3. Read-result toggle could leave keyboard focus on the hidden diff: focus the visible candidate/diff, with a native regression.
4. Inline emphasis could include edge whitespace and render literal markers: preserve outer whitespace outside markers; test Unicode whitespace.
5. Ordered-list fences escaped conservative code refusal: recognize supported ordered/nested containers and test them.

Final corrected source review: **no remaining actionable findings**. A tentative tab-list fence finding was withdrawn after checking valid CommonMark continuation indentation; valid witnesses were retained as tests, without the speculative production change.

## Native observation

Used a uniquely named local `BRN Writing Polish.app` and fresh synthetic data outside Git. Existing BRN installations and owner data were not opened. No provider call was made.

- Native Read displayed rendered headings, inline bold, lists, quote and table; chat was separate and the comments remained adjacent to the document.
- Select-all in formatted Read produced a supported exact-source comment; Add saved feedback without starting AI. Native Write keyboard selection and Bold preserved the words and made a recoverable unsaved edit.
- Switching that dirty note to Read, selecting, and Save and comment advanced the saved note to v2 and opened the composer on the captured source. Posting remained a separate action.
- A synthetic fixture created through the shared application service supplied three distinct passage comments and two posted chat messages, plus a scripted complete candidate (not model output). Native Show in note → Read result → Apply whole note succeeded, then Mark 3 comments addressed and receipt Undo restored the original note at v3 and returned feedback to pending. Unlocated anchors remain honestly visible; no reattachment was guessed.

The fixture source is preserved locally at `/Users/evokessler/.codex/writing-polish-data/writing-polish-native-fixture.rs`; it was run as a temporary workspace example using the repository lockfile, then removed from the checkout. Observation data is preserved in task-owned folders for the save flow and review/Undo flow. The final owner-preview workspace is separate, with a fresh pending candidate. A transient screen-capture failure was resolved by reacquiring the synthetic app handle.

These native checks establish the scoped UI interactions, not provider rewrite quality, broad IME/daily-use qualification or owner acceptance. Narrow-window behavior retains the existing horizontally scrollable resizable tiles and existing passing narrow-workbench tests; no new narrow-Mac native acceptance is claimed.

## Delivery gates

| Gate | State |
|---|---|
| Owner-selected layout/workflow | Accepted in this chat; implementation UX acceptance pending |
| Defined/build ready | Complete; independent plan review passed |
| Implemented | Complete locally |
| Local checks | Integrated native build, strict lint, 141 native tests, 14 core/app writing tests, 18 AI rewrite checks, format/docs pass |
| Independent code review | Passed after findings resolved |
| Native observation | Scoped synthetic reading/comments/formatting/Save/review/Apply/Undo observed |
| Hosted CI | Initial `640911442f` required checks passed; refreshed integrated-head checks pending |
| Owner acceptance | Pending review of the concrete native candidate |
| Merge/release | Unassigned; no merge or release claimed |

## Owner correction: all writing interactions stay in the Note tile

The owner rejected the preview's writing proposal/continuation cards in chat and asked for simpler comments. The revised candidate puts passage comments, overall feedback, generation progress/Stop, saved retry, proposal review, Apply, addressed confirmation and Undo in the Note tile. Exact-run explanations/questions with no pending proposal also appear beside overall feedback, so answering them needs no chat Send. Ordinary chat attention and queue continuation remain available. Writing request/addressed bookkeeping rows are filtered out of chat.

Overall feedback is captured atomically in the same immutable bounded request. Empty-feedback envelopes and replay bindings remain compatible; nonempty feedback binds operation intent and is available to the agent through the existing serialized snapshot. No schema or authority change. Unsent overall drafts are per-note/window-local; saved requests preserve captured feedback across restart.

Comments now lead with the feedback, a muted clickable passage quote and one inclusion toggle. Ordinary saved-version/inclusion explanation banners and full card borders are removed. Unlocated, excluded and addressed states remain truthful. Feedback inputs suppress code line numbers and automatic bracket insertion.

Independent Sol review found and the lead fixed two defects: blanket routing could hide unrelated record-less chat attention; opening review during an active rewrite could hide Stop. Regression checks cover both. A supposed multiple-proposal overflow reproduction was disproved by the core's existing one-active-candidate guard and withdrawn; the footer is bounded, and repeated requests preserve the pending exact review.

Synthetic final-build native observation with Chat closed: enter overall feedback → Rewrite saves inputs while offline startup refuses inference → review a separately scripted exact candidate → Read result → Apply whole note → confirm captured comments → Undo restores the original v3 and pending comments. The feedback draft remains visible. No provider call; scripted text proves the UI/checked lifecycle, not generation quality. A fresh owner preview preserves its pending candidate; image at `/Users/evokessler/.codex/writing-polish-data/writing-tile-preview.png`.

Failed verification attempts remain in task-owned logs: unsupported Div scrolling lacked a stateful ID; obsolete tests still sought retired chat-card controls; the first accumulated-candidate fixture correctly hit the core guard. A parallel full run also missed two pre-existing dialog controls, while 128 other tests passed; serial execution passed all 129 on that same source. The final expanded checks below supersede these attempts.

Revised hosted checks and owner UX acceptance are pending; merge/release remain unassigned.

## UI polish integration qualification

Integrated [#136](https://github.com/ewq100/brn-rust/pull/136), `c0188a3f7daa66885b32427c14323517aaf0a189`, by normal merge. Retained its reusable workspace areas, shared system-font sizing, quiet controls and navigation contract. Reconciled conflict hunks for comments/Save-and-comment/review and their tests without retiring either flow. Overall-feedback navigation preserves the independent selected chat and its draft. Explicit opening of a writing review retains the merged contract of selecting its intended conversation; this does not open a closed Chat panel.

Final integrated source checks: `CARGO_TARGET_DIR=/Users/evokessler/repos/brn-rust/target RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` passes native build, strict all-target Clippy and **141/141 native tests**. Log: `/Users/evokessler/.codex/writing-polish-data/native-verification-integrated-final.log`. Core/app writing **14/14** and AI rewrite **18/18** pass; fmt/diff and docs **65 files / 280 local links / 0 failures** pass. Two obsolete navigation/review test assumptions were updated to the merged contract and Note-owned proposal controls.

Additional reviewed fixes: an old Apply cannot hide the latest rewrite response/Retry; prior Undo remains reachable and current Apply offers Rewrite again. Responses and captured-input figures resolve against their own bound thread, independently of optional Chat selection. The seven new native journeys include exact terminal-run response association, R1 Apply→R2 clarification, old Undo/new Retry, per-note drafts, closed Chat and Stop during review. Independent final/integration review found no remaining actionable defects. An attempted review-thread-preservation finding was withdrawn against #136's explicit accepted review-selection contract.

Native-observed on this integrated build, using fresh `native-integrated-writing-20261014` synthetic data: Chat closed, overall feedback entry, Rewrite persisted before offline refusal, scripted proposal review, Read result, Apply, individual captured-comment addressed confirmation and prior receipt Undo restored the original at v3 and all three pending comments. This also exercised a newer saved request beside an older applied candidate. No provider call. Fresh pending owner-preview data is kept separately. Broader native IME/daily-use and owner UX acceptance remain pending; writing PR merge/release are unassigned.
